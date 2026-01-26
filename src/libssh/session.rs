use std::ffi::{CStr, c_char, c_int, c_void};
use std::os::fd::AsRawFd;
use std::os::unix::io::RawFd;
use std::pin::{Pin, UnsafePinned};
use std::{marker, mem, ptr};

use libssh_rs_sys as libssh;
use tokio::io::unix::AsyncFd;
use tokio::io::{self, Interest};

use crate::libssh::channel::ChannelState;
use crate::libssh::error;

pub struct Session {
    handle: AsyncFd<HandleBox>,
}

struct HandleBox(Pin<Box<UnsafePinned<Handle>>>);

impl AsRawFd for HandleBox {
    fn as_raw_fd(&self) -> RawFd {
        let handle = unsafe { &*self.0.as_ref().get_ref().get() };
        unsafe { libssh::ssh_get_fd(handle.session) }
    }
}

impl Session {
    pub(super) fn new(session: libssh::ssh_session) -> Self {
        let handle = Handle::new(session).unwrap();

        Self {
            handle: AsyncFd::new(HandleBox(handle)).unwrap(),
        }
    }

    fn handle_mut(&mut self) -> &mut Handle {
        unsafe {
            &mut *self
                .handle
                .get_mut()
                .0
                .as_mut()
                .get_unchecked_mut()
                .get_mut_unchecked()
        }
    }

    fn handle_ref(&self) -> &Handle {
        unsafe { &*self.handle.get_ref().0.as_ref().get_ref().get() }
    }

    pub fn configure(&mut self) {
        let handle = self.handle_mut();
        unsafe {
            libssh::ssh_set_auth_methods(handle.session, libssh::SSH_AUTH_METHOD_PUBLICKEY as i32);
        }
    }

    pub fn allowed_keys(&mut self, keys: Vec<(String, String)>) {
        self.handle_mut().keys = keys;
    }

    pub async fn handle_key_exchange(&mut self) -> io::Result<()> {
        loop {
            let mut guard = self
                .handle
                .ready(Interest::READABLE | Interest::WRITABLE)
                .await
                .unwrap();

            let handle = unsafe { &*guard.get_inner().0.as_ref().get_ref().get() };

            match unsafe { libssh::ssh_handle_key_exchange(handle.session) } {
                error::SSH_OK => break,
                error::SSH_AGAIN => guard.clear_ready(),
                error::SSH_ERROR => return Err(error::libssh(handle.session as _)),
                _ => unreachable!(),
            }
        }

        let handle = self.handle_ref();

        unsafe {
            let rc = libssh::ssh_event_add_session(handle.ssh_event, handle.session);
            if rc != error::SSH_OK {
                return Err(error::libssh(handle.session as _));
            }
        }

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

            // UnsafePinned allows this reference to alias with callback pointers
            let handle = unsafe {
                &mut *guard
                    .get_inner_mut()
                    .0
                    .as_mut()
                    .get_unchecked_mut()
                    .get_mut_unchecked()
            };

            let rc = unsafe { libssh::ssh_event_dopoll(handle.ssh_event, 0) };

            match rc {
                error::SSH_OK => (),
                error::SSH_AGAIN => {
                    guard.clear_ready();
                    break Ok(());
                }
                error::SSH_ERROR => {
                    break Err(error::libssh(handle.session as _));
                }
                _ => unreachable!(),
            }
        }
    }

    pub fn channel_state(&mut self) -> Option<Pin<&mut UnsafePinned<ChannelState>>> {
        let handle = self.handle_mut();
        handle.channel.as_mut().map(|c| c.as_mut())
    }

    pub fn close_channel(&mut self) {
        self.handle_mut().channel.take();
    }

    pub fn authenticated_user(&self) -> Option<&str> {
        self.handle_ref().authenticated_user.as_deref()
    }
}

struct Handle {
    session: libssh::ssh_session,
    ssh_event: libssh::ssh_event,
    callbacks: libssh::ssh_server_callbacks_struct,
    keys: Vec<(String, String)>,
    authenticated_user: Option<String>,
    channel: Option<Pin<Box<UnsafePinned<ChannelState>>>>,
    _pinned: marker::PhantomPinned,
}

impl Handle {
    fn new(session: libssh::ssh_session) -> io::Result<Pin<Box<UnsafePinned<Self>>>> {
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

        let mut handle = Box::pin(UnsafePinned::new(Self {
            session,
            ssh_event,
            callbacks,
            keys: Vec::new(),
            authenticated_user: None,
            channel: None,
            _pinned: marker::PhantomPinned,
        }));

        unsafe {
            let handle_ref = &mut *handle.as_mut().get_unchecked_mut().get_mut_unchecked();
            handle_ref.callbacks.userdata = handle_ref as *mut _ as *mut c_void;
        }

        unsafe {
            let handle_ref = &*handle.as_ref().get_ref().get();
            libssh::ssh_set_server_callbacks(session, &handle_ref.callbacks as *const _ as _);
        }

        Ok(handle)
    }

    unsafe extern "C" fn callback_auth_pubkey(
        _ssh_session: libssh::ssh_session,
        username: *const c_char,
        pubkey: libssh::ssh_key,
        signature_state: c_char,
        userdata: *mut c_void,
    ) -> c_int {
        unsafe {
            let handle = &mut *(userdata as *mut Handle);

            const SSH_PUBLICKEY_STATE_NONE: c_char =
                libssh::ssh_publickey_state_e::SSH_PUBLICKEY_STATE_NONE as _;

            const SSH_PUBLICKEY_STATE_VALID: c_char =
                libssh::ssh_publickey_state_e::SSH_PUBLICKEY_STATE_VALID as _;

            if signature_state == SSH_PUBLICKEY_STATE_NONE {
                return libssh::ssh_auth_e_SSH_AUTH_SUCCESS;
            }

            if signature_state != SSH_PUBLICKEY_STATE_VALID {
                return libssh::ssh_auth_e_SSH_AUTH_DENIED;
            }

            let maybe_username = CStr::from_ptr(username).to_str();
            let Ok(username) = maybe_username else {
                return libssh::ssh_auth_e_SSH_AUTH_DENIED;
            };

            if username != "git" {
                return libssh::ssh_auth_e_SSH_AUTH_DENIED;
            }

            let ty = libssh::ssh_key_type(pubkey);
            if ty != libssh::ssh_keytypes_e_SSH_KEYTYPE_ED25519 {
                return libssh::ssh_auth_e_SSH_AUTH_DENIED;
            }

            let mut pubkey_buf: *mut c_char = ptr::null_mut();

            let rc = libssh::ssh_pki_export_pubkey_base64(pubkey, &mut pubkey_buf);
            if rc != error::SSH_OK {
                return libssh::ssh_auth_e_SSH_AUTH_DENIED;
            }

            let maybe_pubkey = CStr::from_ptr(pubkey_buf).to_str();
            let Ok(pubkey) = maybe_pubkey.map(ToOwned::to_owned) else {
                return libssh::ssh_auth_e_SSH_AUTH_DENIED;
            };

            libssh::ssh_string_free_char(pubkey_buf);

            let entry = handle.keys.iter().find(|(key, _)| key == &pubkey);
            if let Some((_, username)) = entry {
                handle.authenticated_user = Some(username.clone());
                return libssh::ssh_auth_e_SSH_AUTH_SUCCESS;
            }

            libssh::ssh_auth_e_SSH_AUTH_DENIED
        }
    }

    unsafe extern "C" fn callback_service_request_function(
        _ssh_session: libssh::ssh_session,
        service: *const c_char,
        _userdata: *mut c_void,
    ) -> c_int {
        unsafe {
            let maybe_service = CStr::from_ptr(service).to_str();
            let Ok(service) = maybe_service else {
                return -1;
            };

            match service {
                "ssh-userauth" => 0,
                _ => -1,
            }
        }
    }

    unsafe extern "C" fn callback_channel_open_request_session(
        _ssh_session: libssh::ssh_session,
        userdata: *mut c_void,
    ) -> libssh::ssh_channel {
        unsafe {
            let handle = &mut *(userdata as *mut Handle);

            if handle.channel.is_some() {
                return ptr::null_mut();
            }

            let channel = libssh::ssh_channel_new(handle.session);
            handle.channel = Some(ChannelState::new(channel));
            channel
        }
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
