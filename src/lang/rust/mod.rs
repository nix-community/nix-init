mod deps;
#[cfg(test)]
mod tests;

use std::{
    collections::BTreeMap,
    fmt::{Display, Write},
    fs::File,
    io::{self, Write as _},
    path::Path,
    process::Command,
};

use anyhow::{anyhow, Context, Result};
use cargo::{
    core::{
        registry::PackageRegistry,
        resolver::{CliFeatures, HasDevUnits},
        Resolve, Shell, Workspace,
    },
    ops::{load_pkg_lockfile, resolve_to_string, resolve_with_previous},
    util::homedir,
    Config,
};
use indoc::writedoc;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rustc_hash::FxHashMap;
use rustyline::{history::History, Editor};
use tracing::error;

use crate::{
    cmd::NURL,
    inputs::AllInputs,
    lang::rust::deps::load_rust_dependency,
    prompt::{ask_overwrite, Prompter},
    utils::{fod_hash, CommandExt, ResultExt, FAKE_HASH},
};

// &mut is required, clippy incorrectly warns about it
#[allow(clippy::needless_pass_by_ref_mut)]
pub async fn cargo_deps_hash(
    inputs: &mut AllInputs,
    pname: impl Display,
    version: impl Display,
    src: impl Display,
    src_dir: &Path,
    has_cargo_lock: bool,
    nixpkgs: &str,
) -> String {
    if has_cargo_lock {
        let (hash, _) = tokio::join!(
            fod_hash(format!(
                r#"(import({nixpkgs}){{}}).rustPlatform.fetchCargoTarball{{name="{pname}-{version}";src={src};hash="{FAKE_HASH}";}}"#,
            )),
            async {
                if let Some(lock) = resolve_workspace(src_dir) {
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
    editor: &mut Editor<Prompter, impl History>,
    out_dir: &Path,
    inputs: &mut AllInputs,
    src_dir: &Path,
) -> Result<Option<Resolve>> {
    let target = &out_dir.join("Cargo.lock");
    let resolve = match File::open(target) {
        Ok(_) if ask_overwrite(editor, target)? => resolve_workspace(src_dir),
        _ => {
            if let Ok(mut lock) = File::open(src_dir.join("Cargo.lock")) {
                if let Err(e) =
                    File::create(target).and_then(|mut target| io::copy(&mut lock, &mut target))
                {
                    error!(
                        "{}",
                        anyhow!(e)
                            .context(format!("Failed to copy lock file to {}", target.display())),
                    );
                }

                resolve_workspace(src_dir)
            } else {
                File::create(target)
                    .map_err(anyhow::Error::from)
                    .and_then(|mut target| {
                        let cfg = cargo_config(src_dir)?;
                        let ws = Workspace::new(&src_dir.join("Cargo.toml"), &cfg)?;
                        let mut resolve = resolve_with_previous(
                            &mut PackageRegistry::new(&cfg)?,
                            &ws,
                            &CliFeatures::new_all(true),
                            HasDevUnits::Yes,
                            None,
                            None,
                            &[],
                            true,
                            None,
                        )?;
                        write!(target, "{}", resolve_to_string(&ws, &mut resolve)?)?;
                        Ok(resolve)
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
                    .ok()
            }
        }
    };

    if let Some(lock) = &resolve {
        load_rust_dependencies(inputs, lock);
    }

    Ok(resolve)
}

pub async fn write_cargo_lock(
    out: &mut impl Write,
    has_cargo_lock: bool,
    resolve: Option<Resolve>,
) -> Result<()> {
    writeln!(out, "{{\n    lockFile = ./Cargo.lock;")?;

    if let Some(resolve) = resolve {
        let mut revs = FxHashMap::default();
        for (k, v) in resolve.iter().filter_map(|pkg| {
            let src = pkg.source_id();
            src.is_git().then_some((src.precise()?, pkg))
        }) {
            revs.entry(k).or_insert(v);
        }

        let hashes: BTreeMap<_, _> = revs
            .into_par_iter()
            .filter_map(|(rev, pkg)| {
                let hash = Command::new(NURL)
                    .arg(pkg.source_id().as_url().to_string())
                    .arg(rev)
                    .arg("-Hf")
                    .arg("fetchgit")
                    .get_stdout()
                    .ok_error()?;
                let hash = String::from_utf8(hash).ok_warn()?;
                Some((format!("{}-{}", pkg.name(), pkg.version()), hash))
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

    if !has_cargo_lock {
        write!(out, "  ")?;
        writedoc! {out, "
            postPatch = ''
                ln -s ${{./Cargo.lock}} Cargo.lock
              '';

        "}?;
    }

    Ok(())
}

fn resolve_workspace(src_dir: &Path) -> Option<Resolve> {
    let mut cfg = cargo_config(src_dir).ok_error()?;
    cfg.configure(0, false, None, false, true, false, &None, &[], &[])
        .ok_error()?;

    let ws = Workspace::new(&src_dir.join("Cargo.toml"), &cfg).ok_error()?;
    let lock = load_pkg_lockfile(&ws).ok_error()?;
    let mut registry = PackageRegistry::new(&cfg).ok_error()?;

    resolve_with_previous(
        &mut registry,
        &ws,
        &CliFeatures::new_all(true),
        HasDevUnits::Yes,
        Some(&lock?),
        None,
        &[],
        true,
        None,
    )
    .ok_error()
}

fn cargo_config(src_dir: &Path) -> Result<Config> {
    Ok(Config::new(
        Shell::new(),
        src_dir.into(),
        homedir(src_dir).context("failed to find cargo home")?,
    ))
}

fn load_rust_dependencies(inputs: &mut AllInputs, resolve: &Resolve) {
    for pkg in resolve.iter() {
        load_rust_dependency(inputs, resolve, pkg);
    }
}
