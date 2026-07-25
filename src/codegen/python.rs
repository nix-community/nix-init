use std::fmt::{self, Display, Formatter, Write as _};

use anyhow::Result;
use heck::{AsSnakeCase, ToKebabCase};

use crate::{
    cli::CargoVendor,
    codegen::{Builder, Codegen},
    lang::python::{Pyproject, parse_requirements_txt},
};

#[derive(Clone, Copy)]
pub struct BuildPythonPackage {
    application: bool,
    rust: Option<CargoVendor>,
}

impl BuildPythonPackage {
    pub fn new(application: bool, rust: Option<CargoVendor>) -> Self {
        Self { application, rust }
    }
}

impl Builder for BuildPythonPackage {
    fn function(&self) -> &'static str {
        if self.application {
            "python3Packages.buildPythonApplication"
        } else {
            "buildPythonPackage"
        }
    }

    fn after_version(&self, _: &mut Codegen<'_>) -> Result<String> {
        Ok("  pyproject = true;\n".into())
    }

    fn cargo_deps(&self) -> Option<CargoVendor> {
        self.rust
    }

    fn extra_lambda_inputs(&self, cg: &Codegen<'_>) -> Vec<String> {
        if self.application {
            return Vec::new();
        }
        cg.python_deps
            .always
            .iter()
            .chain(cg.python_deps.optional.values().flatten())
            .map(|name| name.to_kebab_case())
            .collect()
    }

    fn native_build_inputs_attr(&self) -> &'static str {
        "build-system"
    }

    fn after_inputs(&self, cg: &mut Codegen<'_>) -> Result<String> {
        let mut out = String::new();
        let mut pyproject = Pyproject::from_path(cg.src_dir.join("pyproject.toml"));
        let import = pyproject.get_name();

        if cg.src_dir.join("poetry.lock").is_file() {
            cg.inputs.native_build_inputs.always.insert(
                if self.application {
                    "python3Packages.poetry-core"
                } else {
                    "poetry-core"
                }
                .into(),
            );
        }

        pyproject.load_license(&mut cg.licenses);
        pyproject.load_build_dependencies(&mut cg.inputs, self.application);

        if let Some(deps) = pyproject.get_dependencies() {
            cg.python_deps = deps;
        }

        if cg.python_deps.always.is_empty()
            && cg.python_deps.optional.is_empty()
            && let Some(deps) = parse_requirements_txt(cg.src_dir)
        {
            cg.python_deps = deps;
        }

        if !cg.python_deps.always.is_empty() {
            write!(out, "  dependencies = ")?;
            if self.application {
                write!(out, "with python3Packages; ")?;
            }
            writeln!(out, "[")?;

            for name in &cg.python_deps.always {
                writeln!(out, "    {name}")?;
            }
            writeln!(out, "  ];\n")?;
        }

        let mut optional = cg
            .python_deps
            .optional
            .iter()
            .filter(|(_, deps)| !deps.is_empty());

        if let Some((extra, deps)) = optional.next() {
            write!(out, "  optional-dependencies = ")?;
            if self.application {
                write!(out, "with python3Packages; ")?;
            }
            writeln!(out, "{{\n    {extra} = [",)?;
            for name in deps {
                writeln!(out, "      {name}")?;
            }
            writeln!(out, "    ];")?;

            for (extra, deps) in optional {
                writeln!(out, "    {extra} = [")?;
                for name in deps {
                    writeln!(out, "      {name}")?;
                }
                writeln!(out, "    ];")?;
            }

            writeln!(out, "  }};\n")?;
        }

        writeln!(
            out,
            "  pythonImportsCheck = [\n    \"{}\"\n  ];\n",
            AsSnakeCase(import.as_deref().unwrap_or(cg.pname)),
        )?;
        Ok(out)
    }

    fn has_main_program(&self) -> bool {
        self.application
    }

    fn allow_by_name(&self) -> bool {
        self.application
    }
}

impl Display for BuildPythonPackage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "buildPython{}",
            if self.application {
                "Application"
            } else {
                "Package"
            },
        )?;
        if let Some(rust) = self.rust {
            write!(f, " + {rust}")?;
        }
        Ok(())
    }
}
