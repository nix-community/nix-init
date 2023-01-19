use clap::Parser;

use std::path::PathBuf;

/// Generate Nix packages with hash prefetching, license detection, and more
/// https://github.com/nix-community/nix-init
#[derive(Parser)]
#[command(version, verbatim_doc_comment)]
pub struct Opts {
    /// The path to output the generated file to
    pub output: PathBuf,

    /// Specify the URL
    #[arg(short, long)]
    pub url: Option<String>,

    /// Specify the config file
    #[arg(short, long)]
    pub config: Option<PathBuf>,
}
