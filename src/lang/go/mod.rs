mod deps;
mod goreleaser;

use std::{
    fs::File,
    io::{BufRead, BufReader},
};

pub use goreleaser::write_ldflags;

use crate::{inputs::AllInputs, lang::go::deps::load_go_dependency};

pub fn load_go_dependencies(inputs: &mut AllInputs, go_sum: &File) {
    for line in BufReader::new(go_sum).lines().map_while(Result::ok) {
        if let Some(pkg) = line.split_whitespace().next() {
            load_go_dependency(inputs, pkg);
        };
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;

    use insta::assert_debug_snapshot;

    use crate::{inputs::AllInputs, lang::go::load_go_dependencies};

    #[test]
    fn basic() {
        let mut inputs = AllInputs::default();
        load_go_dependencies(
            &mut inputs,
            &File::open("src/lang/go/fixtures/basic/go.sum").unwrap(),
        );
        assert_debug_snapshot!(inputs);
    }
}
