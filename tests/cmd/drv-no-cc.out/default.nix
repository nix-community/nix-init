{
  lib,
  stdenvNoCC,
  fetchFromGitHub,
  nix-update-script,
}:

stdenvNoCC.mkDerivation (finalAttrs: {
  pname = "zi";
  version = "1.0.3";

  src = fetchFromGitHub {
    owner = "z-shell";
    repo = "zi";
    tag = "v${finalAttrs.version}";
    hash = "sha256-nuw/riQaAdk0fYUpm3z978YGPDJnzc66DnOj774tPu0=";
  };

  passthru.updateScript = nix-update-script { };

  meta = {
    description = "[..]";
    homepage = "https://github.com/z-shell/zi";
    changelog = "https://github.com/z-shell/zi/releases/tag/${finalAttrs.src.tag}";
    license = lib.licenses.mit;
    maintainers = with lib.maintainers; [ alice ];
    mainProgram = "zi";
    platforms = lib.platforms.all;
  };
})
