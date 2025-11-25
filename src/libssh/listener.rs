use libssh_rs_sys::{self as libssh};
use std::os::fd::AsRawFd;
use std::sync::Once;
use std::ffi::{CString, CStr};
use tokio::io::unix::AsyncFd;
use std::os::unix::io::RawFd;
use tokio::io::Interest;
use super::assert_libssh_initialized;
use super::Session;
use std::io;

struct Bind(libssh::ssh_bind);

impl Bind {
    fn new() -> Self {
        assert_libssh_initialized();
        unsafe {
            let bind = libssh::ssh_bind_new();
            Bind(bind)
        }
    }

    fn set_addr(&mut self, addr: &str) {
        let c_addr = CString::new(addr).unwrap();

        unsafe {
            let rc = libssh::ssh_bind_options_set(
                self.0,
                libssh::ssh_bind_options_e::SSH_BIND_OPTIONS_BINDADDR,
                c_addr.as_ptr() as *const std::os::raw::c_void,
            );
            if rc != libssh::SSH_OK as i32 {
                panic!("failed to set bind address");
            }
        }
    }
}

impl AsRawFd for Bind {
    fn as_raw_fd(&self) -> RawFd {
        unsafe { libssh::ssh_bind_get_fd(self.0) }
    }
}

impl Drop for Bind {
    fn drop(&mut self) {
        unsafe {
            libssh::ssh_bind_free(self.0);
        }
    }
}

unsafe impl Send for Bind {}
unsafe impl Sync for Bind {}

pub struct Listener {
    bind: AsyncFd<Bind>,
}

impl Listener {
    pub fn bind(addr: &str) -> Self {
        let mut bind = Bind::new();
        bind.set_addr(addr);

        unsafe {
            libssh::ssh_bind_set_blocking(bind.0, 0);
            let rc = libssh::ssh_bind_listen(bind.0);
            if rc != libssh::SSH_OK as i32 {
                panic!("failed to listen on bind: code {}", rc);
            }
        }

        Self { bind: AsyncFd::new(bind).unwrap() }
    }

    pub async fn accept(&mut self) -> Session {
        self.bind.async_io(Interest::READABLE, |bind| {
            unsafe {
                let session = libssh::ssh_new();
                let rc = libssh::ssh_bind_accept(bind.0, session);
                match rc {
                    rc if rc == libssh::SSH_OK as i32 => Ok(Session::new(session)),
                    rc if rc == libssh::SSH_AGAIN as i32 => Err(io::Error::from(io::ErrorKind::WouldBlock)),
                    _ => panic!(),
                }
            }
        }).await.unwrap()
    }
}
