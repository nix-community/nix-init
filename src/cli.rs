use std::path::PathBuf;

use clap::Parser;

/// Generate Nix packages with hash prefetching, license detection, and more
/// https://github.com/nix-community/nix-init
#[derive(Parser)]
#[command(version, verbatim_doc_comment)]
pub struct Opts {
    /// The path or directory to output the generated file to
    pub output: Option<PathBuf>,

    /// Specify the URL
    #[arg(short, long)]
    pub url: Option<String>,

    /// Specify the git revision, tag, or commit hash
    #[arg(short, long)]
    pub rev: Option<String>,

    /// Specify the package version
    #[arg(long = "version-name")]
    pub version: Option<String>,

    /// Specify the package name
    #[arg(short = 'p', long)]
    pub pname: Option<String>,

    /// Specify the build type
    ///
    /// Options: {n}  go-mod (buildGoModule) {n}  python-app (buildPythonApplication) {n}  python-pkg (buildPythonPackage) {n}  rust-pkg (buildRustPackage) {n}  drv (stdenv.mkDerivation)
    #[arg(short, long)]
    pub build: Option<String>,

    /// Path to nixpkgs (in nix)
    ///
    /// Examples:
    /// {n}  -n ./. (use the current directory)
    /// {n}  -n 'builtins.getFlake "nixpkgs"' (use the nixpkgs from the flake registry)
    /// {n}  -n '<nixpkgs>' (default, use the nixpkgs from channels)
    #[arg(short, long)]
    pub nixpkgs: Option<String>,

    /// Commit the changes if the output path is name-based (RFC 140)
    ///
    /// see https://github.com/NixOS/nixpkgs/tree/master/pkgs/by-name for more information
    #[arg(short = 'C', long, num_args=0..=1, require_equals = true, default_missing_value = "true")]
    pub commit: Option<bool>,

    /// Specify the config file
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Run in headless mode - accept defaults for all prompts
    ///
    /// URL must be provided via --url flag in headless mode.
    /// All other options will use sensible defaults if not specified.
    #[arg(long)]
    pub headless: bool,

    /// Fetch git submodules (defaults to false in headless mode, prompts in interactive mode)
    #[arg(long)]
    pub fetch_submodules: bool,

    /// Allow overwriting existing output files in headless mode
    #[arg(short, long)]
    pub force: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_headless_with_url() {
        let opts = Opts::try_parse_from(&[
            "nix-init",
            "--headless",
            "--url",
            "https://github.com/owner/repo",
        ])
        .unwrap();

        assert!(opts.headless);
        assert_eq!(opts.url.unwrap(), "https://github.com/owner/repo");
    }

    #[test]
    fn parse_all_headless_flags() {
        let opts = Opts::try_parse_from(&[
            "nix-init",
            "--headless",
            "--url",
            "https://example.com",
            "--rev",
            "v1.0.0",
            "--version-name",
            "1.0.0",
            "--pname",
            "test-pkg",
            "--build",
            "rust-pkg",
            "--fetch-submodules",
            "--force",
        ])
        .unwrap();

        assert!(opts.headless);
        assert_eq!(opts.rev.unwrap(), "v1.0.0");
        assert_eq!(opts.version.unwrap(), "1.0.0");
        assert_eq!(opts.pname.unwrap(), "test-pkg");
        assert_eq!(opts.build.unwrap(), "rust-pkg");
        assert!(opts.fetch_submodules);
        assert!(opts.force);
    }

    #[test]
    fn headless_defaults_to_false() {
        let opts = Opts::try_parse_from(&["nix-init"]).unwrap();
        assert!(!opts.headless);
        assert!(!opts.fetch_submodules);
        assert!(!opts.force);
    }
}
