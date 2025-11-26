use libssh_rs_sys::{self as libssh};
use tokio::net::TcpListener;
use std::os::fd::AsRawFd;
use std::ffi::{CString, CStr};
use tokio::io::unix::AsyncFd;
use std::os::unix::io::RawFd;
use std::os::fd::OwnedFd;
use tokio::io::Interest;
use super::Session;
use std::{io, mem};

const HOST_KEY: &str = r#"
-----BEGIN OPENSSH PRIVATE KEY-----
b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW
QyNTUxOQAAACBGAQ7+vwHah7hlZRoY7+8G9vfbtp8slX6YbQLVQSw0TQAAAKDY5qT42Oak
+AAAAAtzc2gtZWQyNTUxOQAAACBGAQ7+vwHah7hlZRoY7+8G9vfbtp8slX6YbQLVQSw0TQ
AAAEB+9L+sh9tW/nVDfax4IOLA2vjyPQiRWispg16gt7yeVEYBDv6/AdqHuGVlGhjv7wb2
99u2nyyVfphtAtVBLDRNAAAAGGpvZWx3ZWpkZW5zdGFsQE1hYy52YWxsYQECAwQF
-----END OPENSSH PRIVATE KEY-----
"#;

pub struct Listener {
    bind: libssh::ssh_bind,
    listener: TcpListener
}

impl Listener {
    pub async fn bind(addr: &str, port: u16) -> Self {
        let listener = TcpListener::bind((addr, port)).await.unwrap();
        let bind = unsafe { libssh::ssh_bind_new()};
        let c_key = CString::new(HOST_KEY).unwrap();

        unsafe {
            let rc = libssh::ssh_bind_options_set(
                bind,
                libssh::ssh_bind_options_e::SSH_BIND_OPTIONS_IMPORT_KEY_STR,
                c_key.into_raw() as *const std::os::raw::c_void,
            );
            if rc != libssh::SSH_OK as i32 {
                let err = CStr::from_ptr(libssh::ssh_get_error(bind as *mut _));
                panic!("failed to set host key: {}", err.to_string_lossy());
            }
        }

        unsafe {
            libssh::ssh_bind_set_blocking(bind, 0);
            let rc = libssh::ssh_bind_listen(bind);
            if rc != libssh::SSH_OK as i32 {
                let err = CStr::from_ptr(libssh::ssh_get_error(bind as *mut _));
                panic!("failed to listen on bind: {}", err.to_string_lossy());
            }
        }

        Self { bind, listener }
    }

    pub async fn accept(&mut self) -> Session {
        let (socket, _) = self.listener.accept().await.unwrap();
        let fd = OwnedFd::from(socket.into_std().unwrap());
    
        unsafe {
            let session = libssh::ssh_new();
            let rc = libssh::ssh_bind_accept_fd(self.bind, session, fd.as_raw_fd());
            mem::forget(fd);
            match rc {
                rc if rc == libssh::SSH_OK as i32 => Session::new(session),
                _ => panic!(),
            }
        }
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        unsafe {
            libssh::ssh_bind_free(self.bind);
        }
    }
}

unsafe impl Send for Listener {}
unsafe impl Sync for Listener {}
