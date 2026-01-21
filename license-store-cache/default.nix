{
  lib,
  rustPlatform,
  spdx-license-list-data,
}:

rustPlatform.buildRustPackage {
  pname = "license-store-cache";
  inherit ((lib.importTOML ../Cargo.toml).workspace.package) version;

  src = lib.sourceByRegex ../. [
    "(license-store-cache)(/.*)?"
    ''Cargo\.(toml|lock)''
  ];

  cargoLock = {
    lockFile = ../Cargo.lock;
  };

  doCheck = false;

  cargoBuildFlags = [ "-p=license-store-cache" ];

  postPatch = ''
    mkdir src
    touch src/main.rs
  '';

  postInstall = ''
    cache=$(mktemp)
    $out/bin/license-store-cache $cache ${spdx-license-list-data.json}/json/details
    rm -rf $out
    mv $cache $out
  '';
}
