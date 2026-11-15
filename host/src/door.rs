//! The doors of a person: each act that a model writes as a word, which a window sends for a
//! person as the same word through [`crate::protocol::Verb::Run`]. The life runs it as a turn of
//! his on the chain that it names, so the transcript of that chain shows his act as it shows the
//! act of a model, and no act of his takes another path.

use furb::skill::granted;

/// An act of a person on a chain, as the word that a model would write for it.
#[derive(Debug, Clone, PartialEq)]
pub enum Door {
  /// A new chain under a label, made on the chain of the word.
  Chain { label: String },
  /// A chain under a label that stands on the chain `source`: with each act of it, or with the
  /// acts that `kept` names and what they caused, as a rewind keeps the acts before a message of
  /// his. The source stays as it is, so the chain that it was is a branch that he can go back to.
  Fork { label: String, source: String, kept: Option<Vec<String>> },
  /// The folder where the chain works from then on.
  Cd { path: String },
  /// The actor that the chain asks from then on, as `claude-cli:sonnet/high`.
  Stand { actor: String },
  /// The program of a prompt written again: the rungs of its ladder that it keeps stand, and the
  /// first word that differs runs again from there, with each word after it.
  Edit { prompt: String, program: String },
  /// A ceiling of the spend of the chain, in dollars, in a share of its window, or both.
  Grant { usd: Option<f64>, share: Option<f64> },
}

impl Door {
  /// The word of the act, in python.
  #[must_use]
  pub fn word(&self) -> String {
    match self {
      Self::Chain { label } => format!("chain({})", quoted(label)),
      Self::Fork { label, source, kept: None } => {
        format!("chain({}, source={})", quoted(label), quoted(source))
      }
      Self::Fork { label, source, kept: Some(kept) } => {
        let kept: Vec<String> = kept.iter().map(|id| quoted(id)).collect();
        let (label, source) = (quoted(label), quoted(source));
        format!("chain({label}, source={source}, filter=take({}))", kept.join(", "))
      }
      Self::Cd { path } => format!("cd({})", quoted(path)),
      Self::Stand { actor } => format!("actor = {}", quoted(actor)),
      Self::Edit { prompt, program } => {
        format!("write(Text({}, {}))", quoted(prompt), quoted(program))
      }
      Self::Grant { usd, share } => granted(*usd, *share),
    }
  }
}

/// A text as a string of python: the string of JSON, whose each escape python reads the same.
fn quoted(text: &str) -> String {
  serde_json::Value::from(text).to_string()
}

#[cfg(test)]
#[path = "door.test.rs"]
mod test;
