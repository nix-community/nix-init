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

impl BuildType {
    pub fn cli_flag(&self) -> Option<&'static str> {
        match self {
            BuildType::BuildGoModule => Some("go-mod"),
            BuildType::BuildPythonPackage {
                application: true, ..
            } => Some("python-app"),
            BuildType::BuildPythonPackage {
                application: false, ..
            } => Some("python-pkg"),
            BuildType::BuildRustPackage { .. } => Some("rust-pkg"),
            BuildType::MkDerivation { rust: None } => Some("drv"),
            BuildType::MkDerivation { rust: Some(_) } => None,
        }
    }

    pub fn from_cli_flag(flag: &str, choices: &[BuildType]) -> Option<BuildType> {
        choices.iter().find(|c| c.cli_flag() == Some(flag)).copied()
    }
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
