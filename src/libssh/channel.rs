use std::collections::VecDeque;
use std::ffi::{CStr, c_char, c_int, c_void};
use std::pin::{Pin, UnsafePinned};
use std::{cmp, marker, mem, ptr, slice};

use libssh_rs_sys as libssh;
use tokio::io;
use tracing::debug;

use crate::libssh::error;

pub struct ChannelState {
    channel: libssh::ssh_channel,
    callbacks: libssh::ssh_channel_callbacks_struct,
    events: VecDeque<ChannelEvent>,
    write_window: usize,
    _pinned: marker::PhantomPinned,
}

impl ChannelState {
    pub(crate) fn new(channel: libssh::ssh_channel) -> Pin<Box<UnsafePinned<Self>>> {
        let callbacks = libssh::ssh_channel_callbacks_struct {
            size: mem::size_of::<libssh::ssh_channel_callbacks_struct>(),
            userdata: ptr::null_mut(),
            channel_data_function: Some(Self::callback_data),
            channel_eof_function: Some(Self::callback_eof),
            channel_close_function: Some(Self::callback_close),
            channel_signal_function: None,
            channel_exit_status_function: None,
            channel_pty_request_function: None,
            channel_shell_request_function: None,
            channel_exit_signal_function: None,
            channel_auth_agent_req_function: None,
            channel_x11_req_function: None,
            channel_pty_window_change_function: None,
            channel_exec_request_function: Some(Self::callback_exec_request),
            channel_env_request_function: None,
            channel_subsystem_request_function: None,
            channel_write_wontblock_function: Some(Self::callback_write_wontblock),
            channel_open_response_function: None,
            channel_request_response_function: None,
        };

        let mut state = Box::pin(UnsafePinned::new(Self {
            channel,
            callbacks,
            events: VecDeque::new(),
            write_window: 0,
            _pinned: marker::PhantomPinned,
        }));

        unsafe {
            let state_ref = &mut *state.as_mut().get_unchecked_mut().get_mut_unchecked();
            state_ref.callbacks.userdata = state_ref as *mut _ as *mut c_void;
        }

        unsafe {
            let state_ref = &*state.as_ref().get_ref().get();
            libssh::ssh_set_channel_callbacks(channel, &state_ref.callbacks as *const _ as _);
        }

        state
    }

    unsafe extern "C" fn callback_data(
        _ssh_session: libssh::ssh_session,
        _ssh_channel: libssh::ssh_channel,
        data: *mut c_void,
        len: u32,
        is_stderr: c_int,
        userdata: *mut c_void,
    ) -> c_int {
        unsafe {
            let state = &mut *(userdata as *mut ChannelState);

            let data = slice::from_raw_parts(data as *const u8, len as usize);
            let is_stderr = is_stderr != 0;
            state.events.push_back(ChannelEvent::Data {
                data: data.to_vec(),
                is_stderr,
            });

            data.len() as c_int
        }
    }

    unsafe extern "C" fn callback_eof(
        _ssh_session: libssh::ssh_session,
        _ssh_channel: libssh::ssh_channel,
        userdata: *mut c_void,
    ) {
        unsafe {
            let state = &mut *(userdata as *mut ChannelState);
            state.events.push_back(ChannelEvent::Eof);
        }
    }

    unsafe extern "C" fn callback_close(
        _ssh_session: libssh::ssh_session,
        _ssh_channel: libssh::ssh_channel,
        userdata: *mut c_void,
    ) {
        unsafe {
            let state = &mut *(userdata as *mut ChannelState);
            state.events.push_back(ChannelEvent::Close);
        }
    }

    unsafe extern "C" fn callback_exec_request(
        _ssh_session: libssh::ssh_session,
        _ssh_channel: libssh::ssh_channel,
        command: *const c_char,
        userdata: *mut c_void,
    ) -> c_int {
        unsafe {
            let state = &mut *(userdata as *mut ChannelState);

            let command = CStr::from_ptr(command).to_string_lossy().into_owned();

            debug!("exec request: {}", command);

            state
                .events
                .push_back(ChannelEvent::ExeqRequest { command });

            0
        }
    }

    unsafe extern "C" fn callback_write_wontblock(
        _ssh_session: libssh::ssh_session,
        _ssh_channel: libssh::ssh_channel,
        bytes: u32,
        userdata: *mut c_void,
    ) -> c_int {
        unsafe {
            let state = &mut *(userdata as *mut ChannelState);
            state.write_window = bytes as usize;
            0
        }
    }
}

impl Drop for ChannelState {
    fn drop(&mut self) {
        unsafe {
            libssh::ssh_channel_close(self.channel);
            libssh::ssh_channel_free(self.channel);
        }
    }
}

unsafe impl Send for ChannelState {}
unsafe impl Sync for ChannelState {}

pub trait ChannelStateExt {
    fn write(&mut self, data: &[u8], stderr: bool) -> io::Result<usize>;
    fn writable(&mut self) -> bool;
    fn send_eof(&mut self) -> io::Result<()>;
    fn send_exit_status(&mut self, status: i32) -> io::Result<()>;
    fn send_close(&mut self) -> io::Result<()>;
    fn events(&mut self) -> &mut VecDeque<ChannelEvent>;
}

impl ChannelStateExt for Pin<&mut UnsafePinned<ChannelState>> {
    fn write(&mut self, data: &[u8], stderr: bool) -> io::Result<usize> {
        let this = unsafe { &mut *self.as_mut().get_unchecked_mut().get_mut_unchecked() };
        let write_fn = if !stderr {
            libssh::ssh_channel_write
        } else {
            libssh::ssh_channel_write_stderr
        };

        let do_write = cmp::min(data.len(), this.write_window);
        if this.write_window == 0 {
            return Err(io::ErrorKind::WouldBlock.into());
        }

        let rc = unsafe {
            write_fn(
                this.channel,
                data.as_ptr() as *const c_void,
                do_write as u32,
            )
        };

        if rc == libssh::SSH_ERROR {
            return Err(error::libssh(this.channel as _));
        }

        assert_eq!(rc as usize, do_write);
        this.write_window -= do_write;
        Ok(do_write)
    }

    fn writable(&mut self) -> bool {
        let this = unsafe { &*self.as_mut().get_unchecked_mut().get_mut_unchecked() };
        this.write_window > 0
    }

    fn send_eof(&mut self) -> io::Result<()> {
        let this = unsafe { &*self.as_mut().get_unchecked_mut().get_mut_unchecked() };
        let rc = unsafe { libssh::ssh_channel_send_eof(this.channel) };

        if rc != 0 {
            return Err(error::libssh(this.channel as _));
        }

        Ok(())
    }

    fn send_exit_status(&mut self, status: i32) -> io::Result<()> {
        let this = unsafe { &*self.as_mut().get_unchecked_mut().get_mut_unchecked() };
        let rc = unsafe { libssh::ssh_channel_request_send_exit_status(this.channel, status) };

        if rc != 0 {
            return Err(error::libssh(this.channel as _));
        }

        Ok(())
    }

    fn send_close(&mut self) -> io::Result<()> {
        let this = unsafe { &*self.as_mut().get_unchecked_mut().get_mut_unchecked() };
        let rc = unsafe { libssh::ssh_channel_close(this.channel) };

        if rc != 0 {
            return Err(error::libssh(this.channel as _));
        }

        Ok(())
    }

    fn events(&mut self) -> &mut VecDeque<ChannelEvent> {
        let this = unsafe { &mut *self.as_mut().get_unchecked_mut().get_mut_unchecked() };
        &mut this.events
    }
}

#[derive(Debug)]
pub enum ChannelEvent {
    Data { data: Vec<u8>, is_stderr: bool },
    Eof,
    Close,
    ExeqRequest { command: String },
}
