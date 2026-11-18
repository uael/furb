//! A ladder as a window draws it: its steps, the rungs that an ear wrote in a row, and the folds
//! of earlier steps.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::act::{Name, read_name};

/// How many of the last prompts of a thread, and of the last rungs of each ladder, show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Shown {
  pub prompts: usize,
  pub rungs: usize,
}

/// The steps that a ladder shows when the window asks no other count.
pub const SHOWN: Shown = Shown { prompts: 6, rungs: 4 };

/// One line of a ladder, in the order that the window draws it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Rung {
  /// A step, with the steps under it after it, each one level deeper.
  Step {
    #[serde(deserialize_with = "read_name")]
    id: Name,
    depth: usize,
  },
  /// The rungs in a row that one maker wrote on the chain with no word of its own, as the words of
  /// the skills and the setup of a chain.
  Ear { rungs: Vec<String>, depth: usize },
  /// A fold of `count` earlier steps of a level, from the step `head`, with the kinds of the steps
  /// that it holds in the order they first come. `key` opens it: prompts: or rungs: and the act
  /// that the steps stand under, or the chain at the top.
  More { head: String, depth: usize, count: usize, of: String, key: String, heads: Vec<String> },
}
