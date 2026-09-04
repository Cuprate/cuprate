fn main() {
    set_commit_env();
}

/// This sets the git `COMMIT` environment variable.
fn set_commit_env() {
    const PATH: &str = "../.git/refs/heads/";

    println!("cargo:rerun-if-changed={PATH}");
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");

    // Docker ARG/ENV without --build-arg is an empty string, not unset.
    let commit = match std::env::var("GITHUB_SHA") {
        Ok(t) if !t.trim().is_empty() => t,
        _ => git_head_commit(),
    }
    .trim()
    .to_lowercase();

    assert_eq!(
        commit.len(),
        40,
        "Commit hash should always be 40 bytes long."
    );

    println!("cargo:rustc-env=COMMIT={commit}");
}

/// Resolve `HEAD` via `git`. Used when `GITHUB_SHA` is missing or empty.
fn git_head_commit() -> String {
    // FIXME: This could also be `std::fs::read({PATH}/{branch})`
    // so the machine building doesn't need `git`, although:
    // 1. Having `git` as a build dependency is probably ok
    // 2. It causes issues on PRs that aren't the `main` branch
    String::from_utf8(
        std::process::Command::new("git")
            .args(["show", "-s", "--format=%H"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
}
