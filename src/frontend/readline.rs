use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use anyhow::Result;
use heck::ToKebabCase;
use owo_colors::{OwoColorize, Style};
use rustyline::{
    CompletionType, Context, Editor, Helper, Highlighter,
    completion::{Completer, FilenameCompleter, Pair},
    config::Configurer,
    hint::{Hint, Hinter},
    history::MemHistory,
    validate::{ValidationContext, ValidationResult, Validator},
};

use crate::{
    builder::Builder,
    fetcher::{Revisions, Version},
    frontend::Frontend,
    utils::by_name_path,
};

pub struct Readline {
    editor: Box<Editor<Prompter, MemHistory>>,
}

#[derive(Helper, Highlighter)]
enum Prompter {
    Path(FilenameCompleter),
    Revision(Revisions),
    NonEmpty,
    YesNo,
    Builder(Vec<Builder>),
}

impl Readline {
    pub fn new() -> Result<Self> {
        let mut editor = Editor::new()?;
        editor.set_completion_type(CompletionType::Fuzzy);
        editor.set_max_history_size(0)?;
        Ok(Self {
            editor: Box::new(editor),
        })
    }

    fn ask(&mut self, msg: impl Display) -> Result<bool, anyhow::Error> {
        self.editor.set_helper(Some(Prompter::YesNo));
        Ok(!self
            .editor
            .readline(&prompt(format_args!("{msg}? (Y/n)")))?
            .starts_with(['n', 'N']))
    }
}

impl Frontend for Readline {
    fn url(&mut self) -> Result<String> {
        self.editor.set_helper(Some(Prompter::NonEmpty));
        Ok(self.editor.readline(&prompt("Enter url"))?)
    }

    fn rev(&mut self, revs: Option<Revisions>) -> Result<(String, Option<Version>)> {
        if let Some(revs) = revs {
            let rev_msg = prompt(format_args!(
                "Enter tag or revision (defaults to {})",
                revs.latest
            ));
            self.editor.set_helper(Some(Prompter::Revision(revs)));

            let rev = self.editor.readline(&rev_msg)?;

            let Some(Prompter::Revision(revs)) = self.editor.helper_mut() else {
                unreachable!();
            };

            let rev = if rev.is_empty() {
                revs.latest.clone()
            } else {
                rev
            };
            let version = revs.versions.remove(&rev);

            Ok((rev, version))
        } else {
            self.editor.set_helper(Some(Prompter::NonEmpty));
            Ok((
                self.editor.readline(&prompt("Enter tag or revision"))?,
                None,
            ))
        }
    }

    fn fetch_submodules(&mut self) -> Result<bool> {
        self.ask("Fetch submodules")
    }

    fn version(&mut self, version: &str) -> Result<String> {
        self.editor.set_helper(Some(Prompter::NonEmpty));
        Ok(self
            .editor
            .readline_with_initial(&prompt("Enter version"), (version, ""))?)
    }

    fn pname(&mut self, pname: Option<String>) -> Result<String> {
        Ok(if let Some(pname) = pname {
            self.editor
                .readline_with_initial(&prompt("Enter pname"), (&pname.to_kebab_case(), ""))?
        } else {
            self.editor.readline(&prompt("Enter pname"))?
        })
    }

    fn builder(&mut self, builders: Vec<Builder>) -> Result<Builder> {
        self.editor.set_helper(Some(Prompter::Builder(builders)));
        let builder = self
            .editor
            .readline(&prompt("How should this package be built?"))?;
        let Some(Prompter::Builder(builders)) = self.editor.helper_mut() else {
            unreachable!();
        };
        Ok(*builder
            .parse()
            .ok()
            .and_then(|i: usize| builders.get(i))
            .unwrap_or_else(|| &builders[0]))
    }

    fn output(&mut self, pname: &str, builder: &Builder) -> Result<PathBuf> {
        self.editor
            .set_helper(Some(Prompter::Path(FilenameCompleter::new())));

        let msg = &prompt("Enter output path (leave as empty for the current directory)");
        let output = match by_name_path(pname, builder) {
            Some(path) => self.editor.readline_with_initial(msg, (&path, "")),
            None => self.editor.readline(msg),
        }?;
        self.editor.set_helper(None);

        Ok(if output.is_empty() {
            PathBuf::from(".")
        } else {
            PathBuf::from(output)
        })
    }

    fn overwrite(&mut self, path: &Path) -> Result<bool> {
        self.ask(format_args!("Overwrite {}", path.display().green()))
    }
}

impl Completer for Prompter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        match self {
            Prompter::Path(completer) => {
                let mut completions = completer.complete_path_unsorted(line, pos)?;
                completions.1.sort_by(
                    |Pair { replacement: x, .. }, Pair { replacement: y, .. }| {
                        y.ends_with('/')
                            .cmp(&x.ends_with('/'))
                            .then_with(|| x.starts_with('.').cmp(&y.starts_with('.')))
                            .then_with(|| y.ends_with(".nix").cmp(&x.ends_with(".nix")))
                            .then_with(|| x.cmp(y))
                    },
                );
                Ok(completions)
            }
            Prompter::Revision(revs) => Ok((0, revs.completions.clone())),
            Prompter::NonEmpty => Ok((0, Vec::new())),
            Prompter::YesNo => Ok((0, Vec::new())),
            Prompter::Builder(builders) => Ok((
                0,
                builders
                    .iter()
                    .enumerate()
                    .map(|(i, builder)| Pair {
                        display: format!("{i} - {builder}"),
                        replacement: i.to_string(),
                    })
                    .collect(),
            )),
        }
    }
}

struct SimpleHint(String);

impl Hint for SimpleHint {
    fn display(&self) -> &str {
        &self.0
    }

    fn completion(&self) -> Option<&str> {
        Some("")
    }
}

impl Hinter for Prompter {
    type Hint = SimpleHint;

    fn hint(&self, line: &str, _: usize, _: &Context<'_>) -> Option<Self::Hint> {
        match self {
            Prompter::Path(_) => None,

            Prompter::Revision(revs) => {
                let style = Style::new().blue().italic();
                if line.is_empty() {
                    return Some(SimpleHint(
                        format_args!("  {}", revs.latest).style(style).to_string(),
                    ));
                }

                revs.versions.get(line).map(|version| {
                    SimpleHint(match version {
                        Version::Latest => "  (latest release)".style(style).to_string(),
                        Version::Tag => "  (tag)".style(style).to_string(),
                        Version::Pypi { format, .. } => {
                            format_args!("  ({format})").style(style).to_string()
                        }
                        Version::Head { date, msg } => format_args!("  ({date} - HEAD) {msg}")
                            .style(style)
                            .to_string(),
                        Version::Commit { date, msg } => {
                            format_args!("  ({date}) {msg}").style(style).to_string()
                        }
                    })
                })
            }

            Prompter::NonEmpty => None,

            Prompter::YesNo => None,

            Prompter::Builder(builders) => Some(SimpleHint(if line.is_empty() {
                format_args!("  ({})", builders[0])
                    .blue()
                    .italic()
                    .to_string()
            } else if let Some(builder) = line.parse().ok().and_then(|i: usize| builders.get(i)) {
                format_args!("  ({builder})").blue().italic().to_string()
            } else {
                "  press <tab> to see options".yellow().italic().to_string()
            })),
        }
    }
}

impl Validator for Prompter {
    fn validate(&self, ctx: &mut ValidationContext) -> rustyline::Result<ValidationResult> {
        Ok(match self {
            Prompter::Path(_) => {
                ValidationResult::Valid(ctx.input().is_empty().then(|| ".".into()))
            }

            Prompter::Revision(revs) => {
                if ctx.input().is_empty() {
                    if revs.latest.is_empty() {
                        ValidationResult::Invalid(None)
                    } else {
                        ValidationResult::Valid(Some(revs.latest.clone()))
                    }
                } else {
                    ValidationResult::Valid(None)
                }
            }

            Prompter::NonEmpty => {
                if ctx.input().is_empty() {
                    ValidationResult::Invalid(None)
                } else {
                    ValidationResult::Valid(None)
                }
            }

            Prompter::YesNo => ValidationResult::Valid(None),

            Prompter::Builder(builders) => {
                let input = ctx.input();
                if input.is_empty() {
                    ValidationResult::Valid(Some(builders[0].to_string()))
                } else if let Some(builder) = input
                    .parse::<usize>()
                    .ok()
                    .and_then(|builder| builders.get(builder))
                {
                    ValidationResult::Valid(Some(format!(" - {builder}")))
                } else {
                    ValidationResult::Invalid(None)
                }
            }
        })
    }
}

fn prompt(prompt: impl Display) -> String {
    format!("{}\n{} ", prompt.bold(), "❯".blue())
}
