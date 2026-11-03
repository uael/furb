//! The events of a life: one scheme of deltas, which each client of the host reads in the same
//! order.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The place of an event in the log of its life, counted from 0 with no gap.
pub type Seq = u64;

/// What a life tells its clients, in the order of its log.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
pub enum Event {
  /// One fact that the life said, as JSON reads it: each fact of the record that the life said
  /// again as it opened, then each fact that it said after.
  Fact(Value),
  /// An act as it stands after a fact about it, which takes the place of each earlier event of the
  /// same act.
  Act(Act),
  /// What a model added to its text and to its thought, as it writes the reply of a rung on a
  /// chain. It is no fact, and the turn of the reply holds all of it.
  Text { rung: String, chain: String, text: String, thinking: String },
  /// A report of the life that no model reads, one line.
  Notice(String),
  /// A chain whose queries give another answer now, once each fact that changed it was told.
  Changed(String),
  /// The end of the life, with the fault that ended it, or none when a client ended it. It is the
  /// last event.
  Ended(Option<String>),
}

/// One act of a life: its kind, its name, who made it, what it is on, the chain it is in, its words
/// after these, and its state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Act {
  pub kind: String,
  pub id: String,
  pub by: String,
  pub on: String,
  pub chain: String,
  pub words: Vec<Value>,
  pub state: State,
}

/// Where an act stands: asked, started by the ear that owns it, or done, with what it came to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "state", content = "value", rename_all = "snake_case")]
pub enum State {
  Asked,
  Started,
  Done(Value),
}
