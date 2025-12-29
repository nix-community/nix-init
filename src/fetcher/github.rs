use reqwest::Client;
use rustc_hash::FxHashMap;
use rustyline::completion::Pair;
use serde::Deserialize;

use crate::fetcher::{json, success, PackageInfo, Revisions, Version};

#[derive(Deserialize)]
struct Repo {
    description: String,
}

#[derive(Deserialize)]
struct LatestRelease {
    tag_name: String,
}

#[derive(Deserialize)]
struct Reference {
    #[serde(rename = "ref")]
    reference: String,
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

pub async fn get_package_info(
    cl: &Client,
    github_base: &str,
    owner: &str,
    repo: &str,
) -> PackageInfo {
    let root = format!("https://api.{github_base}/repos/{owner}/{repo}");

    let (description, latest_release, tags, commits) = tokio::join!(
        async {
            json(cl, &root)
                .await
                .map_or_else(String::new, |repo: Repo| repo.description)
        },
        async {
            json(cl, format!("{root}/releases/latest"))
                .await
                .map(|latest_release: LatestRelease| latest_release.tag_name)
        },
        json::<Vec<_>>(cl, format!("{root}/git/matching-refs/tags/")),
        json::<Vec<_>>(cl, format!("{root}/commits?per_page=12")),
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
        let mut tags = tags
            .into_iter()
            .rev()
            .filter_map(|Reference { reference }| {
                reference.strip_prefix("refs/tags/").map(ToOwned::to_owned)
            })
            .take(12);

        if latest.is_empty() {
            if let Some(tag) = tags.next() {
                latest = tag.clone();
                completions.push(Pair {
                    display: format!("{tag} (tag)"),
                    replacement: tag.clone(),
                });
                versions.insert(tag, Version::Tag);
            }
        }

        for tag in tags {
            if matches!(&latest_release, Some(latest) if latest == &tag) {
                continue;
            }
            completions.push(Pair {
                display: format!("{tag} (tag)"),
                replacement: tag.clone(),
            });
            versions.insert(tag, Version::Tag);
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

            completions.push(Pair {
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
            completions.push(Pair {
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
        file_url_prefix: Some(format!(
            "https://{github_base}/{owner}/{repo}/blob/${{src.rev}}/",
        )),
        license: Vec::new(),
        python_dependencies: Default::default(),
        revisions: Revisions {
            latest,
            completions,
            versions,
        },
    }
}

pub async fn get_version(
    cl: &Client,
    github_base: &str,
    owner: &str,
    repo: &str,
    rev: &str,
) -> Option<Version> {
    let Commit { sha, commit } = json(
        cl,
        format!("https://api.{github_base}/repos/{owner}/{repo}/commits/{rev}"),
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

pub async fn has_submodules(
    cl: &Client,
    github_base: &str,
    owner: &str,
    repo: &str,
    rev: &str,
) -> bool {
    success(
        cl,
        format!("https://api.{github_base}/repos/{owner}/{repo}/contents/.gitmodules?ref={rev}"),
    )
    .await
}
