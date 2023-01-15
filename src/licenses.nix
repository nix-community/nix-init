{ lib, writeText }:

let
  inherit (builtins) concatLists concatStringsSep length;
  inherit (lib) flip licenses mapAttrsToList optional;

  inserts = concatLists
    (flip mapAttrsToList licenses
      (k: v: optional (v ? spdxId) ''  xs.insert("${v.spdxId}", "${k}");''));
in

writeText "licenses.rs" ''
  use rustc_hash::FxHashMap;

  use std::collections::HashMap;

  pub fn get_nix_licenses() -> FxHashMap<&'static str, &'static str> {
      let mut xs = HashMap::with_capacity_and_hasher(${toString (length inserts)}, Default::default());
      ${concatStringsSep "\n    " inserts}
      xs
  }
''
