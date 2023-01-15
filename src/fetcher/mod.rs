mod crates_io;
mod github;
mod gitlab;

use reqwest::{Client, IntoUrl};
use rustc_hash::FxHashMap;
use serde::Deserialize;

use crate::prompt::Completion;

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Deserialize)]
#[serde(tag = "fetcher", content = "args", rename_all = "camelCase")]
pub enum Fetcher {
    FetchCrate {
        pname: String,
    },
    FetchFromGitHub {
        owner: String,
        repo: String,
    },
    FetchFromGitLab {
        #[serde(default = "default_gitlab_domain")]
        domain: String,
        group: Option<String>,
        owner: String,
        repo: String,
    },
}

fn default_gitlab_domain() -> String {
    "gitlab.com".into()
}

pub enum Version {
    Latest,
    Tag,
    Head { date: String, msg: String },
    Commit { date: String, msg: String },
}

pub struct Revisions {
    pub latest: String,
    pub completions: Vec<Completion>,
    pub versions: FxHashMap<String, Version>,
}

pub struct PackageInfo {
    pub pname: String,
    pub revisions: Revisions,
    pub description: String,
}

impl Fetcher {
    pub async fn get_package_info(&self, cl: &Client) -> PackageInfo {
        match self {
            Fetcher::FetchCrate { pname } => crates_io::get_package_info(cl, pname).await,
            Fetcher::FetchFromGitHub { owner, repo } => {
                github::get_package_info(cl, owner, repo).await
            }
            Fetcher::FetchFromGitLab {
                domain,
                group,
                owner,
                repo,
            } => gitlab::get_package_info(cl, domain, group, owner, repo).await,
        }
    }

    pub async fn get_version(&self, cl: &Client, rev: &str) -> Option<Version> {
        match self {
            Fetcher::FetchCrate { .. } => Some(Version::Tag),
            Fetcher::FetchFromGitHub { owner, repo } => {
                github::get_version(cl, owner, repo, rev).await
            }
            Fetcher::FetchFromGitLab {
                domain,
                group,
                owner,
                repo,
            } => gitlab::get_version(cl, domain, group, owner, repo, rev).await,
        }
    }
}

pub async fn json<T: for<'a> Deserialize<'a>>(cl: &Client, url: impl IntoUrl) -> Option<T> {
    cl.get(url).send().await.ok()?.json().await.ok()
}
