use std::fmt::{self, Display, Formatter};

use parse_display::Display;

#[derive(Clone, Copy)]
pub enum BuildType {
    BuildGoModule,
    BuildPythonPackage {
        application: bool,
        rust: Option<RustVendor>,
    },
    BuildRustPackage {
        vendor: RustVendor,
    },
    MkDerivation {
        rust: Option<RustVendor>,
    },
}

#[derive(Clone, Copy, Display)]
#[display(style = "camelCase")]
pub enum RustVendor {
    FetchCargoTarball,
    ImportCargoLock,
}

impl Display for BuildType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            BuildType::BuildGoModule => {
                write!(f, "buildGoModule")?;
            }

            BuildType::BuildPythonPackage { application, rust } => {
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

            BuildType::BuildRustPackage { vendor } => {
                write!(
                    f,
                    "buildRustPackage - {}",
                    match vendor {
                        RustVendor::FetchCargoTarball => "cargoHash",
                        RustVendor::ImportCargoLock => "cargoLock",
                    }
                )?;
            }

            BuildType::MkDerivation { rust } => {
                write!(f, "stdenv.mkDerivation")?;
                if let Some(rust) = rust {
                    write!(f, " + {rust}")?;
                }
            }
        }

        Ok(())
    }
}
