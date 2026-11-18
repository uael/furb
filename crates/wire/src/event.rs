//! The events of a life: one scheme of deltas, which each client of the host reads in the same
//! order.

use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{act::Act, page::Retrying};

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
  Act(Arc<Act>),
  /// What a model added to its text and to its thought, as it writes the reply of a rung on a
  /// chain. It is no fact, and the turn of the reply holds all of it.
  Text { rung: String, chain: String, text: String, thinking: String },
  /// A call of a model that failed for a cause that passes, as it writes the reply of a rung on a
  /// chain, and the retry of its provider after a wait. What the model wrote for the rung before
  /// it is void. It is no fact.
  Retried { rung: String, chain: String, retry: Retrying },
  /// The follow-ups that wait on a chain, in order, each time that a fact of the operator or a send
  /// of the life changed them.
  Queued { chain: String, waiting: Vec<FollowUp> },
  /// A report of the life that no model reads, one line.
  Notice(String),
  /// A chain whose queries give another answer now, once each fact that changed it was told.
  Changed(String),
  /// The life told the past of its record and is born on it, with no fault: each event after it
  /// is of the birth, which tells each chain, its turns and each open prompt again as it stood.
  Reborn(Option<String>),
  /// The end of the life, with the fault that ended it, or none when a client ended it. It is the
  /// last event.
  Ended(Option<String>),
}

/// One follow-up that waits on a chain: its entry, which the window that queued it chose, the
/// thread that it goes to as a message, none for a new thread of the chain, the shape that its
/// prompt wants, its message, and why its send failed, when it did. A follow-up that failed holds
/// the queue of its chain until the operator takes it out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct FollowUp {
  pub entry: String,
  pub chain: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub thread: Option<String>,
  pub shape: String,
  pub message: String,
  pub failed: Option<String>,
}
