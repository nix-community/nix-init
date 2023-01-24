use reqwest::Client;
use serde::Deserialize;

use crate::{
    fetcher::{json, PackageInfo, Revisions, Version},
    prompt::Completion,
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
    num: String,
    yanked: bool,
}

pub async fn get_package_info(cl: &Client, pname: &str) -> PackageInfo {
    let mut completions = Vec::new();
    let mut versions = Default::default();

    let Some(info) = json::<CrateInfo>(cl, format!("https://crates.io/api/v1/crates/{pname}")).await else {
        return PackageInfo {
            pname: pname.into(),
            file_url_prefix: None,
            revisions: Revisions {
                latest: "".into(),
                completions,
                versions,
            },
            description: "".into(),
        };
    };

    let mut crate_versions = info
        .versions
        .into_iter()
        .filter_map(|CrateVersion { num, yanked }| (!yanked).then_some(num))
        .peekable();

    let (mut found_latest, mut latest) = if let Some(version) = crate_versions.next() {
        completions.push(Completion {
            display: version.clone(),
            replacement: version.clone(),
        });
        versions.insert(version.clone(), Version::Tag);

        (
            version
                .parse::<semver::Version>()
                .map_or(false, |version| version.pre.is_empty()),
            version,
        )
    } else {
        (false, "".into())
    };

    for version in crate_versions {
        if !found_latest
            && version
                .parse::<semver::Version>()
                .map_or(false, |version| version.pre.is_empty())
        {
            found_latest = true;
            latest = version.clone();
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
        revisions: Revisions {
            latest,
            completions,
            versions,
        },
        description: info.krate.description,
    }
}
