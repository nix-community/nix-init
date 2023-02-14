use reqwest::Client;
use rustyline::completion::Pair;
use serde::Deserialize;

use std::collections::BTreeSet;

use crate::{
    fetcher::{json, PackageInfo, Revisions, Version},
    license::parse_spdx_expression,
};

#[derive(Deserialize)]
struct CrateInfo {
    #[serde(rename = "crate")]
    krate: Crate,
    versions: Vec<CrateVersion>,
}

#[derive(Deserialize)]
struct Crate {
    description: String,
}

#[derive(Deserialize)]
struct CrateVersion {
    license: String,
    num: String,
    yanked: bool,
}

pub async fn get_package_info(cl: &Client, pname: &str) -> PackageInfo {
    let mut completions = Vec::new();
    let mut versions = Default::default();

    let Some(info) = json::<CrateInfo>(cl, format!("https://crates.io/api/v1/crates/{pname}")).await else {
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

    let mut crate_versions = info
        .versions
        .into_iter()
        .filter_map(
            |CrateVersion {
                 num,
                 yanked,
                 license,
             }| (!yanked).then_some((num, license)),
        )
        .peekable();

    let (mut found_latest, mut latest, mut latest_license) =
        if let Some((version, license)) = crate_versions.next() {
            completions.push(Pair {
                display: version.clone(),
                replacement: version.clone(),
            });
            versions.insert(version.clone(), Version::Tag);

            (
                version
                    .parse::<semver::Version>()
                    .map_or(false, |version| version.pre.is_empty()),
                version,
                Some(license),
            )
        } else {
            (false, "".into(), None)
        };

    for (version, license) in crate_versions {
        if !found_latest
            && version
                .parse::<semver::Version>()
                .map_or(false, |version| version.pre.is_empty())
        {
            found_latest = true;
            latest = version.clone();
            latest_license = Some(license);
        }

        completions.push(Pair {
            display: version.clone(),
            replacement: version.clone(),
        });
        versions.insert(version, Version::Tag);
    }

    PackageInfo {
        pname: pname.into(),
        description: info.krate.description,
        file_url_prefix: None,
        license: latest_license.map_or_else(Vec::new, |license| {
            parse_spdx_expression(&license, "crates.io")
        }),
        python_dependencies: BTreeSet::new(),
        revisions: Revisions {
            latest,
            completions,
            versions,
        },
    }
}
