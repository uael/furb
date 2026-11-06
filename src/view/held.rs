//! The acts of a life as a window last heard them, which the life keeps as it tells them, so a
//! window asks the life for the meaning that it draws: the desk, the threads of a chain, the acts
//! as they stood at a moment, and the moments.

use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
  act::{Act, Kind},
  desk::{Desk, desk},
  state::State,
  threads::{
    Attention, ChainAside, Rows, ThreadNode, attention, chain_asides, chain_threads, nest_threads,
  },
  time::{as_of, moments_of},
};

/// The threads of one chain as a window draws them: the tree of its threads, its asides, what the
/// tree says to the operator, and the state of the chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Threads {
  pub threads: Vec<ThreadNode>,
  pub asides: Vec<ChainAside>,
  pub attention: Attention,
  pub state: State,
}

/// Each act of a life, as the last event of it told it, in the order the acts were first told.
#[derive(Debug, Default)]
pub struct Held {
  acts: Vec<Act>,
  at: HashMap<String, usize>,
}

impl Held {
  /// An act of the life heard as a host tells it: it takes the place of the earlier one of the same
  /// id.
  pub fn heard(&mut self, act: &Act) {
    if let Some(&at) = self.at.get(&act.id) {
      self.acts[at] = act.clone();
    } else {
      self.at.insert(act.id.clone(), self.acts.len());
      self.acts.push(act.clone());
    }
  }

  /// The acts now, or as they stood just after a moment.
  fn then(&self, at: Option<u64>) -> Vec<Act> {
    let mut acts = match at {
      Some(at) => as_of(&self.acts, at),
      None => self.acts.clone(),
    };
    acts.sort_by_key(|act| act.began);
    acts
  }

  /// What waits for the operator, with each notice that the window still shows.
  pub fn desk(&self, notices: &[String]) -> Desk {
    let notices: Vec<&str> = notices.iter().map(String::as_str).collect();
    desk(&self.then(None), &notices)
  }

  /// The threads of a chain, now or as they stood just after a moment.
  pub fn threads(&self, chain: &str, at: Option<u64>) -> Threads {
    let acts = self.then(at);
    let rows: Rows<'_> = acts.iter().map(|act| (act.id.as_str(), act)).collect();
    let on: Vec<&Act> = acts.iter().filter(|act| act.chain == chain && act.id != chain).collect();
    let threads = nest_threads(chain_threads(&on, &rows), &rows);
    let state =
      rows.get(chain).filter(|act| act.kind == Kind::Chain).map_or(State::Idle, |act| act.state);
    Threads { attention: attention(&threads), asides: chain_asides(&on, &rows), threads, state }
  }

  /// The acts as they stood just after a moment.
  pub fn as_of(&self, at: u64) -> Vec<Act> {
    self.then(Some(at))
  }

  /// Each moment at which an act began or ended.
  pub fn moments(&self) -> Vec<u64> {
    moments_of(&self.acts)
  }
}
