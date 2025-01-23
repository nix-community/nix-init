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
    outputHashes."cargo-0.87.0" = "sha256-LCJmBIRfga9bG1qJLLhNxN+SUGQBrhND5J+k8eixYiA=";
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
