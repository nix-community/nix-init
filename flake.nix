{
  description = "Generate Nix packages from URLs with hash prefetching, dependency inference, license detection, and more";

  inputs = {
    crane = {
      url = "github:ipetkov/crane";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-compat.follows = "";
        rust-overlay.follows = "";
      };
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs = inputs@{ crane, flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-darwin"
        "x86_64-linux"
      ];

      perSystem = { inputs', lib, pkgs, self', system, ... }:
        let
          inherit (builtins)
            attrValues
            getAttr
            listToAttrs
            readDir
            ;
          inherit (lib)
            concatMapAttrs
            flip
            getExe
            hasSuffix
            importTOML
            licenses
            maintainers
            nameValuePair
            optionalAttrs
            optionals
            pipe
            sourceByRegex
            ;
          inherit (crane.lib.${system}.overrideToolchain inputs'.fenix.packages.default.toolchain)
            buildDepsOnly
            buildPackage
            cargoClippy
            cargoFmt
            cargoNextest
            ;
          inherit (pkgs)
            bzip2
            callPackage
            curl
            darwin
            installShellFiles
            libgit2
            libiconv
            mkShell
            nix
            nixpkgs-fmt
            nurl
            openssl
            pkg-config
            spdx-license-list-data
            stdenv
            zlib
            zstd
            ;

          src = sourceByRegex ./. [
            "(license-store-cache|src)(/.*)?"
            ''Cargo\.(toml|lock)''
            ''build\.rs''
            ''rustfmt\.toml''
          ];

          get-nix-license = callPackage ./src/get_nix_license.nix { };

          license-store-cache = buildPackage {
            pname = "license-store-cache";

            inherit src;

            buildInputs = optionals stdenv.isDarwin [
              libiconv
            ];

            doCheck = false;
            doNotRemoveReferencesToVendorDir = true;

            cargoArtifacts = null;
            cargoExtraArgs = "-p license-store-cache";

            CARGO_PROFILE = "";

            postInstall = ''
              cache=$(mktemp)
              $out/bin/license-store-cache $cache ${spdx-license-list-data.json}/json/details
              rm -rf $out
              mv $cache $out
            '';
          };

          args = {
            inherit src;

            nativeBuildInputs = [
              curl
              installShellFiles
              pkg-config
            ];

            buildInputs = [
              bzip2
              curl
              libgit2
              openssl
              zlib
              zstd
            ] ++ optionals stdenv.isDarwin [
              darwin.apple_sdk.frameworks.Security
            ] ++ optionals (stdenv.isDarwin && stdenv.isx86_64) [
              darwin.apple_sdk.frameworks.CoreFoundation
            ];

            cargoArtifacts = buildDepsOnly args;
            cargoExtraArgs = "--no-default-features";
            doInstallCargoArtifacts = false;

            postPatch = ''
              mkdir -p data
              ln -s ${get-nix-license} data/get_nix_license.rs
              ln -s ${license-store-cache} data/license-store-cache.zstd
            '';

            env = {
              GEN_ARTIFACTS = "artifacts";
              NIX = getExe nix;
              NURL = getExe nurl;
              ZSTD_SYS_USE_PKG_CONFIG = true;
            };

            meta = {
              license = licenses.mpl20;
              maintainers = with maintainers; [ figsoda ];
            };
          };
        in
        {
          checks = {
            build = self'.packages.default;
            clippy = cargoClippy (args // {
              cargoClippyExtraArgs = "-- -D warnings";
            });
            fmt = cargoFmt (removeAttrs args [ "cargoExtraArgs" ]);
            test =
              let
                fixtures = src + "/src/lang/rust/fixtures";
                lock = src + "/Cargo.lock";
                getPackages = flip pipe [
                  importTOML
                  (getAttr "package")
                  (map ({ name, version, ... }@pkg:
                    nameValuePair "${name}-${version}" pkg))
                  listToAttrs
                ];
              in
              cargoNextest (args // {
                cargoArtifacts = null;
                cargoLockParsed = importTOML lock // {
                  package = attrValues (getPackages lock // concatMapAttrs
                    (name: _: optionalAttrs
                      (hasSuffix "-lock.toml" name)
                      (getPackages (fixtures + "/${name}")))
                    (readDir fixtures));
                };
              });
          };

          devShells.default = mkShell {
            NIX_INIT_LOG = "nix_init=trace";
            RUST_BACKTRACE = true;

            shellHook = ''
              mkdir -p data
              ln -sf ${get-nix-license} data/get_nix_license.rs
              ln -sf ${license-store-cache} data/license-store-cache.zstd
            '';
          };

          formatter = nixpkgs-fmt;

          packages.default = buildPackage (args // {
            doCheck = false;
            postInstall = ''
              installManPage artifacts/nix-init.1
              installShellCompletion artifacts/nix-init.{bash,fish} --zsh artifacts/_nix-init
            '';
          });
        };
    };
}
