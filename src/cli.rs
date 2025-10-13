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
}
