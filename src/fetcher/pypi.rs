use itertools::Itertools;
use reqwest::Client;
use serde::Deserialize;
use serde_with::{serde_as, Map};
use time::OffsetDateTime;

use crate::{
    fetcher::{json, PackageInfo, Revisions, Version},
    license::parse_spdx_expression,
    prompt::Completion,
};

#[serde_as]
#[derive(Deserialize)]
struct Project {
    info: Info,
    #[serde_as(as = "Map<_, _>")]
    releases: Vec<(String, Vec<Release>)>,
}

#[derive(Deserialize)]
struct Info {
    version: String,
    summary: String,
    license: String,
}

#[derive(Deserialize)]
struct Release {
    #[serde(rename = "upload_time_iso_8601", with = "time::serde::iso8601")]
    upload_time: OffsetDateTime,
    yanked: bool,
}

pub async fn get_package_info(cl: &Client, pname: &str) -> PackageInfo {
    let mut completions = Vec::new();
    let mut versions = Default::default();

    let Some(project) = json::<Project>(cl, format!("https://pypi.org/pypi/{pname}/json")).await else {
        return PackageInfo {
            pname: pname.into(),
            file_url_prefix: None,
            license: Vec::new(),
            revisions: Revisions {
                latest: "".into(),
                completions,
                versions,
            },
            description: "".into(),
        };
    };

    versions.insert(project.info.version.clone(), Version::Latest);
    completions.push(Completion {
        display: format!("{} (latest release)", project.info.version),
        replacement: project.info.version.clone(),
    });

    for version in project
        .releases
        .into_iter()
        .filter_map(|(version, releases)| {
            releases.into_iter().find_map(|release| {
                (!release.yanked).then_some((version.clone(), release.upload_time))
            })
        })
        .sorted_unstable_by_key(|(_, time)| *time)
        .map(|(version, _)| version)
        .rev()
    {
        if version == project.info.version {
            continue;
        }
        completions.push(Completion {
            display: version.clone(),
            replacement: version.clone(),
        });
        versions.insert(version, Version::Tag);
    }

    PackageInfo {
        pname: pname.into(),
        file_url_prefix: None,
        license: parse_spdx_expression(&project.info.license, "pypi"),
        revisions: Revisions {
            latest: project.info.version,
            completions,
            versions,
        },
        description: project.info.summary,
    }
}
