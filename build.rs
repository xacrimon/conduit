use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    generate_css();
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
}
