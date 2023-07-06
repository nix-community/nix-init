# Changelog

## v0.2.4 - 2023-07-06

### Features

- Go: support dependency inference
- Rust: improve dependency inference for the following crates: alsa-sys, curl-sys, gtk-sys, gtk4-sys, librocksdb-sys, llvm-sys
- Go: improve ldflags formatting
- Add nix and nurl to runtime with environment variables instead of relying on a wrapper

### Changes

- Drop support for nixpkgs 22.11

### Fixes

- Rust: use `cargo` and `rustc` instead of `rustPlatform.rust.cargo` and `rustPlatform.rust.rustc`
- Python: fix `pythonImportsCheck`
- Python(pyproject): default build-system.requires to setuptools
- Fix interaction with `showAliases = false` on nixpkgs 23.11 ([#153](https://github.com/nix-community/nix-init/issues/153))

## v0.2.3 - 2023-04-29

### Features

- Python: improve name normalization

### Fixes

- Python: fix parsing requirements.txt ([#111](https://github.com/nix-community/nix-init/pull/111))
- Python: acknowledge requirements.txt when pyproject.toml is absent

## v0.2.2 - 2023-04-23

### Features

- Logo and Matrix chat
- Rust: use the first git dependency when multiple have the same revisions
- Ask to fetch submodules
- Improve error messages

## v0.2.1 - 2023-03-22

### Features

- Rust: default to importCargoLock if git sources were found
- Rust: deduplicate outputHashes based on git revisions

## v0.2.0 - 2023-03-19

### Features

- Rust: support `importCargoLock`
- Rust: improve dependency inference, it now sets environment variables and is feature-aware
- Prompt for output path when it is unspecified
- Go: infer `ldflags` from GoReleaser configuration
- Python: recognize `maturinBuildHook`
- Improve builder completions and validator
- Improve description normalization
- Improve version inference

### Changes

- Python: update `buildPythonPackage`'s style to be more conventional
- Disallow empty urls
- Normalize pname

### Fixes

- PyPI: support optional dependencies ([#34](https://github.com/nix-community/nix-init/issues/34))
- PyPI: support zip sdists ([#33](https://github.com/nix-community/nix-init/issues/33))
- PyPI: don't strip digits from package names ([#35](https://github.com/nix-community/nix-init/issues/35))
- PyPI: handle normalized sdist file names ([#32](https://github.com/nix-community/nix-init/issues/32))
- PyPI: accept packages without licenses ([#32](https://github.com/nix-community/nix-init/issues/32))
- PyPI: filter out non-sdist versions
- Go: detect empty vendor from go.sum instead of FOD hash

## v0.1.1 - 2023-02-06

### Fixes

- Fix compatibility with nixpkgs 22.11 ([#15](https://github.com/nix-community/nix-init/issues/15))
- GitHub: sort tags by chronological order ([#18](https://github.com/nix-community/nix-init/issues/18))
- Python: fix dependency parsing ([#22](https://github.com/nix-community/nix-init/issues/22))

### Features

- `--nixpkgs` to override nixpkgs ([#14](https://github.com/nix-community/nix-init/issues/14))
- Handle deprecated spdx license identifiers
- Python: detect build dependencies in build-system.requires ([#23](https://github.com/nix-community/nix-init/issues/23))

## v0.1.0 - 2023-01-28

First release
