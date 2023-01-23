use anyhow::{bail, Result};
use async_trait::async_trait;
use tokio::process::Command;
use tracing::{info, warn};

use std::{fmt::Display, io::BufRead, process::Output};

pub const FAKE_HASH: &str = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";

pub trait ResultExt {
    type Output;

    fn ok_warn(self) -> Option<Self::Output>;
}

impl<T, E: Display> ResultExt for Result<T, E> {
    type Output = T;

    fn ok_warn(self) -> Option<Self::Output> {
        self.map_err(|e| warn!("{e}")).ok()
    }
}

#[async_trait]
pub trait CommandExt {
    async fn get_stdout(&mut self) -> Result<Vec<u8>>;
}

#[async_trait]
impl CommandExt for Command {
    async fn get_stdout(&mut self) -> Result<Vec<u8>> {
        info!("{:?}", &self);

        let Output {
            status,
            stdout,
            stderr,
        } = self.output().await?;

        if !status.success() {
            bail!(
                "command exited with {status}\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&stdout),
                String::from_utf8_lossy(&stderr),
            );
        }

        Ok(stdout)
    }
}

pub async fn fod_hash(expr: String) -> Option<String> {
    let mut cmd = Command::new("nix");
    cmd.arg("build")
        .arg("--extra-experimental-features")
        .arg("nix-command")
        .arg("--impure")
        .arg("--no-link")
        .arg("--expr")
        .arg(expr);

    info!("{cmd:?}");
    let Output { stderr, status, .. } = cmd.output().await.ok_warn()?;

    if status.success() {
        warn!("command succeeded unexpectedly");
        return None;
    }

    let mut lines = stderr.lines();
    loop {
        let Ok(line) = lines.next()? else { continue; };
        if !line.trim_start().starts_with("specified:") {
            continue;
        }
        let Ok(line) = lines.next()? else { continue; };
        let Some(hash) = line.trim_start().strip_prefix("got:") else {
            continue;
        };
        return Some(hash.trim().to_owned());
    }
}