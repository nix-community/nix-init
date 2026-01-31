mod crates_io;
mod gitea;
mod github;
mod gitlab;
mod pypi;

use anyhow::Result;
use enum_dispatch::enum_dispatch;
use parse_display::Display;
use reqwest::{Client, IntoUrl};
use rustc_hash::FxHashMap;
use rustyline::completion::Pair;
use serde::Deserialize;
use tracing::{error, warn};

use crate::{
    cfg::AccessTokens,
    fetcher::{
        crates_io::FetchCrate, gitea::FetchFromGitea, github::FetchFromGitHub,
        gitlab::FetchFromGitLab, pypi::FetchPypi,
    },
    lang::python::PythonDependencies,
    utils::ResultExt,
};

#[enum_dispatch]
pub trait Fetcher {
    async fn create_client(&self, tokens: AccessTokens) -> Result<Client>;

    async fn get_package_info(&self, cl: &Client) -> PackageInfo;

    async fn get_version(&self, cl: &Client, rev: &str) -> Option<Version>;

    async fn has_submodules(&self, cl: &Client, rev: &str) -> bool;
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Deserialize, Display)]
#[display("{}", style = "camelCase")]
#[enum_dispatch(Fetcher)]
#[serde(tag = "fetcher", content = "args", rename_all = "camelCase")]
pub enum FetcherDispatch {
    FetchCrate(FetchCrate),
    FetchFromGitHub(FetchFromGitHub),
    FetchFromGitLab(FetchFromGitLab),
    FetchFromGitea(FetchFromGitea),
    FetchPypi(FetchPypi),
}

pub enum Version {
    Latest,
    Tag,
    Pypi { pname: String, format: PypiFormat },
    Head { date: String, msg: String },
    Commit { date: String, msg: String },
}

#[derive(Display)]
pub enum PypiFormat {
    #[display("tar.gz")]
    TarGz,
    #[display("zip")]
    Zip,
}

pub struct Revisions {
    pub latest: String,
    pub completions: Vec<Pair>,
    pub versions: FxHashMap<String, Version>,
}

pub struct PackageInfo {
    pub pname: String,
    pub description: String,
    pub file_url_prefix: Option<String>,
    pub homepage: String,
    pub license: Vec<&'static str>,
    pub python_dependencies: PythonDependencies,
    pub revisions: Revisions,
}

pub async fn json<T: for<'a> Deserialize<'a>>(cl: &Client, url: impl IntoUrl) -> Option<T> {
    cl.get(url)
        .send()
        .await
        .ok_inspect(|e| error!("{e}"))?
        .json()
        .await
        .ok_inspect(|e| warn!("{e}"))
}

pub async fn success(cl: &Client, url: impl IntoUrl) -> bool {
    cl.get(url)
        .send()
        .await
        .is_ok_and(|resp| resp.status().is_success())
}
