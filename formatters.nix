{ inputs, ... }:
{
  imports = [ inputs.treefmt-nix.flakeModule ];

  perSystem =
    { pkgs, ... }:
    {
      treefmt = {
        # Used to find the project root
        projectRootFile = ".git/config";

        programs = {
          rustfmt.enable = true;
          nixfmt.enable = true;
          nixfmt.package = pkgs.nixfmt-rfc-style;
          deno.enable = true;
          actionlint.enable = true;
        };
        settings.global.excludes = [
          "*.toml"
          "*.snap"
          "*/go.mod"
          "*/go.sum"
          "*.go"
          ".github/dependabot.yml"
          "assets/*"
          "LICENSE"
        ];
      };
    };
}
