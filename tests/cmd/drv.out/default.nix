{
  lib,
  stdenv,
  fetchFromGitHub,
  meson,
  ninja,
}:

stdenv.mkDerivation (finalAttrs: {
  pname = "bubblewrap";
  version = "0.11.0";

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

  meta = {
    description = "[..]";
    homepage = "https://github.com/containers/bubblewrap";
    license = lib.licenses.lgpl2Only;
    maintainers = with lib.maintainers; [ alice ];
    mainProgram = "bubblewrap";
    platforms = lib.platforms.all;
  };
})
