mod crates_io;
mod gitea;
mod github;
mod gitlab;

use anyhow::Result;
use reqwest::{header::HeaderMap, Client, IntoUrl};
use rustc_hash::FxHashMap;
use serde::Deserialize;
use tracing::warn;

use crate::{cfg::AccessTokens, prompt::Completion};

#[allow(clippy::enum_variant_names)]
#[derive(Debug, serde::Serialize, Deserialize)]
#[serde(tag = "fetcher", content = "args", rename_all = "camelCase")]
pub enum Fetcher {
    FetchCrate {
        pname: String,
    },
    #[serde(rename_all = "camelCase")]
    FetchFromGitHub {
        #[serde(default = "default_github_base")]
        github_base: String,
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
    FetchFromGitea {
        domain: String,
        owner: String,
        repo: String,
    },
}

fn default_github_base() -> String {
    "github.com".into()
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
    pub async fn create_client(&self, mut tokens: AccessTokens) -> Result<Client> {
        match self {
            Fetcher::FetchCrate { .. } => Ok(Client::new()),

            Fetcher::FetchFromGitHub { github_base, .. } => {
                let mut headers = HeaderMap::new();
                tokens.insert_header(&mut headers, github_base).await;
                Client::builder()
                    .user_agent("Mozilla/5.0")
                    .default_headers(headers)
                    .build()
                    .map_err(Into::into)
            }

            Fetcher::FetchFromGitLab { domain, .. } => {
                let mut headers = HeaderMap::new();
                tokens.insert_header(&mut headers, domain).await;
                Client::builder()
                    .default_headers(headers)
                    .build()
                    .map_err(Into::into)
            }

            Fetcher::FetchFromGitea { domain, .. } => {
                let mut headers = HeaderMap::new();
                tokens.insert_header(&mut headers, domain).await;
                Client::builder()
                    .default_headers(headers)
                    .build()
                    .map_err(Into::into)
            }
        }
    }

    pub async fn get_package_info(&self, cl: &Client) -> PackageInfo {
        match self {
            Fetcher::FetchCrate { pname } => crates_io::get_package_info(cl, pname).await,
            Fetcher::FetchFromGitHub {
                github_base,
                owner,
                repo,
            } => github::get_package_info(cl, github_base, owner, repo).await,
            Fetcher::FetchFromGitLab {
                domain,
                group,
                owner,
                repo,
            } => gitlab::get_package_info(cl, domain, group, owner, repo).await,
            Fetcher::FetchFromGitea {
                domain,
                owner,
                repo,
            } => gitea::get_package_info(cl, domain, owner, repo).await,
        }
    }

    pub async fn get_version(&self, cl: &Client, rev: &str) -> Option<Version> {
        match self {
            Fetcher::FetchCrate { .. } => Some(Version::Tag),
            Fetcher::FetchFromGitHub {
                github_base,
                owner,
                repo,
            } => github::get_version(cl, github_base, owner, repo, rev).await,
            Fetcher::FetchFromGitLab {
                domain,
                group,
                owner,
                repo,
            } => gitlab::get_version(cl, domain, group, owner, repo, rev).await,
            Fetcher::FetchFromGitea {
                domain,
                owner,
                repo,
            } => gitea::get_version(cl, domain, owner, repo, rev).await,
        }
    }
}

pub async fn json<T: for<'a> Deserialize<'a>>(cl: &Client, url: impl IntoUrl) -> Option<T> {
    cl.get(url)
        .send()
        .await
        .map_err(|e| {
            warn!("{e}");
        })
        .ok()?
        .json()
        .await
        .map_err(|e| {
            warn!("{e}");
        })
        .ok()
}
