use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    let text = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    (output.status.success() && !text.is_empty()).then_some(text)
}

fn watch(git_path: &str) {
    if let Some(path) = git(&["rev-parse", "--git-path", git_path]) {
        if std::path::Path::new(&path).exists() {
            println!("cargo:rerun-if-changed={path}");
        }
    }
}

fn main() {
    println!("cargo:rerun-if-env-changed=BUILD_SHA");
    watch("HEAD");
    watch("packed-refs");
    if let Some(branch) = git(&["symbolic-ref", "-q", "HEAD"]) {
        watch(&branch);
    }
    let sha = std::env::var("BUILD_SHA")
        .ok()
        .filter(|sha| !sha.is_empty())
        .or_else(|| git(&["rev-parse", "--short=7", "HEAD"]));
    if let Some(sha) = sha {
        println!("cargo:rustc-env=TAWKS_GIT_SHA={sha}");
    }
}
