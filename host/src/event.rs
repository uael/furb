//! The events of a life: one scheme of deltas, which each client of the host reads in the same
//! order.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{queue::FollowUp, view::Act};

/// The place of an event in the log of its life, counted from 0 with no gap.
pub type Seq = u64;

/// What a life tells its clients, in the order of its log.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
pub enum Event {
  /// One fact that the life said, as JSON reads it: each fact of the record that the life said
  /// again as it opened, then each fact that it said after.
  Fact(Value),
  /// An act with its meaning as it stands after a fact about it, which takes the place of each
  /// earlier event of the same act.
  Act(Box<Act>),
  /// What a model added to its text and to its thought, as it writes the reply of a rung on a
  /// chain. It is no fact, and the turn of the reply holds all of it.
  Text { rung: String, chain: String, text: String, thinking: String },
  /// The follow-ups that wait on a chain, in order, each time that a fact of the operator or a send
  /// of the life changed them.
  Queued { chain: String, waiting: Vec<FollowUp> },
  /// A report of the life that no model reads, one line.
  Notice(String),
  /// A chain whose queries give another answer now, once each fact that changed it was told.
  Changed(String),
  /// The life opened again from its checkpoint, on the same thread and for the same readers, with
  /// the fault that stopped it when one did. Each event after it is of the new open, which tells
  /// each chain, its turns and each open prompt again as it stood.
  Reborn(Option<String>),
  /// The end of the life, with the fault that ended it, or none when a client ended it. It is the
  /// last event.
  Ended(Option<String>),
}
