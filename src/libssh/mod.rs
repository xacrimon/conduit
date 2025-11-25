mod listener;
mod session;
mod error;

pub use listener::Listener;
pub use session::Session;
pub use error::Error;

use libssh_rs_sys::{self as libssh};
use std::os::fd::AsRawFd;
use std::sync::Once;
use std::ffi::{CString, CStr};
use tokio::io::unix::AsyncFd;
use std::os::unix::io::RawFd;
use tokio::io::Interest;

static LIBSSH_INIT: Once = Once::new();
static LIBSSH_FINALIZE: Once = Once::new();

pub fn init() {
    LIBSSH_INIT.call_once(|| unsafe {
        let rc = libssh::ssh_init();
        if rc != libssh::SSH_OK as i32 {
            panic!("failed to initialize libssh: code {}", rc);
        }
    });
}

pub fn finalize() {
    assert_libssh_initialized();
    LIBSSH_FINALIZE.call_once(|| unsafe {
        libssh::ssh_finalize();
    });
}

fn assert_libssh_initialized() {
    assert!(LIBSSH_INIT.is_completed(), "libssh is not initialized");
}
