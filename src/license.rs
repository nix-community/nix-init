use askalono::Store;
use once_cell::sync::Lazy;
use rustc_hash::FxHashMap;
use spdx::{Expression, ParseMode};
use tracing::debug;

use crate::utils::ResultExt;

pub static LICENSE_STORE: Lazy<Option<Store>> = Lazy::new(|| {
    Store::from_cache(include_bytes!("../cache/askalono-cache.zstd") as &[_]).ok_warn()
});
pub static NIX_LICENSES: Lazy<FxHashMap<&'static str, &'static str>> = Lazy::new(get_nix_licenses);

include!(env!("NIX_LICENSES"));

pub fn parse_spdx_expression(license: &str, source: &'static str) -> Vec<&'static str> {
    Expression::parse_mode(license, ParseMode::LAX)
        .ok_warn()
        .map_or_else(Vec::new, |expr| {
            expr.requirements()
                .filter_map(|req| {
                    let &license = NIX_LICENSES.get(req.req.license.id()?.name)?;
                    debug!("license from {source}: {license}");
                    Some(license)
                })
                .collect()
        })
}
