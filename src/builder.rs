use std::fmt::{self, Display, Formatter};

use crate::cli::CargoVendor;

#[derive(Clone, Copy)]
pub enum Builder {
    BuildGoModule,
    BuildPythonPackage {
        application: bool,
        rust: Option<CargoVendor>,
    },
    BuildRustPackage {
        vendor: CargoVendor,
    },
    MkDerivation {
        rust: Option<CargoVendor>,
    },
    MkDerivationNoCC,
}

impl Display for Builder {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Builder::BuildGoModule => {
                write!(f, "buildGoModule")?;
            }

            Builder::BuildPythonPackage { application, rust } => {
                write!(
                    f,
                    "buildPython{}",
                    if *application {
                        "Application"
                    } else {
                        "Package"
                    },
                )?;
                if let Some(rust) = rust {
                    write!(f, " + {rust}")?;
                }
            }

            Builder::BuildRustPackage { vendor } => {
                write!(
                    f,
                    "buildRustPackage - {}",
                    match vendor {
                        CargoVendor::FetchCargoVendor => "cargoHash",
                        CargoVendor::ImportCargoLock => "cargoLock",
                    }
                )?;
            }

            Builder::MkDerivation { rust } => {
                write!(f, "stdenv.mkDerivation")?;
                if let Some(rust) = rust {
                    write!(f, " + {rust}")?;
                }
            }

            Builder::MkDerivationNoCC => {
                write!(f, "stdenvNoCC.mkDerivation")?;
            }
        }

        Ok(())
    }
}
