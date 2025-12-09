use std::ffi::{c_char, c_int, c_void};
use std::pin::Pin;
use std::sync::Arc;
use std::{marker, mem, ptr};
use slab::Slab;

use libssh_rs_sys as libssh;
use tokio::sync::{mpsc, oneshot};
use tokio::{io, task};
use tracing::debug;

use crate::libssh::error;
use crate::libssh::channel::ChannelState;

pub struct Session {
    handle: Pin<Arc<Handle>>,
    events_rx: mpsc::Receiver<SessionEvent>,
    close_tx: Option<oneshot::Sender<()>>,
}

impl Session {
    pub(super) fn new(session: libssh::ssh_session) -> Self {
        let (events_tx, events_rx) = mpsc::channel(64);
        let (close_tx, mut close_rx) = oneshot::channel();
        let handle = Handle::new(session, events_tx).unwrap();

        {
            let handle = handle.clone();

            task::spawn_blocking(move || {
                let handle = handle.as_ref();

                unsafe {
                    handle.handle_key_exchange().unwrap();
                }

                debug!("key exchange complete");

                loop {
                    if close_rx.try_recv().is_ok() {
                        break;
                    }

                    unsafe {
                        handle.poll().unwrap();
                    }
                }
            });
        }

        Self {
            handle,
            events_rx,
            close_tx: Some(close_tx),
        }
    }

    pub fn close(&mut self) {
        if let Some(close_tx) = self.close_tx.take() {
            let _ = close_tx.send(());
        }
    }

}

impl Drop for Session {
    fn drop(&mut self) {
        self.close();
    }
}

struct Handle {
    session: libssh::ssh_session,
    ssh_event: libssh::ssh_event,
    callbacks: libssh::ssh_server_callbacks_struct,
    events_tx: mpsc::Sender<SessionEvent>,
    channels: Slab<ChannelState>,
    _pinned: marker::PhantomPinned,
}

impl Handle {
    fn new(
        session: libssh::ssh_session,
        events_tx: mpsc::Sender<SessionEvent>,
    ) -> io::Result<Pin<Arc<Self>>> {
        let ssh_event = unsafe { libssh::ssh_event_new() };

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

        let mut handle = Arc::new(Self {
            session,
            ssh_event,
            callbacks,
            events_tx,
            channels: Slab::new(),
            _pinned: marker::PhantomPinned,
        });

        Arc::get_mut(&mut handle).unwrap().callbacks.userdata = &*handle as *const _ as _;

        unsafe {
            libssh::ssh_set_server_callbacks(session, &handle.callbacks as *const _ as _);
            libssh::ssh_set_auth_methods(handle.session, libssh::SSH_AUTH_METHOD_NONE as i32);
        }

        let pinned = unsafe { Pin::new_unchecked(handle) };
        Ok(pinned)
    }

    unsafe fn handle_key_exchange(self: Pin<&Self>) -> io::Result<()> {
        loop {
            match unsafe { libssh::ssh_handle_key_exchange(self.session) } {
                error::SSH_OK => break,
                error::SSH_AGAIN => continue,
                error::SSH_ERROR => return Err(error::libssh(self.session as _)),
                _ => unreachable!(),
            }
        }

        unsafe {
            let rc = libssh::ssh_event_add_session(self.ssh_event, self.session);
            if rc != error::SSH_OK {
                panic!("{}", error::libssh(self.session as _));
            }
        }

        Ok(())
    }

    unsafe fn poll(self: Pin<&Self>) -> io::Result<()> {
        let rc = unsafe { libssh::ssh_event_dopoll(self.ssh_event, 10) };

        match rc {
            error::SSH_OK | error::SSH_AGAIN => Ok(()),
            error::SSH_ERROR => Err(error::libssh(self.session as _)),
            _ => unreachable!(),
        }
    }

    unsafe extern "C" fn callback_auth_none(
        _ssh_session: libssh::ssh_session,
        _username: *const c_char,
        _userdata: *mut c_void,
    ) -> c_int {
        debug!("callback_auth_none");
        libssh::ssh_auth_e_SSH_AUTH_SUCCESS
    }

    unsafe extern "C" fn callback_service_request_function(
        _ssh_session: libssh::ssh_session,
        _service: *const c_char,
        _userdata: *mut c_void,
    ) -> c_int {
        debug!("callback_service_request_function");
        0
    }

    unsafe extern "C" fn callback_channel_open_request_session(
        _ssh_session: libssh::ssh_session,
        _userdata: *mut c_void,
    ) -> libssh::ssh_channel {
        debug!("callback_channel_open_request_session");
        ptr::null_mut()
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            libssh::ssh_disconnect(self.session);
            libssh::ssh_event_free(self.ssh_event);
            libssh::ssh_free(self.session);
        }
    }
}

unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}

enum SessionEvent {}
