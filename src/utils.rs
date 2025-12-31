use std::{fmt::Display, future::Future, io::BufRead, pin::Pin, process::Output};

use anyhow::{Result, bail};
use tokio::process::Command;
use tracing::{error, info, warn};

use crate::{builder::Builder, cmd::NIX};

pub const FAKE_HASH: &str = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";

pub trait ResultExt {
    type Output;

    fn ok_warn(self) -> Option<Self::Output>;
    fn ok_error(self) -> Option<Self::Output>;
}

impl<T, E: Display> ResultExt for Result<T, E> {
    type Output = T;

    fn ok_warn(self) -> Option<Self::Output> {
        self.map_err(|e| warn!("{e}")).ok()
    }

    fn ok_error(self) -> Option<Self::Output> {
        self.map_err(|e| error!("{e}")).ok()
    }
}

pub trait CommandExt {
    type Output<'a, T: 'a>
    where
        Self: 'a;

    fn get_stdout(&mut self) -> Self::Output<'_, Result<Vec<u8>>>;
    fn run(&mut self) -> Self::Output<'_, Result<()>>;
}

impl CommandExt for Command {
    type Output<'a, T: 'a> = Pin<Box<dyn Future<Output = T> + 'a>>;

    fn get_stdout(&mut self) -> Self::Output<'_, Result<Vec<u8>>> {
        Box::pin(async move {
            info!("{:?}", &self);
            into_stdout(self.output().await?)
        })
    }

    fn run(&mut self) -> Self::Output<'_, Result<()>> {
        Box::pin(async move {
            info!("{:?}", &self);
            let status = self.status().await?;
            if !status.success() {
                bail!("command failed with {status}");
            }
            Ok(())
        })
    }
}

impl CommandExt for std::process::Command {
    type Output<'a, T: 'a> = T;

    fn get_stdout(&mut self) -> Self::Output<'_, Result<Vec<u8>>> {
        info!("{:?}", &self);
        into_stdout(self.output()?)
    }

    fn run(&mut self) -> Self::Output<'_, Result<()>> {
        info!("{:?}", &self);
        let status = self.status()?;
        if !status.success() {
            bail!("command failed with {status}");
        }
        Ok(())
    }
}

fn into_stdout(output: Output) -> Result<Vec<u8>> {
    if !output.status.success() {
        bail!(
            "command exited with {}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }

    Ok(output.stdout)
}

pub async fn fod_hash(expr: String) -> Option<String> {
    let mut cmd = Command::new(NIX);
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
        let Ok(line) = lines.next()? else {
            continue;
        };
        if !line.trim_start().starts_with("specified:") {
            continue;
        }
        let Ok(line) = lines.next()? else {
            continue;
        };
        let Some(hash) = line.trim_start().strip_prefix("got:") else {
            continue;
        };
        return Some(hash.trim().to_owned());
    }
}
