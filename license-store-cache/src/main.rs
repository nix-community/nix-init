use askalono::Store;

use std::{env::args_os, fs::File};

fn main() {
    let mut args = args_os();
    args.next().unwrap();
    let cache = File::create(args.next().unwrap()).unwrap();
    let mut store = Store::new();
    store
        .load_spdx(args.next().unwrap().as_ref(), false)
        .unwrap();
    store.to_cache(cache).unwrap();
}
