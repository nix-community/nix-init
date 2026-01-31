use std::fmt::Write;

use anyhow::Result;
use reqwest::{Client, header::HeaderMap};
use rustc_hash::FxHashMap;
use rustyline::completion::Pair;
use serde::Deserialize;

use crate::{
    Revisions,
    cfg::AccessTokens,
    fetcher::{Fetcher, PackageInfo, Version, json, success},
};

#[derive(Debug, Deserialize)]
pub struct FetchFromGitLab {
    #[serde(default = "default_gitlab_domain")]
    domain: String,
    group: Option<String>,
    owner: String,
    repo: String,
}

#[derive(Deserialize)]
struct Repo {
    #[serde(default)]
    description: String,
}

#[derive(Deserialize)]
struct LatestRelease {
    tag_name: String,
}

#[derive(Deserialize)]
struct Tag {
    name: String,
}

#[derive(Deserialize)]
struct Commit {
    id: String,
    committed_date: String,
    title: String,
}

impl Fetcher for FetchFromGitLab {
    async fn create_client(&self, mut tokens: AccessTokens) -> Result<Client> {
        let mut headers = HeaderMap::new();
        tokens.insert_header(&mut headers, &self.domain).await;
        Client::builder()
            .default_headers(headers)
            .build()
            .map_err(Into::into)
    }

    async fn get_package_info(&self, cl: &Client) -> PackageInfo {
        let root = self.get_api_root();

        let (description, latest_release, tags, commits) = tokio::join!(
            async {
                json(cl, &root)
                    .await
                    .map_or_else(String::new, |repo: Repo| repo.description)
            },
            async {
                json(cl, format!("{root}/releases/permalink/latest"))
                    .await
                    .map(|latest_release: LatestRelease| latest_release.tag_name)
            },
            json::<Vec<_>>(cl, format!("{root}/repository/tags?per_page=12")),
            json::<Vec<_>>(cl, format!("{root}/repository/commits?per_page=12")),
        );

        let mut completions = vec![];
        let mut versions = FxHashMap::default();

        let mut latest = if let Some(latest) = &latest_release {
            versions.insert(latest.clone(), Version::Latest);
            completions.push(Pair {
                display: format!("{latest} (latest release)"),
                replacement: latest.clone(),
            });
            latest.clone()
        } else {
            "".into()
        };

        if let Some(tags) = tags {
            if latest.is_empty()
                && let Some(Tag { name }) = tags.first()
            {
                latest = name.clone();
            }

            for Tag { name } in tags {
                if matches!(&latest_release, Some(tag) if tag == &name) {
                    continue;
                }
                completions.push(Pair {
                    display: format!("{name} (tag)"),
                    replacement: name.clone(),
                });
                versions.insert(name, Version::Tag);
            }
        }

        if let Some(commits) = commits {
            let mut commits = commits.into_iter();

            if let Some(Commit {
                id,
                committed_date,
                title,
            }) = commits.next()
            {
                if latest.is_empty() {
                    latest = id.clone();
                }

                let date = &committed_date[0 .. 10];

                completions.push(Pair {
                    display: format!("{id} ({date} - HEAD) {title}"),
                    replacement: id.clone(),
                });
                versions.insert(
                    id,
                    Version::Head {
                        date: date.into(),
                        msg: title,
                    },
                );
            }

            for Commit {
                id,
                committed_date,
                title,
            } in commits
            {
                let date = &committed_date[0 .. 10];
                completions.push(Pair {
                    display: format!("{id} ({date}) {title}"),
                    replacement: id.clone(),
                });
                versions.insert(
                    id,
                    Version::Commit {
                        date: date.into(),
                        msg: title,
                    },
                );
            }
        };

        let mut homepage = format!("https://{}/", self.domain);
        if let Some(group) = &self.group {
            let _ = write!(homepage, "{group}/");
        }
        let _ = write!(homepage, "{}/{}", self.owner, self.repo);

        PackageInfo {
            pname: self.repo.clone(),
            description,
            file_url_prefix: Some(format!("{homepage}/-/blob/${{finalAttrs.src.rev}}/")),
            homepage,
            license: Vec::new(),
            python_dependencies: Default::default(),
            revisions: Revisions {
                latest,
                completions,
                versions,
            },
        }
    }

    async fn get_version(&self, cl: &Client, rev: &str) -> Option<Version> {
        let Commit {
            id, committed_date, ..
        } = json(
            cl,
            format!("{}/repository/commits/{rev}", self.get_api_root()),
        )
        .await?;

        Some(if id.starts_with(rev) {
            Version::Commit {
                date: committed_date[0 .. 10].into(),
                msg: "".into(),
            }
        } else {
            Version::Tag
        })
    }

    async fn has_submodules(&self, cl: &Client, rev: &str) -> bool {
        success(
            cl,
            format!(
                "{}/repository/files/.gitmodules/raw?ref={rev}",
                self.get_api_root(),
            ),
        )
        .await
    }
}

impl FetchFromGitLab {
    fn get_api_root(&self) -> String {
        let mut root = format!("https://{}/api/v4/projects/", self.domain);
        if let Some(group) = &self.group {
            let _ = write!(root, "{}%2F", group.replace("/", "%2F"));
        }
        let _ = write!(root, "{}%2F{}", &self.owner, &self.repo);
        root
    }
}

fn default_gitlab_domain() -> String {
    "gitlab.com".into()
}
