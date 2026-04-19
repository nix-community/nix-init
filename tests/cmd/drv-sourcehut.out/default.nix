{
  lib,
  stdenv,
  fetchFromSourcehut,
}:

stdenv.mkDerivation (finalAttrs: {
  pname = "scdoc";
  version = "1.11.3";
  __structuredAttrs = true;
  strictDeps = true;

  src = fetchFromSourcehut {
    owner = "~sircmpwn";
    repo = "scdoc";
    tag = finalAttrs.version;
    hash = "sha256-MbLDhLn/JY6OcdOz9/mIPAQRp5TZ6IKuQ/FQ/R3wjGc=";
  };

  meta = {
    description = "[..]";
    homepage = "https://git.sr.ht/~sircmpwn/scdoc";
    license = lib.licenses.mit;
    maintainers = with lib.maintainers; [ alice ];
    mainProgram = "scdoc";
    platforms = lib.platforms.all;
  };
})
