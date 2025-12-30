use std::env;
use std::path::{Path, PathBuf};
use std::pin::pin;
use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;
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

    let mut child = None;

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
                                dbg!(&bin, &user, &repo);
                            }
                            ChannelEvent::Close => break 'outer,
                            _ => todo!(),
                        }
                    }
                }
            }

        }
    }

    Ok(())
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
