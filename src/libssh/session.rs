use libssh_rs_sys::{self as libssh};
use std::os::fd::AsRawFd;
use std::sync::Once;
use std::ffi::{CString, CStr};
use tokio::io::unix::AsyncFd;
use std::os::unix::io::RawFd;
use tokio::io::Interest;

struct Handle(libssh::ssh_session);

impl AsRawFd for Handle {
    fn as_raw_fd(&self) -> RawFd {
        unsafe { libssh::ssh_get_fd(self.0) }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            libssh::ssh_free(self.0);
        }
    }
}

unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}

pub struct Session {
    session: AsyncFd<Handle>,
}

impl Session {
    pub(super) fn new(session: libssh::ssh_session) -> Self {
        let handle = Handle(session);
        Self { session: AsyncFd::new(handle).unwrap() }
    }
}
