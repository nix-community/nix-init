use anyhow::Result;
use paste::paste;
use rustc_hash::FxHashMap;
use serde::Deserialize;

use std::{
    collections::BTreeSet,
    io::{Read, Write},
};

#[derive(Default)]
pub struct AllInputs {
    pub native_build_inputs: Inputs,
    pub build_inputs: Inputs,
}

#[derive(Default, Debug)]
pub struct Inputs {
    pub always: BTreeSet<String>,
    darwin: BTreeSet<String>,
    aarch64_darwin: BTreeSet<String>,
    x86_64_darwin: BTreeSet<String>,
    linux: BTreeSet<String>,
    aarch64_linux: BTreeSet<String>,
    x86_64_linux: BTreeSet<String>,
}

#[derive(Deserialize)]
struct CargoLock {
    package: Vec<CargoPackage>,
}

#[derive(Deserialize)]
struct CargoPackage {
    name: String,
}

#[derive(Deserialize)]
struct RiffRegistry {
    language: RiffLanguages,
}

#[derive(Deserialize)]
struct RiffLanguages {
    rust: RiffLanguage,
}

#[derive(Deserialize)]
struct RiffLanguage {
    dependencies: FxHashMap<String, RiffDependency>,
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct RiffDependency {
    #[serde(flatten)]
    inputs: RiffInputs,
    targets: RiffTargets,
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct RiffTargets {
    #[serde(rename = "aarch64-apple-darwin")]
    aarch64_darwin: RiffInputs,
    #[serde(rename = "aarch64-unknown-linux-gnu")]
    aarch64_linux: RiffInputs,
    #[serde(rename = "x86_64-apple-darwin")]
    x86_64_darwin: RiffInputs,
    #[serde(rename = "x86_64-unknown-linux-gnu")]
    x86_64_linux: RiffInputs,
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
struct RiffInputs {
    native_build_inputs: Vec<String>,
    build_inputs: Vec<String>,
}

pub async fn load_riff_dependencies(inputs: &mut AllInputs, mut lock: impl Read) {
    let (Some(lock), Some(mut registry)) = tokio::join!(
        async {
            let mut buf = String::new();
            lock.read_to_string(&mut buf).ok()?;
            toml::from_str::<CargoLock>(&buf).ok()
        },
        async {
            reqwest::get("https://registry.riff.determinate.systems/riff-registry.json")
                .await
                .ok()?
                .json::<RiffRegistry>()
                .await
                .ok()
        },
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

pub fn write_all_lambda_inputs<const N: usize>(
    out: &mut impl Write,
    inputs: &AllInputs,
    written: [&'static str; N],
) -> Result<(bool, bool)> {
    let written = &mut written.into_iter().map(Into::into).collect();
    Ok((
        write_lambda_inputs(out, written, &inputs.native_build_inputs)?,
        write_lambda_inputs(out, written, &inputs.build_inputs)?,
    ))
}

fn write_lambda_inputs(
    out: &mut impl Write,
    written: &mut BTreeSet<String>,
    inputs: &Inputs,
) -> Result<bool> {
    let mut non_empty = false;

    for input in inputs
        .always
        .iter()
        .filter_map(|input| input.split('.').next())
    {
        non_empty = true;
        if written.insert(input.into()) {
            writeln!(out, ", {input}")?;
        }
    }

    for input in [
        &inputs.darwin,
        &inputs.aarch64_darwin,
        &inputs.x86_64_darwin,
        &inputs.linux,
        &inputs.aarch64_linux,
        &inputs.x86_64_linux,
    ]
    .into_iter()
    .flat_map(IntoIterator::into_iter)
    .filter_map(|input| input.split('.').next())
    {
        non_empty = true;
        if written.insert("stdenv".into()) {
            writeln!(out, ", stdenv")?;
        }
        if written.insert(input.into()) {
            writeln!(out, ", {input}")?;
        }
    }

    Ok(non_empty)
}

pub fn write_inputs(out: &mut impl Write, inputs: &Inputs, name: &'static str) -> Result<()> {
    write!(out, "  {name} =")?;

    let mut inputs = [
        ("", &inputs.always),
        ("lib.optionals stdenv.isDarwin ", &inputs.darwin),
        (
            "lib.optionals (stdenv.isDarwin && stdenv.isAarch64) ",
            &inputs.aarch64_darwin,
        ),
        (
            "lib.optionals (stdenv.isDarwin && stdenv.isx86_64) ",
            &inputs.x86_64_darwin,
        ),
        ("lib.optionals stdenv.isLinux ", &inputs.linux),
        (
            "lib.optionals (stdenv.isLinux && stdenv.isAarch64) ",
            &inputs.aarch64_linux,
        ),
        (
            "lib.optionals (stdenv.isLinux && stdenv.isx86_64) ",
            &inputs.x86_64_linux,
        ),
    ]
    .into_iter()
    .filter(|(_, inputs)| !inputs.is_empty());

    if let Some((prefix, inputs)) = inputs.next() {
        write!(out, " {prefix}")?;
        write_input_list(out, inputs)?;
    }

    for (prefix, inputs) in inputs {
        write!(out, " ++ {prefix}")?;
        write_input_list(out, inputs)?;
    }

    writeln!(out, ";\n")?;

    Ok(())
}

fn write_input_list(out: &mut impl Write, inputs: &BTreeSet<String>) -> Result<()> {
    writeln!(out, "[")?;
    for input in inputs {
        writeln!(out, "    {input}")?;
    }
    write!(out, "  ]")?;
    Ok(())
}
