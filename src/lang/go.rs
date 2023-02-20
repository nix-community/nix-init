use anyhow::Result;
use heck::ToLowerCamelCase;
use regex::{Captures, Regex};
use serde::Deserialize;

use std::{borrow::Cow, fmt::Write, fs::File, path::Path};

use crate::utils::ResultExt;

#[derive(Deserialize)]
struct GoReleaser {
    #[serde(default)]
    builds: Vec<Build>,
}

#[derive(Deserialize)]
struct Build {
    #[serde(default)]
    ldflags: Vec<String>,
}

pub fn write_ldflags(out: &mut impl Write, src_dir: &Path) -> Result<()> {
    let names = [
        ".goreleaser.yml",
        ".goreleaser.yaml",
        "goreleaser.yml",
        "goreleaser.yaml",
    ];
    let Some(build) = names
        .into_iter()
        .find_map(|name| File::open(src_dir.join(name)).ok())
        .and_then(|file| serde_yaml::from_reader(file).ok_warn())
        .and_then(|GoReleaser { builds }| builds.into_iter().next()) else {
        return Ok(());
    };

    let Some(re) = regex() else { return Ok(()); };

    let mut ldflags = build.ldflags.into_iter().flat_map(|ldflags| {
        shlex::split(&parse_ldflags(&re, &ldflags))
            .unwrap_or_default()
            .into_iter()
    });

    if let Some(flag) = ldflags.next() {
        write!(out, r#"  ldflags = [ "{flag}" "#)?;
        for flag in ldflags {
            write!(out, r#""{flag}" "#)?;
        }
        writeln!(out, "];\n")?;
    }

    Ok(())
}

fn regex() -> Option<Regex> {
    Regex::new(r"\{\{\s*(.*?)\s*}}").ok_warn()
}

fn parse_ldflags<'a>(re: &Regex, ldflags: &'a str) -> Cow<'a, str> {
    re.replace_all(ldflags, |caps: &Captures| match &caps[1] {
        // https://goreleaser.com/customization/templates
        ".ProjectName" => "${pname}".into(),
        ".Version" | ".RawVersion" => "${version}".into(),
        ".Branch" | ".PrefixedTag" | ".Tag" | ".ShortCommit" | ".FullCommit" | ".Commit" => {
            "${src.rev}".into()
        }
        ".Major" => "${lib.versions.major version}".into(),
        ".Minor" => "${lib.versions.minor version}".into(),
        ".Patch" => "${lib.versions.patch version}".into(),
        ".Date" | ".CommitDate" => "1970-01-01T00:00:00Z".into(),
        ".Timestamp" | ".CommitTimestamp" => "0".into(),
        x => format!("${{{}}}", x.to_lower_camel_case()),
    })
}

#[cfg(test)]
mod tests {
    use super::{parse_ldflags, regex};

    #[test]
    fn basic() {
        let re = regex().unwrap();
        assert_eq!(parse_ldflags(&re, "-s -w"), "-s -w");
        assert_eq!(
            parse_ldflags(&re, "-X=main.Version={{ .Version }}"),
            "-X=main.Version=${version}",
        );
        assert_eq!(
            parse_ldflags(
                &re,
                "-s -w -X main.Version={{ .Version }} -X main.Tag={{ .Tag }}",
            ),
            "-s -w -X main.Version=${version} -X main.Tag=${src.rev}",
        );

        assert_eq!(
            parse_ldflags(&re, "-X main.Bad={{ func .Env.UNKNOWN_VAR }} -s -w"),
            "-X main.Bad=${funcEnvUnknownVar} -s -w",
        );
    }
}
