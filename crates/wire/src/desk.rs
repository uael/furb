//! What waits for the operator in a life: each question that he owes and each notice that a
//! window still shows.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::act::{Name, read_name};

/// A prompt to the operator that waits for him. A question waits for an answer of its shape. A
/// notice is a prompt of None, which the console closed as it took it, and which waits only until
/// the operator dismisses it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Ask {
  #[serde(deserialize_with = "read_name")]
  pub id: Name,
  /// The chain that the prompt stands in.
  #[serde(deserialize_with = "read_name")]
  pub chain: Name,
  /// Who made the prompt: a rung, an ear, or the operator.
  #[serde(deserialize_with = "read_name")]
  pub by: Name,
  pub shape: String,
  pub message: String,
  pub notice: bool,
  /// The lines that answer a choice, one for each value that the shape allows.
  pub choices: Vec<String>,
  /// The values of a list of a Literal, which the operator ticks.
  pub ticks: Vec<Tick>,
  /// What the line of the operator is, in words, when its shape is more than text or a choice.
  pub hint: String,
  pub began: u64,
}

/// A value of a list of a Literal: as the operator reads it, and as the line of the list writes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Tick {
  pub shown: String,
  pub json: String,
}

/// What waits for the operator in a life: each ask, the oldest first, and how many of them are
/// questions and notices.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Desk {
  pub asks: Vec<Ask>,
  pub questions: usize,
  pub notices: usize,
}
