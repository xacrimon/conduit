use libssh_rs_sys as libssh;
use std::sync::Once;
use std::ffi::{};

static LIBSSH_INIT: Once = Once::new();
static LIBSSH_FINALIZE: Once = Once::new();

pub fn init() {
    LIBSSH_INIT.call_once(|| unsafe {
        let rc = libssh::ssh_init();
        if rc != 0 {
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

pub struct Listener {
    bind: libssh::ssh_bind,
}

impl Drop for Listener {
    fn drop(&mut self) {
        unsafe {
            libssh::ssh_bind_free(self.bind);
        }
    }
}

pub struct Session {
    session: libssh::ssh_session,
}

impl Drop for Session {
    fn drop(&mut self) {
        unsafe {
            libssh::ssh_free(self.session);
        }
    }
}
