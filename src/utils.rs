use tokio::process::Command;
use tracing::warn;

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

pub async fn fod_hash(expr: String) -> Option<String> {
    let Output { stderr, status, .. } = Command::new("nix")
        .arg("build")
        .arg("--extra-experimental-features")
        .arg("nix-command")
        .arg("--impure")
        .arg("--no-link")
        .arg("--expr")
        .arg(expr)
        .output()
        .await
        .ok_warn()?;

    if status.success() {
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
