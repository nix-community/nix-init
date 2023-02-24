{
  inputs = {
    crane = {
      url = "github:ipetkov/crane";
      inputs.flake-utils.follows = "flake-utils";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-compat.follows = "";
      inputs.rust-overlay.follows = "";
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs = { self, crane, fenix, flake-utils, nixpkgs }: {
    herculesCI.ciSystems = [
      "x86_64-linux"
      "aarch64-linux"
    ];
  } // flake-utils.lib.eachDefaultSystem (system:
    let
      inherit (crane.lib.${system}.overrideToolchain fenix.packages.${system}.default.toolchain)
        buildDepsOnly
        buildPackage
        cargoClippy
        cargoFmt
        cargoNextest
        ;
      inherit (nixpkgs.legacyPackages.${system})
        bzip2
        callPackage
        curl
        darwin
        installShellFiles
        libgit2
        makeWrapper
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
      inherit (nixpkgs.lib)
        licenses
        maintainers
        makeBinPath
        optionals
        sourceByRegex
        ;

      NIX_LICENSES = callPackage ./src/license.nix { };
      SPDX_LICENSE_LIST_DATA = "${spdx-license-list-data.json}/json/details";

      args' = {
        src = sourceByRegex self [
          "src(/.*)?"
          "Cargo\\.(toml|lock)"
          "build.rs"
          "rustfmt.toml"
        ];

        nativeBuildInputs = [
          curl
          installShellFiles
          makeWrapper
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

        cargoExtraArgs = "--no-default-features --features=reqwest/rustls-tls";

        inherit NIX_LICENSES SPDX_LICENSE_LIST_DATA;
        GEN_ARTIFACTS = "artifacts";
        ZSTD_SYS_USE_PKG_CONFIG = true;

        meta = {
          license = licenses.mpl20;
          maintainers = with maintainers; [ figsoda ];
        };
      };

      args = args' // {
        cargoArtifacts = buildDepsOnly args';
      };
    in
    {
      checks = {
        build = self.packages.${system}.default;
        clippy = cargoClippy (args // {
          cargoClippyExtraArgs = "-- -D warnings";
        });
        fmt = cargoFmt (removeAttrs args [ "cargoExtraArgs" ]);
        test = cargoNextest args;
      };

      devShells.default = mkShell {
        inherit NIX_LICENSES SPDX_LICENSE_LIST_DATA;
        NIX_INIT_LOG = "nix_init=trace";
        RUST_BACKTRACE = true;
      };

      formatter = nixpkgs-fmt;

      packages.default = buildPackage (args // {
        doCheck = false;
        postInstall = ''
          wrapProgram $out/bin/nix-init \
            --prefix PATH : ${makeBinPath [ nix nurl ]}
          installManPage artifacts/nix-init.1
          installShellCompletion artifacts/nix-init.{bash,fish} --zsh artifacts/_nix-init
        '';
      });
    });
}
