{
  lib,
  rustPlatform,
  curl,
  installShellFiles,
  pkg-config,
  bzip2,
  libgit2,
  openssl,
  zlib,
  zstd,
  stdenv,
  darwin,
  spdx-license-list-data,
  nix,
  nurl,
  get-nix-license,
  license-store-cache,
  enableClippy ? false,
}:

rustPlatform.buildRustPackage rec {
  pname = "nix-init";
  version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.version;

  src = ./.;
  cargoLock = {
    lockFile = ./Cargo.lock;
    outputHashes."cargo-0.87.0" = "sha256-LCJmBIRfga9bG1qJLLhNxN+SUGQBrhND5J+k8eixYiA=";
  };

  nativeBuildInputs = [
    curl
    installShellFiles
    pkg-config
  ];

  buildInputs =
    [
      bzip2
      curl
      libgit2
      openssl
      zlib
      zstd
    ]
    ++ lib.optionals stdenv.isDarwin [ darwin.apple_sdk.frameworks.Security ]
    ++ lib.optionals (stdenv.isDarwin && stdenv.isx86_64) [
      darwin.apple_sdk.frameworks.CoreFoundation
    ];

  buildNoDefaultFeatures = true;

  checkFlags = [
    # requires internet access
    "--skip=lang::rust::tests"
  ];

  postPatch = ''
    mkdir -p data
    ln -s ${get-nix-license} data/get_nix_license.rs
  '';

  shellHook = ''
    mkdir -p data
    ln -sf ${get-nix-license} data/get_nix_license.rs
    ln -sf ${license-store-cache} data/license-store-cache.zstd
  '';

  preBuild = ''
    cargo run -p license-store-cache \
      -j $NIX_BUILD_CORES --frozen \
      data/license-store-cache.zstd ${spdx-license-list-data.json}/json/details
  '';

  postInstall = ''
    installManPage artifacts/nix-init.1
    installShellCompletion artifacts/nix-init.{bash,fish} --zsh artifacts/_nix-init
  '';

  env = {
    GEN_ARTIFACTS = "artifacts";
    LIBGIT2_NO_VENDOR = 1;
    NIX = lib.getExe nix;
    NURL = lib.getExe nurl;
    ZSTD_SYS_USE_PKG_CONFIG = true;
  };

  meta = {
    description = "Command line tool to generate Nix packages from URLs";
    mainProgram = "nix-init";
    homepage = "https://github.com/nix-community/nix-init";
    changelog = "https://github.com/nix-community/nix-init/blob/${src.rev}/CHANGELOG.md";
    license = lib.licenses.mpl20;
    maintainers = with lib.maintainers; [ figsoda ];
  };
}
// lib.optionalAttrs enableClippy {
  buildPhase = ''
    cargo clippy --all-targets --all-features -- -D warnings
  '';
  installPhase = ''
    touch $out
  '';
}
