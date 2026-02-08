{
  lib,
  rustPlatform,
  fetchFromGitHub,
  curl,
  pkg-config,
  libgit2,
  openssl,
  sqlite,
  zlib,
  zstd,
}:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "nix-init";
  version = "0.3.3";

  src = fetchFromGitHub {
    owner = "nix-community";
    repo = "nix-init";
    tag = "v${finalAttrs.version}";
    hash = "sha256-S0dlcbjaClCa82sqHHW5nqLE2zcJdCsYFj6SxffHk1U=";
  };

  cargoHash = "sha256-oiPjkPRd1P6THKAuZva6wJR1posXglK+emIYb4ruzU8=";

  nativeBuildInputs = [
    curl
    pkg-config
  ];

  buildInputs = [
    curl
    libgit2
    openssl
    sqlite
    zlib
    zstd
  ];

  env = {
    LIBGIT2_NO_VENDOR = true;
    OPENSSL_NO_VENDOR = true;
    ZSTD_SYS_USE_PKG_CONFIG = true;
  };

  meta = {
    description = "[..]";
    homepage = "https://github.com/nix-community/nix-init";
    changelog = "https://github.com/nix-community/nix-init/blob/${finalAttrs.src.rev}/CHANGELOG.md";
    license = lib.licenses.mpl20;
    maintainers = with lib.maintainers; [ alice ];
    mainProgram = "nix-init";
  };
})
