//! GitHub API client.

use std::{collections::BTreeSet, time::Duration};

use chrono::{DateTime, Utc};
use serde::Deserialize;
use ureq::{Agent, AgentBuilder};

use crate::free::fmt_date;

pub struct CommitData {
    pub commit_msgs: Vec<String>,
    pub contributors: BTreeSet<String>,
}

pub struct GithubApiClient {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    agent: Agent,
}

impl GithubApiClient {
    const API: &str = "https://api.github.com/repos/Cuprate/cuprate";

    pub fn new(start_ts: u64, end_ts: u64) -> Self {
        let start_date = DateTime::from_timestamp(start_ts.try_into().unwrap(), 0).unwrap();
        let end_date = DateTime::from_timestamp(end_ts.try_into().unwrap(), 0).unwrap();

        Self {
            start_date,
            end_date,
            agent: AgentBuilder::new().build(),
        }
    }

    pub fn commit_data(&self) -> CommitData {
        #[derive(Deserialize)]
        struct Response {
            commit: Commit,
            /// When there is no GitHub author, [`Commit::author`] will be used.
            author: Option<Author>,
        }
        #[derive(Deserialize)]
        struct Author {
            login: String,
        }

        #[derive(Deserialize)]
        struct Commit {
            message: String,
            author: CommitAuthor,
        }

        #[derive(Deserialize)]
        struct CommitAuthor {
            name: String,
        }

        let mut url = format!(
            "{}/commits?per_page=100?since={}&until={}",
            Self::API,
            fmt_date(&self.start_date),
            fmt_date(&self.end_date)
        );

        let mut responses = Vec::new();

        // GitHub will split up large responses, so we must make multiple calls:
        // <https://docs.github.com/en/rest/using-the-rest-api/using-pagination-in-the-rest-api>.
        loop {
            let r = self.agent.get(&url).call().unwrap();

            let link = r
                .header("link")
                .map_or_else(String::new, ToString::to_string);

            responses.extend(r.into_json::<Vec<Response>>().unwrap());

            if !link.contains(r#"rel="next""#) {
                break;
            }

            url = link
                .split_once("<")
                .unwrap()
                .1
                .split_once(">")
                .unwrap()
                .0
                .to_string();

            std::thread::sleep(Duration::from_secs(1));
        }

        let (mut commits, authors): (Vec<String>, Vec<String>) = responses
            .into_iter()
            .map(|r| {
                (
                    r.commit.message,
                    r.author.map_or(r.commit.author.name, |a| a.login),
                )
            })
            .collect();

        // Extract contributors.
        let contributors = authors.into_iter().collect::<BTreeSet<String>>();

        // Extract commit msgs.
        commits.sort();
        let commit_msgs = commits
            .into_iter()
            .map(|c| {
                // The commit message may be separated by `\n` due to
                // subcommits being included in squashed GitHub PRs.
                //
                // This extracs the first, main message.
                c.lines().next().unwrap().to_string()
            })
            .collect();

        CommitData {
            commit_msgs,
            contributors,
        }
    }
}
