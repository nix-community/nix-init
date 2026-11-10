use std::{
    fmt::Write as _,
    path::{Path, PathBuf},
};

use anyhow::Result;
use parse_display::Display;

use crate::codegen::{Builder, Codegen};

#[derive(Clone, Copy, Display)]
#[display("buildNimPackage")]
pub struct BuildNimPackage;

impl Builder for BuildNimPackage {
    fn function(&self) -> &'static str {
        "buildNimPackage"
    }

    fn explicit_strict_deps(&self) -> bool {
        true
    }

    async fn after_src(&self, cg: &mut Codegen<'_>) -> Result<String> {
        let mut out = String::new();

        let lock_path = get_lock_path(cg.out_dir);

        writeln!(
            out,
            "  # Generate lockfile with: nix run -f . nim_lk ./result | jq --sort-keys > {lock_path}"
        )?;
        writeln!(out, "  lockFile = ./lock.json;\n")?;

        if let Some(file) = find_nimble_file(cg.src_dir) {
            writeln!(out, "  nimbleFile = \"{file}\";\n")?;
        }

        writeln!(out, "  nimFlags = [ ];\n")?;

        Ok(out)
    }
}

fn get_lock_path(out_dir: Option<&Path>) -> String {
    out_dir
        .map(|d| d.join("lock.json"))
        .unwrap_or_else(|| PathBuf::from("lock.json"))
        .display()
        .to_string()
}

fn find_nimble_file(src_dir: &Path) -> Option<String> {
    std::fs::read_dir(src_dir)
        .ok()?
        .filter_map(Result::ok)
        .find(|entry| entry.path().extension().is_some_and(|ext| ext == "nimble"))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
}
