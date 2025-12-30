{
  description = "Generate Nix packages from URLs with hash prefetching, dependency inference, license detection, and more";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-darwin"
        "x86_64-linux"
      ];
      imports = [ ./formatters.nix ];

      perSystem =
        {
          config,
          self',
          pkgs,
          lib,
          ...
        }:
        {
          devShells.default = pkgs.mkShell {
            env = {
              NIX_INIT_LOG = "nix_init=trace";
              RUST_BACKTRACE = true;
            };

            shellHook = ''
              mkdir -p data
              ln -sf ${config.packages.get-nix-license} data/get_nix_license.rs
              ln -sf ${config.packages.license-store-cache} data/license-store-cache.zstd
            '';
          };

          packages.nix-init = pkgs.callPackage ./default.nix {
            inherit (config.packages) get-nix-license license-store-cache;
          };
          packages.get-nix-license = pkgs.callPackage ./src/get_nix_license.nix { };
          packages.license-store-cache = pkgs.callPackage ./license-store-cache.nix { };
          packages.default = config.packages.nix-init;

          checks =
            let
              packages = lib.mapAttrs' (n: lib.nameValuePair "package-${n}") self'.packages;
              devShells = lib.mapAttrs' (n: lib.nameValuePair "devShell-${n}") self'.devShells;
              otherChecks = {
                clippy = config.packages.nix-init.override { enableClippy = true; };
              };
            in
            packages // devShells // otherChecks;
        };
    };
}
