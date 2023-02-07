use askalono::Store;
use clap::{CommandFactory, ValueEnum};
use clap_complete::{generate_to, Shell};
use clap_mangen::Man;

use std::{
    env,
    fs::{create_dir_all, File},
    path::Path,
};

include!("src/cli.rs");

fn main() {
    println!("cargo:rerun-if-changed=cache/askalono-cache.zstd");
    println!("cargo:rerun-if-env-changed=GEN_ARTIFACTS");
    println!("cargo:rerun-if-env-changed=SPDX_LICENSE_LIST_DATA");

    // by default, the cache will not be rebuilt
    // remove the file to rebuild the cache
    let cache = Path::new("cache/askalono-cache.zstd");
    if !cache.is_file() {
        create_dir_all("cache").unwrap();
        let mut store = Store::new();
        store
            .load_spdx(
                env::var_os("SPDX_LICENSE_LIST_DATA").unwrap().as_ref(),
                false,
            )
            .unwrap();
        store.to_cache(File::create(cache).unwrap()).unwrap();
    }

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
