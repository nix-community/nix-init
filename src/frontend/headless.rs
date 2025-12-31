use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use tracing::error;

use crate::{
    builder::Builder,
    fetcher::{Revisions, Version},
    frontend::Frontend,
    utils::by_name_path,
};

pub struct Headless;

impl Frontend for Headless {
    fn url(&mut self) -> Result<String> {
        bail!("specifying a URL with --url is required in headless mode");
    }

    fn rev(&mut self, revs: Option<Revisions>) -> Result<(String, Option<Version>)> {
        Ok(if let Some(mut revs) = revs {
            (revs.latest.clone(), revs.versions.remove(&revs.latest))
        } else {
            (String::new(), None)
        })
    }

    fn fetch_submodules(&mut self) -> Result<bool> {
        Ok(true)
    }

    fn version(&mut self, version: &str) -> Result<String> {
        Ok(version.to_owned())
    }

    fn pname(&mut self, pname: Option<String>) -> Result<String> {
        Ok(pname.unwrap_or_default())
    }

    fn builder(&mut self, builders: Vec<Builder>) -> Result<Builder> {
        Ok(builders[0])
    }

    fn output(&mut self, pname: &str, builder: &Builder) -> Result<PathBuf> {
        Ok(match by_name_path(pname, builder) {
            Some(path) => path.into(),
            None => PathBuf::from("."),
        })
    }

    fn overwrite(&mut self, path: &Path) -> Result<bool> {
        error!(
            "path {} already exists, use --overwrite to overwrite to always overwrite files",
            path.display()
        );
        Ok(false)
    }
}
