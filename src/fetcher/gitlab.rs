use reqwest::Client;
use rustc_hash::FxHashMap;
use rustyline::completion::Pair;
use serde::Deserialize;

use crate::{
    Revisions,
    fetcher::{PackageInfo, Version, json, success},
};

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

pub async fn get_package_info(
    cl: &Client,
    domain: &str,
    group: &Option<String>,
    owner: &str,
    repo: &str,
) -> PackageInfo {
    let root = get_api_root(domain, group, owner, repo);

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

    PackageInfo {
        pname: repo.into(),
        description,
        file_url_prefix: Some(format!(
            "https://{domain}/{owner}/{repo}/-/blob/${{src.rev}}/",
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
    domain: &str,
    group: &Option<String>,
    owner: &str,
    repo: &str,
    rev: &str,
) -> Option<Version> {
    let Commit {
        id, committed_date, ..
    } = json(
        cl,
        format!(
            "{}/repository/commits/{rev}",
            get_api_root(domain, group, owner, repo),
        ),
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

pub async fn has_submodules(
    cl: &Client,
    domain: &str,
    group: &Option<String>,
    owner: &str,
    repo: &str,
    rev: &str,
) -> bool {
    success(
        cl,
        format!(
            "{}/repository/files/.gitmodules/raw?ref={rev}",
            get_api_root(domain, group, owner, repo),
        ),
    )
    .await
}

fn get_api_root(domain: &str, group: &Option<String>, owner: &str, repo: &str) -> String {
    let mut root = format!("https://{domain}/api/v4/projects/");
    if let Some(group) = group {
        root.push_str(group);
        root.push_str("%2F")
    }
    root.push_str(owner);
    root.push_str("%2F");
    root.push_str(repo);
    root
}
