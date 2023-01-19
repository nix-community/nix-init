use anyhow::Result;
use serde::Deserialize;
use xdg::BaseDirectories;

use std::{fs, path::PathBuf};

#[derive(Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub maintainers: Vec<String>,
}

pub fn load_config(cfg: Option<PathBuf>) -> Result<Config> {
    Ok(cfg
        .or_else(|| {
            BaseDirectories::with_prefix("nix-init")
                .ok()
                .and_then(|dirs| dirs.find_config_file("config.toml"))
        })
        .map(|cfg| anyhow::Ok(toml::from_str(&fs::read_to_string(cfg)?)?))
        .transpose()?
        .unwrap_or_default())
}
