use libssh_rs_sys as libssh;
use tokio::io::{Interest, Ready};

pub fn poll_flags_to_interests(poll_flags: u32) -> Interest {
    match poll_flags {
        f if f & libssh::SSH_READ_PENDING != 0 && f & libssh::SSH_WRITE_PENDING != 0 => {
            Interest::READABLE | Interest::WRITABLE
        }
        f if f & libssh::SSH_READ_PENDING != 0 => Interest::READABLE,
        f if f & libssh::SSH_WRITE_PENDING != 0 => Interest::WRITABLE,
        _ => unreachable!(),
    }
}

pub fn poll_flags_to_ready(poll_flags: u32) -> Ready {
    match poll_flags {
        f if f & libssh::SSH_READ_PENDING != 0 && f & libssh::SSH_WRITE_PENDING != 0 => {
            Ready::READABLE | Ready::WRITABLE
        }
        f if f & libssh::SSH_READ_PENDING != 0 => Ready::READABLE,
        f if f & libssh::SSH_WRITE_PENDING != 0 => Ready::WRITABLE,
        _ => unreachable!(),
    }
}
