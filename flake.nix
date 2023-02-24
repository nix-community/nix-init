{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      inherit (builtins) path;
      inherit (nixpkgs.lib) genAttrs importTOML licenses maintainers makeBinPath optionals sourceByRegex;
      inherit (importTOML (self + "/Cargo.toml")) package;

      forEachSystem = genAttrs [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-darwin"
        "x86_64-linux"
      ];
    in
    {
      devShells = forEachSystem (system:
        let
          inherit (nixpkgs.legacyPackages.${system}) callPackage mkShell spdx-license-list-data;
        in
        {
          default = mkShell {
            NIX_INIT_LOG = "nix_init=trace";
            NIX_LICENSES = callPackage ./src/license.nix { };
            RUST_BACKTRACE = true;
            SPDX_LICENSE_LIST_DATA = "${spdx-license-list-data.json}/json/details";
          };
        });

      formatter = forEachSystem
        (system: nixpkgs.legacyPackages.${system}.nixpkgs-fmt);

      herculesCI.ciSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      packages = forEachSystem (system:
        let
          inherit (nixpkgs.legacyPackages.${system})
            bzip2
            callPackage
            curl
            darwin
            installShellFiles
            libgit2
            makeWrapper
            nix
            nurl
            openssl
            pkg-config
            rustPlatform
            spdx-license-list-data
            stdenv
            zlib
            zstd
            ;
        in
        {
          default = rustPlatform.buildRustPackage {
            pname = "nix-init";
            inherit (package) version;

            src = sourceByRegex self [
              "src(/.*)?"
              "Cargo\\.(toml|lock)"
              "build.rs"
            ];

            cargoLock = {
              allowBuiltinFetchGit = true;
              lockFile = path {
                path = self + "/Cargo.lock";
              };
            };

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

            doCheck = false;

            postInstall = ''
              wrapProgram $out/bin/nix-init \
                --prefix PATH : ${makeBinPath [ nix nurl ]}
              installManPage artifacts/nix-init.1
              installShellCompletion artifacts/nix-init.{bash,fish} --zsh artifacts/_nix-init
            '';

            buildNoDefaultFeatures = true;
            buildFeatures = [ "reqwest/rustls-tls" ];

            GEN_ARTIFACTS = "artifacts";
            NIX_LICENSES = callPackage ./src/license.nix { };
            SPDX_LICENSE_LIST_DATA = "${spdx-license-list-data.json}/json/details";
            ZSTD_SYS_USE_PKG_CONFIG = true;

            meta = {
              inherit (package) description;
              license = licenses.mpl20;
              maintainers = with maintainers; [ figsoda ];
            };
          };
        });
    };
}
