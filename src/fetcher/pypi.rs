use itertools::Itertools;
use reqwest::Client;
use rustyline::completion::Pair;
use serde::Deserialize;
use serde_with::{serde_as, DefaultOnNull, Map};
use time::OffsetDateTime;

use crate::{
    fetcher::{json, PackageInfo, PypiFormat, Revisions, Version},
    lang::python::get_python_dependencies,
    license::parse_spdx_expression,
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
    license: Option<String>,
    #[serde_as(as = "DefaultOnNull")]
    requires_dist: Vec<String>,
    summary: String,
}

#[derive(Deserialize)]
struct Release {
    filename: String,
    packagetype: String,
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
            python_dependencies: Default::default(),
            revisions: Revisions {
                latest: "".into(),
                completions,
                versions,
            },
        };
    };

    for (version, format) in project
        .releases
        .into_iter()
        .filter_map(|(version, releases)| {
            let mut zip = None;
            for release in releases {
                if release.yanked || release.packagetype != "sdist" {
                    continue;
                }
                if release.filename.ends_with(".tar.gz") {
                    return Some((version, release.upload_time, PypiFormat::TarGz));
                }
                if zip.is_none() && release.filename.ends_with(".zip") {
                    zip = Some(release.upload_time);
                }
            }
            zip.map(|time| (version, time, PypiFormat::Zip))
        })
        .sorted_unstable_by_key(|(_, time, _)| *time)
        .map(|(version, _, format)| (version, format))
        .rev()
    {
        completions.push(Pair {
            display: format!("{version} ({format})"),
            replacement: version.clone(),
        });
        versions.insert(version, Version::Pypi { format });
    }

    PackageInfo {
        pname: pname.into(),
        description: project.info.summary,
        file_url_prefix: None,
        license: project
            .info
            .license
            .map_or_else(Vec::new, |license| parse_spdx_expression(&license, "pypi")),
        python_dependencies: get_python_dependencies(project.info.requires_dist),
        revisions: Revisions {
            latest: completions
                .first()
                .map_or_else(String::new, |pair| pair.replacement.clone()),
            completions,
            versions,
        },
    }
}
