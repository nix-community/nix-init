use anyhow::Result;

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
};

#[derive(Debug, Default)]
pub struct AllInputs {
    pub native_build_inputs: Inputs,
    pub build_inputs: Inputs,
    pub env: BTreeMap<&'static str, &'static str>,
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
