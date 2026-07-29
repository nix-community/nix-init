{
  lib,
  buildNpmPackage,
  fetchFromGitHub,
  nix-update-script,
}:

buildNpmPackage (finalAttrs: {
  pname = "cowsay";
  version = "1.5.0";
  __structuredAttrs = true;

  src = fetchFromGitHub {
    owner = "piuccio";
    repo = "cowsay";
    tag = "v${finalAttrs.version}";
    hash = "sha256-TZ3EQGzVptNqK3cNrkLnyP1FzBd81XaszVucEnmBy4Y=";
  };

  npmDepsHash = "sha256-MIvLeuElaN9IbdB+SMgOLNTeycaK0k/M/R+xRxSD4U8=";

  dontNpmBuild = true;

  passthru.updateScript = nix-update-script { };

  meta = {
    description = "[..]";
    homepage = "https://github.com/piuccio/cowsay";
    changelog = "https://github.com/piuccio/cowsay/releases/tag/${finalAttrs.src.tag}";
    license = lib.licenses.mit;
    maintainers = with lib.maintainers; [ alice ];
    mainProgram = "cowsay";
  };
})
