use std::fmt::{self, Display, Formatter};

use crate::{cli::CargoVendor, codegen::Builder};

#[derive(Clone, Copy)]
pub struct MkDerivation {
    cc: bool,
    rust: Option<CargoVendor>,
}

impl MkDerivation {
    pub fn new(rust: Option<CargoVendor>) -> Self {
        Self { cc: true, rust }
    }

    pub fn no_cc() -> Self {
        Self {
            cc: false,
            rust: None,
        }
    }
}

impl Builder for MkDerivation {
    fn function(&self) -> &'static str {
        if self.cc {
            "stdenv.mkDerivation"
        } else {
            "stdenvNoCC.mkDerivation"
        }
    }

    fn explicit_strict_deps(&self) -> bool {
        true
    }

    fn cargo_deps(&self) -> Option<CargoVendor> {
        self.rust
    }

    fn infer_setup_hooks(&self) -> bool {
        true
    }

    fn explicit_platforms(&self) -> bool {
        true
    }
}

impl Display for MkDerivation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.function())?;
        if let Some(rust) = self.rust {
            write!(f, " + {rust}")?;
        }
        Ok(())
    }
}
