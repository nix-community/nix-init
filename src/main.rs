mod build;
mod cfg;
mod cli;
mod fetcher;
mod inputs;
mod lang;
mod license;
mod prompt;
mod utils;

use anyhow::{Context, Result};
use askalono::{IdentifiedLicense, ScanResult, ScanStrategy, TextData};
use bstr::{ByteSlice, ByteVec};
use cargo::core::Resolve;
use clap::Parser;
use expand::expand;
use flate2::read::GzDecoder;
use heck::{AsKebabCase, ToKebabCase};
use indoc::{formatdoc, writedoc};
use is_terminal::IsTerminal;
use itertools::Itertools;
use once_cell::sync::Lazy;
use rustyline::{completion::FilenameCompleter, config::Configurer, CompletionType, Editor};
use serde::Deserialize;
use tempfile::tempdir;
use tokio::process::Command;
use tracing::debug;
use tracing_subscriber::EnvFilter;
use zip::ZipArchive;

use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs::{create_dir_all, metadata, read_dir, read_to_string, File},
    io::{stderr, Write as _},
    path::PathBuf,
};

use crate::{
    build::{BuildType, PythonFormat, RustVendor},
    cfg::load_config,
    cli::Opts,
    fetcher::{Fetcher, PackageInfo, PypiFormat, Revisions, Version},
    inputs::{write_all_lambda_inputs, write_inputs, write_lambda_input, AllInputs},
    lang::{
        go::write_ldflags,
        python::{parse_requirements_txt, Pyproject},
        rust::{cargo_deps_hash, load_cargo_lock, write_cargo_lock},
    },
    license::{get_nix_license, LICENSE_STORE},
    prompt::{ask_overwrite, prompt, Prompter},
    utils::{fod_hash, CommandExt, ResultExt, FAKE_HASH},
};

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

#[tokio::main]
async fn main() -> Result<()> {
    run().await
}

async fn run() -> Result<()> {
    tracing_subscriber::fmt()
        .with_ansi(stderr().is_terminal())
        .with_env_filter(EnvFilter::from_env("NIX_INIT_LOG"))
        .with_writer(stderr)
        .init();

    let opts = Opts::parse();

    tokio::spawn(async {
        Lazy::force(&LICENSE_STORE);
    });

    let cfg = load_config(opts.config)?;
    let mut editor = Editor::new()?;
    editor.set_completion_type(CompletionType::Fuzzy);
    editor.set_max_history_size(0)?;

    let output = match opts.output {
        Some(output) => output,
        None => {
            editor.set_helper(Some(Prompter::Path(FilenameCompleter::new())));
            let output =
                editor.readline(&prompt("Enter output path (defaults to current directory)"))?;
            editor.set_helper(None);
            if output.is_empty() {
                PathBuf::from(".")
            } else {
                PathBuf::from(output)
            }
        }
    };

    let (out_dir, out_path) = if let Ok(metadata) = metadata(&output) {
        if metadata.is_dir() {
            let out_path = output.join("default.nix");
            if out_path.exists() && ask_overwrite(&mut editor, &out_path)? {
                return Ok(());
            }
            (Some(output.as_path()), out_path)
        } else if ask_overwrite(&mut editor, &output)? {
            return Ok(());
        } else {
            (output.parent(), output.clone())
        }
    } else if <[u8] as ByteSlice>::from_path(&output)
        .map_or(false, |out_path| out_path.ends_with_str(b"/"))
    {
        let _ = create_dir_all(&output);
        (Some(output.as_ref()), output.join("default.nix"))
    } else {
        let out_dir = output.parent();
        if let Some(out_dir) = out_dir {
            let _ = create_dir_all(out_dir);
        }
        (out_dir, output.clone())
    };

    let mut out_file = File::options()
        .create(true)
        .write(true)
        .open(out_path)
        .context("failed to create output file")?;

    let mut out = String::new();
    writeln!(out, "{{ lib")?;

    let url = match opts.url {
        Some(url) => url,
        None => {
            editor.set_helper(Some(Prompter::NonEmpty));
            editor.readline(&prompt("Enter url"))?
        }
    };

    let mut fetcher = serde_json::from_slice(
        &Command::new("nurl")
            .arg(&url)
            .arg("-p")
            .get_stdout()
            .await?,
    )
    .context("failed to parse nurl output")?;

    let mut licenses = BTreeMap::new();
    let mut pypi_format = PypiFormat::TarGz;
    let (pname, rev, version, desc, prefix, mut python_deps) =
        if let MaybeFetcher::Known(ref mut fetcher) = &mut fetcher {
            let cl = fetcher.create_client(cfg.access_tokens).await?;

            let PackageInfo {
                pname,
                description,
                file_url_prefix,
                license,
                python_dependencies,
                revisions,
            } = fetcher.get_package_info(&cl).await;

            for license in license {
                licenses.insert(license, 1.0);
            }

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
                Some(Version::Pypi {
                    pname: pypi_pname,
                    format,
                }) => {
                    if let Fetcher::FetchPypi { ref mut pname } = fetcher {
                        *pname = pypi_pname;
                    }
                    pypi_format = format;
                    rev.clone()
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
                python_dependencies,
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
                Default::default(),
            )
        };

    let pname = if let Some(pname) = pname {
        editor.readline_with_initial(&prompt("Enter pname"), (&pname.to_kebab_case(), ""))?
    } else {
        editor.readline(&prompt("Enter pname"))?
    };

    let nixpkgs = opts
        .nixpkgs
        .or(cfg.nixpkgs)
        .unwrap_or_else(|| "<nixpkgs>".into());
    let mut cmd = Command::new("nurl");
    cmd.arg("-n").arg(&nixpkgs);

    let src_expr = {
        match fetcher {
            MaybeFetcher::Known(Fetcher::FetchCrate { pname: ref name }) => {
                let hash = String::from_utf8(cmd.arg(&url).arg(&rev).arg("-H").get_stdout().await?)
                    .context("failed to parse nurl output")?;

                if &pname == name {
                    formatdoc!(
                        r#"
                        fetchCrate {{
                            inherit pname version;
                            hash = "{hash}";
                          }}"#,
                    )
                } else {
                    formatdoc!(
                        r#"
                        fetchCrate {{
                            pname = {name:?};
                            inherit version;
                            hash = "{hash}";
                          }}"#,
                    )
                }
            }

            MaybeFetcher::Known(Fetcher::FetchPypi { pname: ref name }) => {
                cmd.arg("-H");
                let mut ext = String::new();
                if !matches!(pypi_format, PypiFormat::TarGz) {
                    write!(ext, "\n    extension = \"{pypi_format}\";")?;
                    cmd.arg("-A").arg("extension").arg(pypi_format.to_string());
                }

                if &pname == name {
                    let hash = String::from_utf8(
                        cmd.arg(format!("https://pypi.org/project/{name}"))
                            .arg(&rev)
                            .get_stdout()
                            .await?,
                    )
                    .context("failed to parse nurl output")?;
                    formatdoc!(
                        r#"
                        fetchPypi {{
                            inherit pname version;
                            hash = "{hash}";{ext}
                          }}"#,
                    )
                } else {
                    let hash = String::from_utf8(
                        cmd.arg(format!("https://pypi.org/project/{name}"))
                            .arg(&rev)
                            .get_stdout()
                            .await?,
                    )
                    .context("failed to parse nurl output")?;
                    formatdoc!(
                        r#"
                        fetchPypi {{
                            pname = {name:?};
                            inherit version;
                            hash = "{hash}";{ext}
                          }}"#,
                    )
                }
            }

            _ => {
                if rev == version {
                    cmd.arg("-o").arg("rev").arg("version");
                } else if rev.contains(&version) {
                    cmd.arg("-O")
                        .arg("rev")
                        .arg(rev.replacen(&version, "${version}", 1));
                }

                String::from_utf8(
                    cmd.arg(&url)
                        .arg(&rev)
                        .arg("-i")
                        .arg("2")
                        .get_stdout()
                        .await?,
                )
                .context("failed to parse nurl output")?
            }
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
            "let pname={pname:?};version={version:?};in(import({nixpkgs}){{}}).{}{src_expr}",
            if matches!(fetcher, MaybeFetcher::Known(Fetcher::FetchPypi { .. })) {
                "python3.pkgs."
            } else {
                ""
            },
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
        let file = File::open(&src)?;
        tmp = tempdir().context("failed to create temporary directory")?;
        let tmp = tmp.path();
        debug!("{}", tmp.display());

        match pypi_format {
            PypiFormat::TarGz => {
                tar::Archive::new(GzDecoder::new(file))
                    .unpack(tmp)
                    .context("failed to unpack pypi package")?;
            }
            PypiFormat::Zip => {
                ZipArchive::new(file)?.extract(tmp)?;
            }
        }

        tmp.join(format!("{pname}-{version}"))
    } else {
        PathBuf::from(&src)
    };

    let mut choices = Vec::new();
    let has_cargo = src_dir.join("Cargo.toml").is_file();
    let has_cmake = src_dir.join("CMakeLists.txt").is_file();
    let has_go = src_dir.join("go.mod").is_file();
    let has_meson = src_dir.join("meson.build").is_file();
    let pyproject_toml = src_dir.join("pyproject.toml");
    let has_pyproject = pyproject_toml.is_file();
    let has_setuptools = src_dir.join("setup.py").is_file();

    if has_go {
        choices.push(BuildType::BuildGoModule);
    }

    let mut python_formats = Vec::with_capacity(2);
    if has_pyproject {
        python_formats.push(PythonFormat::Pyproject);
    }
    if has_setuptools {
        python_formats.push(PythonFormat::Setuptools);
    }
    if !python_formats.is_empty() {
        for &rust in if has_cargo {
            &[
                Some(RustVendor::FetchCargoTarball),
                Some(RustVendor::ImportCargoLock),
                None,
            ]
        } else {
            &[None] as &[_]
        } {
            for &format in &python_formats {
                for application in [true, false] {
                    choices.push(BuildType::BuildPythonPackage {
                        application,
                        format,
                        rust,
                    });
                }
            }
        }
    }

    if has_cargo {
        for vendor in [RustVendor::FetchCargoTarball, RustVendor::ImportCargoLock] {
            let drv = BuildType::MkDerivation { rust: Some(vendor) };
            let rust = BuildType::BuildRustPackage { vendor };
            choices.extend(if has_meson { [drv, rust] } else { [rust, drv] });
        }
    }

    choices.push(BuildType::MkDerivation { rust: None });

    editor.set_helper(Some(Prompter::Build(choices)));
    let choice = editor.readline(&prompt("How should this package be built?"))?;
    let Some(Prompter::Build(choices)) = editor.helper_mut() else {
        unreachable!();
    };
    let choice = *choice
        .parse()
        .ok()
        .and_then(|i: usize| choices.get(i))
        .unwrap_or_else(|| &choices[0]);

    let mut inputs = AllInputs::default();
    match choice {
        BuildType::BuildGoModule => {
            writeln!(out, ", buildGoModule")?;
        }
        BuildType::BuildPythonPackage {
            application, rust, ..
        } => {
            writeln!(
                out,
                ", {}",
                if application {
                    "python3"
                } else {
                    "buildPythonPackage"
                }
            )?;

            if src_dir.join("poetry.lock").is_file() {
                inputs.native_build_inputs.always.insert(
                    if application {
                        "python3.pkgs.poetry-core"
                    } else {
                        "poetry-core"
                    }
                    .into(),
                );
            }

            if rust.is_some() {
                inputs.native_build_inputs.always.extend([
                    "rustPlatform.cargoSetupHook".into(),
                    "rustPlatform.rust.cargo".into(),
                    "rustPlatform.rust.rustc".into(),
                ]);
            }
        }
        BuildType::BuildRustPackage { .. } => {
            writeln!(out, ", rustPlatform")?;
        }
        BuildType::MkDerivation { rust } => {
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
            if rust.is_some() {
                inputs.native_build_inputs.always.extend([
                    "rustPlatform.cargoSetupHook".into(),
                    "rustPlatform.rust.cargo".into(),
                    "rustPlatform.rust.rustc".into(),
                ]);
            }
        }
    }

    match fetcher {
        MaybeFetcher::Known(fetcher) => {
            writeln!(out, ", {fetcher}")?;
        }
        MaybeFetcher::Unknown { fetcher } => {
            writeln!(out, ", {fetcher}")?;
        }
    }

    let mut pyproject = None;
    let (native_build_inputs, build_inputs) = match choice {
        BuildType::BuildGoModule => {
            let hash = if src_dir.join("vendor").is_dir()
                || src_dir
                    .join("go.sum")
                    .metadata()
                    .map_or(true, |metadata| metadata.len() == 0)
            {
                "null".into()
            } else if let Some(hash) = fod_hash(format!(
                r#"(import({nixpkgs}){{}}).buildGoModule{{pname={pname:?};version={version:?};src={src};vendorHash="{FAKE_HASH}";}}"#,
            )).await {
                format!(r#""{hash}""#)
            } else {
                format!(r#""{FAKE_HASH}""#)
            };

            let res = write_all_lambda_inputs(&mut out, &inputs, &mut BTreeSet::new())?;
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
            rust,
        } => {
            enum RustVendorData {
                Hash(String),
                Lock(bool, Option<Resolve>),
                None,
            }
            let rust = match rust {
                Some(RustVendor::FetchCargoTarball) => RustVendorData::Hash(
                    cargo_deps_hash(&mut inputs, &pname, &version, &src, &src_dir, &nixpkgs).await,
                ),
                Some(RustVendor::ImportCargoLock) => {
                    if let Some(out_dir) = out_dir {
                        editor.set_helper(None);
                        let (missing, resolve) =
                            load_cargo_lock(&mut editor, out_dir, &mut inputs, &src_dir).await?;
                        RustVendorData::Lock(missing, resolve)
                    } else {
                        RustVendorData::Lock(false, None)
                    }
                }
                None => RustVendorData::None,
            };

            if matches!(format, PythonFormat::Pyproject) {
                if let Some(mut pyproject_found) = Pyproject::from_path(pyproject_toml) {
                    pyproject_found.load_license(&mut licenses);
                    pyproject_found.load_build_dependencies(&mut inputs, application);

                    if let Some(deps) = pyproject_found
                        .get_dependencies()
                        .or_else(|| parse_requirements_txt(&src_dir))
                    {
                        python_deps = deps;
                    }

                    pyproject = Some(pyproject_found)
                }
            }

            let mut written = BTreeSet::new();
            if application {
                written.insert("python3".into());
            }
            let res = write_all_lambda_inputs(&mut out, &inputs, &mut written)?;
            if !application {
                for name in python_deps
                    .always
                    .iter()
                    .chain(python_deps.optional.values().flatten())
                {
                    write_lambda_input(&mut out, &mut written, &name.to_kebab_case())?;
                }
            }

            writedoc!(
                out,
                r#"
                    }}:

                    {} rec {{
                      pname = {pname:?};
                      version = {version:?};
                      format = "{format}";

                      src = {src_expr};

                "#,
                if application {
                    "python3.pkgs.buildPythonApplication"
                } else {
                    "buildPythonPackage"
                },
            )?;

            match rust {
                RustVendorData::Hash(hash) => {
                    write!(out, "  ")?;
                    writedoc!(
                        out,
                        r#"
                            cargoDeps = rustPlatform.fetchCargoTarball {{
                                inherit src;
                                name = "${{pname}}-${{version}}";
                                hash = "{hash}";
                              }};

                        "#,
                    )?;
                }
                RustVendorData::Lock(missing, lock) => {
                    write!(out, "  cargoDeps = rustPlatform.importCargoLock ")?;
                    write_cargo_lock(&mut out, missing, lock).await?;
                }
                RustVendorData::None => {}
            }

            res
        }

        BuildType::BuildRustPackage {
            vendor: RustVendor::FetchCargoTarball,
        } => {
            let hash =
                cargo_deps_hash(&mut inputs, &pname, &version, &src, &src_dir, &nixpkgs).await;
            let res =
                write_all_lambda_inputs(&mut out, &inputs, &mut ["rustPlatform".into()].into())?;
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

        BuildType::BuildRustPackage {
            vendor: RustVendor::ImportCargoLock,
        } => {
            let (missing, lock) = if let Some(out_dir) = out_dir {
                editor.set_helper(None);
                load_cargo_lock(&mut editor, out_dir, &mut inputs, &src_dir).await?
            } else {
                (false, None)
            };

            let res =
                write_all_lambda_inputs(&mut out, &inputs, &mut ["rustPlatform".into()].into())?;
            writedoc!(
                out,
                r#"
                    }}:

                    rustPlatform.buildRustPackage rec {{
                      pname = "{pname}";
                      version = "{version}";

                      src = {src_expr};

                      cargoLock = "#,
            )?;
            write_cargo_lock(&mut out, missing, lock).await?;
            res
        }

        BuildType::MkDerivation { rust: None } => {
            let res = write_all_lambda_inputs(&mut out, &inputs, &mut ["stdenv".into()].into())?;
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

        BuildType::MkDerivation {
            rust: Some(RustVendor::FetchCargoTarball),
        } => {
            let hash =
                cargo_deps_hash(&mut inputs, &pname, &version, &src, &src_dir, &nixpkgs).await;
            let res = write_all_lambda_inputs(&mut out, &inputs, &mut ["stdenv".into()].into())?;
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

        BuildType::MkDerivation {
            rust: Some(RustVendor::ImportCargoLock),
        } => {
            let (missing, lock) = if let Some(out_dir) = out_dir {
                editor.set_helper(None);
                load_cargo_lock(&mut editor, out_dir, &mut inputs, &src_dir).await?
            } else {
                (false, None)
            };

            let res = write_all_lambda_inputs(&mut out, &inputs, &mut ["stdenv".into()].into())?;
            writedoc!(
                out,
                r#"
                    }}:

                    stdenv.mkDerivation rec {{
                      pname = "{pname}";
                      version = "{version}";

                      src = {src_expr};

                      cargoDeps = rustPlatform.importCargoLock "#,
            )?;
            write_cargo_lock(&mut out, missing, lock).await?;
            res
        }
    };

    if native_build_inputs {
        write_inputs(&mut out, &inputs.native_build_inputs, "nativeBuildInputs")?;
    }
    if build_inputs {
        write_inputs(&mut out, &inputs.build_inputs, "buildInputs")?;
    }

    match choice {
        BuildType::BuildGoModule => {
            write_ldflags(&mut out, &src_dir)?;
        }

        BuildType::BuildPythonPackage { application, .. } => {
            if !python_deps.always.is_empty() {
                write!(out, "  propagatedBuildInputs = ")?;
                if application {
                    write!(out, "with python3.pkgs; ")?;
                }
                writeln!(out, "[")?;

                for name in python_deps.always {
                    writeln!(out, "    {}", AsKebabCase(name))?;
                }
                writeln!(out, "  ];\n")?;
            }

            let mut optional = python_deps
                .optional
                .into_iter()
                .filter(|(_, deps)| !deps.is_empty());

            if let Some((extra, deps)) = optional.next() {
                write!(out, "  passthru.optional-dependencies = ")?;
                if application {
                    write!(out, "with python3.pkgs; ")?;
                }
                writeln!(out, "{{\n    {extra} = [",)?;
                for name in deps {
                    writeln!(out, "      {}", AsKebabCase(name))?;
                }
                writeln!(out, "    ];")?;

                for (extra, deps) in optional {
                    writeln!(out, "    {extra} = [")?;
                    for name in deps {
                        writeln!(out, "      {}", AsKebabCase(name))?;
                    }
                    writeln!(out, "    ];")?;
                }

                writeln!(out, "  }};\n")?;
            }

            writeln!(
                out,
                "  pythonImportsCheck = [ {:?} ];\n",
                pyproject
                    .as_mut()
                    .and_then(Pyproject::get_name)
                    .unwrap_or(pname),
            )?;
        }

        _ => {}
    }

    if !inputs.env.is_empty() {
        for (k, v) in inputs.env {
            writeln!(out, "  {k} = {v};")?;
        }
        writeln!(out)?;
    }

    let mut desc = desc.trim_matches(|c: char| !c.is_alphanumeric()).to_owned();
    desc.get_mut(0 .. 1).map(str::make_ascii_uppercase);
    write!(out, "  ")?;
    writedoc!(
        out,
        r"
            meta = with lib; {{
                description = {desc:?};
                homepage = {url:?};
        ",
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
                    writeln!(out, r#"    changelog = "{prefix}{name}";"#)?;
                    break;
                }
            }
        }
    }

    if let (Some(store), Some(walk)) = (&*LICENSE_STORE, read_dir(src_dir).ok_warn()) {
        let strategy = ScanStrategy::new(store).confidence_threshold(0.8);

        for entry in walk {
            let Ok(entry) = entry else { continue; };
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            let file_name = entry.file_name();
            let name = <Vec<u8> as ByteVec>::from_os_str_lossy(&file_name);
            if !matches!(
                name.to_ascii_lowercase()[..],
                expand!([@b"license", ..] | [@b"licence", ..] | [@b"copying", ..]),
            ) {
                continue;
            }

            let Some(text) = read_to_string(&path).ok_warn() else { continue; };
            let Some(ScanResult {
                score,
                license: Some(IdentifiedLicense { name, .. }),
                ..
            }) = strategy.scan(&TextData::from(text)).ok_warn() else {
                continue;
            };

            if let Some(license) = get_nix_license(name) {
                debug!(
                    "license found in {}: {license}",
                    file_name.to_string_lossy(),
                );
                licenses.entry(license).or_insert(score);
            }
        }
    }

    let licenses: Vec<_> = licenses
        .into_iter()
        .sorted_unstable_by(|x, y| match x.1.partial_cmp(&y.1) {
            None | Some(Ordering::Equal) => x.0.cmp(y.0),
            Some(cmp) => cmp,
        })
        .map(|(license, _)| license)
        .collect();

    write!(out, "    license = ")?;
    if let [license] = &licenses[..] {
        write!(out, "licenses.{license}")?;
    } else {
        write!(out, "with licenses; [ ")?;
        for license in licenses {
            write!(out, "{license} ")?;
        }
        write!(out, "]")?;
    }

    write!(out, ";\n    maintainers = with maintainers; [ ")?;
    for maintainer in cfg.maintainers {
        write!(out, "{maintainer} ")?;
    }
    writeln!(out, "];\n  }};\n}}")?;

    out_file.set_len(0)?;
    write!(out_file, "{out}")?;

    Ok(())
}
