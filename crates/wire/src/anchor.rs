//! The place in a file that a thread is about, as a review of a pull request anchors a comment:
//! its message carries the path, the line and the text of that line as it stood when the message
//! was said, which anchors it in time and which the model reads. The anchor is part of the text
//! of the message, so a person and a model say it with the same `prompt`, with no verb of its
//! own. A prompt on a thread whose message carries an anchor is a sub thread: a thread of its own
//! under that thread, which holds its answers and resolves apart.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A line of a file that a message is about: its path, its number in the file after the change,
/// or before it for a line that the change removed, and its text then.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Anchor {
  pub path: String,
  pub line: u64,
  pub before: bool,
  pub quote: String,
}

/// The words that open the head of a message with an anchor.
const ON: &str = "On `";

impl Anchor {
  /// The text of a message about the anchor: a head that names the path and the line, the line
  /// as a quote, then what the operator says of it.
  pub fn said(&self, comment: &str) -> String {
    let side = if self.before { "before the change" } else { "after the change" };
    format!("{ON}{}`, line {} {side}:\n> {}\n\n{comment}", self.path, self.line, self.quote)
  }

  /// The anchor of a message and what it says of it, when the message carries one.
  pub fn of(message: &str) -> Option<(Self, &str)> {
    let (head, rest) = message.strip_prefix(ON)?.split_once('\n')?;
    let (path, place) = head.split_once("`, line ")?;
    let (line, side) = place.split_once(' ')?;
    let before = match side {
      "before the change:" => true,
      "after the change:" => false,
      _ => return None,
    };
    let (quote, comment) = rest.strip_prefix("> ")?.split_once("\n\n")?;
    let anchor =
      Self { path: path.to_owned(), line: line.parse().ok()?, before, quote: quote.to_owned() };
    Some((anchor, comment))
  }
}
