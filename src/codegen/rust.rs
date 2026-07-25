use std::fmt::{self, Display, Formatter, Write as _};

use anyhow::Result;

use crate::{
    cli::CargoVendor,
    codegen::{Builder, CargoDeps, Codegen, prepare_cargo_deps},
    lang::rust::write_cargo_lock,
};

#[derive(Clone, Copy)]
pub struct BuildRustPackage {
    deps: CargoVendor,
}

impl BuildRustPackage {
    pub fn new(deps: CargoVendor) -> Self {
        Self { deps }
    }
}

impl Builder for BuildRustPackage {
    fn function(&self) -> &'static str {
        "rustPlatform.buildRustPackage"
    }

    async fn after_src(&self, cg: &mut Codegen<'_>) -> Result<String> {
        let mut out = String::new();
        match prepare_cargo_deps(cg, self.deps).await? {
            CargoDeps::Hash(hash) => {
                writeln!(out, "  cargoHash = \"{hash}\";\n")?;
            }
            CargoDeps::Lock {
                has_cargo_lock,
                resolve,
            } => {
                write!(out, "  cargoLock = ")?;
                write_cargo_lock(&mut out, has_cargo_lock, *resolve).await?;
            }
        }
        Ok(out)
    }
}

impl Display for BuildRustPackage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "buildRustPackage - {}",
            match self.deps {
                CargoVendor::FetchCargoVendor => "cargoHash",
                CargoVendor::ImportCargoLock => "cargoLock",
            }
        )
    }
}
