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
use paste::paste;
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
    inputs::{get_riff_registry, AllInputs, RiffRegistry},
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
        let (hash, lock, registry) = tokio::join!(
            fod_hash(format!(
                r#"(import({nixpkgs}){{}}).rustPlatform.fetchCargoTarball{{name="{pname}-{version}";src={src};hash="{FAKE_HASH}";}}"#,
            )),
            parse_cargo_lock(lock),
            get_riff_registry(),
        );
        if let (Some(lock), Some(registry)) = (lock, registry) {
            load_riff_dependencies(inputs, &lock, registry);
        }
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
        if let Some(registry) = get_riff_registry().await {
            load_riff_dependencies(inputs, lock, registry);
        }
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

fn load_riff_dependencies(inputs: &mut AllInputs, lock: &CargoLock, mut registry: RiffRegistry) {
    for dep in &lock.package {
        let Some(dep) = registry.language.rust.dependencies.remove(&dep.name) else {
            continue;
        };

        for input in dep.inputs.native_build_inputs {
            inputs.native_build_inputs.always.insert(input);
        }
        for input in dep.inputs.build_inputs {
            inputs.build_inputs.always.insert(input);
        }

        macro_rules! load {
            ($inputs:ident, $sys:ident) => {
                paste! {
                    for input in dep.targets.[<aarch64_ $sys>].$inputs {
                        if inputs.$inputs.always.contains(&input)
                            || inputs.$inputs.$sys.contains(&input) {
                            continue;
                        } else if inputs.$inputs.[<x86_64_ $sys>].remove(&input) {
                            inputs.$inputs.$sys.insert(input);
                        } else {
                            inputs.$inputs.[<aarch64_ $sys>].insert(input);
                        }
                    }

                    for input in dep.targets.[<x86_64_ $sys>].$inputs {
                        if inputs.$inputs.always.contains(&input)
                            || inputs.$inputs.$sys.contains(&input) {
                            continue;
                        } else if inputs.$inputs.[<aarch64_ $sys>].remove(&input) {
                            inputs.$inputs.$sys.insert(input);
                        } else {
                            inputs.$inputs.[<x86_64_ $sys>].insert(input);
                        }
                    }
                }
            };
        }

        load!(native_build_inputs, darwin);
        load!(native_build_inputs, linux);
        load!(build_inputs, darwin);
        load!(build_inputs, linux);
    }
}
