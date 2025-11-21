use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let output = Command::new("git")
        .args(&["rev-parse", "--short=8", "HEAD"])
        .output()
        .unwrap();

    let git_hash = String::from_utf8(output.stdout).unwrap();
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);

    let out_dir = env::var("OUT_DIR").unwrap();
    let css_path = PathBuf::from(&out_dir).join("index.css");
    let css_path_str = css_path.to_str().unwrap();

    Command::new("tailwindcss")
        .args(&["-i", "styles/index.css", "-o", &css_path_str])
        .status()
        .unwrap();

    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=styles");
}
