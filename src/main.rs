mod cfg;
mod cli;
mod cmd;
mod codegen;
mod fetcher;
mod frontend;
mod inputs;
mod lang;
mod license;
mod macros;
mod utils;

use std::{
    collections::BTreeMap,
    fmt::Write as _,
    fs::{File, create_dir_all, metadata},
    io::{IsTerminal, Seek, Write as _, pipe, stderr},
    os::unix::ffi::OsStrExt,
    path::{Component, Path, PathBuf},
    process::Stdio,
    sync::LazyLock,
};

use anyhow::{Context as _, Result, bail};
use clap::{Parser, crate_version};
use flate2::read::GzDecoder;
use heck::ToKebabCase;
use indoc::formatdoc;
use serde::Deserialize;
use tempfile::tempdir;
use tokio::process::Command;
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;
use which::which;
use zip::ZipArchive;

use crate::{
    cfg::load_config,
    cli::{
        BuilderFunction::{self},
        CargoVendor, Opts,
    },
    cmd::{NIX, NURL},
    codegen::{
        BuilderDispatch, Codegen, SourceLayout, drv::MkDerivation, dune::BuildDunePackage,
        go::BuildGoModule, npm::BuildNpmPackage, python::BuildPythonPackage,
        rust::BuildRustPackage,
    },
    fetcher::{Fetcher, FetcherDispatch, PackageInfo, PypiFormat, Revisions, Version},
    frontend::{Frontend, headless, readline},
    lang::python::PythonDependencies,
    license::LICENSE_STORE,
    utils::{CommandExt, ResultExt},
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

#[derive(Default)]
struct SourceInfo {
    pname: Option<String>,
    rev: String,
    version: String,
    description: String,
    file_url_prefix: Option<String>,
    releases_page: Option<String>,
    python_dependencies: PythonDependencies,
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
    let SourceInfo {
        pname,
        rev,
        version,
        description,
        file_url_prefix,
        releases_page,
        python_dependencies,
    } = if let MaybeFetcher::Known(fetcher) = &mut fetcher {
        let cl = fetcher.create_client(cfg.access_tokens).await?;

        let PackageInfo {
            pname,
            description,
            file_url_prefix,
            homepage,
            license,
            mut releases_page,
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
            if !matches!(version, Some(Version::Latest | Version::Tag)) {
                releases_page = None;
            }
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

        SourceInfo {
            pname: Some(pname),
            rev,
            version,
            description,
            file_url_prefix,
            releases_page,
            python_dependencies,
        }
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

        SourceInfo {
            pname,
            rev,
            version,
            ..Default::default()
        }
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

    let layout = SourceLayout::detect(&src_dir);

    let builder = match (opts.builder, opts.cargo_vendor) {
        (Some(builder), rust @ Some(vendor)) if layout.has_cargo => match builder {
            BuilderFunction::BuildDunePackage => BuildDunePackage.into(),
            BuilderFunction::BuildGoModule => BuildGoModule.into(),
            BuilderFunction::BuildNpmPackage => BuildNpmPackage.into(),
            BuilderFunction::BuildPythonApplication => BuildPythonPackage::new(true, rust).into(),
            BuilderFunction::BuildPythonPackage => BuildPythonPackage::new(false, rust).into(),
            BuilderFunction::BuildRustPackage => BuildRustPackage::new(vendor).into(),
            BuilderFunction::MkDerivation => MkDerivation::new(rust).into(),
            BuilderFunction::MkDerivationNoCC => MkDerivation::no_cc().into(),
        },
        (Some(builder), _) => {
            let rust = layout.has_cargo.then_some(CargoVendor::FetchCargoVendor);
            match builder {
                BuilderFunction::BuildDunePackage => BuildDunePackage.into(),
                BuilderFunction::BuildGoModule => BuildGoModule.into(),
                BuilderFunction::BuildNpmPackage => BuildNpmPackage.into(),
                BuilderFunction::BuildPythonApplication => {
                    BuildPythonPackage::new(true, rust).into()
                }
                BuilderFunction::BuildPythonPackage => BuildPythonPackage::new(false, rust).into(),
                BuilderFunction::BuildRustPackage => {
                    BuildRustPackage::new(CargoVendor::FetchCargoVendor).into()
                }
                BuilderFunction::MkDerivation => MkDerivation::new(rust).into(),
                BuilderFunction::MkDerivationNoCC => MkDerivation::no_cc().into(),
            }
        }
        (None, rust) => {
            let mut builders = Vec::new();
            if layout.has_go {
                builders.push(BuildGoModule.into());
            }

            if layout.has_cargo {
                let cargo_deps_options: &[_] = match rust {
                    Some(vendor) => &[vendor],
                    None => &[CargoVendor::FetchCargoVendor, CargoVendor::ImportCargoLock],
                };

                for &vendor in cargo_deps_options {
                    if layout.has_python {
                        for application in [true, false] {
                            builders
                                .push(BuildPythonPackage::new(application, Some(vendor)).into());
                        }
                    }

                    let drv = BuilderDispatch::from(MkDerivation::new(Some(vendor)));
                    let rust = BuilderDispatch::from(BuildRustPackage::new(vendor));
                    builders.extend(if layout.has_meson {
                        [drv, rust]
                    } else {
                        [rust, drv]
                    });
                }
            }

            if layout.has_dune {
                builders.push(BuildDunePackage.into());
            }

            if layout.has_python {
                for application in [true, false] {
                    builders.push(BuildPythonPackage::new(application, None).into());
                }
            }

            if layout.has_npm {
                builders.push(BuildNpmPackage.into());
            }

            builders.push(MkDerivation::new(None).into());
            builders.push(MkDerivation::no_cc().into());

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

    let nix_update_script = matches!(fetcher, MaybeFetcher::Known(_));
    let fetcher_input = match fetcher {
        MaybeFetcher::Known(fetcher) => fetcher.to_string(),
        MaybeFetcher::Unknown { fetcher } => fetcher,
    };
    let cg = Codegen {
        description,
        fetcher_input,
        file_url_prefix,
        frontend: &mut frontend,
        inputs: Default::default(),
        layout,
        licenses,
        maintainers: &cfg.maintainers,
        nix_update_script,
        nixpkgs: &nixpkgs,
        out: String::new(),
        out_dir,
        overwrite: opts.overwrite,
        pname: &pname,
        python_deps: python_dependencies,
        releases_page,
        src: &src,
        src_dir: &src_dir,
        src_expr: &src_expr,
        url: &url,
        version: &version,
    };
    let out = cg.generate(builder).await?;

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
        let mut cmd = Command::new("nixfmt");
        cmd.arg("-");
        maybe_format(&out, out_file, cmd).await?;
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

    let msg = formatdoc! {r#"
      {attr}: init at {version}

      {url}

      Assisted-by: nix-init"#,
    };
    Command::new("git")
        .arg("commit")
        .arg(out_dir)
        .arg("-om")
        .arg(msg)
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
