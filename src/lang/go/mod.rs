mod deps;
mod goreleaser;

use std::{
    fs::File,
    io::{BufRead, BufReader},
    str::SplitWhitespace,
};

use semver::Version;

pub use self::goreleaser::write_ldflags;
use crate::{inputs::AllInputs, lang::go::deps::load_go_dependency};

struct GoPackage<'a> {
    name: &'a str,
    version: GoVersion<'a>,
}

struct GoVersion<'a> {
    line: SplitWhitespace<'a>,
}

impl<'a> GoPackage<'a> {
    fn from_line(mut line: SplitWhitespace<'a>) -> Option<Self> {
        Some(GoPackage {
            name: line.next()?,
            version: GoVersion { line },
        })
    }
}

impl<'a> GoVersion<'a> {
    fn get(mut self) -> Option<Version> {
        self.line
            .next()?
            .strip_prefix('v')?
            .split('/')
            .next()?
            .parse()
            .ok()
    }
}

pub fn load_go_dependencies(inputs: &mut AllInputs, go_sum: &File) {
    for line in BufReader::new(go_sum).lines().map_while(Result::ok) {
        if let Some(pkg) = GoPackage::from_line(line.split_whitespace()) {
            load_go_dependency(inputs, pkg);
        };
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;

    use insta::assert_debug_snapshot;
    use semver::Version;

    use crate::{
        inputs::AllInputs,
        lang::go::{load_go_dependencies, GoPackage},
    };

    #[test]
    fn basic() {
        let mut inputs = AllInputs::default();
        load_go_dependencies(
            &mut inputs,
            &File::open("src/lang/go/fixtures/basic/go.sum").unwrap(),
        );
        assert_debug_snapshot!(inputs);
    }

    #[test]
    fn parse() {
        assert_debug_snapshot!(parse_line(
            "golang.org/x/mod v0.11.0 h1:bUO06HqtnRcc/7l71XBe4WcqTZ+3AH1J59zWDDwLKgU="
        ));
        assert_debug_snapshot!(parse_line(
            "github.com/spf13/cobra v1.7.0/go.mod h1:uLxZILRyS/50WlhOIKD7W6V5bgeIt+4sICxh6uRMrb0="
        ));
        assert_debug_snapshot!(parse_line(
            "golang.org/x/crypto v0.0.0-20221012134737-56aed061732a/go.mod h1:IxCIyHEi3zRg3s0A5j5BB6A9Jmi73HwBIUl50j+osU4="
        ));
    }

    fn parse_line(line: &str) -> (&str, Version) {
        let pkg = GoPackage::from_line(line.split_whitespace()).unwrap();
        (pkg.name, pkg.version.get().unwrap())
    }
}
