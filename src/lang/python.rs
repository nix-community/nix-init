use chumsky::{error::Cheap, primitive::end, Parser};
use heck::{AsKebabCase, ToKebabCase};
use pep_508::{Comparator, Dependency, Marker, Operator, Variable};
use serde::Deserialize;
use serde_with::{serde_as, DefaultOnError};

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{BufRead, BufReader},
    mem,
    path::{Path, PathBuf},
};

use crate::{inputs::AllInputs, license::parse_spdx_expression, utils::ResultExt};

#[derive(Default)]
pub struct PythonDependencies {
    pub always: BTreeSet<String>,
    pub optional: BTreeMap<String, BTreeSet<String>>,
}

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
#[serde(default, rename_all = "kebab-case")]
struct Project {
    name: Option<String>,
    #[serde_as(as = "DefaultOnError")]
    license: Option<String>,
    dependencies: Option<Vec<String>>,
    optional_dependencies: Option<BTreeMap<String, BTreeSet<String>>>,
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
    #[serde_as(as = "Option<BTreeMap<_, DefaultOnError>>")]
    dependencies: Option<BTreeMap<String, PoetryDependency>>,
    extras: BTreeMap<String, BTreeSet<String>>,
}

#[serde_as]
#[derive(Default, Deserialize)]
struct PoetryDependency {
    optional: bool,
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

    pub fn load_build_dependencies(&self, inputs: &mut AllInputs, application: bool) {
        let parser = parser();
        inputs.native_build_inputs.always.extend(
            self.build_system
                .requires
                .iter()
                .filter_map(|dep| parser.parse(dep.as_str()).ok())
                .map(|Dependency { name, .. }| {
                    if name == "maturin" {
                        "rustPlatform.maturinBuildHook".into()
                    } else if application {
                        format!("python3.pkgs.{}", AsKebabCase(name))
                    } else {
                        name.to_kebab_case()
                    }
                }),
        );
    }

    pub fn get_dependencies(&mut self) -> Option<PythonDependencies> {
        if let Some(mut deps) = self.tool.poetry.dependencies.take() {
            deps.remove("python");
            return Some(PythonDependencies {
                always: deps
                    .into_iter()
                    .filter_map(|(name, PoetryDependency { optional })| (!optional).then_some(name))
                    .collect(),
                optional: mem::take(&mut self.tool.poetry.extras),
            });
        }

        match (
            self.project.dependencies.take(),
            self.project.optional_dependencies.take(),
        ) {
            (Some(always), None) => Some(get_python_dependencies(parser(), always)),
            (always, Some(optional)) => {
                let parser = parser();
                let mut all_deps = always.map_or_else(Default::default, |always| {
                    get_python_dependencies(&parser, always)
                });

                for (extra, deps) in optional {
                    let entry = all_deps.optional.entry(extra).or_insert_with(BTreeSet::new);
                    for dep in deps {
                        if let Ok(Dependency { name, .. }) = parser.parse(dep) {
                            entry.insert(name);
                        }
                    }
                }

                Some(all_deps)
            }
            (None, None) => None,
        }
    }
}

pub fn parse_requirements_txt(src: &Path) -> Option<PythonDependencies> {
    File::open(src.join("requirements.txt")).ok().map(|file| {
        get_python_dependencies(
            parser(),
            BufReader::new(file).lines().filter_map(ResultExt::ok_warn),
        )
    })
}

pub fn get_python_dependencies(
    parser: impl Parser<char, Dependency, Error = Cheap<char>>,
    xs: impl IntoIterator<Item = impl AsRef<str>>,
) -> PythonDependencies {
    let mut deps: PythonDependencies = Default::default();
    for Dependency { name, marker, .. } in xs
        .into_iter()
        .filter_map(|dep| parser.parse(dep.as_ref()).ok())
    {
        let mut extras = Vec::new();
        if let Some(marker) = marker {
            load_extras(&mut extras, marker);
        }

        if extras.is_empty() {
            deps.always.insert(name);
        } else {
            for extra in extras {
                deps.optional
                    .entry(extra)
                    .or_insert_with(BTreeSet::new)
                    .insert(name.clone());
            }
        }
    }
    deps
}

fn load_extras(extras: &mut Vec<String>, marker: Marker) {
    match marker {
        Marker::And(x, y) | Marker::Or(x, y) => {
            load_extras(extras, *x);
            load_extras(extras, *y);
        }
        Marker::Operator(
            Variable::Extra,
            Operator::Comparator(Comparator::Eq),
            Variable::String(extra),
        )
        | Marker::Operator(
            Variable::String(extra),
            Operator::Comparator(Comparator::Eq),
            Variable::Extra,
        ) => {
            extras.push(extra);
        }
        _ => {}
    }
}

pub fn parser() -> impl Parser<char, Dependency, Error = Cheap<char>> {
    pep_508::parser().then_ignore(end())
}
