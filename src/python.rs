use serde::Deserialize;
use serde_with::{serde_as, Map};
use tracing::warn;

use std::{cmp::Ordering, collections::BTreeSet, fs, path::PathBuf};

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct Pyproject {
    project: Project,
    tool: Tool,
}

#[derive(Default, Deserialize)]
struct Project {
    name: Option<String>,
    dependencies: Option<Vec<String>>,
}

#[derive(Default, Deserialize)]
struct Tool {
    poetry: Poetry,
}

#[serde_as]
#[derive(Default, Deserialize)]
struct Poetry {
    name: Option<String>,
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
        toml::from_str(&fs::read_to_string(path).map_err(|e| warn!("{e}")).ok()?)
            .map_err(|e| warn!("{e}"))
            .ok()
    }

    pub fn get_name(&mut self) -> Option<String> {
        self.project
            .name
            .take()
            .or_else(|| self.tool.poetry.name.take())
    }

    pub fn get_dependencies(&mut self) -> Vec<String> {
        if let Some(deps) = self.project.dependencies.take() {
            deps.into_iter().filter_map(get_dependency).collect()
        } else if let Some(deps) = self.tool.poetry.dependencies.take() {
            deps.into_iter()
                .filter_map(|(dep, _)| (dep != "python").then_some(dep))
                .collect()
        } else {
            Vec::new()
        }
    }
}

fn get_dependency(dep: String) -> Option<String> {
    let mut chars = dep.chars().skip_while(|c| c.is_whitespace());

    let x = chars.next()?;
    if !x.is_alphabetic() {
        return None;
    }
    let mut name = String::from(x);

    while let Some(c) = chars.next() {
        if c.is_alphabetic() {
            name.push(c);
        } else if matches!(c, '-' | '.' | '_') {
            match chars.next() {
                Some(c) if c.is_alphabetic() => {
                    name.push('-');
                    name.push(c);
                }
                _ => break,
            }
        }
    }

    Some(name)
}
