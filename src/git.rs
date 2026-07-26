use std::path::{Path, PathBuf};

use anyhow::Result;
use tokio::process::Command;

use crate::config::Config;

pub struct TreeEntry {
    pub kind: String,
    pub name: String,
}

pub fn repo_disk_path(config: &Config, username: &str, name: &str) -> PathBuf {
    config
        .git
        .repository_path
        .join(username)
        .join(format!("{}.git", name))
}

pub async fn init_bare_repo(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let output = Command::new("git")
        .args(["init", "--bare"])
        .arg(path)
        .output()
        .await?;

    if !output.status.success() {
        anyhow::bail!(
            "git init --bare failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

pub async fn is_empty(repo_path: &Path) -> bool {
    let output = Command::new("git")
        .arg("--git-dir")
        .arg(repo_path)
        .args(["rev-parse", "HEAD"])
        .output()
        .await;

    match output {
        Ok(o) => !o.status.success(),
        Err(_) => true,
    }
}

pub async fn ls_tree(repo_path: &Path, path: &str) -> Result<Vec<TreeEntry>> {
    let tree_ref = if path.is_empty() {
        "HEAD".to_owned()
    } else {
        format!("HEAD:{}", path)
    };

    let output = Command::new("git")
        .arg("--git-dir")
        .arg(repo_path)
        .args(["ls-tree", &tree_ref])
        .output()
        .await?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8(output.stdout)?;
    let mut entries: Vec<TreeEntry> = stdout
        .lines()
        .filter_map(|line| {
            let (meta, name) = line.split_once('\t')?;
            let parts: Vec<&str> = meta.split_whitespace().collect();
            if parts.len() != 3 {
                return None;
            }
            Some(TreeEntry {
                kind: parts[1].to_owned(),
                name: name.to_owned(),
            })
        })
        .collect();

    // Sort: directories first, then alphabetically
    entries.sort_by(|a, b| {
        let a_is_tree = a.kind == "tree";
        let b_is_tree = b.kind == "tree";
        b_is_tree.cmp(&a_is_tree).then(a.name.cmp(&b.name))
    });

    Ok(entries)
}

pub async fn show_blob(repo_path: &Path, path: &str) -> Result<Option<Vec<u8>>> {
    let output = Command::new("git")
        .arg("--git-dir")
        .arg(repo_path)
        .args(["show", &format!("HEAD:{}", path)])
        .output()
        .await?;

    if !output.status.success() {
        return Ok(None);
    }

    Ok(Some(output.stdout))
}
