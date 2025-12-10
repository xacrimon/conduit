use std::env;
use std::path::{Path, PathBuf};

use tokio::{fs, process};

use crate::libssh::{ChannelEvent, Session};
use crate::utils::re;

pub async fn handle_session(mut session: Session) -> anyhow::Result<()> {
    session.configure();
    session.handle_key_exchange().await.unwrap();
    session.authenticate().await.unwrap();

    'outer: loop {
        session.wait().await.unwrap();

        if let Some(mut channel_state) = session.channel_state() {
            while let Some(event) = channel_state.as_mut().events().pop_front() {
                dbg!(&event);

                match event {
                    ChannelEvent::ExeqRequest { command } => {
                        let (bin, user, repo) = parse_command(&command);
                        dbg!(&bin, &user, &repo);
                    }
                    ChannelEvent::Close => break 'outer,
                    _ => todo!(),
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

fn parse_command(command: &str) -> (&str, &str, &str) {
    let caps = re!(r#"^([a-zA-Z\-]+) '/([a-zA-Z0-9]+)/([a-zA-Z0-9\.]+)'$"#)
        .captures(command)
        .unwrap();

    let (_, [command, user, repo]) = caps.extract();
    (command, user, repo)
}
