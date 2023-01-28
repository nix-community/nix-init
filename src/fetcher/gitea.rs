use reqwest::Client;
use rustc_hash::FxHashMap;
use serde::Deserialize;

use std::collections::BTreeSet;

use crate::{
    fetcher::{json, PackageInfo, Revisions, Version},
    prompt::Completion,
};

#[derive(Deserialize)]
struct Repo {
    description: String,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
}

#[derive(Deserialize)]
struct Tag {
    name: String,
}

#[derive(Deserialize)]
struct Commit {
    sha: String,
    commit: CommitInfo,
}

#[derive(Deserialize)]
struct CommitInfo {
    committer: Committer,
    message: String,
}

#[derive(Deserialize)]
struct Committer {
    date: String,
}

pub async fn get_package_info(cl: &Client, domain: &str, owner: &str, repo: &str) -> PackageInfo {
    let root = format!("https://{domain}/api/v1/repos/{owner}/{repo}");

    let (description, latest_release, tags, commits) = tokio::join!(
        async {
            json(cl, &root)
                .await
                .map_or_else(String::new, |repo: Repo| repo.description)
        },
        async {
            json::<[Release; 1]>(cl, format!("{root}/releases?limit=1"))
                .await
                .map(|[release]| release.tag_name)
        },
        async { json::<Vec<_>>(cl, format!("{root}/tags?page=1&limit=12")).await },
        async { json::<Vec<_>>(cl, format!("{root}/commits?limit=12&stat=false")).await },
    );

    let mut completions = vec![];
    let mut versions = FxHashMap::default();

    let mut latest = if let Some(latest) = &latest_release {
        versions.insert(latest.clone(), Version::Latest);
        completions.push(Completion {
            display: format!("{latest} (latest release)"),
            replacement: latest.clone(),
        });
        latest.clone()
    } else {
        "".into()
    };

    if let Some(tags) = tags {
        if latest.is_empty() {
            if let Some(Tag { name }) = tags.first() {
                latest = name.clone();
            }
        }

        for Tag { name } in tags {
            if matches!(&latest_release, Some(tag) if tag == &name) {
                continue;
            }
            completions.push(Completion {
                display: format!("{name} (tag)"),
                replacement: name.clone(),
            });
            versions.insert(name, Version::Tag);
        }
    }

    if let Some(commits) = commits {
        let mut commits = commits.into_iter();

        if let Some(Commit { sha, commit }) = commits.next() {
            if latest.is_empty() {
                latest = sha.clone();
            }

            let date = &commit.committer.date[0 .. 10];
            let msg = commit.message.lines().next().unwrap_or_default();

            completions.push(Completion {
                display: format!("{sha} ({date} - HEAD) {msg}"),
                replacement: sha.clone(),
            });
            versions.insert(
                sha,
                Version::Head {
                    date: date.into(),
                    msg: msg.into(),
                },
            );
        }

        for Commit { sha, commit } in commits {
            let date = &commit.committer.date[0 .. 10];
            let msg = commit.message.lines().next().unwrap_or_default();
            completions.push(Completion {
                display: format!("{sha} ({date}) {msg}"),
                replacement: sha.clone(),
            });
            versions.insert(
                sha,
                Version::Commit {
                    date: date.into(),
                    msg: msg.into(),
                },
            );
        }
    };

    PackageInfo {
        pname: repo.into(),
        description,
        file_url_prefix: None,
        license: Vec::new(),
        python_dependencies: BTreeSet::new(),
        revisions: Revisions {
            latest,
            completions,
            versions,
        },
    }
}

pub async fn get_version(
    cl: &Client,
    domain: &str,
    owner: &str,
    repo: &str,
    rev: &str,
) -> Option<Version> {
    let Commit { sha, commit } = json(
        cl,
        format!("https://{domain}/api/v1/repos/{owner}/{repo}/git/commits/{rev}"),
    )
    .await?;

    Some(if sha.starts_with(rev) {
        Version::Commit {
            date: commit.committer.date[0 .. 10].into(),
            msg: "".into(),
        }
    } else {
        Version::Tag
    })
}
