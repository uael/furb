//! The doors of a person: each act that a model writes as a word, which a window sends for a
//! person as the same word through [`crate::protocol::Verb::Run`]. The life runs it as a turn of
//! his on the chain that it names, so the transcript of that chain shows his act as it shows the
//! act of a model, and no act of his takes another path.

use furb::{skill::granted, value::repr};

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
  /// A stand of the life, which reads the roster, the actor and the settings of the World again.
  Stand,
  /// The actor that the chain asks from then on, as `claude-cli:sonnet/high`.
  Actor { actor: String },
  /// A command, with the text that its stdin is fed whole, or with its stdin open for each write
  /// to `<bash act>/stdin` when the text is none.
  Bash { command: String, stdin: Option<String> },
  /// The text at a path, which the transcript tells.
  Read { path: String },
  /// A text written at a path. The path of a prompt is the door of its program: the rungs of its
  /// ladder whose words stay stand, and the first word that differs runs again from there.
  Write { path: String, content: String },
  /// A ceiling of the spend of the chain, in dollars, in a share of its window, or both.
  Grant { usd: Option<f64>, share: Option<f64> },
}

impl Door {
  /// The word of the act, in python.
  #[must_use]
  pub fn word(&self) -> String {
    match self {
      Self::Chain { label } => format!("chain({})", repr(label)),
      Self::Fork { label, source, kept: None } => {
        format!("chain({}, source={})", repr(label), repr(source))
      }
      Self::Fork { label, source, kept: Some(kept) } => {
        let kept: Vec<String> = kept.iter().map(|id| repr(id)).collect();
        let (label, source) = (repr(label), repr(source));
        format!("chain({label}, source={source}, filter=take({}))", kept.join(", "))
      }
      Self::Cd { path } => format!("cd({})", repr(path)),
      Self::Stand => "stand()".to_owned(),
      Self::Actor { actor } => format!("actor = {}", repr(actor)),
      Self::Bash { command, stdin: None } => format!("bash({}, stdin=None)", repr(command)),
      Self::Bash { command, stdin: Some(stdin) } if stdin.is_empty() => {
        format!("bash({})", repr(command))
      }
      Self::Bash { command, stdin: Some(stdin) } => {
        format!("bash({}, stdin={})", repr(command), repr(stdin))
      }
      Self::Read { path } => format!("read({})", repr(path)),
      Self::Write { path, content } => format!("write(Text({}, {}))", repr(path), repr(content)),
      Self::Grant { usd, share } => granted(*usd, *share),
    }
  }
}

#[cfg(test)]
#[path = "door.test.rs"]
mod test;
