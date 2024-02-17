fn main() {
    #[cfg(feature = "constants")]
    set_commit_env();
}

#[cfg(feature = "constants")]
/// This sets the git `COMMIT` environment variable.
fn set_commit_env() {
    const PATH: &str = "../.git/refs/heads/";

    println!("cargo:rerun-if-changed={PATH}");

    // FIXME: This could also be `std::fs::read({PATH}/{branch})`
    // so the machine building doesn't need `git`, although:
    // 1. Having `git` as a build dependency is probably ok
    // 2. It causes issues on PRs that aren't the `main` branch
    let output = std::process::Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .unwrap();

    let commit = std::str::from_utf8(&output.stdout)
        .unwrap()
        .trim()
        .to_lowercase();

    // Commit hash should always be 40 bytes long.
    assert_eq!(commit.as_bytes().len(), 40);

    println!("cargo:rustc-env=COMMIT={commit}");
}
