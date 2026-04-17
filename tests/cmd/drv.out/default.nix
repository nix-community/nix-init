{
  lib,
  stdenv,
  fetchFromGitHub,
  meson,
  ninja,
  nix-update-script,
}:

stdenv.mkDerivation (finalAttrs: {
  pname = "bubblewrap";
  version = "0.11.0";
  __structuredAttrs = true;

  src = fetchFromGitHub {
    owner = "containers";
    repo = "bubblewrap";
    tag = "v${finalAttrs.version}";
    hash = "sha256-8IDMLQPeO576N1lizVudXUmTV6hNOiowjzRpEWBsZ+U=";
  };

  nativeBuildInputs = [
    meson
    ninja
  ];

  passthru.updateScript = nix-update-script { };

  meta = {
    description = "[..]";
    homepage = "https://github.com/containers/bubblewrap";
    changelog = "https://github.com/containers/bubblewrap/releases/tag/${finalAttrs.src.tag}";
    license = lib.licenses.lgpl2Only;
    maintainers = with lib.maintainers; [ alice ];
    mainProgram = "bubblewrap";
    platforms = lib.platforms.all;
  };
})
