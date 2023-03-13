mod deps;

use anyhow::{anyhow, Context, Result};
use cargo::{
    core::{
        registry::PackageRegistry,
        resolver::{CliFeatures, HasDevUnits},
        Shell, Workspace,
    },
    ops::{resolve_to_string, resolve_with_previous},
    util::homedir,
    Config,
};
use indoc::writedoc;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use regex::Regex;
use rustyline::{history::History, Editor, Helper};
use serde::Deserialize;
use std::process::Command;
use tracing::error;

use std::{
    collections::BTreeMap,
    fmt::{Display, Write},
    fs::File,
    io::{self, Read, Seek, Write as _},
    path::Path,
};

use crate::{
    inputs::AllInputs,
    lang::rust::deps::load_rust_depenendency,
    prompt::ask_overwrite,
    utils::{fod_hash, CommandExt, ResultExt, FAKE_HASH},
};

#[derive(Deserialize)]
pub struct CargoLock {
    package: Vec<CargoPackage>,
}

#[derive(Deserialize)]
struct CargoPackage {
    name: String,
    version: String,
    source: Option<String>,
}

pub async fn cargo_deps_hash(
    inputs: &mut AllInputs,
    pname: impl Display,
    version: impl Display,
    src: impl Display,
    src_dir: &Path,
    nixpkgs: &str,
) -> String {
    if let Ok(lock) = File::open(src_dir.join("Cargo.lock")) {
        let (hash, ()) = tokio::join!(
            fod_hash(format!(
                r#"(import({nixpkgs}){{}}).rustPlatform.fetchCargoTarball{{name="{pname}-{version}";src={src};hash="{FAKE_HASH}";}}"#,
            )),
            async {
                if let Some(lock) = parse_cargo_lock(lock).await {
                    load_rust_dependencies(inputs, &lock);
                }
            }
        );
        hash.unwrap_or_else(|| FAKE_HASH.into())
    } else {
        FAKE_HASH.into()
    }
}

pub async fn load_cargo_lock(
    editor: &mut Editor<impl Helper, impl History>,
    out_dir: &Path,
    inputs: &mut AllInputs,
    src_dir: &Path,
) -> Result<(bool, Option<CargoLock>)> {
    let target = &out_dir.join("Cargo.lock");
    let (missing, lock) = match File::open(target) {
        Ok(file) if ask_overwrite(editor, target)? => (
            !src_dir.join("Cargo.lock").exists(),
            parse_cargo_lock(file).await,
        ),
        _ => {
            if let Ok(mut lock) = File::open(src_dir.join("Cargo.lock")) {
                if let Err(e) = File::create(target).and_then(|mut target| {
                    let res = io::copy(&mut lock, &mut target);
                    lock.rewind()?;
                    res
                }) {
                    error!(
                        "{}",
                        anyhow!(e)
                            .context(format!("Failed to copy lock file to {}", target.display())),
                    );
                }
                (false, parse_cargo_lock(lock).await)
            } else {
                let lock = File::create(target)
                    .map_err(anyhow::Error::from)
                    .and_then(|mut target| {
                        let cfg = Config::new(
                            Shell::new(),
                            src_dir.into(),
                            homedir(src_dir).context("a")?,
                        );
                        let ws = Workspace::new(&src_dir.join("Cargo.toml"), &cfg)?;
                        let lock = resolve_to_string(
                            &ws,
                            &mut resolve_with_previous(
                                &mut PackageRegistry::new(&cfg)?,
                                &ws,
                                &CliFeatures::new_all(true),
                                HasDevUnits::Yes,
                                None,
                                None,
                                &[],
                                true,
                            )?,
                        )?;

                        write!(target, "{lock}")?;
                        toml::from_str(&lock).map_err(Into::into)
                    })
                    .map_err(|e| {
                        error!(
                            "{}",
                            e.context(format!(
                                "Failed to generate lock file to {}",
                                target.display(),
                            ))
                        );
                    })
                    .ok();
                (true, lock)
            }
        }
    };

    if let Some(lock) = &lock {
        load_rust_dependencies(inputs, lock);
    }

    Ok((missing, lock))
}

pub async fn write_cargo_lock(
    out: &mut impl Write,
    missing: bool,
    lock: Option<CargoLock>,
) -> Result<()> {
    writeln!(out, "{{\n    lockFile = ./Cargo.lock;")?;

    if let (Some(lock), Some(re)) = (
        lock,
        Regex::new(r"^git\+([^?]+)(\?(rev|tag|branch)=(.*))?#(.*)$").ok_warn(),
    ) {
        let hashes: BTreeMap<_, _> = lock
            .package
            .into_par_iter()
            .filter_map(|pkg| {
                let source = pkg.source?;
                let m = re.captures(&source)?;
                let hash = Command::new("nurl")
                    .arg(m.get(1)?.as_str())
                    .arg(m.get(5)?.as_str())
                    .arg("-Hf")
                    .arg("fetchgit")
                    .get_stdout()
                    .ok_warn()?;
                let hash = String::from_utf8(hash).ok_warn()?;
                Some((format!("{}-{}", pkg.name, pkg.version), hash))
            })
            .collect();

        if !hashes.is_empty() {
            writeln!(out, "    outputHashes = {{")?;
            for (name, hash) in hashes {
                writeln!(out, r#"      "{name}" = "{hash}";"#)?;
            }
            writeln!(out, "    }};")?;
        }
    }

    writeln!(out, "  }};\n")?;

    if missing {
        write!(out, "  ")?;
        writedoc!(
            out,
            "
                postPatch = ''
                    ln -s ${{./Cargo.lock}} Cargo.lock
                  '';

            ",
        )?;
    }

    Ok(())
}

async fn parse_cargo_lock(mut file: impl Read) -> Option<CargoLock> {
    let mut buf = String::new();
    file.read_to_string(&mut buf).ok_warn()?;
    toml::from_str(&buf).ok_warn()
}

fn load_rust_dependencies(inputs: &mut AllInputs, lock: &CargoLock) {
    for pkg in &lock.package {
        load_rust_depenendency(inputs, pkg);
    }
}
