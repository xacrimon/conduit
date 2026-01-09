use std::path::{Path, PathBuf};
use std::pin::pin;
use std::process::Stdio;
use std::time::Duration;
use std::{env, vec};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};
use tokio::{select, time};
use tokio_util::sync::CancellationToken;
use tracing::debug;

use crate::config::Config;
use crate::libssh::{ChannelEvent, Session};
use crate::state::AppState;
use crate::utils::{RingBuf, re};

pub async fn handle_session(
    state: &AppState,
    mut session: Session,
    ct: CancellationToken,
) -> anyhow::Result<()> {
    session.configure();
    // TODO: load keys from database
    session.allowed_keys(vec![]);
    session.handle_key_exchange().await.unwrap();

    let mut cancel = pin!(async {
        ct.cancelled().await;
        time::sleep(Duration::from_secs(10)).await;
    });

    let mut child: Option<Child> = None;
    let mut stdout: Option<ChildStdout> = None;
    let mut stderr: Option<ChildStderr> = None;
    let mut stdin: Option<ChildStdin> = None;

    let buffer_size = 4096;
    let mut buf_stdout = RingBuf::new(buffer_size);
    let mut buf_stderr = RingBuf::new(buffer_size);

    let mut channel_closed = false;
    loop {
        // If channel is closed, keep processing events until socket would block
        // This ensures all queued data is transmitted before we disconnect
        if channel_closed {
            loop {
                match session.wait().await {
                    Ok(()) => continue, // More work done, keep going
                    Err(_) => break,    // Nothing more to do
                }
            }
            break;
        }

        select! {
            _ = &mut cancel => break,
            res = session.wait() => {
                res.unwrap();
                let mut close_channel = false;

                if let Some(mut channel_state) = session.channel_state() {
                    'inner: while let Some(event) = channel_state.as_mut().events().pop_front() {
                        match event {
                            ChannelEvent::ExeqRequest { command } => {
                                assert!(child.is_none());

                                let (bin, user, repo) = parse_command(&command);
                                let bin_path = search_path(Path::new(bin)).unwrap();
                                dbg!(repo_path(&state.config, user, repo));

                                let mut cmd = Command::new(bin_path);
                                cmd.stdin(Stdio::piped());
                                cmd.stdout(Stdio::piped());
                                cmd.stderr(Stdio::piped());
                                cmd.arg(repo_path(&state.config, user, repo));

                                child = Some(cmd.spawn().unwrap());
                                stdout = Some(child.as_mut().unwrap().stdout.take().unwrap());
                                stderr = Some(child.as_mut().unwrap().stderr.take().unwrap());
                                stdin = Some(child.as_mut().unwrap().stdin.take().unwrap());
                                dbg!(&bin, &user, &repo);
                            }
                            ChannelEvent::Close => {
                                close_channel = true;
                                debug!("channel close received");
                                break 'inner;
                            },
                            ChannelEvent::Data { data, is_stderr } => {
                                assert!(!is_stderr);
                                let stdin = stdin.as_mut().unwrap();
                                stdin.write_all(&data).await.unwrap();
                            }
                            ChannelEvent::Eof => {
                                let mut stdin = stdin.take().unwrap();
                                stdin.flush().await.unwrap();
                                drop(stdin); // close stdin
                            }
                        }
                    }
                }

                if close_channel {
                    session.close_channel();
                    break;
                }
            }
            n = unwrap_await(stdout.as_mut().map(|stdout| stdout.read(buf_stdout.writable_slice()))), if stdout.is_some() && buf_stdout.is_empty() => {
                let n = n.unwrap();
                if n == 0 {
                    debug!("stdout EOF");
                    drop(stdout.take()); // close stdout

                    let channel_state = session.channel_state().unwrap();
                    channel_state.send_eof().unwrap();
                } else {
                    buf_stdout.advance_write(n);
                }
            }
            n = unwrap_await(stderr.as_mut().map(|stderr| stderr.read(buf_stderr.writable_slice()))), if stderr.is_some() && buf_stderr.is_empty() => {
                let n = n.unwrap();
                if n == 0 {
                    debug!("stderr zero read");
                    drop(stderr.take()); // close stderr
                } else {
                    buf_stderr.advance_write(n);
                }
            }
            status = unwrap_await(child.as_mut().map(|child| child.wait())), if child.is_some() && stdout.is_none() && stderr.is_none() && buf_stdout.is_empty() && buf_stderr.is_empty() => {
                let status = status.unwrap();
                child = None;
                debug!("child exited: {:?}", status);

                let mut channel = session.channel_state().unwrap();
                if let Some(code) = status.code() {
                    channel.as_mut().send_exit_status(code).unwrap();
                }
                // Send close message but keep channel alive for flushing
                channel.as_mut().send_close().unwrap();
                channel_closed = true;
            }
        }

        if let Some(mut channel) = session.channel_state()
            && channel.as_mut().writable()
        {
            if !buf_stdout.is_empty() {
                let slice = buf_stdout.readable_slice();
                match channel.as_mut().write(slice, false) {
                    Ok(n) => buf_stdout.advance_read(n),
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(_) => break,
                }
            }

            if !buf_stderr.is_empty() {
                let slice = buf_stderr.readable_slice();
                match channel.as_mut().write(slice, true) {
                    Ok(n) => buf_stderr.advance_read(n),
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(e) => {
                        eprintln!("channel stderr write error: {:?}", e);
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

async fn unwrap_await<Fut, U>(fut: Option<Fut>) -> U
where
    Fut: Future<Output = U>,
{
    fut.unwrap().await
}

fn search_path(filename: &Path) -> Option<PathBuf> {
    if let Some(paths) = env::var_os("PATH") {
        for path in env::split_paths(&paths) {
            let full_path = path.join(filename);
            if full_path.exists() {
                return Some(full_path);
            }
        }
    }

    None
}

fn repo_path(config: &Config, user: &str, repo: &str) -> PathBuf {
    config.git.repository_path.join(user).join(repo)
}

fn parse_command(command: &str) -> (&str, &str, &str) {
    let caps = re!(r#"^([a-zA-Z\-]+) '/?([a-zA-Z0-9]+)/([\.\-a-zA-Z0-9]+\.git)'$"#)
        .captures(command)
        .unwrap();

    let (_, [command, user, repo]) = caps.extract();
    if !matches!(command, "git-upload-pack" | "git-receive-pack") {
        panic!("unsupported command: {}", command);
    }

    (command, user, repo)
}
