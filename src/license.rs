use askalono::Store;
use once_cell::sync::Lazy;
use rustc_hash::FxHashMap;

use crate::utils::ResultExt;

pub static LICENSE_STORE: Lazy<Option<Store>> = Lazy::new(|| {
    Store::from_cache(include_bytes!("../cache/askalono-cache.zstd") as &[_]).ok_warn()
});
pub static NIX_LICENSES: Lazy<FxHashMap<&'static str, &'static str>> = Lazy::new(get_nix_licenses);

include!(env!("NIX_LICENSES"));
