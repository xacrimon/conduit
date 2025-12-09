use std::collections::VecDeque;
use std::ffi::{c_char, c_int, c_void};
use std::os::fd::AsRawFd;
use std::os::unix::io::RawFd;
use std::pin::Pin;
use std::{marker, mem, ptr};

use libssh_rs_sys as libssh;
use tokio::io::unix::AsyncFd;
use tokio::io::{self, Interest, Ready};

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
            libssh::ssh_set_auth_methods(handle.session, libssh::SSH_AUTH_METHOD_NONE as i32);
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
                error::SSH_OK => break Ok(()),
                error::SSH_AGAIN => guard.clear_ready(),
                error::SSH_ERROR => break Err(error::libssh(handle.session as _)),
                _ => unreachable!(),
            }
        }
    }

    pub async fn wait(&mut self) -> io::Result<()> {
        loop {
            let poll_flags =
                unsafe { libssh::ssh_get_poll_flags(self.handle.get_ref().session) as u32 };

            let interests = match poll_flags {
                libssh::SSH_READ_PENDING => Interest::READABLE,
                libssh::SSH_WRITE_PENDING => Interest::WRITABLE,
                f if f == libssh::SSH_READ_PENDING | libssh::SSH_WRITE_PENDING => {
                    Interest::READABLE | Interest::WRITABLE
                }
                _ => unreachable!(),
            };

            let mut guard = self.handle.ready_mut(interests).await.unwrap();

            let handle = guard.get_inner_mut();

            match handle.as_mut().poll() {
                Ok(()) => {
                    debug_assert!(!handle.events.is_empty());
                    break Ok(());
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    guard.clear_ready_matching(Ready::READABLE);
                }
                Err(e) => break Err(e),
            }
        }
    }
}

// TODO: needs https://doc.rust-lang.org/std/pin/struct.UnsafePinned.html
struct Handle {
    session: libssh::ssh_session,
    ssh_event: libssh::ssh_event,
    callbacks: libssh::ssh_server_callbacks_struct,
    events: VecDeque<SessionEvent>,
    _pinned: marker::PhantomPinned,
}

impl Handle {
    fn new(session: libssh::ssh_session) -> io::Result<Pin<Box<Self>>> {
        let ssh_event = unsafe {
            let ssh_event = libssh::ssh_event_new();
            let rc = libssh::ssh_event_add_session(ssh_event, session);
            if rc != error::SSH_OK {
                libssh::ssh_event_free(ssh_event);
                return Err(error::libssh(session as _));
            }

            ssh_event
        };

        let callbacks = libssh::ssh_server_callbacks_struct {
            size: mem::size_of::<libssh::ssh_server_callbacks_struct>(),
            userdata: ptr::null_mut(),
            auth_password_function: None,
            auth_none_function: Some(Self::callback_auth_none),
            auth_gssapi_mic_function: None,
            auth_pubkey_function: None,
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
            events: VecDeque::new(),
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

    fn poll(self: Pin<&mut Self>) -> io::Result<()> {
        let rc = unsafe { libssh::ssh_event_dopoll(self.ssh_event, 0) };

        match rc {
            error::SSH_OK => Ok(()),
            error::SSH_AGAIN => Err(io::Error::from(io::ErrorKind::WouldBlock)),
            error::SSH_ERROR => Err(error::libssh(self.session as _)),
            _ => unreachable!(),
        }
    }

    unsafe extern "C" fn callback_auth_none(
        ssh_session: libssh::ssh_session,
        username: *const c_char,
        userdata: *mut c_void,
    ) -> c_int {
        libssh::ssh_auth_e_SSH_AUTH_SUCCESS
    }

    unsafe extern "C" fn callback_service_request_function(
        ssh_session: libssh::ssh_session,
        service: *const c_char,
        userdata: *mut c_void,
    ) -> c_int {
        -1
    }

    unsafe extern "C" fn callback_channel_open_request_session(
        ssh_session: libssh::ssh_session,
        userdata: *mut c_void,
    ) -> libssh::ssh_channel {
        ptr::null_mut()
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
            libssh::ssh_event_free(self.ssh_event);
            libssh::ssh_disconnect(self.session);
            libssh::ssh_free(self.session);
        }
    }
}

unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}

enum SessionEvent {}

// TODO: needs https://doc.rust-lang.org/std/pin/struct.UnsafePinned.html
struct Channel {
    channel: libssh::ssh_channel,
    callbacks: libssh::ssh_channel_callbacks_struct,
    events: VecDeque<SessionEvent>,
    _pinned: marker::PhantomPinned,
}

impl Channel {
    fn new(channel: libssh::ssh_channel) -> Pin<Box<Self>> {
        let callbacks = libssh::ssh_channel_callbacks_struct {
            size: mem::size_of::<libssh::ssh_channel_callbacks_struct>(),
            userdata: ptr::null_mut(),
            channel_data_function: None,
            channel_eof_function: None,
            channel_close_function: None,
            channel_signal_function: None,
            channel_exit_status_function: None,
            channel_pty_request_function: None,
            channel_shell_request_function: None,
            channel_exit_signal_function: None,
            channel_auth_agent_req_function: None,
            channel_x11_req_function: None,
            channel_pty_window_change_function: None,
            channel_exec_request_function: None,
            channel_env_request_function: None,
            channel_subsystem_request_function: None,
            channel_write_wontblock_function: None,
            channel_open_response_function: None,
            channel_request_response_function: None,
        };

        let mut channel = Box::pin(Self {
            channel,
            callbacks,
            events: VecDeque::new(),
            _pinned: marker::PhantomPinned,
        });

        unsafe {
            channel.as_mut().get_unchecked_mut().callbacks.userdata = &*channel as *const _ as _;
        }

        channel
    }
}

enum ChannelEvent {}
