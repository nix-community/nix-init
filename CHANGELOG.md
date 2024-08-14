# Changelog

## v0.3.1 - 2024-08-14

### Fixes

- many crate updates
- update dependencies by @figsoda in
  https://github.com/nix-community/nix-init/pull/332,
  https://github.com/nix-community/nix-init/pull/409
- Update python derivation template by @mweinelt in
  https://github.com/nix-community/nix-init/pull/419
- Fix `time` compilation failure by @a-kenji in
  https://github.com/nix-community/nix-init/pull/457

## v0.3.0 - 2023-09-16

### Features

- `pkgs/by-name` support: sensible defaults for the output path, and a `commit`
  option to automatically commit the changes
- set `meta.mainProgram` by default
- Zig: support `zig.hook`
- Python: also add `wheel` when using `setuptools` as the build backend
- add a meaningful comment when no licenses were found
- Rust: improve dependency inference for the following crates: clipboard_macos,
  gspell-sys, libhandy-sys, libpanel-sys, libseat-sys, locate-dwarf,
  poppler-sys-rs, readkey, readmosue, soup-sys, soup2-sys, soup3-sys,
  sourceview4-sys, tracker-sys, trash, vte4-sys, webkit6-sys, wholesym,
  wireplumber, x11, xcb
- Go: support the following fields in GoReleaser templates: `.IsGitDirty`,
  `.PrefixedSummary`, `.Summary`
- Go: improve dependency inference for gotk4
- mkDerivation: set `meta.platforms` by default
- improve documentation for the nixpkgs option

### Changes

- Python: use `pyproject = true` instead of `format = "..."` (no longer suggests
  `format = "setuptools"`)
- prompt for the output path last
- Go: remove dependency inference for glfw due to false positives

### Fixes

- Go: fix parsing of GoReleaser configuration files when `ldflags` is a string
  instead of a list

## v0.2.4 - 2023-07-06

### Features

- Go: support dependency inference
- Rust: improve dependency inference for the following crates: alsa-sys,
  curl-sys, gtk-sys, gtk4-sys, librocksdb-sys, llvm-sys
- Go: improve ldflags formatting
- Add nix and nurl to runtime with environment variables instead of relying on a
  wrapper

### Changes

- Drop support for nixpkgs 22.11

### Fixes

- Rust: use `cargo` and `rustc` instead of `rustPlatform.rust.cargo` and
  `rustPlatform.rust.rustc`
- Python: fix `pythonImportsCheck`
- Python(pyproject): default build-system.requires to setuptools
- Fix interaction with `showAliases = false` on nixpkgs 23.11
  ([#153](https://github.com/nix-community/nix-init/issues/153))

## v0.2.3 - 2023-04-29

### Features

- Python: improve name normalization

### Fixes

- Python: fix parsing requirements.txt
  ([#111](https://github.com/nix-community/nix-init/pull/111))
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
- Rust: improve dependency inference, it now sets environment variables and is
  feature-aware
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

- PyPI: support optional dependencies
  ([#34](https://github.com/nix-community/nix-init/issues/34))
- PyPI: support zip sdists
  ([#33](https://github.com/nix-community/nix-init/issues/33))
- PyPI: don't strip digits from package names
  ([#35](https://github.com/nix-community/nix-init/issues/35))
- PyPI: handle normalized sdist file names
  ([#32](https://github.com/nix-community/nix-init/issues/32))
- PyPI: accept packages without licenses
  ([#32](https://github.com/nix-community/nix-init/issues/32))
- PyPI: filter out non-sdist versions
- Go: detect empty vendor from go.sum instead of FOD hash

## v0.1.1 - 2023-02-06

### Fixes

- Fix compatibility with nixpkgs 22.11
  ([#15](https://github.com/nix-community/nix-init/issues/15))
- GitHub: sort tags by chronological order
  ([#18](https://github.com/nix-community/nix-init/issues/18))
- Python: fix dependency parsing
  ([#22](https://github.com/nix-community/nix-init/issues/22))

### Features

- `--nixpkgs` to override nixpkgs
  ([#14](https://github.com/nix-community/nix-init/issues/14))
- Handle deprecated spdx license identifiers
- Python: detect build dependencies in build-system.requires
  ([#23](https://github.com/nix-community/nix-init/issues/23))

## v0.1.0 - 2023-01-28

First release
