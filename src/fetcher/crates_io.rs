use anyhow::Result;
use reqwest::Client;
use rustyline::completion::Pair;
use serde::Deserialize;
use tracing::error;

use crate::{
    cfg::AccessTokens,
    fetcher::{Fetcher, PackageInfo, Revisions, Version, json},
    license::parse_spdx_expression,
};

#[derive(Debug, Deserialize)]
pub struct FetchCrate {
    pub pname: String,
}

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

impl Fetcher for FetchCrate {
    async fn create_client(&self, _: AccessTokens) -> Result<Client> {
        Client::builder()
            .user_agent("https://github.com/nix-community/nix-init")
            .build()
            .map_err(Into::into)
    }

    async fn get_package_info(&self, cl: &Client) -> PackageInfo {
        let mut completions = Vec::new();
        let mut versions = Default::default();

        let Some(info) = json::<CrateInfo>(
            cl,
            format!("https://crates.io/api/v1/crates/{}", self.pname),
        )
        .await
        else {
            return PackageInfo {
                pname: self.pname.clone(),
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
                        .is_ok_and(|version| version.pre.is_empty()),
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
                    .is_ok_and(|version| version.pre.is_empty())
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

        if !found_latest {
            error!("crate '{}' has no releases available", self.pname);
        }

        PackageInfo {
            pname: self.pname.clone(),
            description: info.krate.description,
            file_url_prefix: None,
            license: latest_license.map_or_else(Vec::new, |license| {
                parse_spdx_expression(&license, "crates.io")
            }),
            python_dependencies: Default::default(),
            revisions: Revisions {
                latest,
                completions,
                versions,
            },
        }
    }

    async fn get_version(&self, _: &Client, _: &str) -> Option<Version> {
        Some(Version::Tag)
    }

    async fn has_submodules(&self, _: &Client, _: &str) -> bool {
        false
    }
}
