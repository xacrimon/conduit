use std::env;
use std::path::PathBuf;
use std::process::Command;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as BASE64_URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

fn main() {
    generate_css();
    generate_asset_map();
    println!("cargo:rustc-env=CONDUIT_VERSION={}", version());
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=styles");
}

fn git_hash() -> String {
    let output = Command::new("git")
        .args(["rev-parse", "--short=8", "HEAD"])
        .output()
        .unwrap();

    let raw = String::from_utf8(output.stdout).unwrap();
    raw.trim().to_owned()
}

fn version() -> String {
    let dirty_suffix = {
        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .output()
            .unwrap();

        if !output.stdout.is_empty() {
            "-dirty"
        } else {
            ""
        }
    };

    format!("dev-{}{}", git_hash(), dirty_suffix)
}

fn generate_css() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let css_path = PathBuf::from(&out_dir).join("index.css");
    let css_path_str = css_path.to_str().unwrap();

    let optimize = env::var("PROFILE").map(|p| p == "release").unwrap_or(false);
    let mut args = vec!["-i", "styles/index.css", "-o", &css_path_str];

    if optimize {
        args.push("-m");
    }

    Command::new("tailwindcss").args(&args).status().unwrap();

    let css_data = std::fs::read(&css_path).unwrap();
    let css_asset_name = compute_asset_name("index", "css", &css_data);
    println!("cargo:rustc-env=CONDUIT_CSS_ASSET_NAME={}", css_asset_name);
}

fn compute_asset_name(name: &str, extension: &str, data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let hash = hasher.finalize();
    let hash_b64 = BASE64_URL_SAFE_NO_PAD.encode(hash);

    format!("{}-{}.{}", name, hash_b64, extension)
}

fn generate_asset_map() {
    let mut map = Vec::new();

    for entry in WalkDir::new("public/assets")
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let path = entry.path();

            if path.starts_with("public/assets/lib") {
                continue;
            }

            let data = std::fs::read(path).unwrap();
            let name = path
                .strip_prefix("public/assets")
                .unwrap()
                .file_stem()
                .unwrap()
                .to_str()
                .unwrap();

            let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            let asset_name = compute_asset_name(name, extension, &data);
            map.push((format!("{}.{}", name, extension), asset_name));
        }
    }

    let out_dir = env::var("OUT_DIR").unwrap();
    let map_path = PathBuf::from(&out_dir).join("asset_map.txt");
    let map_data = map
        .into_iter()
        .map(|(name, asset_name)| format!("{}={}", name, asset_name))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(map_path, map_data).unwrap();
}
