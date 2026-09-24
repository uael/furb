//! The extensions: what a host plays on a chain, and where it finds it.
//!
//! The python part of an extension is a file that a host plays as a rung, the World on every chain without a
//! source. The file is a word as it is, or a python module that imports what it uses from the engine and from the
//! extensions it requires, so that an editor, ruff and ty read it. The module of a chain binds every name of the
//! engine and of each word played before it already, so the host makes the word of a module by blanking those
//! imports, and the word reads every name through the globals of the chain, where a later rung may rebind it.

use ruff_python_ast::Stmt;
use ruff_text_size::{Ranged, TextRange};

/// The package the engine and the extensions are imported from, whose imports a word leaves out.
const PACKAGE: &str = "furb";

/// The word of a python part: the file with its line ends made LF, and every top-level `from furb... import` made
/// blank, and nothing else changed.
///
/// A line end is LF first, so a word is the same on every machine, and a clone that writes CRLF plays no word
/// again. Each line of such an import becomes an empty line, so every other line keeps its number and a finding of
/// the gate points at the line of the file. An import that shares its line with another statement leaves that
/// statement, and the semicolon between the two goes with the import. Every other byte stays as it is, the space at
/// the end of a line among them, since a string may hold it. A file python cannot parse is its own word, since the
/// gate refuses it with what it finds, and a file with no such import is a word already.
pub fn word(source: &str) -> String {
  let source = source.replace("\r\n", "\n");
  let Ok(parsed) = ruff_python_parser::parse_module(&source) else {
    return source;
  };
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
    let (start, end) = (usize::from(cut.start()), usize::from(cut.end()));
    let (start, end) = joined(&out, start, end);
    let kept: String = out[start..end].chars().filter(|&c| c == '\n').collect();
    out.replace_range(start..end, &kept);
  }
  out
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
