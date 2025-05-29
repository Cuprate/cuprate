fn main() {
    set_commit_env();
}

/// This sets the git `COMMIT` environment variable.
fn set_commit_env() {
    const PATH: &str = "../.git/refs/heads/";

    println!("cargo:rerun-if-changed={PATH}");

    let commit = if let Ok(t) = std::env::var("GITHUB_SHA") {
        t
    } else {
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
    .trim()
    .to_lowercase();

    assert_eq!(
        commit.len(),
        40,
        "Commit hash should always be 40 bytes long."
    );

    println!("cargo:rustc-env=COMMIT={commit}");
}
