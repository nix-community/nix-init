use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use rustc_hash::FxHashMap;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use tokio::process::Command;
use xdg::BaseDirectories;

use std::{fs, path::PathBuf};

use crate::utils::{CommandExt, ResultExt};

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Config {
    pub maintainers: Vec<String>,
    pub access_tokens: AccessTokens,
}

#[derive(Default, Deserialize)]
pub struct AccessTokens(FxHashMap<String, AccessToken>);

impl AccessTokens {
    pub async fn insert_header(&mut self, headers: &mut HeaderMap, host: &str) {
        let value = match self.0.get(host) {
            Some(AccessToken::Text(token)) => format!("Bearer {}", token.expose_secret()),

            Some(AccessToken::Command { command }) => {
                let mut args = command.iter();
                let Some(cmd) = args.next() else { return; };
                let Some(stdout) = Command::new(cmd).args(args).get_stdout().await.ok_warn() else {
                    return;
                };
                let Some(token) = String::from_utf8(stdout).ok_warn() else { return; };
                format!("Bearer {}", token.trim())
            }

            Some(AccessToken::File { file }) => {
                let Some(token) = fs::read_to_string(file).ok_warn() else { return; };
                format!("Bearer {}", token.trim())
            }

            None => return,
        };

        let Some(mut value) = HeaderValue::from_str(&value).ok_warn() else { return; };
        value.set_sensitive(true);
        headers.insert(AUTHORIZATION, value);
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum AccessToken {
    Text(SecretString),
    Command { command: Vec<String> },
    File { file: PathBuf },
}

pub fn load_config(cfg: Option<PathBuf>) -> Result<Config> {
    Ok(cfg
        .or_else(|| {
            BaseDirectories::with_prefix("nix-init")
                .ok()
                .and_then(|dirs| dirs.find_config_file("config.toml"))
        })
        .map(|cfg| {
            anyhow::Ok(
                toml::from_str(&fs::read_to_string(cfg).context("failed to read config file")?)
                    .context("failed to parse config file")?,
            )
        })
        .transpose()?
        .unwrap_or_default())
}
