use std::ffi::{CStr, CString};
use std::os::fd::{AsRawFd, OwnedFd};
use std::os::unix::io::RawFd;
use std::{io, mem};

use libssh_rs_sys::{self as libssh};
use tokio::io::Interest;
use tokio::io::unix::AsyncFd;
use tokio::net::TcpListener;

use super::Session;

pub struct Listener {
    bind: libssh::ssh_bind,
    listener: TcpListener,
}

impl Listener {
    pub async fn bind(host_key: &str, addr: &str, port: u16) -> Self {
        let listener = TcpListener::bind((addr, port)).await.unwrap();
        let bind = unsafe { libssh::ssh_bind_new() };
        let c_key = CString::new(host_key).unwrap();
        let c_banner = CString::new("conduit").unwrap();

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

            let rc = libssh::ssh_bind_options_set(
                bind,
                libssh::ssh_bind_options_e::SSH_BIND_OPTIONS_BANNER,
                c_banner.into_raw() as *const std::os::raw::c_void,
            );
            if rc != libssh::SSH_OK as i32 {
                let err = CStr::from_ptr(libssh::ssh_get_error(bind as *mut _));
                panic!("failed to set banner: {}", err.to_string_lossy());
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
