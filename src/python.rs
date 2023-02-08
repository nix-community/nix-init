use serde::Deserialize;
use serde_with::{serde_as, DefaultOnError, Map};

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use crate::{inputs::AllInputs, license::parse_spdx_expression, utils::ResultExt};

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Pyproject {
    build_system: BuildSystem,
    project: Project,
    tool: Tool,
}

#[serde_as]
#[derive(Default, Deserialize)]
#[serde(default)]
struct BuildSystem {
    requires: Vec<String>,
}

#[serde_as]
#[derive(Default, Deserialize)]
#[serde(default)]
struct Project {
    name: Option<String>,
    #[serde_as(as = "DefaultOnError")]
    license: Option<String>,
    dependencies: Option<Vec<String>>,
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct Tool {
    poetry: Poetry,
}

#[serde_as]
#[derive(Default, Deserialize)]
#[serde(default)]
struct Poetry {
    name: Option<String>,
    #[serde_as(as = "DefaultOnError")]
    license: Option<String>,
    #[serde_as(as = "Option<Map<_, DefaultOnError>>")]
    dependencies: Option<BTreeSet<(String, ())>>,
}

impl Pyproject {
    pub fn from_path(path: PathBuf) -> Option<Pyproject> {
        toml::from_str(&fs::read_to_string(path).ok_warn()?).ok_warn()
    }

    pub fn get_name(&mut self) -> Option<String> {
        self.project
            .name
            .take()
            .or_else(|| self.tool.poetry.name.take())
    }

    pub fn load_license(&self, licenses: &mut BTreeMap<&'static str, f32>) {
        if let Some(license) = self
            .project
            .license
            .as_ref()
            .or(self.tool.poetry.license.as_ref())
        {
            for license in parse_spdx_expression(license, "pyproject.toml") {
                licenses.insert(license, 1.0);
            }
        }
    }

    pub fn load_build_dependencies(&self, inputs: &mut AllInputs) {
        inputs.native_build_inputs.always.extend(
            self.build_system
                .requires
                .iter()
                .filter_map(get_python_dependency)
                .map(|dep| {
                    if dep == "maturin" {
                        "rustPlatform.maturinBuildHook".into()
                    } else {
                        format!("python3.pkgs.{dep}")
                    }
                }),
        );
    }

    pub fn get_dependencies(&self) -> Option<BTreeSet<String>> {
        if let Some(deps) = &self.project.dependencies {
            Some(deps.iter().filter_map(get_python_dependency).collect())
        } else if let Some(deps) = &self.tool.poetry.dependencies {
            let mut deps: BTreeSet<_> = deps
                .iter()
                .map(|(dep, _)| dep.to_lowercase().replace(['_', '.'], "-"))
                .collect();
            deps.remove("python");
            Some(deps)
        } else {
            None
        }
    }
}

pub fn parse_requirements_txt(src: &Path) -> Option<BTreeSet<String>> {
    File::open(src.join("requirements.txt")).ok().map(|file| {
        BufReader::new(file)
            .lines()
            .filter_map(|line| line.ok_warn().and_then(get_python_dependency))
            .collect()
    })
}

pub fn get_python_dependency(dep: impl AsRef<str>) -> Option<String> {
    let mut chars = dep.as_ref().chars().skip_while(|c| c.is_whitespace());

    let x = chars.next()?;
    if !x.is_alphabetic() {
        return None;
    }
    let mut name = String::from(x.to_ascii_lowercase());

    while let Some(c) = chars.next() {
        if c.is_alphabetic() {
            name.push(c.to_ascii_lowercase());
        } else if matches!(c, '-' | '.' | '_') {
            match chars.next() {
                Some(c) if c.is_alphabetic() => {
                    name.push('-');
                    name.push(c.to_ascii_lowercase());
                }
                _ => break,
            }
        } else {
            break;
        }
    }

    Some(name)
}

#[cfg(test)]
mod tests {
    use super::get_python_dependency;

    #[test]
    fn basic() {
        assert_eq!(get_python_dependency("requests"), Some("requests".into()));
        assert_eq!(get_python_dependency("Click>=7.0"), Some("click".into()));
        assert_eq!(
            get_python_dependency("tomli;python_version<'3.11'"),
            Some("tomli".into()),
        );

        assert_eq!(get_python_dependency(""), None);
        assert_eq!(get_python_dependency("# comment"), None);
    }
}
