use anyhow::Result;
use parse_display::Display;

use crate::{
    codegen::{Builder, Codegen},
    lang::dune::get_dune_dependencies,
};

#[derive(Clone, Copy, Display)]
#[display("buildDunePackage")]
pub struct BuildDunePackage;

impl Builder for BuildDunePackage {
    fn function(&self) -> &'static str {
        "buildDunePackage"
    }

    fn after_inputs(&self, cg: &mut Codegen<'_>) -> Result<String> {
        let deps = get_dune_dependencies(cg.src_dir);

        for tool in deps.tools {
            cg.inputs
                .native_build_inputs
                .always
                .insert(format!("ocamlPackages.{tool}"));
        }
        for dep in &deps.build {
            cg.inputs
                .build_inputs
                .always
                .insert(format!("ocamlPackages.{dep}"));
        }
        for dep in &deps.check {
            cg.inputs
                .check_inputs
                .always
                .insert(format!("ocamlPackages.{dep}"));
        }
        for license in deps.licenses {
            cg.licenses.insert(license, 1.0);
        }

        Ok(String::new())
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, fs::write};

    use insta::assert_snapshot;
    use tempfile::TempDir;

    use super::*;
    use crate::codegen::SourceLayout;

    async fn generate_dune(dune_project: bool) -> String {
        let dir = TempDir::new().unwrap();
        if dune_project {
            write(
                dir.path().join("dune-project"),
                include_str!("../lang/fixtures/ocaml-lsp.dune-project"),
            )
            .unwrap();
        } else {
            write(
                dir.path().join("ocaml-lsp-server.opam"),
                include_str!("../lang/fixtures/ocaml-lsp-server.opam"),
            )
            .unwrap();
        }
        let dir_path = dir.path().to_owned();

        let mut frontend = crate::frontend::headless();
        let cg = Codegen {
            description: "LSP Server for OCaml".into(),
            fetcher_input: "fetchFromGitHub".into(),
            file_url_prefix: None,
            frontend: &mut frontend,
            inputs: Default::default(),
            layout: SourceLayout::detect(dir_path.as_path()),
            licenses: BTreeMap::new(),
            maintainers: &["alice".to_string()],
            nix_update_script: true,
            nixpkgs: "<nixpkgs>",
            out: String::new(),
            out_dir: None,
            overwrite: None,
            pname: "ocaml-lsp",
            python_deps: Default::default(),
            releases_page: None,
            src: "src",
            src_dir: &dir_path,
            src_expr: "src",
            url: "https://github.com/ocaml/ocaml-lsp",
            version: "1.27.0",
        };

        cg.generate(BuildDunePackage).await.unwrap()
    }

    #[tokio::test]
    async fn code_dune_project() {
        assert_snapshot!(generate_dune(true).await);
    }

    #[tokio::test]
    async fn code_opam() {
        assert_snapshot!(generate_dune(false).await);
    }
}
