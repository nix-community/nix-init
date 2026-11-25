{
  lib,
  rustPlatform,
  fetchFromGitHub,
  pkg-config,
  openssl,
  xz,
  nix-update-script,
}:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "typst";
  version = "0.15.1";
  __structuredAttrs = true;

  src = fetchFromGitHub {
    owner = "typst";
    repo = "typst";
    tag = "v${finalAttrs.version}";
    hash = "sha256-5C3ussV1YBDB97S4nIlzLBTRiJSc7aDqDJ0b+9vybac=";
  };

  cargoLock = {
    lockFile = ./Cargo.lock;
    outputHashes = {
      "typst-dev-assets-0.15.1" = "sha256-1ICmdrTcC2v5MdONrrvWetCExHe9nYYwqghRCrM5lOs=";
    };
  };

  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs = [
    openssl
    xz
  ];

  passthru.updateScript = nix-update-script { };

  meta = {
    description = "[..]";
    homepage = "https://github.com/typst/typst";
    changelog = "https://github.com/typst/typst/releases/tag/${finalAttrs.src.tag}";
    license = lib.licenses.tost;
    maintainers = with lib.maintainers; [ alice ];
    mainProgram = "typst";
  };
})
