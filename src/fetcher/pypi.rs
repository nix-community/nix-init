use anyhow::Result;
use itertools::Itertools;
use reqwest::Client;
use rustyline::completion::Pair;
use serde::Deserialize;
use serde_with::{DefaultOnNull, Map, serde_as};
use time::OffsetDateTime;
use tracing::error;

use crate::{
    cfg::AccessTokens,
    fetcher::{Fetcher, PackageInfo, PypiFormat, Revisions, Version, json},
    lang::python::get_python_dependencies,
    license::parse_spdx_expression,
};

#[derive(Debug, Deserialize)]
pub struct FetchPypi {
    pub pname: String,
}

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

impl Fetcher for FetchPypi {
    async fn create_client(&self, _: AccessTokens) -> Result<Client> {
        Ok(Client::new())
    }

    async fn get_package_info(&self, cl: &Client) -> PackageInfo {
        let homepage = format!("https://pypi.org/project/{}", self.pname);

        let mut completions = Vec::new();
        let mut versions = Default::default();

        let Some(project) =
            json::<Project>(cl, format!("https://pypi.org/pypi/{}/json", self.pname)).await
        else {
            return PackageInfo {
                pname: self.pname.clone(),
                description: "".into(),
                file_url_prefix: None,
                homepage,
                license: Vec::new(),
                python_dependencies: Default::default(),
                revisions: Revisions {
                    latest: "".into(),
                    completions,
                    versions,
                },
            };
        };

        for (pname, version, format) in project
            .releases
            .into_iter()
            .filter_map(|(version, releases)| {
                let mut zip = None;
                for release in releases {
                    if release.yanked || release.packagetype != "sdist" {
                        continue;
                    }
                    if let Some(pname) = get_pname(&release, &version, ".tar.gz") {
                        return Some((
                            pname.into(),
                            version,
                            release.upload_time,
                            PypiFormat::TarGz,
                        ));
                    }
                    if zip.is_none()
                        && let Some(pname) = get_pname(&release, &version, ".zip")
                    {
                        zip = Some((pname.into(), release.upload_time));
                    }
                }
                zip.map(|(pname, time)| (pname, version, time, PypiFormat::Zip))
            })
            .sorted_unstable_by_key(|(_, _, time, _)| *time)
            .map(|(pname, version, _, format)| (pname, version, format))
            .rev()
        {
            completions.push(Pair {
                display: format!("{version} ({format})"),
                replacement: version.clone(),
            });
            versions.insert(version, Version::Pypi { pname, format });
        }

        if completions.is_empty() {
            error!(
                "pypi package '{}' has no source distribution files available",
                self.pname
            );
        }

        PackageInfo {
            pname: self.pname.clone(),
            description: project.info.summary,
            file_url_prefix: None,
            homepage,
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

    async fn get_version(&self, _: &Client, _: &str) -> Option<Version> {
        None
    }

    async fn has_submodules(&self, _: &Client, _: &str) -> bool {
        false
    }
}

fn get_pname<'a>(release: &'a Release, version: &str, ext: &'static str) -> Option<&'a str> {
    release
        .filename
        .strip_suffix(ext)?
        .strip_suffix(version)?
        .strip_suffix('-')
}

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;

    use super::{Release, get_pname};

    #[test]
    fn basic() {
        let release = Release {
            filename: "foo-bar-0.1.0.tar.gz".into(),
            packagetype: "sdist".into(),
            upload_time: OffsetDateTime::from_unix_timestamp(0).unwrap(),
            yanked: false,
        };

        assert_eq!(get_pname(&release, "0.1.0", ".tar.gz"), Some("foo-bar"));
        assert_eq!(get_pname(&release, "0.1.0", ".zip"), None);
        assert_eq!(get_pname(&release, "0.2.0", ".tar.gz"), None);
    }
}
