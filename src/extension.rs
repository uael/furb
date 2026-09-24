//! The extensions: what a host plays on a chain, and where it finds it.
//!
//! The python part of an extension is a file that a host plays as a rung, the World on every chain without a
//! source. The file is a word as it is, or a python module that imports what it uses from the engine and from the
//! extensions it requires, so that an editor, ruff and ty read it. The module of a chain binds every name of the
//! engine and of each word played before it already, so the host makes the word of a module by blanking those
//! imports, and the word reads every name through the globals of the chain, where a later rung may rebind it.
//!
//! Every host shares this module: the host in TypeScript, the host in python and a host in rust read the same
//! words of the builtins, make the same word of a module, and play a word on a chain by the same rule.

use std::{fmt, path::PathBuf};

use ruff_python_ast::Stmt;
use ruff_text_size::{Ranged, TextRange};

use crate::value::Fault;

/// The package the engine and the extensions are imported from, whose imports a word leaves out.
const PACKAGE: &str = "furb";

/// What went wrong with an extension, which a host says once when it loads the extensions and plays nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
  /// A python part that python cannot parse, by the name of its extension, and the line of the fault.
  Word { name: String, line: usize, why: String },
}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Error::Word { name, line, why } if name.is_empty() => write!(f, "line {line}: {why}"),
      Error::Word { name, line, why } => write!(f, "the extension {name}: line {line}: {why}"),
    }
  }
}

impl std::error::Error for Error {}

impl From<Error> for Fault {
  fn from(error: Error) -> Self {
    Fault::refused(error.to_string())
  }
}

/// The parts of an extension for a World, one file for each language a host writes its World in.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Worlds {
  pub ts: Option<PathBuf>,
  pub py: Option<PathBuf>,
}

/// One extension as a host plays it: its name, the directory it stands in (none for a builtin), the word of its
/// python part, the names it requires, and the files of its other parts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extension {
  pub name: String,
  pub root: Option<PathBuf>,
  pub word: String,
  pub requires: Vec<String>,
  pub world: Worlds,
  pub tui: Option<PathBuf>,
}

/// The builtin extensions, in the order a host plays them: files, bash, which requires files, and grant. Their
/// words are the words of the files the python package ships, and their other parts are each host's own.
pub fn builtins() -> Vec<Extension> {
  let builtin = |name: &str, source: &str, requires: &[&str]| Extension {
    name: name.to_owned(),
    root: None,
    word: word(source).unwrap_or_else(|error| panic!("the builtin {name} does not parse: {error}")),
    requires: requires.iter().map(|&one| one.to_owned()).collect(),
    world: Worlds::default(),
    tui: None,
  };
  vec![
    builtin("files", include_str!("furb/builtin/files.py"), &[]),
    builtin("bash", include_str!("furb/builtin/bash.py"), &["files"]),
    builtin("grant", include_str!("furb/builtin/grant.py"), &[]),
  ]
}

/// The words of the extensions, in their order, which is what a host plays.
pub fn words(extensions: &[Extension]) -> Vec<String> {
  extensions.iter().map(|one| one.word.clone()).collect()
}

/// The words that a program lacks, in their order: a word is missing when no word of the program is the same
/// string. This is the rule a host plays by, after boot on every chain without a source and at the birth of each.
pub fn missing<'a>(program: &[&str], words: &'a [String]) -> Vec<&'a str> {
  words.iter().map(String::as_str).filter(|one| !program.contains(one)).collect()
}

/// The word of a python part: the file with its line ends made LF, and every top-level `from furb... import` made
/// blank, and nothing else changed.
///
/// A line end is LF first, so a word is the same on every machine, and a clone that writes CRLF plays no word
/// again. Each line of such an import becomes an empty line, so every other line keeps its number and a finding of
/// the gate points at the line of the file. An import that shares its line with another statement leaves that
/// statement, and the semicolon between the two goes with the import. Every other byte stays as it is, the space at
/// the end of a line among them, since a string may hold it. A file with no such import is a word already. A file
/// python cannot parse is refused with the line of the fault, so a host says it once when it loads the extension
/// and no chain plays a broken word.
pub fn word(source: &str) -> Result<String, Error> {
  let source = source.replace("\r\n", "\n");
  let parsed = ruff_python_parser::parse_module(&source).map_err(|fault| Error::Word {
    name: String::new(),
    line: source[..usize::from(fault.location.start())].matches('\n').count() + 1,
    why: fault.error.to_string(),
  })?;
  let mut cuts: Vec<TextRange> = parsed
    .syntax()
    .body
    .iter()
    .filter_map(|statement| match statement {
      Stmt::ImportFrom(import)
        if import.level == 0
          && import.module.as_ref().is_some_and(|module| {
            let name = module.id.as_str();
            name == PACKAGE || name.strip_prefix(PACKAGE).is_some_and(|rest| rest.starts_with('.'))
          }) =>
      {
        Some(import.range())
      }
      _ => None,
    })
    .collect();
  cuts.reverse();
  let mut out = source.clone();
  for cut in cuts {
    let (start, end) = joined(&out, usize::from(cut.start()), usize::from(cut.end()));
    let kept: String = out[start..end].chars().filter(|&c| c == '\n').collect();
    out.replace_range(start..end, &kept);
  }
  Ok(out)
}

/// The span of an import with the semicolon that joins it to a statement on its line: the one after it, or else
/// the one before it.
fn joined(text: &str, start: usize, end: usize) -> (usize, usize) {
  let after = &text[end..];
  let gap = after.len() - after.trim_start_matches([' ', '\t']).len();
  if after[gap..].starts_with(';') {
    let rest = &after[gap + 1..];
    let more = rest.len() - rest.trim_start_matches([' ', '\t']).len();
    return (start, end + gap + 1 + more);
  }
  let before = &text[..start];
  let gap = before.len() - before.trim_end_matches([' ', '\t']).len();
  if before[..before.len() - gap].ends_with(';') {
    return (start - gap - 1, end);
  }
  (start, end)
}

#[cfg(test)]
#[path = "extension.test.rs"]
mod tests;
