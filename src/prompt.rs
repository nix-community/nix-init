use std::{fmt::Display, path::Path};

use owo_colors::{OwoColorize, Style};
use rustyline::{
    Context, Editor, Helper, Highlighter,
    completion::{Completer, FilenameCompleter, Pair},
    hint::{Hint, Hinter},
    history::History,
    validate::{ValidationContext, ValidationResult, Validator},
};

use crate::{
    build::BuildType,
    fetcher::{Revisions, Version},
};

#[derive(Helper, Highlighter)]
pub enum Prompter {
    Path(FilenameCompleter),
    Revision(Revisions),
    NonEmpty,
    YesNo,
    Build(Vec<BuildType>),
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
            Prompter::Revision(revisions) => Ok((0, revisions.completions.clone())),
            Prompter::NonEmpty => Ok((0, Vec::new())),
            Prompter::YesNo => Ok((0, Vec::new())),
            Prompter::Build(choices) => Ok((
                0,
                choices
                    .iter()
                    .enumerate()
                    .map(|(i, choice)| Pair {
                        display: format!("{i} - {choice}"),
                        replacement: i.to_string(),
                    })
                    .collect(),
            )),
        }
    }
}

pub struct SimpleHint(String);

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

            Prompter::Revision(revisions) => {
                let style = Style::new().blue().italic();
                if line.is_empty() {
                    return Some(SimpleHint(
                        format_args!("  {}", revisions.latest)
                            .style(style)
                            .to_string(),
                    ));
                }

                revisions.versions.get(line).map(|version| {
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

            Prompter::Build(choices) => Some(SimpleHint(if line.is_empty() {
                format_args!("  ({})", choices[0])
                    .blue()
                    .italic()
                    .to_string()
            } else if let Some(choice) = line.parse().ok().and_then(|i: usize| choices.get(i)) {
                format_args!("  ({choice})").blue().italic().to_string()
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

            Prompter::Revision(revisions) => {
                if ctx.input().is_empty() {
                    if revisions.latest.is_empty() {
                        ValidationResult::Invalid(None)
                    } else {
                        ValidationResult::Valid(Some(revisions.latest.clone()))
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

            Prompter::Build(choices) => {
                let input = ctx.input();
                if input.is_empty() {
                    ValidationResult::Valid(Some(choices[0].to_string()))
                } else if let Some(choice) = input
                    .parse::<usize>()
                    .ok()
                    .and_then(|choice| choices.get(choice))
                {
                    ValidationResult::Valid(Some(format!(" - {choice}")))
                } else {
                    ValidationResult::Invalid(None)
                }
            }
        })
    }
}

pub fn prompt(prompt: impl Display) -> String {
    format!("{}\n{} ", prompt.bold(), "❯".blue())
}

pub fn ask_overwrite(
    editor: &mut Editor<Prompter, impl History>,
    path: &Path,
) -> Result<bool, anyhow::Error> {
    ask(editor, format_args!("Overwrite {}", path.display().green()))
}

pub fn ask(
    editor: &mut Editor<Prompter, impl History>,
    msg: impl Display,
) -> Result<bool, anyhow::Error> {
    editor.set_helper(Some(Prompter::YesNo));
    Ok(editor
        .readline(&prompt(format_args!("{msg}? (Y/n)")))?
        .starts_with(['n', 'N']))
}
