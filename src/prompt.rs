use std::fmt::Display;

use owo_colors::{OwoColorize, Style};
use rustyline::{
    completion::{Candidate, Completer},
    hint::{Hint, Hinter},
    validate::{ValidationContext, ValidationResult, Validator},
    Context, Helper, Highlighter,
};

use crate::{
    fetcher::{Revisions, Version},
    BuildType,
};

#[derive(Helper, Highlighter)]
pub enum Prompter {
    Revision(Revisions),
    NonEmpty,
    Build(Vec<(BuildType, &'static str)>),
}

#[derive(Clone)]
pub struct Completion {
    pub display: String,
    pub replacement: String,
}

impl Candidate for Completion {
    fn display(&self) -> &str {
        &self.display
    }

    fn replacement(&self) -> &str {
        &self.replacement
    }
}

impl Completer for Prompter {
    type Candidate = Completion;

    fn complete(
        &self,
        _: &str,
        _: usize,
        _: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        match self {
            Prompter::Revision(revisions) => Ok((0, revisions.completions.clone())),
            Prompter::NonEmpty => Ok((0, Vec::new())),
            Prompter::Build(choices) => Ok((
                0,
                choices
                    .iter()
                    .enumerate()
                    .map(|(i, &(_, msg))| Completion {
                        display: format!("{i} - {msg}"),
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

            Prompter::Build(choices) => Some(SimpleHint(if line.is_empty() {
                format_args!("  ({})", choices[0].1)
                    .blue()
                    .italic()
                    .to_string()
            } else if let Some((_, msg)) = line.parse().ok().and_then(|i: usize| choices.get(i)) {
                format_args!("  ({msg})").blue().italic().to_string()
            } else {
                "  press <tab> to see options".yellow().italic().to_string()
            })),
        }
    }
}

impl Validator for Prompter {
    fn validate(&self, ctx: &mut ValidationContext) -> rustyline::Result<ValidationResult> {
        Ok(match self {
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

            Prompter::Build(choices) => {
                let input = ctx.input();
                if input.is_empty() {
                    ValidationResult::Valid(Some(choices[0].1.into()))
                } else if let Some(&(_, choice)) = input
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
