use std::{fs, path::PathBuf};

use serde::Deserialize;
use tracing::warn;

#[derive(Deserialize)]
pub struct Pyproject {
    pub project: Project,
}

#[derive(Deserialize)]
pub struct Project {
    pub name: String,
    pub dependencies: Vec<String>,
}

pub fn parse_pyproject(path: PathBuf) -> Option<Pyproject> {
    toml::from_str(&fs::read_to_string(path).map_err(|e| warn!("{e}")).ok()?)
        .map_err(|e| warn!("{e}"))
        .ok()
}

pub fn get_dependency(dep: String) -> Option<String> {
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
