mod readline;

use std::path::{Path, PathBuf};

use anyhow::Result;
use enum_dispatch::enum_dispatch;

use crate::{
    builder::Builder,
    fetcher::{Revisions, Version},
    frontend::readline::Readline,
};

#[enum_dispatch]
pub trait Frontend {
    fn url(&mut self) -> Result<String>;

    fn rev(&mut self, revs: Option<Revisions>) -> Result<(String, Option<Version>)>;

    fn fetch_submodules(&mut self) -> Result<bool>;

    fn version(&mut self, version: &str) -> Result<String>;

    fn pname(&mut self, pname: Option<String>) -> Result<String>;

    fn builder(&mut self, builders: Vec<Builder>) -> Result<Builder>;

    fn output(&mut self, pname: &str, builder: &Builder) -> Result<PathBuf>;

    fn overwrite(&mut self, path: &Path) -> Result<bool>;
}

#[enum_dispatch(Frontend)]
pub enum FrontendDispatch {
    Readline(Readline),
}

pub fn readline() -> Result<FrontendDispatch> {
    Readline::new().map(Into::into)
}
