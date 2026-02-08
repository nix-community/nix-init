use std::{
    fs::{File, create_dir_all},
    io::Write,
};

use tempfile::TempDir;
use trycmd::TestCases;

#[test]
fn e2e() {
    let tmp = TempDir::new().unwrap();
    let tmp = tmp.path();
    let dir = tmp.join("nix-init");

    create_dir_all(&dir).unwrap();
    File::create(dir.join("config.toml"))
        .unwrap()
        .write_all(include_bytes!("default-config.toml"))
        .unwrap();

    TestCases::new()
        .default_bin_name("nix-init")
        .env("XDG_CONFIG_DIRS", tmp.to_str().unwrap())
        .case("tests/cmd/**/*.toml");
}
