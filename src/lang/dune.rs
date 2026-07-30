use std::{collections::BTreeSet, fs, path::Path};

use tracing::warn;

use crate::{license::parse_spdx_expression, utils::ResultExt};

#[derive(Debug, Default)]
pub struct DuneDependencies {
    pub build: BTreeSet<String>,
    pub check: BTreeSet<String>,
    /// tools required by `(using ...)` extensions, e.g. `cinaps`
    pub tools: BTreeSet<&'static str>,
    pub licenses: BTreeSet<&'static str>,
}

/// Extract dependencies from the `dune-project` file, falling back to the
/// `.opam` files in the source directory if it doesn't declare any packages.
pub fn get_dune_dependencies(src_dir: &Path) -> DuneDependencies {
    let mut deps = DuneDependencies::default();

    if let Some(content) = read_file(src_dir.join("dune-project")) {
        load_dune_project(&content, &mut deps);
        if !deps.build.is_empty() || !deps.check.is_empty() {
            return deps;
        }
        deps = DuneDependencies::default();
    }

    if let Some(entries) = fs::read_dir(src_dir).ok_inspect(|e| warn!("{e}")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "opam")
                && let Some(content) = read_file(&path)
            {
                load_opam(&content, &mut deps);
            }
        }
    }
    deps
}

fn read_file(path: impl AsRef<Path>) -> Option<String> {
    fs::read_to_string(path).ok_inspect(|e| warn!("{e}"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Filter {
    None,
    Test,
    Skip,
}

fn add_dep(name: &str, filter: Filter, deps: &mut DuneDependencies) {
    if filter == Filter::Skip {
        return;
    }

    let name = name.to_lowercase();
    let name = match name.as_str() {
        // provided by `buildDunePackage` itself
        "ocaml" | "ocaml-base-compiler" | "ocaml-variants" | "dune" => return,
        // system dependencies, e.g. `conf-pkg-config`
        _ if name.starts_with("conf-") => return,
        "ocamlfind" => "findlib",
        _ => name.as_str(),
    };

    let set = match filter {
        Filter::Test => &mut deps.check,
        Filter::Skip | Filter::None => &mut deps.build,
    };
    set.insert(name.into());
}

/// tools required by dune extensions
fn dune_tool(tool: &str) -> Option<&'static str> {
    match tool {
        "cinaps" => Some("cinaps"),
        "melange" => Some("melange"),
        "reason" => Some("reason"),
        _ => None,
    }
}

fn load_dune_license(items: &[Node], deps: &mut DuneDependencies) {
    for item in items {
        if let Node::Sym(license) | Node::Str(license) = item {
            for license in parse_spdx_expression(license, "dune-project") {
                deps.licenses.insert(license);
            }
        }
    }
}

// Parsing dune-project files

fn load_dune_project(src: &str, deps: &mut DuneDependencies) {
    for node in lex(src, b';') {
        let Node::Group('(', items) = node else {
            continue;
        };
        let Some(Node::Sym(name)) = items.first() else {
            continue;
        };
        match name.as_str() {
            "license" => load_dune_license(&items[1..], deps),
            "using" => {
                let Some(Node::Sym(tool)) = items.get(1) else {
                    continue;
                };
                if let Some(pkg) = dune_tool(tool) {
                    deps.tools.insert(pkg);
                }
            }
            "package" => {
                for item in &items[1..] {
                    let Node::Group('(', fields) = item else {
                        continue;
                    };
                    let Some(Node::Sym(field)) = fields.first() else {
                        continue;
                    };
                    if field == "depends" {
                        for dep in &fields[1..] {
                            add_dune_project_dep(dep, deps);
                        }
                    } else if field == "license" {
                        load_dune_license(&fields[1..], deps);
                    }
                }
            }
            _ => {}
        }
    }
}

fn add_dune_project_dep(node: &Node, deps: &mut DuneDependencies) {
    let (name, filter) = match node {
        Node::Sym(name) | Node::Str(name) => (name, Filter::None),
        Node::Group('(', items) => match items.first() {
            Some(Node::Sym(name)) | Some(Node::Str(name)) => {
                (name, filter_dune_project(&items[1..]))
            }
            _ => return,
        },
        _ => return,
    };
    add_dep(name, filter, deps);
}

fn filter_dune_project(items: &[Node]) -> Filter {
    let mut filter = Filter::None;
    for item in items {
        match item {
            Node::Sym(flag) => match flag.as_str() {
                ":with-test" => filter = filter.max(Filter::Test),
                ":with-doc" | ":with-dev-setup" | ":post" => filter = Filter::Skip,
                _ => {}
            },
            Node::Group('(', items) => filter = filter.max(filter_dune_project(items)),
            _ => {}
        }
    }
    filter
}

// Parsing opam files

fn load_opam(src: &str, deps: &mut DuneDependencies) {
    let nodes = lex(src, b'#');
    for (i, node) in nodes.iter().enumerate() {
        let Node::Sym(field) = node else {
            continue;
        };
        // `depends:` is lexed as a single atom since `:` isn't an atom terminator
        let Some(field) = field.strip_suffix(':') else {
            continue;
        };
        if !matches!(field, "depends" | "depopts" | "license") {
            continue;
        }
        if field == "license" {
            if let Some(Node::Str(license) | Node::Sym(license)) = nodes.get(i + 1) {
                for license in parse_spdx_expression(license, "opam file") {
                    deps.licenses.insert(license);
                }
            }
            continue;
        }
        let Some(Node::Group('[', items)) = nodes.get(i + 1) else {
            continue;
        };

        let mut items = items.iter().peekable();
        while let Some(item) = items.next() {
            let name = match item {
                Node::Sym(name) | Node::Str(name) => name,
                _ => continue,
            };
            let filter = match items.peek() {
                Some(Node::Group('{', filter)) => filter_opam(filter),
                _ => Filter::None,
            };
            add_dep(name, filter, deps);
        }
    }
}

fn filter_opam(items: &[Node]) -> Filter {
    let mut filter = Filter::None;
    for item in items {
        match item {
            Node::Sym(flag) => match flag.as_str() {
                "with-test" => filter = filter.max(Filter::Test),
                "with-doc" | "with-dev-setup" | "post" | "dev" => filter = Filter::Skip,
                _ => {}
            },
            Node::Group('(', items) | Node::Group('{', items) => {
                filter = filter.max(filter_opam(items))
            }
            _ => {}
        }
    }
    filter
}

// Lexing dune-project and opam files

#[derive(Debug)]
enum Node {
    Sym(String),
    Str(String),
    Group(char, Vec<Node>),
}

/// `comment` is `;` for dune-project files and `#` for opam files
fn lex(src: &str, comment: u8) -> Vec<Node> {
    let mut lex = Lex {
        src: src.as_bytes(),
        pos: 0,
        comment,
    };
    lex.nodes(0).unwrap_or_default()
}

struct Lex<'a> {
    src: &'a [u8],
    pos: usize,
    comment: u8,
}

impl Lex<'_> {
    fn nodes(&mut self, depth: u8) -> Option<Vec<Node>> {
        if depth >= 48 {
            return None;
        }

        let mut nodes = Vec::new();
        loop {
            self.skip_trivia();
            match self.src.get(self.pos) {
                None | Some(b')' | b'}' | b']') => return Some(nodes),
                Some(_) => nodes.push(self.node(depth)?),
            }
        }
    }

    fn node(&mut self, depth: u8) -> Option<Node> {
        match self.src[self.pos] {
            b'"' => Some(self.string()),
            c @ (b'(' | b'{' | b'[') => {
                self.pos += 1;
                let group = self.nodes(depth + 1)?;
                if self.src.get(self.pos) == Some(&closing(c)) {
                    self.pos += 1;
                }
                Some(Node::Group(c as char, group))
            }
            _ => Some(self.sym()),
        }
    }

    fn skip_trivia(&mut self) {
        while let Some(&c) = self.src.get(self.pos) {
            if c == self.comment {
                while self.src.get(self.pos).is_some_and(|&c| c != b'\n') {
                    self.pos += 1;
                }
            } else if c.is_ascii_whitespace() {
                self.pos += 1;
            } else {
                return;
            }
        }
    }

    fn string(&mut self) -> Node {
        self.pos += 1;
        let mut bytes = Vec::new();
        while let Some(&c) = self.src.get(self.pos) {
            match c {
                b'"' => {
                    self.pos += 1;
                    break;
                }
                b'\\' => {
                    self.pos += 1;
                    match self.src.get(self.pos) {
                        Some(&c) => {
                            bytes.push(match c {
                                b'n' => b'\n',
                                b't' => b'\t',
                                b'r' => b'\r',
                                _ => c,
                            });
                            self.pos += 1;
                        }
                        None => break,
                    }
                }
                _ => {
                    bytes.push(c);
                    self.pos += 1;
                }
            }
        }
        Node::Str(String::from_utf8_lossy(&bytes).into_owned())
    }

    fn sym(&mut self) -> Node {
        let start = self.pos;
        while let Some(&c) = self.src.get(self.pos) {
            if c.is_ascii_whitespace()
                || c == self.comment
                || matches!(c, b'(' | b')' | b'{' | b'}' | b'[' | b']' | b'"')
            {
                break;
            }
            self.pos += 1;
        }
        Node::Sym(String::from_utf8_lossy(&self.src[start..self.pos]).into_owned())
    }
}

const fn closing(open: u8) -> u8 {
    match open {
        b'(' => b')',
        b'{' => b'}',
        _ => b']',
    }
}

#[cfg(test)]
mod tests {
    use std::fs::write;

    use insta::assert_debug_snapshot;
    use tempfile::TempDir;

    use super::get_dune_dependencies;

    const OCAML_LSP_DUNE_PROJECT: &str = include_str!("fixtures/ocaml-lsp.dune-project");
    const OCAML_LSP_OPAM: &str = include_str!("fixtures/ocaml-lsp-server.opam");

    #[test]
    fn ocaml_lsp() {
        let dir = TempDir::new().unwrap();
        write(dir.path().join("dune-project"), OCAML_LSP_DUNE_PROJECT).unwrap();
        assert_debug_snapshot!(get_dune_dependencies(dir.path()));
    }

    #[test]
    fn ocaml_lsp_opam() {
        let dir = TempDir::new().unwrap();
        write(dir.path().join("ocaml-lsp-server.opam"), OCAML_LSP_OPAM).unwrap();
        assert_debug_snapshot!(get_dune_dependencies(dir.path()));
    }
}
