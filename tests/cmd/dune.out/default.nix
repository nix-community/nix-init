{
  lib,
  buildDunePackage,
  fetchFromGitHub,
  nix-update-script,
}:

buildDunePackage (finalAttrs: {
  pname = "ocaml-lsp";
  version = "1.27.0";
  __structuredAttrs = true;

  src = fetchFromGitHub {
    owner = "ocaml";
    repo = "ocaml-lsp";
    tag = finalAttrs.version;
    hash = "sha256-XrNYqNeaJAvkSYU0vha0apbN9uAYiGIuiJJMJMAN5ug=";
  };

  passthru.updateScript = nix-update-script { };

  meta = {
    description = "[..]";
    homepage = "https://github.com/ocaml/ocaml-lsp";
    changelog = "https://github.com/ocaml/ocaml-lsp/blob/${finalAttrs.src.rev}/CHANGES.md";
    license = lib.licenses.isc;
    maintainers = with lib.maintainers; [ alice ];
    mainProgram = "ocaml-lsp";
  };
})
