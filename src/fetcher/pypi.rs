use itertools::Itertools;
use reqwest::Client;
use serde::Deserialize;
use serde_with::{serde_as, DefaultOnNull, Map};
use time::OffsetDateTime;

use std::collections::BTreeSet;

use crate::{
    fetcher::{json, PackageInfo, Revisions, Version},
    license::parse_spdx_expression,
    prompt::Completion,
    python::get_python_dependency,
};

#[serde_as]
#[derive(Deserialize)]
struct Project {
    info: Info,
    #[serde_as(as = "Map<_, _>")]
    releases: Vec<(String, Vec<Release>)>,
}

#[serde_as]
#[derive(Deserialize)]
struct Info {
    license: String,
    #[serde_as(as = "DefaultOnNull")]
    requires_dist: Vec<String>,
    summary: String,
    version: String,
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
            description: "".into(),
            file_url_prefix: None,
            license: Vec::new(),
            python_dependencies: BTreeSet::new(),
            revisions: Revisions {
                latest: "".into(),
                completions,
                versions,
            },
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
        description: project.info.summary,
        file_url_prefix: None,
        license: parse_spdx_expression(&project.info.license, "pypi"),
        python_dependencies: project
            .info
            .requires_dist
            .into_iter()
            .filter_map(get_python_dependency)
            .collect(),
        revisions: Revisions {
            latest: project.info.version,
            completions,
            versions,
        },
    }
}
