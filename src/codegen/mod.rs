pub mod drv;
pub mod dune;
pub mod go;
pub mod npm;
pub mod python;
pub mod rust;

use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs::read_dir,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

use anyhow::Result;
use askalono::ScanStrategy;
use cargo::core::Resolve;
use enum_dispatch::enum_dispatch;
use expand::expand;
use indoc::writedoc;
use itertools::Itertools;
use parse_display::Display;
use tracing::warn;

use crate::{
    cli::CargoVendor,
    codegen::{
        drv::MkDerivation, dune::BuildDunePackage, go::BuildGoModule, npm::BuildNpmPackage,
        python::BuildPythonPackage, rust::BuildRustPackage,
    },
    frontend::FrontendDispatch,
    inputs::{AllInputs, write_all_lambda_inputs, write_inputs, write_lambda_input},
    lang::{
        python::PythonDependencies,
        rust::{cargo_deps_hash, load_cargo_lock, write_cargo_lock},
    },
    license::{LICENSE_STORE, load_license},
    utils::ResultExt,
};

#[enum_dispatch(Builder)]
#[derive(Clone, Copy, Display)]
#[display("{0}")]
pub enum BuilderDispatch {
    BuildDunePackage(BuildDunePackage),
    BuildGoModule(BuildGoModule),
    BuildNpmPackage(BuildNpmPackage),
    BuildPythonPackage(BuildPythonPackage),
    BuildRustPackage(BuildRustPackage),
    MkDerivation(MkDerivation),
}

pub struct Codegen<'a> {
    pub description: String,
    pub fetcher_input: String,
    pub file_url_prefix: Option<String>,
    pub frontend: &'a mut FrontendDispatch,
    pub inputs: AllInputs,
    pub layout: SourceLayout,
    pub licenses: BTreeMap<&'static str, f32>,
    pub maintainers: &'a [String],
    pub nix_update_script: bool,
    pub nixpkgs: &'a str,
    pub out: String,
    pub out_dir: Option<&'a Path>,
    pub overwrite: Option<bool>,
    pub pname: &'a str,
    pub python_deps: PythonDependencies,
    pub releases_page: Option<String>,
    pub src: &'a str,
    pub src_dir: &'a Path,
    pub src_expr: &'a str,
    pub url: &'a str,
    pub version: &'a str,
}

pub struct SourceLayout {
    pub has_cargo: bool,
    pub has_cargo_lock: bool,
    pub has_dune: bool,
    pub has_cmake: bool,
    pub has_go: bool,
    pub has_meson: bool,
    pub has_npm: bool,
    pub has_npm_lock: bool,
    pub has_python: bool,
    pub has_zig: bool,
}

enum CargoDeps {
    Hash(String),
    Lock {
        has_cargo_lock: bool,
        resolve: Box<Option<Resolve>>,
    },
}

#[enum_dispatch]
pub trait Builder {
    fn function(&self) -> &'static str;

    fn after_version(&self, _: &mut Codegen<'_>) -> Result<String> {
        Ok(String::new())
    }

    fn explicit_strict_deps(&self) -> bool {
        false
    }

    async fn after_src(&self, _: &mut Codegen<'_>) -> Result<String> {
        Ok(String::new())
    }

    fn cargo_deps(&self) -> Option<CargoVendor> {
        None
    }

    fn infer_setup_hooks(&self) -> bool {
        false
    }

    fn extra_lambda_inputs(&self, _: &Codegen<'_>) -> Vec<String> {
        Vec::new()
    }

    fn native_build_inputs_attr(&self) -> &'static str {
        "nativeBuildInputs"
    }

    fn after_inputs(&self, _: &mut Codegen<'_>) -> Result<String> {
        Ok(String::new())
    }

    fn has_main_program(&self) -> bool {
        true
    }

    fn explicit_platforms(&self) -> bool {
        false
    }

    fn allow_by_name(&self) -> bool {
        true
    }
}

impl Codegen<'_> {
    pub async fn generate(mut self, builder: impl Builder) -> Result<String> {
        let function = builder.function();
        let builder_input = function
            .split_once('.')
            .map_or(function, |(input, _)| input);

        writedoc!(
            self.out,
            "
                {{
                  lib,
                  {builder_input},
                  {},
            ",
            self.fetcher_input,
        )?;

        if builder.infer_setup_hooks() {
            if self.layout.has_cmake {
                self.inputs
                    .native_build_inputs
                    .always
                    .insert("cmake".into());
            }
            if self.layout.has_meson {
                self.inputs
                    .native_build_inputs
                    .always
                    .extend(["meson".into(), "ninja".into()]);
            }
            if self.layout.has_zig {
                self.inputs.native_build_inputs.always.insert("zig".into());
            }
        }

        let after_version = builder.after_version(&mut self)?;
        let mut after_src = builder.after_src(&mut self).await?;
        let cargo_deps = builder.cargo_deps();
        if let Some(vendor) = cargo_deps {
            self.inputs.native_build_inputs.always.extend([
                "cargo".into(),
                "rustPlatform.cargoSetupHook".into(),
                "rustc".into(),
            ]);
            match prepare_cargo_deps(&mut self, vendor).await? {
                CargoDeps::Hash(hash) => {
                    write!(after_src, "  ")?;
                    writedoc! {
                        after_src,
                        r#"
                            cargoDeps = rustPlatform.fetchCargoVendor {{
                                inherit (finalAttrs) pname version src;
                                hash = "{hash}";
                              }};

                        "#,
                    }?;
                }
                CargoDeps::Lock {
                    has_cargo_lock,
                    resolve,
                } => {
                    write!(after_src, "  cargoDeps = rustPlatform.importCargoLock ")?;
                    write_cargo_lock(&mut after_src, has_cargo_lock, *resolve).await?;
                }
            }
        }
        let after_inputs = builder.after_inputs(&mut self)?;

        let mut written = BTreeSet::from([builder_input.into()]);
        if cargo_deps.is_some() {
            write_lambda_input(&mut self.out, &mut written, "rustPlatform")?;
        }
        let (native_build_inputs, build_inputs) =
            write_all_lambda_inputs(&mut self.out, &self.inputs, &mut written)?;
        for input in builder.extra_lambda_inputs(&self) {
            write_lambda_input(&mut self.out, &mut written, &input)?;
        }
        if self.nix_update_script {
            writeln!(self.out, "  nix-update-script,")?;
        }

        writedoc! {
            self.out,
            r#"
                }}:

                {function} (finalAttrs: {{
                  pname = {pname:?};
                  version = {version:?};
            "#,
            pname = self.pname,
            version = self.version,
        }?;
        write!(self.out, "{after_version}")?;
        writeln!(self.out, "  __structuredAttrs = true;")?;
        if builder.explicit_strict_deps() {
            writeln!(self.out, "  strictDeps = true;")?;
        }
        writeln!(self.out, "\n  src = {};\n", self.src_expr)?;

        write!(self.out, "{after_src}")?;
        if native_build_inputs {
            write_inputs(
                &mut self.out,
                &self.inputs.native_build_inputs,
                builder.native_build_inputs_attr(),
            )?;
        }
        if build_inputs {
            write_inputs(&mut self.out, &self.inputs.build_inputs, "buildInputs")?;
        }
        write!(self.out, "{after_inputs}")?;

        if !self.inputs.env.is_empty() {
            writeln!(self.out, "  env = {{")?;
            for (k, (v, _)) in std::mem::take(&mut self.inputs.env) {
                writeln!(self.out, "    {k} = {v};")?;
            }
            writeln!(self.out, "  }};\n")?;
        }

        if self.nix_update_script {
            writeln!(
                self.out,
                "  passthru.updateScript = nix-update-script {{ }};\n"
            )?;
        }

        self.write_meta(&builder)?;
        writeln!(self.out, "}})")?;

        Ok(self.out)
    }

    fn write_meta(&mut self, builder: &impl Builder) -> Result<()> {
        let mut description = self
            .description
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_owned();
        description.get_mut(0 .. 1).map(str::make_ascii_uppercase);
        write!(self.out, "  ")?;
        writedoc! {
            self.out,
            r"
                meta = {{
                    description = {description:?};
                    homepage = {:?};
            ",
            self.url,
        }?;

        self.write_changelog()?;
        self.write_licenses()?;
        if self.maintainers.len() < 2 {
            write!(self.out, "    maintainers = with lib.maintainers; [ ")?;
            for maintainer in self.maintainers {
                write!(self.out, "{maintainer} ")?;
            }
            writeln!(self.out, "];")?;
        } else {
            writeln!(self.out, "    maintainers = with lib.maintainers; [")?;
            for maintainer in self.maintainers {
                writeln!(self.out, "      {maintainer}")?;
            }
            writeln!(self.out, "    ];")?;
        }

        if builder.has_main_program() {
            writeln!(self.out, "    mainProgram = {:?};", self.pname)?;
        }

        if builder.explicit_platforms() {
            writeln!(self.out, "    platforms = lib.platforms.all;")?;
        }

        writeln!(self.out, "  }};")?;
        Ok(())
    }

    fn write_changelog(&mut self) -> Result<()> {
        let mut found_changelog = false;
        if let Some(file_url_prefix) = &self.file_url_prefix
            && let Some(walk) = read_dir(self.src_dir).ok_inspect(|e| warn!("{e}"))
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
                    writeln!(self.out, r#"    changelog = "{file_url_prefix}{name}";"#)?;
                    found_changelog = true;
                    break;
                }
            }
        }
        if !found_changelog && let Some(releases_page) = &self.releases_page {
            writeln!(self.out, r#"    changelog = "{releases_page}";"#)?;
        }
        Ok(())
    }

    fn write_licenses(&mut self) -> Result<()> {
        if let Some(store) = &*LICENSE_STORE
            && let Some(entries) = read_dir(self.src_dir).ok_inspect(|e| warn!("{e}"))
        {
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
                                &mut self.licenses,
                                PathBuf::from(&name).join(entry.file_name()).display(),
                                &strategy,
                                &path,
                            );
                        }
                    }
                } else if metadata.is_file() {
                    load_license(&mut self.licenses, name.display(), &strategy, &path);
                }
            }
        }

        let licenses: Vec<_> = self
            .licenses
            .iter()
            .sorted_unstable_by(|x, y| match x.1.partial_cmp(y.1) {
                None | Some(Ordering::Equal) => x.0.cmp(y.0),
                Some(cmp) => cmp,
            })
            .map(|(&license, _)| license)
            .collect();

        write!(self.out, "    license = ")?;
        if licenses.is_empty() {
            writeln!(
                self.out,
                "lib.licenses.unfree; # FIXME: nix-init did not find a license",
            )?;
        } else if let [license] = &licenses[..] {
            writeln!(self.out, "lib.licenses.{license};")?;
        } else {
            writeln!(self.out, "with lib.licenses; [")?;
            for license in licenses {
                writeln!(self.out, "      {license}")?;
            }
            writeln!(self.out, "    ];")?;
        }
        Ok(())
    }
}

async fn prepare_cargo_deps(cg: &mut Codegen<'_>, vendor: CargoVendor) -> Result<CargoDeps> {
    Ok(match vendor {
        CargoVendor::FetchCargoVendor => CargoDeps::Hash(
            cargo_deps_hash(
                &mut cg.inputs,
                cg.pname,
                cg.version,
                cg.src,
                cg.src_dir,
                cg.layout.has_cargo_lock,
                cg.nixpkgs,
            )
            .await,
        ),
        CargoVendor::ImportCargoLock => {
            let resolve = if let Some(out_dir) = cg.out_dir {
                load_cargo_lock(
                    cg.frontend,
                    out_dir,
                    &mut cg.inputs,
                    cg.src_dir,
                    cg.overwrite,
                )
                .await?
            } else {
                None
            };
            CargoDeps::Lock {
                has_cargo_lock: cg.layout.has_cargo_lock,
                resolve: Box::new(resolve),
            }
        }
    })
}

impl SourceLayout {
    pub fn detect(src_dir: &Path) -> Self {
        Self {
            has_cargo: src_dir.join("Cargo.toml").is_file(),
            has_cargo_lock: src_dir.join("Cargo.lock").is_file(),
            has_cmake: src_dir.join("CMakeLists.txt").is_file(),
            has_go: src_dir.join("go.mod").is_file(),
            has_dune: src_dir.join("dune-project").is_file(),
            has_meson: src_dir.join("meson.build").is_file(),
            has_npm: src_dir.join("package.json").is_file(),
            has_npm_lock: src_dir.join("package-lock.json").is_file()
                || src_dir.join("npm-shrinkwrap.json").is_file(),
            has_python: src_dir.join("pyproject.toml").is_file()
                || src_dir.join("setup.py").is_file(),
            has_zig: src_dir.join("build.zig").is_file(),
        }
    }
}
