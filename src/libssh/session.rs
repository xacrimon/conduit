use libssh_rs_sys::{self as libssh};
use std::os::fd::AsRawFd;
use std::pin::Pin;
use std::sync::Once;
use std::ffi::{CString, CStr};
use tokio::io::unix::AsyncFd;
use std::os::unix::io::RawFd;
use tokio::io::Interest;

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
        Self { handle: AsyncFd::new(handle).unwrap() }
    }

    pub async fn handle_key_exchange(self) {
        loop {
            let guard = self.handle.ready(Interest::READABLE | Interest::WRITABLE).await.unwrap();
            let handle = guard.get_inner();

            match unsafe { libssh::ssh_handle_key_exchange(handle.session) } {
                rc if rc == libssh::SSH_AGAIN as i32 => continue,
                rc if rc == libssh::SSH_OK as i32 => {
                    break;
                },
                _ => {
                    let err = unsafe {CStr::from_ptr(libssh::ssh_get_error(handle.session as *mut _))};
                    panic!("key exchange failed: {}", err.to_string_lossy());
                },
            }
        }
    }
}
