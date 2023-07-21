use std::{borrow::Cow, fmt::Write, fs::File, path::Path};

use anyhow::Result;
use heck::ToLowerCamelCase;
use regex::{Captures, Regex};
use serde::Deserialize;
use serde_with::{serde_as, OneOrMany};

use crate::utils::ResultExt;

#[derive(Deserialize)]
struct GoReleaser {
    #[serde(default)]
    builds: Vec<Build>,
}

#[serde_as]
#[derive(Deserialize)]
struct Build {
    #[serde_as(as = "Option<OneOrMany<_>>")]
    ldflags: Option<Vec<String>>,
}

pub fn write_ldflags(out: &mut impl Write, src_dir: &Path) -> Result<()> {
    let (Some(raw), Some(re)) = (get_ldflags(src_dir), regex()) else {
        writeln!(out, "  ldflags = [ \"-s\" \"-w\" ];\n")?;
        return Ok(());
    };

    let mut raw = raw
        .into_iter()
        .flat_map(|ldflags| shlex::split(&parse_ldflags(&re, &ldflags)).unwrap_or_default());

    let mut len = 0;
    let mut processed = Vec::new();
    while let Some(mut flag) = raw.next() {
        if flag == "-X" {
            if let Some(xflag) = raw.next() {
                flag.push('=');
                flag.push_str(&xflag);
            }
        }
        len += flag.len();
        processed.push(flag);
    }

    if processed.is_empty() {
        return Ok(());
    }

    if len > 16 {
        writeln!(out, "  ldflags = [")?;
        for flag in processed {
            writeln!(out, r#"    "{flag}""#)?;
        }
        writeln!(out, "  ];\n")?;
    } else {
        write!(out, "  ldflags = [")?;
        for flag in processed {
            write!(out, r#" "{flag}""#)?;
        }
        writeln!(out, " ];\n")?;
    }

    Ok(())
}

fn get_ldflags(src_dir: &Path) -> Option<Vec<String>> {
    let names = [
        ".goreleaser.yml",
        ".goreleaser.yaml",
        "goreleaser.yml",
        "goreleaser.yaml",
    ];

    let file = names
        .into_iter()
        .find_map(|name| File::open(src_dir.join(name)).ok())?;

    serde_yaml::from_reader::<_, GoReleaser>(file)
        .ok_warn()?
        .builds
        .into_iter()
        .next()?
        .ldflags
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
        ".Summary" | ".PrefixedSummary" => "${src.rev}".into(),
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
