//! The watch over the word of a reply while its model writes it.
//!
//! A word is python, and a statement of it that cannot be python stays so whatever the model writes after it. So the
//! provider reads the statements that a model finished, each as it finishes, and ends the reply at the first one
//! that cannot be python, with the word as far as that statement. The gate refuses that word, as it refuses the
//! whole one, and the model is not made to write the rest of a word that the chain will refuse.

use ruff_python_parser::{ParseErrorType, parse_module};

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

/// Whether a quote stands open in a word, since the text of a quote is no python until its end is written.
pub(super) fn open(word: &str) -> bool {
  word.rfind(QUOTE) > word.rfind(UNQUOTE)
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

#[cfg(test)]
#[path = "watch.test.rs"]
mod test;
