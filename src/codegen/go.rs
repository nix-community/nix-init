use std::fmt::Write as _;

use anyhow::Result;
use parse_display::Display;
use tracing::warn;

use crate::{
    codegen::{Builder, Codegen},
    lang::go::{load_go_dependencies, write_ldflags},
    utils::{FAKE_HASH, ResultExt, fod_hash},
};

#[derive(Clone, Copy, Display)]
#[display("buildGoModule")]
pub struct BuildGoModule;

impl Builder for BuildGoModule {
    fn function(&self) -> &'static str {
        "buildGoModule"
    }

    async fn after_src(&self, cg: &mut Codegen<'_>) -> Result<String> {
        let mut out = String::new();
        let go_sum = std::fs::File::open(cg.src_dir.join("go.sum")).ok_inspect(|e| warn!("{e}"));

        if let Some(go_sum) = &go_sum {
            load_go_dependencies(&mut cg.inputs, go_sum);
        }

        let hash = if cg.src_dir.join("vendor").is_dir()
            || go_sum
                .and_then(|go_sum| go_sum.metadata().ok_inspect(|e| warn!("{e}")))
                .is_none_or(|metadata| metadata.len() == 0)
        {
            "null".into()
        } else if let Some(hash) = fod_hash(format!(
            r#"(import({}){{}}).buildGoModule{{pname={:?};version={:?};src={};vendorHash="{FAKE_HASH}";}}"#,
            cg.nixpkgs, cg.pname, cg.version, cg.src,
        ))
        .await
        {
            format!(r#""{hash}""#)
        } else {
            format!(r#""{FAKE_HASH}""#)
        };

        writeln!(out, "  vendorHash = {hash};\n")?;
        Ok(out)
    }

    fn after_inputs(&self, cg: &mut Codegen<'_>) -> Result<String> {
        let mut out = String::new();
        write_ldflags(&mut out, cg.src_dir)?;
        Ok(out)
    }
}
