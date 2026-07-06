use std::{fs::read_to_string, path::Path};

use serde::Deserialize;
use tracing::warn;

use crate::utils::ResultExt;

#[derive(Deserialize)]
struct PackageJson {
    #[serde(default)]
    scripts: Scripts,
}

#[derive(Default, Deserialize)]
struct Scripts {
    build: Option<String>,
}

// assumes a build script exists when package.json can't be read or parsed
pub fn npm_has_build_script(src_dir: &Path) -> bool {
    let Some(package_json) =
        read_to_string(src_dir.join("package.json")).ok_inspect(|e| warn!("{e}"))
    else {
        return true;
    };

    match serde_json::from_str::<PackageJson>(&package_json) {
        Ok(package) => package.scripts.build.is_some(),
        Err(e) => {
            warn!("{e}");
            true
        }
    }
}
