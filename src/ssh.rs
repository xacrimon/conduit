use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::pin::pin;
use std::process::Stdio;
use std::time::Duration;
use std::{env, future};

use futures_util::future::BoxFuture;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};
use tokio::{select, time};
use tokio_util::sync::CancellationToken;

use crate::config::Config;
use crate::libssh::{ChannelEvent, Session};
use crate::utils::re;

pub async fn handle_session(
    config: &Config,
    mut session: Session,
    ct: CancellationToken,
) -> anyhow::Result<()> {
    session.configure();
    session.handle_key_exchange().await.unwrap();
    session.authenticate().await.unwrap();

    let mut cancel = pin!(async {
        ct.cancelled().await;
        time::sleep(Duration::from_secs(10)).await;
    });

    let mut child: Option<Child> = None;
    let mut stdout: Option<ChildStdout> = None;
    let mut stderr: Option<ChildStderr> = None;
    let mut stdin: Option<ChildStdin> = None;

    let mut buf_stdout = [0u8; 32];
    let mut buf_stderr = [0u8; 32];

    'outer: loop {
        select! {
            _ = &mut cancel => break,
            res = session.wait() => {
                res.unwrap();

                if let Some(mut channel_state) = session.channel_state() {
                    while let Some(event) = channel_state.as_mut().events().pop_front() {
                        dbg!(&event);

                        match event {
                            ChannelEvent::ExeqRequest { command } => {
                                assert!(child.is_none());

                                let (bin, user, repo) = parse_command(&command);
                                let bin_path = search_path(Path::new(bin)).unwrap();
                                dbg!(repo_path(config, user, repo));

                                let mut cmd = Command::new(bin_path);
                                cmd.stdin(Stdio::piped());
                                cmd.stdout(Stdio::piped());
                                cmd.stderr(Stdio::piped());
                                cmd.arg(repo_path(config, user, repo));

                                child = Some(cmd.spawn().unwrap());
                                stdout = Some(child.as_mut().unwrap().stdout.take().unwrap());
                                stderr = Some(child.as_mut().unwrap().stderr.take().unwrap());
                                stdin = Some(child.as_mut().unwrap().stdin.take().unwrap());
                                dbg!(&bin, &user, &repo);
                            }
                            ChannelEvent::Close => break 'outer,
                            ChannelEvent::Data { data, is_stderr } => {
                                assert!(!is_stderr);
                                let Some(stdin) = &mut stdin else { continue };
                                let _ = stdin.write_all(&data).await;
                            }
                            _ => todo!(),
                        }
                    }
                }
            }
            // TODO: backpressure?
            n = some_await(stdout.as_mut().map(|stdout| stdout.read(&mut buf_stdout))), if stdout.is_some() => {
                let n = n.unwrap();
                if n == 0 {
                    // TODO: send channel eof
                }

                session.channel_state().unwrap().as_mut().write(&buf_stdout[..n], false).unwrap();
            }
            n = some_await(stderr.as_mut().map(|stderr| stderr.read(&mut buf_stderr))), if stderr.is_some() => {
                let n = n.unwrap();
                if n == 0 {
                    // TODO: send channel eof
                }

                session.channel_state().unwrap().as_mut().write(&buf_stderr[..n], true).unwrap();
            }
            status = some_await(child.as_mut().map(|child| child.wait())), if child.is_some() => {
                let status = status.unwrap();

                if let Some(code) = status.code() {
                    // TODO: send exit status
                }

                // TODO: handle exit signal
            }
        }
    }

    Ok(())
}

async fn some_await<Fut, U>(fut: Option<Fut>) -> U
where
    Fut: Future<Output = U>,
{
    match fut {
        Some(fut) => fut.await,
        None => panic!(),
    }
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
    let caps = re!(r#"^([a-zA-Z\-]+) '/([a-zA-Z0-9]+)/([a-zA-Z0-9\.]+)'$"#)
        .captures(command)
        .unwrap();

    let (_, [command, user, repo]) = caps.extract();
    if !matches!(command, "git-upload-pack") {
        panic!("unsupported command: {}", command);
    }

    (command, user, repo)
}
