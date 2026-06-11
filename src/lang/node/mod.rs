use std::{fs::File, io::BufReader, path::Path};

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
pub fn npm_lacks_build_script(src_dir: &Path) -> bool {
    let Some(file) = File::open(src_dir.join("package.json")).ok_inspect(|e| warn!("{e}")) else {
        return false;
    };

    match serde_json::from_reader::<_, PackageJson>(BufReader::new(file)) {
        Ok(package) => package.scripts.build.is_none(),
        Err(e) => {
            warn!("{e}");
            false
        }
    }
}
