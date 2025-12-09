use std::collections::VecDeque;
use std::pin::Pin;
use std::{marker, mem, ptr};

use libssh_rs_sys as libssh;

// TODO: needs https://doc.rust-lang.org/std/pin/struct.UnsafePinned.html
struct Channel {
    channel: libssh::ssh_channel,
    callbacks: libssh::ssh_channel_callbacks_struct,
    events: VecDeque<ChannelEvent>,
    _pinned: marker::PhantomPinned,
}

impl Channel {
    fn new(channel: libssh::ssh_channel) -> Pin<Box<Self>> {
        let callbacks = libssh::ssh_channel_callbacks_struct {
            size: mem::size_of::<libssh::ssh_channel_callbacks_struct>(),
            userdata: ptr::null_mut(),
            channel_data_function: None,
            channel_eof_function: None,
            channel_close_function: None,
            channel_signal_function: None,
            channel_exit_status_function: None,
            channel_pty_request_function: None,
            channel_shell_request_function: None,
            channel_exit_signal_function: None,
            channel_auth_agent_req_function: None,
            channel_x11_req_function: None,
            channel_pty_window_change_function: None,
            channel_exec_request_function: None,
            channel_env_request_function: None,
            channel_subsystem_request_function: None,
            channel_write_wontblock_function: None,
            channel_open_response_function: None,
            channel_request_response_function: None,
        };

        let mut channel = Box::pin(Self {
            channel,
            callbacks,
            events: VecDeque::new(),
            _pinned: marker::PhantomPinned,
        });

        unsafe {
            channel.as_mut().get_unchecked_mut().callbacks.userdata = &*channel as *const _ as _;
        }

        channel
    }
}

enum ChannelEvent {}
