use std::fmt::{self, Display, Formatter};

use parse_display::Display;

#[derive(Clone, Copy)]
pub enum Builder {
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
    FetchCargoVendor,
    ImportCargoLock,
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
                        RustVendor::FetchCargoVendor => "cargoHash",
                        RustVendor::ImportCargoLock => "cargoLock",
                    }
                )?;
            }

            Builder::MkDerivation { rust } => {
                write!(f, "stdenv.mkDerivation")?;
                if let Some(rust) = rust {
                    write!(f, " + {rust}")?;
                }
            }
        }

        Ok(())
    }
}
