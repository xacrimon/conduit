use std::ffi::{CStr, c_char, c_int, c_void};
use std::os::fd::AsRawFd;
use std::os::unix::io::RawFd;
use std::pin::Pin;
use std::{marker, mem, ptr};

use libssh_rs_sys as libssh;
use tokio::io::unix::AsyncFd;
use tokio::io::{self, Interest};

use crate::libssh::channel::ChannelState;
use crate::libssh::error;

pub struct Session {
    handle: AsyncFd<Pin<Box<Handle>>>,
}

impl Session {
    pub(super) fn new(session: libssh::ssh_session) -> Self {
        let handle = Handle::new(session).unwrap();

        Self {
            handle: AsyncFd::new(handle).unwrap(),
        }
    }

    pub fn configure(&mut self) {
        let handle = self.handle.get_mut();

        unsafe {
            libssh::ssh_set_auth_methods(handle.session, libssh::SSH_AUTH_METHOD_PUBLICKEY as i32);
        }
    }

    pub async fn handle_key_exchange(&mut self) -> io::Result<()> {
        loop {
            let mut guard = self
                .handle
                .ready(Interest::READABLE | Interest::WRITABLE)
                .await
                .unwrap();

            let handle = guard.get_inner();

            match unsafe { libssh::ssh_handle_key_exchange(handle.session) } {
                error::SSH_OK => break,
                error::SSH_AGAIN => guard.clear_ready(),
                error::SSH_ERROR => return Err(error::libssh(handle.session as _)),
                _ => unreachable!(),
            }
        }

        let handle = self.handle.get_ref();

        unsafe {
            let rc = libssh::ssh_event_add_session(handle.ssh_event, handle.session);
            if rc != error::SSH_OK {
                return Err(error::libssh(handle.session as _));
            }
        }

        Ok(())
    }

    pub async fn authenticate(&mut self) -> io::Result<()> {
        // TODO: handle authentication in a blocking task
        Ok(())
    }

    pub async fn wait(&mut self) -> io::Result<()> {
        loop {
            let mut guard = self
                .handle
                .ready_mut(Interest::READABLE | Interest::WRITABLE)
                .await
                .unwrap();

            if !matches!(guard.ready(), r if r.is_readable() || r.is_writable()) {
                continue;
            }

            let handle = guard.get_inner_mut();

            match handle.as_mut().process_events() {
                Ok(()) => (),
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    // TODO: is this correct?
                    // https://github.com/libssh/libssh-mirror/blob/ac6d2fad4a8bf07277127736367e90387646363f/src/socket.c#L294
                    guard.clear_ready();
                    break Ok(());
                }
                Err(e) => break Err(e),
            }
        }
    }

    pub fn channel_state(&mut self) -> Option<Pin<&mut ChannelState>> {
        let handle = self.handle.get_mut().as_mut();
        handle.channel().as_mut().map(|c| c.as_mut())
    }
}

// TODO: needs https://doc.rust-lang.org/std/pin/struct.UnsafePinned.html
struct Handle {
    session: libssh::ssh_session,
    ssh_event: libssh::ssh_event,
    callbacks: libssh::ssh_server_callbacks_struct,
    channel: Option<Pin<Box<ChannelState>>>,
    _pinned: marker::PhantomPinned,
}

impl Handle {
    fn new(session: libssh::ssh_session) -> io::Result<Pin<Box<Self>>> {
        let ssh_event = unsafe { libssh::ssh_event_new() };

        let callbacks = libssh::ssh_server_callbacks_struct {
            size: mem::size_of::<libssh::ssh_server_callbacks_struct>(),
            userdata: ptr::null_mut(),
            auth_password_function: None,
            auth_none_function: None,
            auth_gssapi_mic_function: None,
            auth_pubkey_function: Some(Self::callback_auth_pubkey),
            service_request_function: Some(Self::callback_service_request_function),
            channel_open_request_session_function: Some(
                Self::callback_channel_open_request_session,
            ),
            gssapi_select_oid_function: None,
            gssapi_accept_sec_ctx_function: None,
            gssapi_verify_mic_function: None,
        };

        let mut handle = Box::pin(Self {
            session,
            ssh_event,
            callbacks,
            channel: None,
            _pinned: marker::PhantomPinned,
        });

        unsafe {
            handle.as_mut().get_unchecked_mut().callbacks.userdata = &*handle as *const _ as _;
        }

        unsafe {
            libssh::ssh_set_server_callbacks(session, &handle.callbacks as *const _ as _);
        }

        Ok(handle)
    }

    fn process_events(self: Pin<&mut Self>) -> io::Result<()> {
        let rc = unsafe { libssh::ssh_event_dopoll(self.ssh_event, 0) };

        match rc {
            error::SSH_OK => Ok(()),
            error::SSH_AGAIN => Err(io::ErrorKind::WouldBlock.into()),
            error::SSH_ERROR => Err(error::libssh(self.session as _)),
            _ => unreachable!(),
        }
    }

    fn channel(self: Pin<&mut Self>) -> &mut Option<Pin<Box<ChannelState>>> {
        unsafe { &mut self.get_unchecked_mut().channel }
    }

    unsafe extern "C" fn callback_auth_pubkey(
        _ssh_session: libssh::ssh_session,
        _username: *const c_char,
        _pubkey: libssh::ssh_key,
        _signature_state: c_char,
        _userdata: *mut c_void,
    ) -> c_int {
        libssh::ssh_auth_e_SSH_AUTH_SUCCESS
    }

    unsafe extern "C" fn callback_service_request_function(
        _ssh_session: libssh::ssh_session,
        service: *const c_char,
        _userdata: *mut c_void,
    ) -> c_int {
        let maybe_service = unsafe { CStr::from_ptr(service).to_str() };
        let Ok(service) = maybe_service else {
            return -1;
        };

        match service {
            "ssh-userauth" => 0,
            _ => -1,
        }
    }

    unsafe extern "C" fn callback_channel_open_request_session(
        _ssh_session: libssh::ssh_session,
        userdata: *mut c_void,
    ) -> libssh::ssh_channel {
        let handle_ptr = userdata as *mut Handle;
        let handle = unsafe { Pin::new_unchecked(&mut *handle_ptr) };

        if handle.channel.is_some() {
            return ptr::null_mut();
        }

        let channel = unsafe { libssh::ssh_channel_new(handle.session) };
        *handle.channel() = Some(ChannelState::new(channel));
        channel
    }
}

impl AsRawFd for Pin<Box<Handle>> {
    fn as_raw_fd(&self) -> RawFd {
        unsafe { libssh::ssh_get_fd(self.session) }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            drop(self.channel.take());
            libssh::ssh_event_free(self.ssh_event);
            libssh::ssh_disconnect(self.session);
            libssh::ssh_free(self.session);
        }
    }
}

unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}
