{ inputs, ... }:
{
  imports = [ inputs.treefmt-nix.flakeModule ];

  perSystem =
    { inputs', ... }:
    {
      treefmt = {
        # Used to find the project root
        projectRootFile = ".git/config";

        programs = {
          rustfmt.enable = true;
          nixfmt.enable = true;
          deno.enable = true;
          deadnix.enable = true;
          actionlint.enable = true;
        };
        settings = {
          formatter.rustfmt.command = "${inputs'.fenix.packages.latest.rustfmt}/bin/rustfmt";
          global.excludes = [
            "*.toml"
            "*.snap"
            "*/go.mod"
            "*/go.sum"
            "*.go"
            ".github/dependabot.yml"
            ".mergify.yml"
            "assets/*"
            "LICENSE"
          ];
        };
      };
    };
}
