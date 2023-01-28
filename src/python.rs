use serde::Deserialize;
use serde_with::{serde_as, DefaultOnError, Map};

use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use crate::{license::parse_spdx_expression, utils::ResultExt};

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct Pyproject {
    project: Project,
    tool: Tool,
}

#[serde_as]
#[derive(Default, Deserialize)]
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
struct Poetry {
    name: Option<String>,
    #[serde_as(as = "DefaultOnError")]
    license: Option<String>,
    #[serde_as(as = "Option<Map<_, _>>")]
    dependencies: Option<BTreeSet<(String, PoetryDependency)>>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum PoetryDependency {
    String(String),
    Table {},
}

impl Eq for PoetryDependency {}

impl Ord for PoetryDependency {
    fn cmp(&self, _: &Self) -> Ordering {
        Ordering::Equal
    }
}

impl PartialEq<PoetryDependency> for PoetryDependency {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl PartialOrd<PoetryDependency> for PoetryDependency {
    fn partial_cmp(&self, _: &Self) -> Option<Ordering> {
        None
    }
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

    pub fn get_dependencies(&mut self) -> Option<BTreeSet<String>> {
        if let Some(deps) = self.project.dependencies.take() {
            Some(deps.into_iter().filter_map(get_python_dependency).collect())
        } else if let Some(mut deps) = self.tool.poetry.dependencies.take() {
            deps.remove(&("python".into(), PoetryDependency::Table {}));
            Some(
                deps.into_iter()
                    .map(|(dep, _)| dep.to_lowercase().replace(['_', '.'], "-"))
                    .collect(),
            )
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

pub fn get_python_dependency(dep: String) -> Option<String> {
    let mut chars = dep.chars().skip_while(|c| c.is_whitespace());

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
        }
    }

    Some(name)
}
