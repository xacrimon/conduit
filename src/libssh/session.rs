use std::os::fd::AsRawFd;
use std::os::unix::io::RawFd;
use std::pin::Pin;

use libssh_rs_sys::{self as libssh};
use tokio::io;
use tokio::io::Interest;
use tokio::io::unix::AsyncFd;

use crate::libssh::error;

struct Handle {
    session: libssh::ssh_session,
}

impl Handle {
    fn new(session: libssh::ssh_session) -> Pin<Box<Self>> {
        let handle = Self { session };
        Box::pin(handle)
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
            libssh::ssh_disconnect(self.session);
            libssh::ssh_free(self.session);
        }
    }
}

unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}

pub struct Session {
    handle: AsyncFd<Pin<Box<Handle>>>,
}

impl Session {
    pub(super) fn new(session: libssh::ssh_session) -> Self {
        let handle = Handle::new(session);

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
}
