use askalono::Store;
use once_cell::sync::Lazy;
use spdx::{Expression, ParseMode};
use tracing::{debug, warn};

use crate::utils::ResultExt;

pub static LICENSE_STORE: Lazy<Option<Store>> = Lazy::new(|| {
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
