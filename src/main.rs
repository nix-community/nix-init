mod cfg;
mod cli;
mod fetcher;
mod inputs;
mod licenses;
mod prompt;
mod python;
mod rust;
mod utils;

use anyhow::{Context, Result};
use askalono::{Match, Store, TextData};
use bstr::ByteVec;
use clap::Parser;
use expand::expand;
use flate2::read::GzDecoder;
use indoc::{formatdoc, writedoc};
use is_terminal::IsTerminal;
use once_cell::sync::Lazy;
use rustc_hash::FxHashMap;
use rustyline::{config::Configurer, CompletionType, Editor};
use serde::Deserialize;
use tar::Archive;
use tempfile::tempdir;
use tokio::process::Command;
use tracing::debug;
use tracing_subscriber::EnvFilter;

use std::{
    cmp::Ordering,
    fs::{create_dir_all, read_dir, read_to_string, File},
    io::{stderr, Write},
    path::PathBuf,
};

use crate::{
    cfg::load_config,
    cli::Opts,
    fetcher::{Fetcher, PackageInfo, Revisions, Version},
    inputs::{write_all_lambda_inputs, write_inputs, AllInputs},
    licenses::get_nix_licenses,
    prompt::{prompt, Prompter},
    python::Pyproject,
    rust::cargo_deps_hash,
    utils::{fod_hash, CommandExt, ResultExt, FAKE_HASH},
};

static LICENSE_STORE: Lazy<Option<Store>> = Lazy::new(|| {
    Store::from_cache(include_bytes!("../cache/askalono-cache.zstd") as &[_]).ok_warn()
});
static NIX_LICENSES: Lazy<FxHashMap<&'static str, &'static str>> = Lazy::new(get_nix_licenses);

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum MaybeFetcher {
    Known(Fetcher),
    Unknown { fetcher: String },
}

#[derive(Deserialize)]
struct BuildResult {
    outputs: Outputs,
}

#[derive(Deserialize)]
struct Outputs {
    out: String,
}

pub enum BuildType {
    BuildGoModule,
    BuildPythonPackage {
        application: bool,
        format: PythonFormat,
        cargo: bool,
    },
    BuildRustPackage,
    MkDerivation,
    MkDerivationCargo,
}

pub enum PythonFormat {
    Pyproject,
    Setuptools,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_ansi(stderr().is_terminal())
        .with_env_filter(EnvFilter::from_env("NIX_INIT_LOG"))
        .with_writer(stderr)
        .init();

    let opts = Opts::parse();

    tokio::spawn(async {
        Lazy::force(&LICENSE_STORE);
    });
    tokio::spawn(async {
        Lazy::force(&NIX_LICENSES);
    });

    let cfg = load_config(opts.config)?;

    if let Some(parent) = opts.output.parent() {
        let _ = create_dir_all(parent);
    }
    let mut out = File::create(opts.output).context("failed to create output file")?;
    writeln!(out, "{{ lib")?;

    let mut editor = Editor::new()?;
    editor.set_completion_type(CompletionType::Fuzzy);
    editor.set_max_history_size(0)?;

    let url = match opts.url {
        Some(url) => url,
        None => editor.readline(&prompt("Enter url"))?,
    };

    let fetcher = serde_json::from_slice(
        &Command::new("nurl")
            .arg(&url)
            .arg("-p")
            .get_stdout()
            .await?,
    )
    .context("failed to parse nurl output")?;

    let (pname, rev, version, desc, prefix) = if let MaybeFetcher::Known(fetcher) = &fetcher {
        let cl = fetcher.create_client(cfg.access_tokens).await?;

        let PackageInfo {
            pname,
            description,
            file_url_prefix,
            revisions,
        } = fetcher.get_package_info(&cl).await;

        let rev_msg = prompt(format_args!(
            "Enter tag or revision (defaults to {})",
            revisions.latest
        ));
        editor.set_helper(Some(Prompter::Revision(revisions)));

        let rev = editor.readline(&rev_msg)?;

        let Some(Prompter::Revision(revisions)) = editor.helper_mut() else {
                unreachable!();
            };
        let rev = if rev.is_empty() {
            revisions.latest.clone()
        } else {
            rev
        };

        let version = match match revisions.versions.remove(&rev) {
            Some(version) => Some(version),
            None => fetcher.get_version(&cl, &rev).await,
        } {
            Some(Version::Latest | Version::Tag) => {
                rev[rev.find(char::is_numeric).unwrap_or_default() ..].into()
            }
            Some(Version::Head { date, .. } | Version::Commit { date, .. }) => {
                format!("unstable-{date}")
            }
            None => "".into(),
        };

        editor.set_helper(Some(Prompter::NonEmpty));
        (
            Some(pname),
            rev,
            editor.readline_with_initial(&prompt("Enter version"), (&version, ""))?,
            description,
            file_url_prefix,
        )
    } else {
        let pname = url.parse::<url::Url>().ok_warn().and_then(|url| {
            url.path_segments()
                .and_then(|xs| xs.last())
                .map(|pname| pname.strip_suffix(".git").unwrap_or(pname).into())
        });

        editor.set_helper(Some(Prompter::NonEmpty));
        (
            pname,
            editor.readline(&prompt("Enter tag or revision"))?,
            editor.readline(&prompt("Enter version"))?,
            "".into(),
            None,
        )
    };

    let pname = if let Some(pname) = pname {
        editor.readline_with_initial(
            &prompt("Enter pname"),
            (
                &pname
                    .to_lowercase()
                    .replace(|c: char| c.is_ascii_punctuation(), "-"),
                "",
            ),
        )?
    } else {
        editor.readline(&prompt("Enter pname"))?
    };

    let mut cmd = Command::new("nurl");
    cmd.arg(&url).arg(&rev);

    let src_expr = {
        if let MaybeFetcher::Known(
            ref fetcher @ (Fetcher::FetchCrate { pname: ref name }
            | Fetcher::FetchPypi { pname: ref name }),
        ) = fetcher
        {
            let hash = String::from_utf8(cmd.arg("-H").get_stdout().await?)
                .context("failed to parse nurl output")?;
            let hash = hash.trim_end();

            let fetcher = if matches!(fetcher, Fetcher::FetchCrate { .. }) {
                "fetchCrate"
            } else {
                "fetchPypi"
            };

            if &pname == name {
                formatdoc!(
                    r#"
                        {fetcher} {{
                            inherit pname version;
                            hash = "{hash}";
                          }}"#,
                )
            } else {
                formatdoc!(
                    r#"
                        fetcher {{
                            pname = {name:?};
                            inherit version;
                            hash = "{hash}";
                          }}"#,
                )
            }
        } else {
            if rev == version {
                cmd.arg("-o").arg("rev").arg("version");
            } else if rev.contains(&version) {
                cmd.arg("-O")
                    .arg("rev")
                    .arg(rev.replacen(&version, "${version}", 1));
            }

            String::from_utf8(cmd.arg("-i").arg("2").get_stdout().await?)
                .context("failed to parse nurl output")?
        }
    };

    let stdout = Command::new("nix")
        .arg("build")
        .arg("--extra-experimental-features")
        .arg("nix-command")
        .arg("--impure")
        .arg("--no-link")
        .arg("--json")
        .arg("--expr")
        .arg(format!(
            "let pname={pname:?};version={version:?};in(import<nixpkgs>{{}}).{src_expr}"
        ))
        .get_stdout()
        .await?;

    let src = serde_json::from_slice::<Vec<BuildResult>>(&stdout)?
        .into_iter()
        .next()
        .context("failed to build source")?
        .outputs
        .out;

    let tmp;
    let src_dir = if let MaybeFetcher::Known(Fetcher::FetchPypi { ref pname }) = fetcher {
        let mut archive = Archive::new(GzDecoder::new(File::open(&src)?));

        tmp = tempdir().context("failed to create temporary directory")?;
        let tmp = tmp.path();
        debug!("{}", tmp.display());
        archive
            .unpack(tmp)
            .context("failed to unpack pypi package")?;

        tmp.join(format!("{pname}-{version}"))
    } else {
        PathBuf::from(&src)
    };

    let mut choices = Vec::new();
    let has_cargo = src_dir.join("Cargo.toml").is_file();
    let has_cmake = src_dir.join("CMakeLists.txt").is_file();
    let has_go = src_dir.join("go.mod").is_file();
    let has_meson = src_dir.join("meson.build").is_file();
    let pyproject = src_dir.join("pyproject.toml");
    let has_pyproject = pyproject.is_file();
    let has_setuptools = src_dir.join("setup.py").is_file();

    if has_go {
        choices.push((BuildType::BuildGoModule, "buildGoModule"));
    }

    if has_pyproject {
        if has_cargo {
            choices.extend([
                (
                    BuildType::BuildPythonPackage {
                        application: true,
                        format: PythonFormat::Pyproject,
                        cargo: true,
                    },
                    "buildPythonApplication - pyproject + cargoSetupHook",
                ),
                (
                    BuildType::BuildPythonPackage {
                        application: false,
                        format: PythonFormat::Pyproject,
                        cargo: true,
                    },
                    "buildPythonPackage - pyproject + cargoSetupHook",
                ),
            ]);
        }
        choices.extend([
            (
                BuildType::BuildPythonPackage {
                    application: true,
                    format: PythonFormat::Pyproject,
                    cargo: false,
                },
                "buildPythonApplication - pyproject",
            ),
            (
                BuildType::BuildPythonPackage {
                    application: false,
                    format: PythonFormat::Pyproject,
                    cargo: false,
                },
                "buildPythonPackage - pyproject",
            ),
        ]);
    }

    if has_setuptools {
        if has_cargo {
            choices.extend([
                (
                    BuildType::BuildPythonPackage {
                        application: true,
                        format: PythonFormat::Setuptools,
                        cargo: true,
                    },
                    "buildPythonApplication - setuptools + cargoSetupHook",
                ),
                (
                    BuildType::BuildPythonPackage {
                        application: false,
                        format: PythonFormat::Setuptools,
                        cargo: true,
                    },
                    "buildPythonPackage - setuptools + cargoSetupHook",
                ),
            ]);
        }
        choices.extend([
            (
                BuildType::BuildPythonPackage {
                    application: true,
                    format: PythonFormat::Setuptools,
                    cargo: false,
                },
                "buildPythonApplication - setuptools",
            ),
            (
                BuildType::BuildPythonPackage {
                    application: false,
                    format: PythonFormat::Setuptools,
                    cargo: false,
                },
                "buildPythonPackage - setuptools",
            ),
        ]);
    }

    if has_cargo {
        choices.extend(if has_meson {
            [
                (
                    BuildType::MkDerivationCargo,
                    "stdenv.mkDerivation + cargoSetupHook",
                ),
                (BuildType::BuildRustPackage, "buildRustPackage"),
            ]
        } else {
            [
                (BuildType::BuildRustPackage, "buildRustPackage"),
                (
                    BuildType::MkDerivationCargo,
                    "stdenv.mkDerivation + cargoSetupHook",
                ),
            ]
        });
    }

    choices.push((BuildType::MkDerivation, "stdenv.mkDerivation"));

    editor.set_helper(Some(Prompter::Build(choices)));
    let choice = editor.readline(&prompt("How should this package be built?"))?;
    let Some(Prompter::Build(choices)) = editor.helper_mut() else {
        unreachable!();
    };
    let (choice, _) = choice
        .parse()
        .ok()
        .and_then(|i: usize| choices.get(i))
        .unwrap_or_else(|| &choices[0]);

    let mut inputs = AllInputs::default();
    match choice {
        BuildType::BuildGoModule => {
            writeln!(out, ", buildGoModule")?;
        }
        BuildType::BuildPythonPackage { cargo, .. } => {
            writeln!(out, ", python3")?;
            if src_dir.join("poetry.lock").is_file() {
                inputs
                    .native_build_inputs
                    .always
                    .insert("python3.pkgs.poetry-core".into());
            }
            if *cargo {
                inputs.native_build_inputs.always.extend([
                    "rustPlatform.cargoSetupHook".into(),
                    "rustPlatform.rust.cargo".into(),
                    "rustPlatform.rust.rustc".into(),
                ]);
            }
        }
        BuildType::BuildRustPackage => {
            writeln!(out, ", rustPlatform")?;
        }
        BuildType::MkDerivation => {
            writeln!(out, ", stdenv")?;
            if has_cargo {
                inputs
                    .native_build_inputs
                    .always
                    .extend(["cargo".into(), "rustc".into()]);
            }
            if has_cmake {
                inputs.native_build_inputs.always.insert("cmake".into());
            }
            if has_meson {
                inputs
                    .native_build_inputs
                    .always
                    .extend(["meson".into(), "ninja".into()]);
            }
        }
        BuildType::MkDerivationCargo => {
            writeln!(out, ", stdenv")?;
            if has_cmake {
                inputs.native_build_inputs.always.insert("cmake".into());
            }
            if has_meson {
                inputs
                    .native_build_inputs
                    .always
                    .extend(["meson".into(), "ninja".into()]);
            }
            inputs.native_build_inputs.always.extend([
                "rustPlatform.cargoSetupHook".into(),
                "rustPlatform.rust.cargo".into(),
                "rustPlatform.rust.rustc".into(),
            ]);
        }
    }

    match fetcher {
        MaybeFetcher::Known(fetcher) => {
            writeln!(
                out,
                ", {}",
                match fetcher {
                    Fetcher::FetchCrate { .. } => "fetchCrate",
                    Fetcher::FetchFromGitHub { .. } => "fetchFromGitHub",
                    Fetcher::FetchFromGitLab { .. } => "fetchFromGitLab",
                    Fetcher::FetchFromGitea { .. } => "fetchFromGitea",
                    Fetcher::FetchPypi { .. } => "fetchPypi",
                },
            )?;
        }
        MaybeFetcher::Unknown { fetcher } => {
            writeln!(out, ", {fetcher}")?;
        }
    }

    let (native_build_inputs, build_inputs) = match choice {
        BuildType::BuildGoModule => {
            let hash = if src_dir.join("vendor").is_dir() {
                "null".into()
            } else if let Some(hash) = fod_hash(format!(
                r#"(import<nixpkgs>{{}}).buildGoModule{{pname={pname:?};version={version:?};src={src};vendorHash="{FAKE_HASH}";}}"#,
            )).await {
                if hash == "sha256-pQpattmS9VmO3ZIQUFn66az8GSmB4IvYhTTCFn6SUmo=" {
                    "null".into()
                } else {
                    format!(r#""{hash}""#)
                }
            } else {
                format!(r#""{FAKE_HASH}""#)
            };

            let res = write_all_lambda_inputs(&mut out, &inputs, ["rustPlatform"])?;
            writedoc!(
                out,
                r#"
                    }}:

                    buildGoModule rec {{
                      pname = {pname:?};
                      version = {version:?};

                      src = {src_expr};

                      vendorHash = {hash};

                "#,
            )?;
            res
        }

        BuildType::BuildPythonPackage {
            application,
            format,
            cargo,
        } => {
            let hash = if *cargo {
                Some(cargo_deps_hash(&mut inputs, &pname, &version, &src, &src_dir).await)
            } else {
                None
            };

            let res = write_all_lambda_inputs(&mut out, &inputs, ["python3"])?;

            writedoc!(
                out,
                r#"
                    }}:

                    python3.pkgs.buildPython{} rec {{
                      pname = {pname:?};
                      version = {version:?};
                      format = "{}";

                      src = {src_expr};

                "#,
                if *application {
                    "Application"
                } else {
                    "Package"
                },
                match format {
                    PythonFormat::Pyproject => "pyproject",
                    PythonFormat::Setuptools => "setuptools",
                },
            )?;

            if let Some(hash) = hash {
                write!(out, "  ")?;
                writedoc!(
                    out,
                    r#"
                        cargoDeps = rustPlatform.fetchCargoTarball {{
                            inherit src;
                            name = "${{pname}}-${{version}}";
                            hash = "{hash}";
                          }};

                    "#
                )?;
            }

            res
        }

        BuildType::BuildRustPackage => {
            let hash = cargo_deps_hash(&mut inputs, &pname, &version, &src, &src_dir).await;
            let res = write_all_lambda_inputs(&mut out, &inputs, ["rustPlatform"])?;
            writedoc!(
                out,
                r#"
                    }}:

                    rustPlatform.buildRustPackage rec {{
                      pname = {pname:?};
                      version = {version:?};

                      src = {src_expr};

                      cargoHash = "{hash}";

                "#,
            )?;
            res
        }

        BuildType::MkDerivation => {
            let res = write_all_lambda_inputs(&mut out, &inputs, ["stdenv"])?;
            writedoc!(
                out,
                r#"
                    }}:

                    stdenv.mkDerivation rec {{
                      pname = {pname:?};
                      version = {version:?};

                      src = {src_expr};

                "#,
            )?;
            res
        }

        BuildType::MkDerivationCargo => {
            let hash = cargo_deps_hash(&mut inputs, &pname, &version, &src, &src_dir).await;
            let res = write_all_lambda_inputs(&mut out, &inputs, ["stdenv"])?;
            writedoc!(
                out,
                r#"
                    }}:

                    stdenv.mkDerivation rec {{
                      pname = {pname:?};
                      version = {version:?};

                      src = {src_expr};

                      cargoDeps = rustPlatform.fetchCargoTarball {{
                        inherit src;
                        name = "${{pname}}-${{version}}";
                        hash = "{hash}";
                      }};

                "#,
            )?;
            res
        }
    };

    if native_build_inputs {
        write_inputs(&mut out, &inputs.native_build_inputs, "nativeBuildInputs")?;
    }
    if build_inputs {
        write_inputs(&mut out, &inputs.build_inputs, "buildInputs")?;
    }

    if let BuildType::BuildPythonPackage { format, .. } = choice {
        let name = match format {
            PythonFormat::Pyproject => {
                if let Some(mut pyproject) = Pyproject::from_path(pyproject) {
                    let mut deps = pyproject.get_dependencies().into_iter();

                    if let Some(dep) = deps.next() {
                        writeln!(out, "  propagatedBuildInputs = with python3.pkgs; [")?;
                        writeln!(out, "    {dep}")?;
                        for dep in deps {
                            writeln!(out, "    {dep}")?;
                        }
                        writeln!(out, "  ];\n")?;
                    }

                    pyproject.get_name().unwrap_or(pname)
                } else {
                    pname
                }
            }
            _ => pname, // unimplemented
        };

        writeln!(out, "  pythonImportsCheck = [ {name:?} ];\n")?;
    }

    let desc = desc.trim();
    write!(out, "  ")?;
    writedoc!(
        out,
        r"
            meta = with lib; {{
                description = {:?};
                homepage = {url:?};
        ",
        desc.strip_suffix('.').unwrap_or(desc),
    )?;

    if let Some(prefix) = prefix {
        if let Some(walk) = read_dir(&src_dir).ok_warn() {
            for entry in walk {
                let Ok(entry) = entry else { continue; };
                let path = entry.path();

                if !path.is_file() {
                    continue;
                }

                let name = entry.file_name();
                let Some(name) = name.to_str() else { continue; };
                if matches!(
                    name.to_ascii_lowercase().as_bytes(),
                    expand!([@b"changelog", ..] | [@b"changes", ..] | [@b"news"] | [@b"releases", ..]),
                ) {
                    writeln!(out, r#"    changelog = "{prefix}{name}";"#,)?;
                    break;
                }
            }
        }
    }

    if let (Some(store), Some(walk)) = (&*LICENSE_STORE, read_dir(src_dir).ok_warn()) {
        let nix_licenses = &*NIX_LICENSES;
        let mut licenses = Vec::new();

        for entry in walk {
            let Ok(entry) = entry else { continue; };
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            let name = entry.file_name();
            let name = <Vec<u8> as ByteVec>::from_os_str_lossy(&name);
            if !matches!(
                name.to_ascii_lowercase()[..],
                expand!([@b"license", ..] | [@b"licence", ..] | [@b"copying", ..]),
            ) {
                continue;
            }

            let Some(text) = read_to_string(path).ok_warn() else { continue; };
            let Match { score, name, .. } = store.analyze(&TextData::from(text));
            if let Some(license) = nix_licenses.get(name) {
                licenses.push((score, license));
            }
        }

        licenses.dedup_by_key(|(_, license)| *license);

        write!(out, "    license = ")?;
        if let [(_, license)] = &licenses[..] {
            write!(out, "licenses.{license}")?;
        } else {
            licenses.sort_unstable_by(|x, y| match x.0.partial_cmp(&y.0) {
                None | Some(Ordering::Equal) => x.1.cmp(y.1),
                Some(cmp) => cmp,
            });

            let n = match licenses.iter().position(|(score, _)| score < &0.75) {
                Some(0) => 1,
                Some(n) => n,
                None => licenses.len(),
            };

            write!(out, "with licenses; [ ")?;
            for (_, license) in licenses.into_iter().take(n) {
                write!(out, "{license} ")?;
            }
            write!(out, "]")?;
        }
    } else {
        write!(out, "    licenses = with licenses; [ ]")?;
    }

    write!(out, ";\n    maintainers = with maintainers; [ ")?;
    for maintainer in cfg.maintainers {
        write!(out, "{maintainer} ")?;
    }
    writeln!(out, "];\n  }};\n}}")?;

    Ok(())
}
