use std::{
    collections::BTreeMap, ffi::OsStr, fmt::Display, fs::read_to_string, path::Path, sync::LazyLock,
};

use askalono::{ScanStrategy, Store, TextData};
use spdx::{Expression, ParseMode};
use tracing::{debug, warn};

use crate::utils::ResultExt;

pub static LICENSE_STORE: LazyLock<Option<Store>> = LazyLock::new(|| {
    Store::from_cache(include_bytes!("../data/license-store-cache.zstd") as &[_])
        .ok_inspect(|e| warn!("{e}"))
});

include!("../data/get_nix_license.rs");

pub fn parse_spdx_expression(license: &str, source: &'static str) -> Vec<&'static str> {
    Expression::parse_mode(license, ParseMode::LAX)
        .ok_inspect(|e| warn!("{e}"))
        .map_or_else(Vec::new, |expr| {
            expr.requirements()
                .filter_map(|req| {
                    let license = get_nix_license(req.req.license.id()?.name)?;
                    debug!("license from {source}: {license}");
                    Some(license)
                })
                .collect()
        })
}

pub fn load_license(
    licenses: &mut BTreeMap<&'static str, f32>,
    relative_path: impl Display,
    strategy: &ScanStrategy,
    path: &Path,
) {
    if let Some((license, score)) = find_license(strategy, path) {
        debug!("license found in {relative_path}: {license}");
        licenses
            .entry(license)
            .and_modify(|max_score| *max_score = max_score.max(score))
            .or_insert(score);
    }
}

fn find_license(strategy: &ScanStrategy, path: &Path) -> Option<(&'static str, f32)> {
    let text = read_to_string(path).ok_inspect(|e| warn!("{e}"))?;

    let res = strategy
        .scan(&TextData::from(text))
        .ok_inspect(|e| warn!("{e}"))?;

    let name = res.license?.name;

    if let Some(prefix) = name.strip_suffix("-only")
        && path
            .file_name()
            .and_then(OsStr::to_str)
            .is_some_and(|name| name.contains("-or-later"))
        && let Some(license) = get_nix_license(&format!("{prefix}-or-later"))
    {
        Some((license, res.score))
    } else {
        Some((get_nix_license(name)?, res.score))
    }
}

#[cfg(test)]
mod tests {
    use super::parse_spdx_expression;

    #[test]
    fn basic() {
        assert_eq!(parse_spdx_expression("MPL-2.0", ""), ["mpl20"]);
        assert_eq!(parse_spdx_expression("GPL-3.0", ""), ["gpl3Only"]);
        assert_eq!(parse_spdx_expression("unknown license", ""), [""; 0]);
        assert_eq!(
            parse_spdx_expression("MIT or Apache-2.0", ""),
            ["mit", "asl20"],
        );
    }
}
