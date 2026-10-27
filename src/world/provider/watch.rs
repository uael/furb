//! The watch over the word of a reply while its model writes it.
//!
//! A word is python, and a statement of it that cannot be python stays so whatever the model writes after it. So the
//! provider reads the statements that a model finished, each as it finishes, and ends the reply at the first one
//! that cannot be python, with the word as far as that statement. The gate refuses that word, as it refuses the
//! whole one, and the model is not made to write the rest of a word that the chain will refuse.
//!
//! A statement that is python can still hold a fault that the gate finds, as a name that nothing binds. A later
//! statement mends no such fault in a statement at the top of the word, which runs before it, unless the statement
//! holds a part that python runs later or never: a function, a class, a lambda, a comprehension or an annotation. So
//! the provider asks the gate about the statements that a model finished, and ends the reply after the first
//! statement with no such part that holds a fault.

use ruff_python_ast::{
  Expr, Stmt,
  visitor::{self, Visitor},
};
use ruff_python_parser::{ParseErrorType, parse_module};
use ruff_text_size::Ranged;

/// The mark that opens a quote in a word: `<s:name>` at the start of a line.
pub(super) const QUOTE: &str = "<s:";

/// The mark that opens the end of a quote in a word: `</s:name>` at the end of a line.
const UNQUOTE: &str = "</s:";

/// What a reply that is watched has of its word so far.
#[derive(Default)]
pub(super) struct Watch {
  text: String,
  /// Where the last statements to read ended: the start of a line at the left edge, or 0.
  seen: usize,
}

impl Watch {
  /// The word grown by a part the model wrote, and where the statements to read end if a new line at the left edge
  /// started: the start of the last such line. A statement that starts there may not be whole, but every statement
  /// before it is, unless a string or a bracket stays open, which the faults of [`broken`] leave to a later read.
  pub(super) fn wrote(&mut self, part: &str) -> Option<usize> {
    self.text.push_str(part);
    let end = self.text[self.seen..]
      .match_indices('\n')
      .filter(|(at, _)| {
        self.text[self.seen + at + 1..].chars().next().is_some_and(|one| !one.is_whitespace())
      })
      .map(|(at, _)| self.seen + at + 1)
      .next_back()?;
    self.seen = end;
    Some(end)
  }

  /// The word up to where the statements to read end.
  pub(super) fn upto(&self, end: usize) -> &str {
    &self.text[..end]
  }

  /// The word as far as the end of a line of it, counted from 1.
  pub(super) fn through(&self, line: usize) -> &str {
    let end = self.text.match_indices('\n').nth(line - 1).map_or(self.text.len(), |(at, _)| at);
    &self.text[..end]
  }

  /// The word forgotten, as when the model is asked again and writes it from its start.
  pub(super) fn forget(&mut self) {
    *self = Watch::default();
  }
}

/// Whether a quote stands open in a word, since the text of a quote is no python until its end is written. A quote
/// opens at `<s:name>` at the start of a line and ends at the first `</s:name>` after it that ends a line, as the
/// engine reads one, so a close mark inside a line of its text ends nothing.
pub(super) fn open(word: &str) -> bool {
  let mut from = 0;
  while let Some(start) = word[from..].find(QUOTE).map(|at| from + at) {
    from = start + QUOTE.len();
    let Some((name, _)) = word[from..].split_once('>') else { continue };
    let named = !name.is_empty() && name.chars().all(|one| one.is_alphanumeric() || one == '_');
    if !named || !(start == 0 || word[..start].ends_with('\n')) {
      continue;
    }
    let text = from + name.len() + 1;
    let close = format!("{UNQUOTE}{name}>");
    let ends = word[text..]
      .match_indices(&close)
      .find(|(at, _)| matches!(word[text + at + close.len()..].chars().next(), None | Some('\n')));
    match ends {
      Some((at, _)) => from = text + at + close.len(),
      None => return true,
    }
  }
  false
}

/// The line, counted from 1, of the first fault of a word that more text cannot mend, if it has one: a fault at the
/// end of the word may be a statement that is not whole yet, and so may a string that has no end yet.
pub(super) fn broken(word: &str) -> Option<usize> {
  let no = parse_module(word).err()?;
  let (start, end) = (usize::from(no.location.start()), usize::from(no.location.end()));
  let last = word.trim_end().len();
  let lexical = matches!(no.error, ParseErrorType::Lexical(_));
  (start < last && !(lexical && end >= last)).then(|| word[..start].matches('\n').count() + 1)
}

/// The first and the last line, counted from 1, of each statement at the top of a word whose faults no later
/// statement can mend, or none when the word is not python.
pub(super) fn settled(word: &str) -> Vec<(usize, usize)> {
  let Ok(parsed) = parse_module(word) else { return vec![] };
  let line = |at: usize| word[..at].matches('\n').count() + 1;
  let body = &parsed.syntax().body;
  body
    .iter()
    .filter(|stmt| {
      let mut later = Later(false);
      later.visit_stmt(stmt);
      !later.0
    })
    .map(|stmt| (line(stmt.start().to_usize()), line(stmt.end().to_usize())))
    .collect()
}

/// The last line of the first settled statement that holds a finding of the gate, if one does. Each finding starts
/// with the line of the word that it is found on, as `line 3: ...`.
pub(super) fn refused(settled: &[(usize, usize)], found: &[String]) -> Option<usize> {
  let lines: Vec<usize> = found
    .iter()
    .filter_map(|one| one.strip_prefix("line ")?.split(':').next()?.parse().ok())
    .collect();
  settled
    .iter()
    .find(|(first, last)| lines.iter().any(|line| (first..=last).contains(&line)))
    .map(|(_, last)| *last)
}

/// Whether a statement holds a part that python runs later or never, which a later statement can change the
/// meaning of.
struct Later(bool);

impl<'a> Visitor<'a> for Later {
  fn visit_stmt(&mut self, stmt: &'a Stmt) {
    match stmt {
      Stmt::FunctionDef(_) | Stmt::ClassDef(_) | Stmt::AnnAssign(_) => self.0 = true,
      _ => visitor::walk_stmt(self, stmt),
    }
  }

  fn visit_expr(&mut self, expr: &'a Expr) {
    match expr {
      Expr::Lambda(_)
      | Expr::ListComp(_)
      | Expr::SetComp(_)
      | Expr::DictComp(_)
      | Expr::Generator(_) => {
        self.0 = true;
      }
      _ => visitor::walk_expr(self, expr),
    }
  }
}

#[cfg(test)]
#[path = "watch.test.rs"]
mod test;
