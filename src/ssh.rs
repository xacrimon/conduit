use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::pin::pin;
use std::process::Stdio;
use std::time::{Duration as StdDuration, Duration};

use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::{select, time};
use tracing::debug;

use crate::config::Config;
use crate::libssh::{ChannelEvent, ChannelStateExt, Session};
use crate::model;
use crate::state::AppState;
use crate::utils::{RingBuf, re};

const LFS_TOKEN_TTL_SECS: u64 = 60 * 60 * 24;

/// Parsed SSH command from client
enum SshCommand<'a> {
    /// Git LFS authentication request
    LfsAuth(LfsAuthRequest),
    /// Standard git command (upload-pack or receive-pack)
    Git {
        bin: &'a str,
        user: &'a str,
        repo: &'a str,
    },
}

/// Result of handling an immediate command (like LFS auth)
struct ImmediateResponse {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    exit_code: i32,
}

impl ImmediateResponse {
    fn success(stdout: Vec<u8>) -> Self {
        Self {
            stdout,
            stderr: Vec::new(),
            exit_code: 0,
        }
    }

    fn error(message: &[u8]) -> Self {
        Self {
            stdout: Vec::new(),
            stderr: message.to_vec(),
            exit_code: 1,
        }
    }
}

pub async fn handle_session(state: &AppState, mut session: Session) -> anyhow::Result<()> {
    session.configure();
    let keys = model::user::get_all_ssh_keys(&state.db).await?;
    session.allowed_keys(keys);
    session.handle_key_exchange().await.unwrap();

    let mut cancel = pin!(async {
        state.cancel_token.cancelled().await;
        time::sleep(StdDuration::from_secs(10)).await;
    });

    // Wait for the exec request to determine what kind of session this is
    let Some(command) = wait_for_exec_request(&mut session, &mut cancel).await else {
        return Ok(());
    };

    // Parse and dispatch to appropriate handler
    match parse_ssh_command(&command) {
        Ok(SshCommand::LfsAuth(request)) => {
            handle_lfs_auth_session(state, &mut session, &request).await
        }
        Ok(SshCommand::Git { bin, user, repo }) => {
            handle_git_session(state, &mut session, &mut cancel, bin, user, repo).await
        }
        Err(e) => {
            send_immediate_response(
                &mut session,
                ImmediateResponse::error(format!("{}\n", e).as_bytes()),
            )
            .await
        }
    }
}

/// Wait for an exec request from the client
async fn wait_for_exec_request(
    session: &mut Session,
    cancel: &mut std::pin::Pin<&mut impl Future<Output = ()>>,
) -> Option<String> {
    loop {
        select! {
            _ = &mut *cancel => return None,
            res = session.wait() => {
                res.unwrap();
                if let Some(mut channel_state) = session.channel_state() {
                    while let Some(event) = channel_state.events().pop_front() {
                        if let ChannelEvent::ExeqRequest { command } = event {
                            return Some(command);
                        }
                        if let ChannelEvent::Close = event {
                            return None;
                        }
                    }
                }
            }
        }
    }
}

/// Handle LFS authentication - sends response and closes immediately
async fn handle_lfs_auth_session(
    state: &AppState,
    session: &mut Session,
    request: &LfsAuthRequest,
) -> anyhow::Result<()> {
    debug!(
        "LFS auth request: user={}, repo={}, op={}",
        request.user, request.repo, request.operation
    );
    let response = handle_lfs_auth(state, session.authenticated_user(), request).await;
    send_immediate_response(session, response).await
}

/// Send an immediate response (stdout/stderr) and close the channel
async fn send_immediate_response(
    session: &mut Session,
    response: ImmediateResponse,
) -> anyhow::Result<()> {
    let mut stdout_pos = 0;
    let mut stderr_pos = 0;
    let mut sent_close = false;

    loop {
        if session.wait().await.is_err() {
            break;
        }

        let Some(mut channel) = session.channel_state() else {
            break;
        };

        // Drain events
        while channel.events().pop_front().is_some() {}

        if !channel.writable() {
            continue;
        }

        // Write stdout
        if stdout_pos < response.stdout.len() {
            match channel.write(&response.stdout[stdout_pos..], false) {
                Ok(n) => stdout_pos += n,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(_) => break,
            }
        }

        // Write stderr
        if stderr_pos < response.stderr.len() {
            match channel.write(&response.stderr[stderr_pos..], true) {
                Ok(n) => stderr_pos += n,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(_) => break,
            }
        }

        // Once all data is sent, send EOF, exit status, and close
        let all_sent = stdout_pos >= response.stdout.len() && stderr_pos >= response.stderr.len();
        if all_sent && !sent_close {
            channel.send_eof().unwrap();
            channel.send_exit_status(response.exit_code).unwrap();
            channel.send_close().unwrap();
            sent_close = true;
        }

        // After close is sent, keep processing until channel is gone
        if sent_close {
            continue;
        }
    }

    Ok(())
}

/// Handle git command - proxies data between SSH channel and git process
async fn handle_git_session(
    state: &AppState,
    session: &mut Session,
    cancel: &mut std::pin::Pin<&mut impl Future<Output = ()>>,
    bin: &str,
    user: &str,
    repo: &str,
) -> anyhow::Result<()> {
    let bin_path = search_path(Path::new(bin)).unwrap();
    debug!("Git command: {} for {}/{}", bin, user, repo);

    let mut cmd = Command::new(bin_path);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.arg(repo_path(&state.config, user, repo));

    let mut child = Some(cmd.spawn().unwrap());
    let mut stdout = child.as_mut().unwrap().stdout.take();
    let mut stderr = child.as_mut().unwrap().stderr.take();
    let mut stdin = child.as_mut().unwrap().stdin.take();

    let buffer_size = 4096;
    let mut buf_stdout = RingBuf::new(buffer_size);
    let mut buf_stderr = RingBuf::new(buffer_size);

    let mut channel_closed = false;

    loop {
        // If channel is closed, keep processing events until socket would block
        if channel_closed {
            loop {
                match session.wait().await {
                    Ok(()) => continue,
                    Err(_) => break,
                }
            }
            break;
        }

        select! {
            _ = &mut *cancel => break,
            res = session.wait() => {
                res.unwrap();
                let mut close_channel = false;

                if let Some(mut channel_state) = session.channel_state() {
                    while let Some(event) = channel_state.events().pop_front() {
                        match event {
                            ChannelEvent::ExeqRequest { .. } => {
                                // Already handled, ignore additional requests
                            }
                            ChannelEvent::Close => {
                                close_channel = true;
                                debug!("channel close received");
                                break;
                            }
                            ChannelEvent::Data { data, is_stderr } => {
                                assert!(!is_stderr);
                                if let Some(stdin) = stdin.as_mut() {
                                    stdin.write_all(&data).await.unwrap();
                                }
                            }
                            ChannelEvent::Eof => {
                                if let Some(mut s) = stdin.take() {
                                    s.flush().await.unwrap();
                                    drop(s);
                                }
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

                    let mut channel_state = session.channel_state().unwrap();
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
                    channel.send_exit_status(code).unwrap();
                }
                // Send close message but keep channel alive for flushing
                channel.send_close().unwrap();
                channel_closed = true;
            }
        }

        // Write buffered data to channel
        if let Some(mut channel) = session.channel_state() {
            if channel.writable() && !buf_stdout.is_empty() {
                let slice = buf_stdout.readable_slice();
                match channel.write(slice, false) {
                    Ok(n) => buf_stdout.advance_read(n),
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(_) => break,
                }
            }

            if channel.writable() && !buf_stderr.is_empty() {
                let slice = buf_stderr.readable_slice();
                match channel.write(slice, true) {
                    Ok(n) => buf_stderr.advance_read(n),
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(e) => {
                        debug!("channel stderr write error: {:?}", e);
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

/// Parse an SSH exec command into a structured command type
fn parse_ssh_command(command: &str) -> Result<SshCommand<'_>, &'static str> {
    // Try LFS auth first
    if let Some(caps) = re!(
        r#"^git-lfs-authenticate '?/?~([a-zA-Z0-9]+)/([\.\-a-zA-Z0-9]+\.git)'? (download|upload)$"#
    )
    .captures(command)
    {
        let (_, [user, repo, operation]) = caps.extract();
        return Ok(SshCommand::LfsAuth(LfsAuthRequest {
            user: user.to_owned(),
            repo: repo.to_owned(),
            operation: operation.to_owned(),
        }));
    }

    // Try standard git command
    let caps = re!(r#"^([a-zA-Z\-]+) '/?~([a-zA-Z0-9]+)/([\.\-a-zA-Z0-9]+\.git)'$"#)
        .captures(command)
        .ok_or("invalid command format")?;

    let (_, [bin, user, repo]) = caps.extract();
    if !matches!(bin, "git-upload-pack" | "git-receive-pack") {
        return Err("unsupported command");
    }

    Ok(SshCommand::Git { bin, user, repo })
}

struct LfsAuthRequest {
    user: String,
    repo: String,
    operation: String,
}

/// Handle LFS authentication request, returning an immediate response
async fn handle_lfs_auth(
    state: &AppState,
    authenticated_user: Option<&str>,
    request: &LfsAuthRequest,
) -> ImmediateResponse {
    let Some(username) = authenticated_user else {
        return ImmediateResponse::error(b"authentication failed\n");
    };

    if username != request.user {
        return ImmediateResponse::error(b"repository access denied\n");
    }

    let user_id = match model::user::get_id_by_username(&state.db, username).await {
        Ok(Some(id)) => id,
        Ok(None) => return ImmediateResponse::error(b"user not found\n"),
        Err(e) => {
            tracing::error!("database error looking up user: {}", e);
            return ImmediateResponse::error(b"internal error\n");
        }
    };

    let token =
        match model::lfs::create(&state.db, user_id, Duration::from_secs(LFS_TOKEN_TTL_SECS)).await
        {
            Ok(token) => token,
            Err(e) => {
                tracing::error!("database error creating LFS token: {}", e);
                return ImmediateResponse::error(b"internal error\n");
            }
        };

    let response = lfs_auth_response(state, &request.user, &request.repo, &token.token);
    let mut payload = match serde_json::to_vec(&response) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("JSON serialization error: {}", e);
            return ImmediateResponse::error(b"internal error\n");
        }
    };
    payload.push(b'\n');

    ImmediateResponse::success(payload)
}

#[derive(Serialize)]
struct LfsAuthResponse {
    href: String,
    header: HashMap<String, String>,
    expires_in: u64,
}

fn lfs_auth_response(state: &AppState, user: &str, repo: &str, token: &str) -> LfsAuthResponse {
    let mut header = HashMap::new();
    header.insert("Authorization".to_string(), format!("RemoteAuth {}", token));

    LfsAuthResponse {
        href: format!("{}/~{}/{}", state.config.http.public_url, user, repo) + "/info/lfs",
        header,
        expires_in: LFS_TOKEN_TTL_SECS,
    }
}
