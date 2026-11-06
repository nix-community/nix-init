use parse_display::Display;

use crate::codegen::Builder;

#[derive(Clone, Copy, Display)]
#[display("buildDunePackage")]
pub struct BuildDunePackage;

impl Builder for BuildDunePackage {
    fn function(&self) -> &'static str {
        "buildDunePackage"
    }
}
