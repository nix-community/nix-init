<img src="assets/logo.svg" align="right" height="280">

# nix-init

[![release](https://img.shields.io/github/v/release/nix-community/nix-init?logo=github&style=flat-square)](https://github.com/nix-community/nix-init/releases)
[![version](https://img.shields.io/crates/v/nix-init?logo=rust&style=flat-square)](https://crates.io/crates/nix-init)
[![deps](https://deps.rs/repo/github/nix-community/nix-init/status.svg?style=flat-square&compact=true)](https://deps.rs/repo/github/nix-community/nix-init)
[![license](https://img.shields.io/badge/license-MPL--2.0-blue?style=flat-square)](https://www.mozilla.org/en-US/MPL/2.0)
[![ci](https://img.shields.io/github/actions/workflow/status/nix-community/nix-init/ci.yml?label=ci&logo=github-actions&style=flat-square)](https://github.com/nix-community/nix-init/actions?query=workflow:ci)

Generate Nix packages from URLs

> Note: It is likely that the generated package will not work without some tweaks, also remember to double check the license and description even if it does work

- Hash prefetching powered by [nurl] with support for `cargoHash` and `vendorHash`
- Dependency inference for Rust and Python projects
- Interactive prompts with fuzzy tab completions
- License detection

![](https://user-images.githubusercontent.com/40620903/226211877-2d583d09-4fbc-4869-8248-6166edde21cc.gif)

## Installation

The latest release of nix-init is packaged in nixpkgs and kept up to date on the unstable branches

![](https://repology.org/badge/vertical-allrepos/nix-init.svg)

If you want to use a more recent snapshot of nix-init, it is also available as a flake.
The following command is equivalent to running `nix-init --help`:

```bash
nix run github:nix-community/nix-init -- --help
```

or if you don't have flakes enabled:

```bash
nix run --extra-experimental-features "flakes nix-command" github:nix-community/nix-init -- --help
```

## Usage

```
Usage: nix-init [OPTIONS] [OUTPUT]

Arguments:
  [OUTPUT]  The path or directory to output the generated file to

Options:
  -u, --url <URL>          Specify the URL
  -n, --nixpkgs <NIXPKGS>  Path to nixpkgs (in nix)
  -c, --config <CONFIG>    Specify the config file
  -h, --help               Print help
  -V, --version            Print version
```

### Supported builders

- `stdenv.mkDerivation`
- `buildRustPackage`
- `buildPythonApplication` and `buildPythonPackage`
- `buildGoModule`

### Supported fetchers

- `fetchCrate`
- `fetchFromGitHub`
- `fetchFromGitLab`
- `fetchFromGitea`
- `fetchPypi`
- All other fetchers supported by [nurl] are also supported, you just have to manually input the tag/revision of the package

## Configuration

nix-init will try to find `nix-init/config.toml` under XDG configuration directories

```toml
# ~/.config/nix-init/config.toml

# maintainers that will get added to the package meta
maintainers = ["figsoda"]

# path to nixpkgs, equivalent to `--nixpkgs`
nixpkgs = "<nixpkgs>"

# access tokens to access private repositories and avoid rate limits
[access-tokens]
"github.com" = "ghp_blahblahblah..."
"gitlab.com".command = ["secret-tool", "or", "whatever", "you", "use"]
"gitlab.gnome.org".file = "/path/to/api/token"
```

## Changelog

See [CHANGELOG.md](CHANGELOG.md)

[nurl]: https://github.com/nix-community/nurl
