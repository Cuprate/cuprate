//! Changelog generation.

use chrono::Utc;

use crate::{
    api::{CommitData, GithubApiClient},
    crates::CuprateCrates,
    free::fmt_date,
};

pub fn generate_changelog(
    crates: CuprateCrates,
    api: GithubApiClient,
    release_name: Option<String>,
) -> String {
    // This variable will hold the final output.
    let mut c = String::new();

    let CommitData {
        commit_msgs,
        contributors,
    } = api.commit_data();

    //----------------------------------------------------------------------------- Initial header.
    let cuprated_version = crates.crate_version("cuprated");
    let release_name = release_name.unwrap_or_else(|| "NAME_OF_METAL".to_string());
    let release_date = fmt_date(&Utc::now());

    c += &format!("# {cuprated_version} {release_name} ({release_date})\n");
    c += "DESCRIPTION ON CHANGES AND ANY NOTABLE INFORMATION RELATED TO THE RELEASE.\n\n";

    //----------------------------------------------------------------------------- Temporary area for commits.
    c += &format!(
        "## COMMIT LIST `{}` -> `{:?}` (SORT INTO THE BELOW CATEGORIES)\n",
        fmt_date(&api.start_date),
        api.end_date,
    );
    for commit_msg in commit_msgs {
        c += &format!("- {commit_msg}\n");
    }
    c += "\n";

    //----------------------------------------------------------------------------- `cuprated` changes.
    c += "## `cuprated`\n";
    c += "- Example change (#PR_NUMBER)\n";
    c += "\n";

    //----------------------------------------------------------------------------- Library changes.
    c += "## `cuprate_library`\n";
    c += "- Example change (#PR_NUMBER)\n";
    c += "\n";

    //----------------------------------------------------------------------------- Contributors footer.
    c += "## Contributors\n";
    c += "Thank you to everyone who contributed to this release:\n";

    for contributor in contributors {
        c += &format!("- @{contributor}\n");
    }

    c
}
