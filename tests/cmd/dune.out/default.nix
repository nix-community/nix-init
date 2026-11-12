{
  lib,
  buildDunePackage,
  fetchFromGitHub,
  ocamlPackages,
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

  nativeBuildInputs = [
    ocamlPackages.cinaps
  ];

  buildInputs = [
    ocamlPackages.astring
    ocamlPackages.base
    ocamlPackages.camlp-streams
    ocamlPackages.chrome-trace
    ocamlPackages.csexp
    ocamlPackages.dune-build-info
    ocamlPackages.dune-rpc
    ocamlPackages.dyn
    ocamlPackages.fiber
    ocamlPackages.jsonrpc
    ocamlPackages.lsp
    ocamlPackages.merlin-lib
    ocamlPackages.ocamlc-loc
    ocamlPackages.ocamlformat-rpc-lib
    ocamlPackages.ordering
    ocamlPackages.pp
    ocamlPackages.ppx_yojson_conv_lib
    ocamlPackages.re
    ocamlPackages.spawn
    ocamlPackages.stdune
    ocamlPackages.uutf
    ocamlPackages.xdg
    ocamlPackages.yojson
  ];

  checkInputs = [
    ocamlPackages.cinaps
    ocamlPackages.ocamlformat
    ocamlPackages.ppx_expect
  ];

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
