{
  rustPlatform,
  stdenv,
  libiconv,
  lib,
  spdx-license-list-data,
}:
rustPlatform.buildRustPackage {
  name = "license-store-cache";

  src = ./.;

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  buildInputs = lib.optionals stdenv.isDarwin [ libiconv ];
  doCheck = false;

  cargoBuildFlags = [ "-p license-store-cache" ];

  postInstall = ''
    cache=$(mktemp)
    $out/bin/license-store-cache $cache ${spdx-license-list-data.json}/json/details
    rm -rf $out
    mv $cache $out
  '';
}
