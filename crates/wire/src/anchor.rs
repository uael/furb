//! The place in a project that a thread is about, as a review of a pull request anchors a comment:
//! a line of a file, or a folder. Its message carries the path, and for a line its number and its
//! text as it stood when the message was said, which anchors it in time and which the model reads.
//! The anchor is part of the text of the message, so a person and a model say it with the same
//! `prompt`, with no verb of its own. A prompt on a thread whose message carries an anchor is a sub
//! thread: a thread of its own under that thread, which holds its answers and resolves apart.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A place that a message is about: its path, and where it stands in it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Anchor {
  pub path: String,
  pub at: Where,
}

/// Where an anchor stands in its path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Where {
  /// A line of the file: its number in the file after the change, or before it for a line that the
  /// change removed, and its text then.
  Line { line: u64, before: bool, quote: String },
  /// The whole folder.
  Folder,
}

/// The words that open the head of a message about a line, and of a message about a folder.
const ON: &str = "On `";
const FOLDER: &str = "On the folder `";

impl Anchor {
  /// The text of a message about the anchor: a head that names the place, the line as a quote,
  /// then what the operator says of it.
  pub fn said(&self, comment: &str) -> String {
    match &self.at {
      Where::Line { line, before, quote } => {
        let side = if *before { "before the change" } else { "after the change" };
        format!("{ON}{}`, line {line} {side}:\n> {quote}\n\n{comment}", self.path)
      }
      Where::Folder => format!("{FOLDER}{}`:\n\n{comment}", self.path),
    }
  }

  /// The anchor of a message and what it says of it, when the message carries one.
  pub fn of(message: &str) -> Option<(Self, &str)> {
    if let Some(rest) = message.strip_prefix(FOLDER) {
      let (path, comment) = rest.split_once("`:\n\n")?;
      return Some((Self { path: path.to_owned(), at: Where::Folder }, comment));
    }
    let (head, rest) = message.strip_prefix(ON)?.split_once('\n')?;
    let (path, place) = head.split_once("`, line ")?;
    let (line, side) = place.split_once(' ')?;
    let before = match side {
      "before the change:" => true,
      "after the change:" => false,
      _ => return None,
    };
    let (quote, comment) = rest.strip_prefix("> ")?.split_once("\n\n")?;
    let at = Where::Line { line: line.parse().ok()?, before, quote: quote.to_owned() };
    Some((Self { path: path.to_owned(), at }, comment))
  }
}
