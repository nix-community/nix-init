use parse_display::Display;

use std::fmt::{self, Display, Formatter};

#[derive(Clone, Copy)]
pub enum BuildType {
    BuildGoModule,
    BuildPythonPackage {
        application: bool,
        format: PythonFormat,
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
}

#[derive(Clone, Copy, Display)]
#[display(style = "camelCase")]
pub enum PythonFormat {
    Pyproject,
    Setuptools,
}

impl Display for BuildType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            BuildType::BuildGoModule => {
                write!(f, "buildGoModule")?;
            }

            BuildType::BuildPythonPackage {
                application,
                format,
                rust,
            } => {
                write!(
                    f,
                    "buildPython{} - {format}",
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
