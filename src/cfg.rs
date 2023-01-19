use anyhow::Result;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use rustc_hash::FxHashMap;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use tokio::process::Command;
use xdg::BaseDirectories;

use std::{fs, path::PathBuf, process::Output};

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
                match Command::new(cmd).args(args).output().await {
                    Ok(Output { status, stdout, .. }) if status.success() => {
                        let Ok(token) = String::from_utf8(stdout) else { return; };
                        format!("Bearer {}", token.trim())
                    }
                    _ => return,
                }
            }

            Some(AccessToken::File { file }) => {
                let Ok(token) = fs::read_to_string(file) else { return; };
                format!("Bearer {}", token.trim())
            }

            None => return,
        };

        let Ok(mut value) = HeaderValue::from_str(&value) else { return; };
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
        .map(|cfg| anyhow::Ok(toml::from_str(&fs::read_to_string(cfg)?)?))
        .transpose()?
        .unwrap_or_default())
}
