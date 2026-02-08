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
  nix,
  nurl,
  get-nix-license,
  license-store-cache,
}:

rustPlatform.buildRustPackage rec {
  pname = "nix-init";
  inherit ((lib.importTOML ./Cargo.toml).workspace.package) version;

  src = lib.sourceByRegex ./. [
    "(license-store-cache|src)(/.*)?"
    "build.rs"
    ''Cargo\.(toml|lock)''
  ];

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  nativeBuildInputs = [
    curl
    installShellFiles
    pkg-config
  ];

  buildInputs = [
    bzip2
    curl
    libgit2
    openssl
    zlib
    zstd
  ];

  buildNoDefaultFeatures = true;

  # lang::rust::tests needs additional cargo dependencies
  doCheck = false;

  # e2e tests require internet access
  checkFlags = [
    "--skip=e2e"
  ];

  postPatch = ''
    mkdir -p data
    ln -s ${get-nix-license} data/get_nix_license.rs
  '';

  preBuild = ''
    ln -s ${license-store-cache} data/license-store-cache.zstd
  '';

  postInstall = ''
    installManPage artifacts/nix-init.1
    installShellCompletion artifacts/nix-init.{bash,fish} --zsh artifacts/_nix-init
  '';

  env = {
    GEN_ARTIFACTS = "artifacts";
    LIBGIT2_NO_VENDOR = true;
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
