use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use serde::Deserialize;
use tokio::fs;

#[derive(Deserialize)]
pub struct Config {
    pub database: Database,
}

impl Config {
    pub async fn load(path: Option<PathBuf>) -> Result<Self> {
        let base = env::current_dir()?;
        let path = match path {
            Some(p) => p,
            None => {
                let mut found = None;
                let list = ["config.toml", "config.example.toml"];

                for fragment in list {
                    let path = base.join(fragment);
                    if fs::try_exists(&path).await.unwrap_or(false) {
                        found = Some(path);
                        break;
                    }
                }

                let Some(path) = found else {
                    bail!("no configuration file found");
                };

                path
            }
        };

        Self::load_from_file(&path).await
    }

    async fn load_from_file(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path).await?;
        let config = toml::from_str(&contents)?;
        Ok(config)
    }
}

#[derive(Deserialize)]
pub struct Database {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
}
