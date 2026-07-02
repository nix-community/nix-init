{
  lib,
  stdenv,
  fetchFromGitLab,
  meson,
  ninja,
  nix-update-script,
}:

stdenv.mkDerivation (finalAttrs: {
  pname = "beast";
  version = "1.1.2";
  __structuredAttrs = true;
  strictDeps = true;

  src = fetchFromGitLab {
    owner = "emilua";
    repo = "beast";
    tag = "v${finalAttrs.version}";
    hash = "sha256-MASaZvhIVKmeBUcn/NjlBZ+xh+2RgwHBH2o08lklGa0=";
  };

  nativeBuildInputs = [
    meson
    ninja
  ];

  passthru.updateScript = nix-update-script { };

  meta = {
    description = "[..]";
    homepage = "https://gitlab.com/emilua/beast";
    license = lib.licenses.boost;
    maintainers = with lib.maintainers; [ alice ];
    mainProgram = "beast";
    platforms = lib.platforms.all;
  };
})
