use paste::paste;
use serde::Deserialize;

use std::{fmt::Display, fs::File, io::Read, path::Path};

use crate::{
    inputs::{get_riff_registry, AllInputs},
    utils::{fod_hash, ResultExt, FAKE_HASH},
};

#[derive(Deserialize)]
struct CargoLock {
    package: Vec<CargoPackage>,
}

#[derive(Deserialize)]
struct CargoPackage {
    name: String,
}

pub async fn cargo_deps_hash(
    inputs: &mut AllInputs,
    pname: impl Display,
    version: impl Display,
    src: impl Display,
    src_dir: &Path,
) -> String {
    if let Ok(lock) = File::open(src_dir.join("Cargo.lock")) {
        let (hash, ()) = tokio::join!(
            fod_hash(format!(
                r#"(import<nixpkgs>{{}}).rustPlatform.fetchCargoTarball{{name="{pname}-{version}";src={src};hash="{FAKE_HASH}";}}"#,
            )),
            load_riff_dependencies(inputs, lock),
        );
        hash.unwrap_or_else(|| FAKE_HASH.into())
    } else {
        FAKE_HASH.into()
    }
}

async fn load_riff_dependencies(inputs: &mut AllInputs, mut lock: impl Read) {
    let (Some(lock), Some(mut registry)) = tokio::join!(
        async {
            let mut buf = String::new();
            lock.read_to_string(&mut buf).ok_warn()?;
            toml::from_str::<CargoLock>(&buf).ok_warn()
        },
        get_riff_registry(),
    ) else {
        return;
    };

    for dep in lock.package {
        let Some(dep) = registry.language.rust.dependencies.remove(&dep.name) else {
            continue;
        };

        for input in dep.inputs.native_build_inputs {
            inputs.native_build_inputs.always.insert(input);
        }
        for input in dep.inputs.build_inputs {
            inputs.build_inputs.always.insert(input);
        }

        macro_rules! load {
            ($inputs:ident, $sys:ident) => {
                paste! {
                    for input in dep.targets.[<aarch64_ $sys>].$inputs {
                        if inputs.$inputs.always.contains(&input)
                            || inputs.$inputs.$sys.contains(&input) {
                            continue;
                        } else if inputs.$inputs.[<x86_64_ $sys>].remove(&input) {
                            inputs.$inputs.$sys.insert(input);
                        } else {
                            inputs.$inputs.[<aarch64_ $sys>].insert(input);
                        }
                    }

                    for input in dep.targets.[<x86_64_ $sys>].$inputs {
                        if inputs.$inputs.always.contains(&input)
                            || inputs.$inputs.$sys.contains(&input) {
                            continue;
                        } else if inputs.$inputs.[<aarch64_ $sys>].remove(&input) {
                            inputs.$inputs.$sys.insert(input);
                        } else {
                            inputs.$inputs.[<x86_64_ $sys>].insert(input);
                        }
                    }
                }
            };
        }

        load!(native_build_inputs, darwin);
        load!(native_build_inputs, linux);
        load!(build_inputs, darwin);
        load!(build_inputs, linux);
    }
}
