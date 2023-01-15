{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      inherit (nixpkgs.lib) genAttrs importTOML licenses optionals maintainers;
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
            NIX_LICENSES = callPackage ./src/licenses.nix { };
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
            callPackage darwin installShellFiles makeWrapper nurl pkg-config rustPlatform spdx-license-list-data stdenv zstd;
        in
        {
          default = rustPlatform.buildRustPackage {
            pname = "nix-init";
            inherit (package) version;

            src = self;

            cargoLock = {
              allowBuiltinFetchGit = true;
              lockFile = self + "/Cargo.lock";
            };

            nativeBuildInputs = [
              installShellFiles
              makeWrapper
              pkg-config
            ];

            buildInputs = [ zstd ] ++ optionals stdenv.isDarwin [
              darwin.apple_sdk.frameworks.Security
            ];

            postInstall = ''
              wrapProgram $out/bin/nix-init \
                --prefix PATH : ${nurl}/bin/nurl
              installManPage artifacts/nix-init.1
              installShellCompletion artifacts/nix-init.{bash,fish} --zsh artifacts/_nix-init
            '';

            GEN_ARTIFACTS = "artifacts";
            NIX_LICENSES = callPackage ./src/licenses.nix { };
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
