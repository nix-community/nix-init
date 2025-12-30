{
  lib,
  rustPlatform,
  spdx-license-list-data,
}:
rustPlatform.buildRustPackage {
  pname = "license-store-cache";
  inherit ((lib.importTOML ../Cargo.toml).workspace.package) version;

  src = ../.;

  cargoLock = {
    lockFile = ../Cargo.lock;
  };

  doCheck = false;

  cargoBuildFlags = [ "-p=license-store-cache" ];

  postInstall = ''
    cache=$(mktemp)
    $out/bin/license-store-cache $cache ${spdx-license-list-data.json}/json/details
    rm -rf $out
    mv $cache $out
  '';
}
