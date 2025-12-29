use std::fs::{File, copy, create_dir};

use heck::AsKebabCase;
use insta::assert_debug_snapshot;
use tempfile::tempdir;

use super::resolve_workspace;
use crate::{inputs::AllInputs, lang::rust::load_rust_dependencies};

macro_rules! tests {
    ($($name:ident)+) => {
        $(
            #[test]
            fn $name() {
                assert_debug_snapshot!(run(stringify!($name)));
            }
        )+
    };
}

tests! {
    libz_ng
    libz_static
    libz_stock
    llvm
    zstd_env
    zstd_old
    zstd_pkg_config
}

fn run(name: &str) -> AllInputs {
    let dir = tempdir().unwrap();
    let dir = dir.path();

    let fixture = format!("src/lang/rust/fixtures/{}", AsKebabCase(name));
    copy(format!("{fixture}.toml"), dir.join("Cargo.toml")).unwrap();
    copy(format!("{fixture}-lock.toml"), dir.join("Cargo.lock")).unwrap();

    let src = dir.join("src");
    create_dir(&src).unwrap();
    File::create(src.join("lib.rs")).unwrap();

    let mut inputs = AllInputs::default();
    load_rust_dependencies(&mut inputs, &resolve_workspace(dir).unwrap());
    inputs
}
