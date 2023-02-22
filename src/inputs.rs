use anyhow::Result;
use rustc_hash::FxHashMap;
use serde::Deserialize;

use std::{collections::BTreeSet, fmt::Write};

use crate::utils::ResultExt;

#[derive(Default)]
pub struct AllInputs {
    pub native_build_inputs: Inputs,
    pub build_inputs: Inputs,
}

#[derive(Default, Debug)]
pub struct Inputs {
    pub always: BTreeSet<String>,
    pub darwin: BTreeSet<String>,
    pub aarch64_darwin: BTreeSet<String>,
    pub x86_64_darwin: BTreeSet<String>,
    pub linux: BTreeSet<String>,
    pub aarch64_linux: BTreeSet<String>,
    pub x86_64_linux: BTreeSet<String>,
}

#[derive(Deserialize)]
pub struct RiffRegistry {
    pub language: RiffLanguages,
}

#[derive(Deserialize)]
pub struct RiffLanguages {
    pub rust: RiffLanguage,
}

#[derive(Deserialize)]
pub struct RiffLanguage {
    pub dependencies: FxHashMap<String, RiffDependency>,
}

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct RiffDependency {
    #[serde(flatten)]
    pub inputs: RiffInputs,
    pub targets: RiffTargets,
}

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct RiffTargets {
    #[serde(rename = "aarch64-apple-darwin")]
    pub aarch64_darwin: RiffInputs,
    #[serde(rename = "aarch64-unknown-linux-gnu")]
    pub aarch64_linux: RiffInputs,
    #[serde(rename = "x86_64-apple-darwin")]
    pub x86_64_darwin: RiffInputs,
    #[serde(rename = "x86_64-unknown-linux-gnu")]
    pub x86_64_linux: RiffInputs,
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct RiffInputs {
    pub native_build_inputs: Vec<String>,
    pub build_inputs: Vec<String>,
}

pub async fn get_riff_registry() -> Option<RiffRegistry> {
    reqwest::get("https://registry.riff.determinate.systems/riff-registry.json")
        .await
        .ok_warn()?
        .json()
        .await
        .ok_warn()
}

pub fn write_all_lambda_inputs(
    out: &mut impl Write,
    inputs: &AllInputs,
    written: &mut BTreeSet<String>,
) -> Result<(bool, bool)> {
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
        write_lambda_input(out, written, "stdenv")?;
        write_lambda_input(out, written, input)?;
    }

    Ok(non_empty)
}

pub fn write_lambda_input(
    out: &mut impl Write,
    written: &mut BTreeSet<String>,
    input: &str,
) -> Result<()> {
    if written.insert(input.into()) {
        writeln!(out, ", {input}")?;
    }
    Ok(())
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
