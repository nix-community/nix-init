mod builder;
mod cfg;
mod cli;
mod cmd;
mod fetcher;
mod frontend;
mod inputs;
mod lang;
mod license;
mod macros;
mod utils;

use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs::{File, create_dir_all, metadata, read_dir},
    io::{IsTerminal, Seek, Write as _, pipe, stderr},
    os::unix::ffi::OsStrExt,
    path::{Component, Path, PathBuf},
    process::Stdio,
    sync::LazyLock,
};

use anyhow::{Context, Result, bail};
use askalono::ScanStrategy;
use cargo::core::Resolve;
use clap::{Parser, crate_version};
use expand::expand;
use flate2::read::GzDecoder;
use heck::{AsSnakeCase, ToKebabCase};
use indoc::{formatdoc, writedoc};
use itertools::Itertools;
use serde::Deserialize;
use tempfile::tempdir;
use tokio::process::Command;
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;
use which::which;
use zip::ZipArchive;

use crate::{
    builder::Builder,
    cfg::load_config,
    cli::{BuilderFunction, CargoVendor, Opts},
    cmd::{NIX, NURL},
    fetcher::{Fetcher, FetcherDispatch, PackageInfo, PypiFormat, Revisions, Version},
    frontend::{Frontend, headless, readline},
    inputs::{AllInputs, write_all_lambda_inputs, write_inputs, write_lambda_input},
    lang::{
        go::{load_go_dependencies, write_ldflags},
        python::{Pyproject, parse_requirements_txt},
        rust::{cargo_deps_hash, load_cargo_lock, write_cargo_lock},
    },
    license::{LICENSE_STORE, load_license},
    utils::{CommandExt, FAKE_HASH, ResultExt, fod_hash},
};

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum MaybeFetcher {
    Known(FetcherDispatch),
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
        .with_file(true)
        .with_line_number(true)
        .with_writer(stderr)
        .init();

    let opts = Opts::parse();
    let opt_version = match opts.version {
        Some(Some(version)) => Some(version),
        Some(None) => {
            println!("nix-init {}", crate_version!());
            return Ok(());
        }
        None => None,
    };

    tokio::spawn(async {
        LazyLock::force(&LICENSE_STORE);
    });

    let cfg = load_config(opts.config)?;

    let mut frontend = if opts.headless {
        headless()
    } else {
        readline()?
    };

    let mut out = String::new();
    writeln!(out, "{{\n  lib,")?;

    let mut url = match opts.url {
        Some(url) => url,
        None => frontend.url()?,
    };

    let mut fetcher =
        serde_json::from_slice(&Command::new(NURL).arg(&url).arg("-p").get_stdout().await?)
            .context("failed to parse nurl output")?;

    let mut cmd = Command::new(NURL);
    let mut licenses = BTreeMap::new();
    let mut pypi_format = PypiFormat::TarGz;
    let (pname, rev, version, desc, prefix, mut python_deps) =
        if let MaybeFetcher::Known(fetcher) = &mut fetcher {
            let cl = fetcher.create_client(cfg.access_tokens).await?;

            let PackageInfo {
                pname,
                description,
                file_url_prefix,
                homepage,
                license,
                python_dependencies,
                mut revisions,
            } = fetcher.get_package_info(&cl).await;

            url = homepage;

            for license in license {
                licenses.insert(license, 1.0);
            }

            let (rev, version) = match opts.rev {
                Some(rev) => {
                    let version = revisions.versions.remove(&rev);
                    (rev, version)
                }
                None => frontend.rev(Some(revisions))?,
            };

            let submodules = match opts.submodules {
                Some(true) => fetcher.has_submodules(&cl, &rev).await,
                Some(false) => false,
                None => fetcher.has_submodules(&cl, &rev).await && frontend.fetch_submodules()?,
            };
            if submodules {
                cmd.arg("-S");
            }

            let version = if let Some(version) = opt_version {
                version
            } else {
                let version = match version {
                    Some(version) => Some(version),
                    None => fetcher.get_version(&cl, &rev).await,
                };
                let version = match version {
                    Some(Version::Latest | Version::Tag) => get_version_number(&rev).into(),
                    Some(Version::Pypi {
                        pname: pypi_pname,
                        format,
                    }) => {
                        if let FetcherDispatch::FetchPypi(fetcher) = fetcher {
                            fetcher.pname = pypi_pname;
                        }
                        pypi_format = format;
                        rev.clone()
                    }
                    Some(Version::Head { date, .. } | Version::Commit { date, .. }) => {
                        format!("0-unstable-{date}")
                    }
                    None => get_version(&rev).into(),
                };

                frontend.version(&version)?
            };

            (
                Some(pname),
                rev,
                version,
                description,
                file_url_prefix,
                python_dependencies,
            )
        } else {
            let pname = url
                .parse::<url::Url>()
                .ok_inspect(|e| warn!("{e}"))
                .and_then(|url| {
                    url.path_segments()
                        .and_then(|mut xs| xs.next_back())
                        .map(|pname| pname.strip_suffix(".git").unwrap_or(pname).into())
                });

            let rev = match opts.rev {
                Some(rev) => rev,
                None => frontend.rev(None)?.0,
            };

            let version = match opt_version {
                Some(version) => version,
                None => frontend.version(get_version(&rev))?,
            };

            (pname, rev, version, "".into(), None, Default::default())
        };

    let pname = match opts.pname {
        Some(rev) => rev,
        None => frontend.pname(pname.map(|pname| pname.to_kebab_case()))?,
    };

    let nixpkgs = opts
        .nixpkgs
        .or(cfg.nixpkgs)
        .unwrap_or_else(|| "<nixpkgs>".into());
    cmd.arg("-n").arg(&nixpkgs);

    let src_expr = match fetcher {
        MaybeFetcher::Known(FetcherDispatch::FetchCrate(ref fetcher)) => {
            let hash: String = cmd
                .arg(&url)
                .arg(&rev)
                .arg("-H")
                .get_stdout()
                .await?
                .try_into()
                .context("failed to parse nurl output")?;

            if pname == fetcher.pname {
                formatdoc! {r#"
                    fetchCrate {{
                        inherit (finalAttrs) pname version;
                        hash = "{hash}";
                      }}"#,
                }
            } else {
                formatdoc! {r#"
                    fetchCrate {{
                        pname = {:?};
                        inherit (finalAttrs) version;
                        hash = "{hash}";
                      }}"#,
                    fetcher.pname,
                }
            }
        }

        MaybeFetcher::Known(FetcherDispatch::FetchPypi(ref fetcher)) => {
            cmd.arg("-H");
            let mut ext = String::new();
            if !matches!(pypi_format, PypiFormat::TarGz) {
                write!(ext, "\n    extension = \"{pypi_format}\";")?;
                cmd.arg("-A").arg("extension").arg(pypi_format.to_string());
            }

            let hash: String = cmd
                .arg(format!("https://pypi.org/project/{}", fetcher.pname))
                .arg(&rev)
                .get_stdout()
                .await?
                .try_into()
                .context("failed to parse nurl output")?;

            if pname == fetcher.pname {
                formatdoc! {r#"
                    fetchPypi {{
                        inherit (finalAttrs) pname version;
                        hash = "{hash}";{ext}
                      }}"#,
                }
            } else {
                formatdoc! {r#"
                    fetchPypi {{
                        pname = {:?};
                        inherit (finalAttrs) version;
                        hash = "{hash}";{ext}
                      }}"#,
                    fetcher.pname,
                }
            }
        }

        _ => {
            if rev == version {
                cmd.arg("--overwrite-rev").arg("finalAttrs.version");
            } else if rev.contains(&version) {
                cmd.arg("--overwrite-rev-str").arg(rev.replacen(
                    &version,
                    "${finalAttrs.version}",
                    1,
                ));
            }

            cmd.arg(&url)
                .arg(&rev)
                .arg("-i")
                .arg("2")
                .get_stdout()
                .await?
                .try_into()
                .context("failed to parse nurl output")?
        }
    };

    let stdout = Command::new(NIX)
        .arg("build")
        .arg("--extra-experimental-features")
        .arg("nix-command")
        .arg("--impure")
        .arg("--no-link")
        .arg("--json")
        .arg("--expr")
        .arg(format!(
            "let finalAttrs={{pname={pname:?};version={version:?};}};in(import({nixpkgs}){{}}).{src_expr}",
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
    let src_dir = if let MaybeFetcher::Known(FetcherDispatch::FetchPypi(ref fetcher)) = fetcher {
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

        tmp.join(format!("{}-{version}", fetcher.pname))
    } else {
        PathBuf::from(&src)
    };

    let has_cargo = src_dir.join("Cargo.toml").is_file();
    let cargo_lock = File::open(src_dir.join("Cargo.lock"));
    let has_cargo_lock = cargo_lock.is_ok();
    let has_cmake = src_dir.join("CMakeLists.txt").is_file();
    let has_go = src_dir.join("go.mod").is_file();
    let has_meson = src_dir.join("meson.build").is_file();
    let has_zig = src_dir.join("build.zig").is_file();
    let pyproject_toml = src_dir.join("pyproject.toml");
    let has_python = pyproject_toml.is_file() || src_dir.join("setup.py").is_file();

    let builder = match (opts.builder, opts.cargo_vendor) {
        (Some(builder), rust @ Some(vendor)) if has_cargo => match builder {
            BuilderFunction::BuildGoModule => Builder::BuildGoModule,
            BuilderFunction::BuildPythonApplication => Builder::BuildPythonPackage {
                application: true,
                rust,
            },
            BuilderFunction::BuildPythonPackage => Builder::BuildPythonPackage {
                application: false,
                rust,
            },
            BuilderFunction::BuildRustPackage => Builder::BuildRustPackage { vendor },
            BuilderFunction::MkDerivation => Builder::MkDerivation { rust },
            BuilderFunction::MkDerivationNoCC => Builder::MkDerivationNoCC,
        },
        (Some(builder), _) => {
            let rust = has_cargo.then_some(CargoVendor::FetchCargoVendor);
            match builder {
                BuilderFunction::BuildGoModule => Builder::BuildGoModule,
                BuilderFunction::BuildPythonApplication => Builder::BuildPythonPackage {
                    application: true,
                    rust,
                },
                BuilderFunction::BuildPythonPackage => Builder::BuildPythonPackage {
                    application: false,
                    rust,
                },
                BuilderFunction::BuildRustPackage => Builder::BuildRustPackage {
                    vendor: CargoVendor::FetchCargoVendor,
                },
                BuilderFunction::MkDerivation => Builder::MkDerivation { rust },
                BuilderFunction::MkDerivationNoCC => Builder::MkDerivationNoCC,
            }
        }
        (None, rust) => {
            let mut builders = Vec::new();
            if has_go {
                builders.push(Builder::BuildGoModule);
            }

            if has_cargo {
                let cargo_vendors: &[_] = match rust {
                    Some(vendor) => &[vendor],
                    None => &[CargoVendor::FetchCargoVendor, CargoVendor::ImportCargoLock],
                };

                for &vendor in cargo_vendors {
                    if has_python {
                        for application in [true, false] {
                            builders.push(Builder::BuildPythonPackage {
                                application,
                                rust: Some(vendor),
                            });
                        }
                    }

                    let drv = Builder::MkDerivation { rust: Some(vendor) };
                    let rust = Builder::BuildRustPackage { vendor };
                    builders.extend(if has_meson { [drv, rust] } else { [rust, drv] });
                }
            }

            if has_python {
                for application in [true, false] {
                    builders.push(Builder::BuildPythonPackage {
                        application,
                        rust: None,
                    });
                }
            }

            builders.push(Builder::MkDerivation { rust: None });
            builders.push(Builder::MkDerivationNoCC);

            frontend.builder(builders)?
        }
    };

    let output = if let Some(output) = opts.output {
        output
    } else {
        frontend.output(&pname, &builder)?
    };

    let (out_dir, out_path) = if let Ok(metadata) = metadata(&output) {
        if metadata.is_dir() {
            let out_path = output.join("default.nix");
            if out_path.exists() && !frontend.should_overwrite(&out_path, opts.overwrite)? {
                return Ok(());
            }
            (Some(output.as_path()), out_path)
        } else if !frontend.should_overwrite(&output, opts.overwrite)? {
            return Ok(());
        } else {
            (output.parent(), output.clone())
        }
    } else if output.as_os_str().as_bytes().ends_with(b"/") {
        let _ = create_dir_all(&output);
        (Some(output.as_ref()), output.join("default.nix"))
    } else {
        let out_dir = output.parent();
        if let Some(out_dir) = out_dir {
            let _ = create_dir_all(out_dir);
        }
        (out_dir, output.clone())
    };

    let mut inputs = AllInputs::default();
    match builder {
        Builder::BuildGoModule => {
            writeln!(out, "  buildGoModule,")?;
        }
        Builder::BuildPythonPackage { application, rust } => {
            writeln!(
                out,
                "  {},",
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
                    "cargo".into(),
                    "rustPlatform.cargoSetupHook".into(),
                    "rustc".into(),
                ]);
            }
        }
        Builder::BuildRustPackage { .. } => {
            writeln!(out, "  rustPlatform,")?;
        }
        Builder::MkDerivation { rust } => {
            writeln!(out, "  stdenv,")?;
            if has_cmake {
                inputs.native_build_inputs.always.insert("cmake".into());
            }
            if has_meson {
                inputs
                    .native_build_inputs
                    .always
                    .extend(["meson".into(), "ninja".into()]);
            }
            if has_zig {
                inputs.native_build_inputs.always.insert("zig.hook".into());
            }
            if rust.is_some() {
                inputs.native_build_inputs.always.extend([
                    "cargo".into(),
                    "rustPlatform.cargoSetupHook".into(),
                    "rustc".into(),
                ]);
            }
        }
        Builder::MkDerivationNoCC => {
            writeln!(out, "  stdenvNoCC,")?;
            if has_cmake {
                inputs.native_build_inputs.always.insert("cmake".into());
            }
            if has_meson {
                inputs
                    .native_build_inputs
                    .always
                    .extend(["meson".into(), "ninja".into()]);
            }
            if has_zig {
                inputs.native_build_inputs.always.insert("zig.hook".into());
            }
        }
    }

    match fetcher {
        MaybeFetcher::Known(fetcher) => {
            writeln!(out, "  {fetcher},")?;
        }
        MaybeFetcher::Unknown { fetcher } => {
            writeln!(out, "  {fetcher},")?;
        }
    }

    let mut python_import = None;
    let (native_build_inputs, build_inputs) = match builder {
        Builder::BuildGoModule => {
            let go_sum = File::open(src_dir.join("go.sum")).ok_inspect(|e| warn!("{e}"));

            if let Some(go_sum) = &go_sum {
                load_go_dependencies(&mut inputs, go_sum);
            }

            let hash = if src_dir.join("vendor").is_dir()
                || go_sum.and_then(|go_sum| go_sum.metadata().ok_inspect(|e| warn!("{e}")))
                    .is_none_or(|metadata| metadata.len() == 0)
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
            writedoc! {out, r#"
                }}:

                buildGoModule (finalAttrs: {{
                  pname = {pname:?};
                  version = {version:?};

                  src = {src_expr};

                  vendorHash = {hash};

            "#}?;
            res
        }

        Builder::BuildPythonPackage { application, rust } => {
            enum CargoVendorData {
                Hash(String),
                Lock(Box<Option<Resolve>>),
                None,
            }
            let rust = match rust {
                Some(CargoVendor::FetchCargoVendor) => CargoVendorData::Hash(
                    cargo_deps_hash(
                        &mut inputs,
                        &pname,
                        &version,
                        &src,
                        &src_dir,
                        has_cargo_lock,
                        &nixpkgs,
                    )
                    .await,
                ),
                Some(CargoVendor::ImportCargoLock) => {
                    if let Some(out_dir) = out_dir {
                        let resolve = load_cargo_lock(
                            &mut frontend,
                            out_dir,
                            &mut inputs,
                            &src_dir,
                            opts.overwrite,
                        )
                        .await?;
                        CargoVendorData::Lock(Box::new(resolve))
                    } else {
                        CargoVendorData::Lock(Box::new(None))
                    }
                }
                None => CargoVendorData::None,
            };

            let mut pyproject = Pyproject::from_path(pyproject_toml);

            if let Some(name) = pyproject.get_name() {
                python_import = Some(name);
            }

            pyproject.load_license(&mut licenses);
            pyproject.load_build_dependencies(&mut inputs, application);

            if let Some(deps) = pyproject.get_dependencies() {
                python_deps = deps;
            }

            if python_deps.always.is_empty()
                && python_deps.optional.is_empty()
                && let Some(deps) = parse_requirements_txt(&src_dir)
            {
                python_deps = deps;
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

            writedoc! {out, r#"
                }}:

                {} (finalAttrs: {{
                  pname = {pname:?};
                  version = {version:?};
                  pyproject = true;

                  src = {src_expr};

                "#,
                if application {
                    "python3.pkgs.buildPythonApplication"
                } else {
                    "buildPythonPackage"
                },
            }?;

            match rust {
                CargoVendorData::Hash(hash) => {
                    write!(out, "  ")?;
                    writedoc! {out, r#"
                        cargoDeps = rustPlatform.fetchCargoVendor {{
                            inherit (finalAttrs) pname version src;
                            hash = "{hash}";
                          }};

                    "#}?;
                }
                CargoVendorData::Lock(resolve) => {
                    write!(out, "  cargoDeps = rustPlatform.importCargoLock ")?;
                    write_cargo_lock(&mut out, has_cargo_lock, *resolve).await?;
                }
                CargoVendorData::None => {}
            }

            res
        }

        Builder::BuildRustPackage {
            vendor: CargoVendor::FetchCargoVendor,
        } => {
            let hash = cargo_deps_hash(
                &mut inputs,
                &pname,
                &version,
                &src,
                &src_dir,
                has_cargo_lock,
                &nixpkgs,
            )
            .await;
            let res =
                write_all_lambda_inputs(&mut out, &inputs, &mut ["rustPlatform".into()].into())?;
            writedoc! {out, r#"
                }}:

                rustPlatform.buildRustPackage (finalAttrs: {{
                  pname = {pname:?};
                  version = {version:?};

                  src = {src_expr};

                  cargoHash = "{hash}";

            "#}?;
            res
        }

        Builder::BuildRustPackage {
            vendor: CargoVendor::ImportCargoLock,
        } => {
            let resolve = if let Some(out_dir) = out_dir {
                load_cargo_lock(
                    &mut frontend,
                    out_dir,
                    &mut inputs,
                    &src_dir,
                    opts.overwrite,
                )
                .await?
            } else {
                None
            };

            let res =
                write_all_lambda_inputs(&mut out, &inputs, &mut ["rustPlatform".into()].into())?;
            writedoc! {out, r#"
                }}:

                rustPlatform.buildRustPackage (finalAttrs: {{
                  pname = "{pname}";
                  version = "{version}";

                  src = {src_expr};

                  cargoLock = "#,
            }?;
            write_cargo_lock(&mut out, has_cargo_lock, resolve).await?;
            res
        }

        Builder::MkDerivation { rust: None } => {
            let res = write_all_lambda_inputs(&mut out, &inputs, &mut ["stdenv".into()].into())?;
            writedoc! { out, r#"
                }}:

                stdenv.mkDerivation (finalAttrs: {{
                  pname = {pname:?};
                  version = {version:?};

                  src = {src_expr};

            "#}?;
            res
        }

        Builder::MkDerivation {
            rust: Some(CargoVendor::FetchCargoVendor),
        } => {
            let hash = cargo_deps_hash(
                &mut inputs,
                &pname,
                &version,
                &src,
                &src_dir,
                has_cargo_lock,
                &nixpkgs,
            )
            .await;
            let res = write_all_lambda_inputs(&mut out, &inputs, &mut ["stdenv".into()].into())?;
            writedoc! {out, r#"
                }}:

                stdenv.mkDerivation (finalAttrs: {{
                  pname = {pname:?};
                  version = {version:?};

                  src = {src_expr};

                  cargoDeps = rustPlatform.fetchCargoVendor {{
                    inherit (finalAttrs) pname version src;
                    hash = "{hash}";
                  }};

            "#}?;
            res
        }

        Builder::MkDerivation {
            rust: Some(CargoVendor::ImportCargoLock),
        } => {
            let resolve = if let Some(out_dir) = out_dir {
                load_cargo_lock(
                    &mut frontend,
                    out_dir,
                    &mut inputs,
                    &src_dir,
                    opts.overwrite,
                )
                .await?
            } else {
                None
            };

            let res = write_all_lambda_inputs(&mut out, &inputs, &mut ["stdenv".into()].into())?;
            writedoc! {out, r#"
                }}:

                stdenv.mkDerivation (finalAttrs: {{
                  pname = "{pname}";
                  version = "{version}";

                  src = {src_expr};

                  cargoDeps = rustPlatform.importCargoLock "#,
            }?;
            write_cargo_lock(&mut out, has_cargo_lock, resolve).await?;
            res
        }

        Builder::MkDerivationNoCC => {
            let res =
                write_all_lambda_inputs(&mut out, &inputs, &mut ["stdenvNoCC".into()].into())?;
            writedoc! { out, r#"
                }}:

                stdenvNoCC.mkDerivation (finalAttrs: {{
                  pname = {pname:?};
                  version = {version:?};

                  src = {src_expr};

            "#}?;
            res
        }
    };

    if native_build_inputs {
        match builder {
            Builder::BuildPythonPackage { .. } => {
                write_inputs(&mut out, &inputs.native_build_inputs, "build-system")?;
            }
            _ => {
                write_inputs(&mut out, &inputs.native_build_inputs, "nativeBuildInputs")?;
            }
        }
    }
    if build_inputs {
        write_inputs(&mut out, &inputs.build_inputs, "buildInputs")?;
    }

    match builder {
        Builder::BuildGoModule => {
            write_ldflags(&mut out, &src_dir)?;
        }

        Builder::BuildPythonPackage { application, .. } => {
            if !python_deps.always.is_empty() {
                write!(out, "  dependencies = ")?;
                if application {
                    write!(out, "with python3.pkgs; ")?;
                }
                writeln!(out, "[")?;

                for name in python_deps.always {
                    writeln!(out, "    {name}")?;
                }
                writeln!(out, "  ];\n")?;
            }

            let mut optional = python_deps
                .optional
                .into_iter()
                .filter(|(_, deps)| !deps.is_empty());

            if let Some((extra, deps)) = optional.next() {
                write!(out, "  optional-dependencies = ")?;
                if application {
                    write!(out, "with python3.pkgs; ")?;
                }
                writeln!(out, "{{\n    {extra} = [",)?;
                for name in deps {
                    writeln!(out, "      {name}")?;
                }
                writeln!(out, "    ];")?;

                for (extra, deps) in optional {
                    writeln!(out, "    {extra} = [")?;
                    for name in deps {
                        writeln!(out, "      {name}")?;
                    }
                    writeln!(out, "    ];")?;
                }

                writeln!(out, "  }};\n")?;
            }

            writeln!(
                out,
                "  pythonImportsCheck = [\n    \"{}\"\n  ];\n",
                AsSnakeCase(python_import.as_ref().unwrap_or(&pname)),
            )?;
        }

        _ => {}
    }

    if !inputs.env.is_empty() {
        writeln!(out, "  env = {{")?;
        for (k, (v, _)) in inputs.env {
            writeln!(out, "    {k} = {v};")?;
        }
        writeln!(out, "  }};\n")?;
    }

    let mut desc = desc.trim_matches(|c: char| !c.is_alphanumeric()).to_owned();
    desc.get_mut(0 .. 1).map(str::make_ascii_uppercase);
    write!(out, "  ")?;
    writedoc! {out, r"
        meta = {{
            description = {desc:?};
            homepage = {url:?};
    "}?;

    if let Some(prefix) = prefix
        && let Some(walk) = read_dir(&src_dir).ok_inspect(|e| warn!("{e}"))
    {
        for entry in walk {
            let Ok(entry) = entry else {
                continue;
            };
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            if matches!(
                name.to_ascii_lowercase().as_bytes(),
                expand!([@b"changelog", ..] | [@b"changes", ..] | [@b"news"] | [@b"releases", ..]),
            ) {
                writeln!(out, r#"    changelog = "{prefix}{name}";"#)?;
                break;
            }
        }
    }

    if let (Some(store), Some(entries)) = (
        &*LICENSE_STORE,
        read_dir(src_dir).ok_inspect(|e| warn!("{e}")),
    ) {
        let strategy = ScanStrategy::new(store).confidence_threshold(0.8);

        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();

            if !matches!(
                name.as_bytes().to_ascii_lowercase()[..],
                expand!([@b"license", ..] | [@b"licence", ..] | [@b"copying", ..]),
            ) {
                continue;
            }

            let Ok(metadata) = path.metadata() else {
                continue;
            };

            if metadata.is_dir() {
                let Some(entries) = path.read_dir().ok_inspect(|e| warn!("{e}")) else {
                    continue;
                };

                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        load_license(
                            &mut licenses,
                            PathBuf::from(&name).join(entry.file_name()).display(),
                            &strategy,
                            &path,
                        );
                    }
                }
            } else if metadata.is_file() {
                load_license(&mut licenses, name.display(), &strategy, &path);
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
    if licenses.is_empty() {
        writeln!(
            out,
            "lib.licenses.unfree; # FIXME: nix-init did not find a license",
        )?;
    } else if let [license] = &licenses[..] {
        writeln!(out, "lib.licenses.{license};")?;
    } else {
        writeln!(out, "with lib.licenses; [")?;
        for license in licenses {
            writeln!(out, "      {license}")?;
        }
        writeln!(out, "    ];")?;
    }

    if cfg.maintainers.len() < 2 {
        write!(out, "    maintainers = with lib.maintainers; [ ")?;
        for maintainer in cfg.maintainers {
            write!(out, "{maintainer} ")?;
        }
        writeln!(out, "];")?;
    } else {
        writeln!(out, "    maintainers = with lib.maintainers; [")?;
        for maintainer in cfg.maintainers {
            writeln!(out, "      {maintainer}")?;
        }
        writeln!(out, "    ];")?;
    }

    if !matches!(builder, Builder::BuildPythonPackage { application, .. } if !application) {
        writeln!(out, "    mainProgram = {pname:?};")?;
    }

    match builder {
        Builder::MkDerivation { .. } if has_zig => {
            writeln!(out, "    inherit (zig.meta) platforms;")?;
        }
        Builder::MkDerivation { .. } | Builder::MkDerivationNoCC => {
            writeln!(out, "    platforms = lib.platforms.all;")?;
        }
        _ => {}
    }

    writeln!(out, "  }};\n}})")?;

    let mut out_file = File::create(&out_path).context("failed to create output file")?;
    if let Some(fmt) = cfg.format {
        let mut args = fmt.command.into_iter();
        if let Some(cmd) = args.next() {
            let mut cmd = Command::new(cmd);
            cmd.args(args);
            maybe_format(&out, out_file, cmd).await?;
        } else {
            error!("format.command should contain at least 1 element");
            write!(out_file, "{out}")?;
        }
    } else if which("nixfmt").is_ok() {
        maybe_format(&out, out_file, Command::new("nixfmt")).await?;
    } else {
        write!(out_file, "{out}")?;
    }

    if !opts.commit.unwrap_or(cfg.commit) || !Path::new(".git").is_dir() {
        return Ok(());
    }
    let Some(out_dir) = out_dir else {
        return Ok(());
    };

    let mut xs = out_path.components();
    let attr: &str = match (
        xs.next(),
        xs.next(),
        xs.next(),
        xs.next(),
        xs.next(),
        xs.next(),
    ) {
        (
            Some(Component::Normal(pkgs)),
            Some(Component::Normal(by_name)),
            Some(Component::Normal(_)),
            Some(Component::Normal(attr)),
            Some(Component::Normal(package_nix)),
            None,
        ) if pkgs == "pkgs" && by_name == "by-name" && package_nix == "package.nix" => {
            attr.try_into()?
        }
        _ => return Ok(()),
    };

    Command::new("git")
        .arg("add")
        .arg("-N")
        .arg(out_dir)
        .run()
        .await?;

    Command::new("git")
        .arg("commit")
        .arg(out_dir)
        .arg("-om")
        .arg(format!("{attr}: init at {version}\n\n{url}"))
        .run()
        .await?;

    Ok(())
}

fn get_version(rev: &str) -> &str {
    if rev.len() == 40 {
        "unstable"
    } else {
        get_version_number(rev)
    }
}

fn get_version_number(rev: &str) -> &str {
    &rev[rev.find(char::is_numeric).unwrap_or_default() ..]
}

async fn maybe_format(content: &str, mut file: File, cmd: Command) -> Result<()> {
    if let Err(e) = try_format(content, &file, cmd).await {
        error!("{e}");
        file.rewind()?;
        file.set_len(0)?;
        write!(file, "{content}")?;
    }
    Ok(())
}

async fn try_format(content: &str, file: &File, mut cmd: Command) -> Result<()> {
    let (reader, mut writer) = pipe()?;
    info!("{cmd:?}");

    let mut child = cmd
        .stdin(reader)
        .stdout(file.try_clone()?)
        .stderr(Stdio::inherit())
        .spawn()?;

    write!(writer, "{content}")?;
    drop(writer);

    let status = child.wait().await?;
    if !status.success() {
        bail!("formatter failed with {status}");
    }

    Ok(())
}
