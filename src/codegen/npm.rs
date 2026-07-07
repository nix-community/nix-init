use std::fmt::Write as _;

use anyhow::Result;
use parse_display::Display;

use crate::{
    codegen::{Builder, Codegen},
    lang::npm::npm_has_build_script,
    utils::{FAKE_HASH, fod_hash},
};

#[derive(Clone, Copy, Display)]
#[display("buildNpmPackage")]
pub struct BuildNpmPackage;

impl Builder for BuildNpmPackage {
    fn function(&self) -> &'static str {
        "buildNpmPackage"
    }

    async fn after_src(&self, cg: &mut Codegen<'_>) -> Result<String> {
        let mut out = String::new();
        let hash = if cg.layout.has_npm_lock
            && let Some(hash) = fod_hash(format!(
                r#"(import({}){{}}).fetchNpmDeps{{src={};hash="{FAKE_HASH}";}}"#,
                cg.nixpkgs, cg.src,
            ))
            .await
        {
            hash
        } else {
            FAKE_HASH.into()
        };

        writeln!(out, "  npmDepsHash = \"{hash}\";\n")?;

        if !npm_has_build_script(cg.src_dir) {
            writeln!(out, "  dontNpmBuild = true;\n")?;
        }

        Ok(out)
    }
}
