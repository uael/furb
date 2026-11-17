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
  /// A line of the file, or the lines from it: its number in the file after the change, or before
  /// it for a line that the change removed, and its text then, one line of the quote for each line.
  Line { line: u64, before: bool, quote: String },
  /// The whole folder.
  Folder,
}

/// The words that open the head of a message about a line, and of a message about a folder.
const ON: &str = "On `";
const FOLDER: &str = "On the folder `";

impl Where {
  /// The last line of a place in a file, the first for one line, none for a folder.
  pub fn last(&self) -> Option<u64> {
    match self {
      Self::Line { line, quote, .. } => {
        let more = u64::try_from(quote.split('\n').count().saturating_sub(1)).unwrap_or_default();
        Some(line + more)
      }
      Self::Folder => None,
    }
  }
}

impl Anchor {
  /// The text of a message about the anchor: a head that names the place, its lines as a quote,
  /// then what the operator says of it.
  pub fn said(&self, comment: &str) -> String {
    match &self.at {
      Where::Line { line, before, quote } => {
        let side = if *before { "before the change" } else { "after the change" };
        let lines = match self.at.last() {
          Some(last) if last > *line => format!("lines {line} to {last}"),
          _ => format!("line {line}"),
        };
        let quoted: Vec<String> = quote.split('\n').map(|one| format!("> {one}")).collect();
        format!("{ON}{}`, {lines} {side}:\n{}\n\n{comment}", self.path, quoted.join("\n"))
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
    let (path, place) = head.split_once("`, ")?;
    let (lines, side) = match place.strip_prefix("lines ") {
      Some(range) => {
        let (first, rest) = range.split_once(" to ")?;
        let (_, side) = rest.split_once(' ')?;
        (first, side)
      }
      None => place.strip_prefix("line ")?.split_once(' ')?,
    };
    let before = match side {
      "before the change:" => true,
      "after the change:" => false,
      _ => return None,
    };
    let (quoted, comment) = rest.split_once("\n\n")?;
    let quote: Option<Vec<&str>> = quoted.split('\n').map(|one| one.strip_prefix("> ")).collect();
    let quote = quote?.join("\n");
    let at = Where::Line { line: lines.parse().ok()?, before, quote };
    Some((Self { path: path.to_owned(), at }, comment))
  }
}

#[cfg(test)]
#[path = "anchor.test.rs"]
mod tests;
