use std::ffi::{CStr, c_void};

use libssh_rs_sys::{self as libssh};
use thiserror::Error;
use tokio::io;

pub const SSH_OK: i32 = libssh::SSH_OK as i32;
pub const SSH_AGAIN: i32 = libssh::SSH_AGAIN;
pub const SSH_ERROR: i32 = libssh::SSH_ERROR;

#[derive(Error, Debug)]
#[error("libssh: {message}")]
pub struct LibsshError {
    message: String,
}

pub fn libssh(obj: *mut c_void) -> io::Error {
    let message = unsafe { CStr::from_ptr(libssh::ssh_get_error(obj)) };
    let message = message.to_string_lossy().into_owned();
    io::Error::other(LibsshError { message })
}
