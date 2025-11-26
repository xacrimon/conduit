mod error;
mod listener;
mod session;

use std::ffi::{CStr, CString};
use std::os::fd::AsRawFd;
use std::os::unix::io::RawFd;
use std::sync::Once;

pub use error::Error;
use libssh_rs_sys::{self as libssh};
pub use listener::Listener;
pub use session::Session;
use tokio::io::Interest;
use tokio::io::unix::AsyncFd;

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
    assert!(
        LIBSSH_INIT.is_completed(),
        "libssh must be initialized before finalizing"
    );
    LIBSSH_FINALIZE.call_once(|| unsafe {
        libssh::ssh_finalize();
    });
}
