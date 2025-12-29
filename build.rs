use std::{
    env,
    fs::{File, create_dir_all},
    path::Path,
};

use clap::{CommandFactory, ValueEnum};
use clap_complete::{Shell, generate_to};
use clap_mangen::Man;

include!("src/cli.rs");

fn main() {
    println!("cargo:rerun-if-env-changed=GEN_ARTIFACTS");

    if let Some(dir) = env::var_os("GEN_ARTIFACTS") {
        let out = &Path::new(&dir);
        create_dir_all(out).unwrap();
        let cmd = &mut Opts::command();

        Man::new(cmd.clone())
            .render(&mut File::create(out.join("nix-init.1")).unwrap())
            .unwrap();

        for shell in Shell::value_variants() {
            generate_to(*shell, cmd, "nix-init", out).unwrap();
        }
    }
}
