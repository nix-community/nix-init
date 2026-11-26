{
  lib,
  buildNimPackage,
  fetchFromGitHub,
  nix-update-script,
}:

buildNimPackage (finalAttrs: {
  pname = "choosenim";
  version = "0.8.16";
  __structuredAttrs = true;
  strictDeps = true;

  src = fetchFromGitHub {
    owner = "nim-lang";
    repo = "choosenim";
    tag = "v${finalAttrs.version}";
    hash = "sha256-Sa1T7uZDIZHsakyI7QeJEWrHW7rSarFDniY9FaTBL7Q=";
  };

  # Generate lockfile with: nix run -f . nim_lk ./result | jq --sort-keys > ./lock.json
  lockFile = ./lock.json;

  nimbleFile = "choosenim.nimble";

  nimFlags = [ ];

  passthru.updateScript = nix-update-script { };

  meta = {
    description = "Official tool for easily installing and managing multiple versions of the Nim programming language";
    homepage = "https://github.com/nim-lang/choosenim";
    changelog = "https://github.com/nim-lang/choosenim/blob/${finalAttrs.src.rev}/changelog.markdown";
    license = lib.licenses.bsd3;
    maintainers = with lib.maintainers; [ alice ];
    mainProgram = "choosenim";
  };
})
