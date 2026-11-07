//! The acts of a life as a window last heard them, which the life keeps as it tells them, so a
//! window asks the life for the meaning that it draws: the desk, the threads of a chain, the acts
//! as they stood at a moment, and the moments.

use std::{
  borrow::Cow,
  collections::{BTreeSet, HashMap},
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
  act::{Act, Kind},
  desk::{Desk, desk},
  ladder::{Index, Rung, Shown, folded, lives},
  state::State,
  threads::{
    Attention, ChainAside, NoteRun, Rows, ThreadNode, attention, chain_asides, chain_notes,
    chain_threads, chain_tree, nest_threads, order_threads,
  },
  time::{as_of, moments_of},
};

/// The threads of one chain as a window draws them: the tree of its threads, its asides, the runs
/// of its notes, what the tree says to the operator, and the state of the chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Threads {
  pub threads: Vec<ThreadNode>,
  pub asides: Vec<ChainAside>,
  pub notes: Vec<NoteRun>,
  pub attention: Attention,
  pub state: State,
}

/// Each act of a life, as the last event of it told it, in the order the acts were first told.
#[derive(Debug, Default)]
pub struct Held {
  acts: Vec<Act>,
  at: HashMap<String, usize>,
  /// The places of the acts of each maker, and of each chain, in the order they were first told.
  made: HashMap<String, Vec<usize>>,
  on: HashMap<String, Vec<usize>>,
  /// The places of the acts that live: not done, or a rung whose word runs.
  live: BTreeSet<usize>,
}

impl Held {
  /// An act of the life heard as a host tells it: it takes the place of the earlier one of the same
  /// id.
  pub fn heard(&mut self, act: &Act) {
    let at = if let Some(&at) = self.at.get(&act.id) {
      self.acts[at] = act.clone();
      at
    } else {
      let at = self.acts.len();
      // A read of now takes the acts in place as the order they began, so an act first told begins
      // after each act held.
      debug_assert!(
        self.acts.last().is_none_or(|last| last.began <= act.began),
        "{} began before {}, which was told first",
        act.id,
        self.acts.last().map_or("", |last| last.id.as_str()),
      );
      self.at.insert(act.id.clone(), at);
      self.made.entry(act.by.clone()).or_default().push(at);
      self.on.entry(act.chain.clone()).or_default().push(at);
      self.acts.push(act.clone());
      at
    };
    if lives(act) {
      self.live.insert(at);
    } else {
      self.live.remove(&at);
    }
  }

  /// The ladder of a chain, now or as it stood just after a moment, with the folds that the window
  /// opened, the prompts that the operator owes, and the only acts at its top when `taken` names
  /// them, as [`ladder`] folds it.
  pub fn ladder(
    &self,
    chain: &str,
    at: Option<u64>,
    opened: &[String],
    owed: &[String],
    taken: Option<&[String]>,
    shown: Shown,
  ) -> Vec<Rung> {
    let Some(at) = at else {
      let index =
        Index { acts: &self.acts, at: &self.at, made: &self.made, on: &self.on, live: &self.live };
      return folded(&index, chain, opened, owed, taken, shown);
    };
    // A moment of the past folds the acts as they stood then, in the order they began.
    let mut then = Self::default();
    for act in self.then(Some(at)).iter() {
      then.heard(act);
    }
    then.ladder(chain, None, opened, owed, taken, shown)
  }

  /// The acts now, or as they stood just after a moment, in the order they began. The acts now
  /// stand in the order they were first told, which is the order they began, so they are read in
  /// place: a read of the threads, the rail or the desk takes no copy of the life.
  fn then(&self, at: Option<u64>) -> Cow<'_, [Act]> {
    let Some(at) = at else { return Cow::Borrowed(&self.acts) };
    let mut acts = as_of(&self.acts, at);
    acts.sort_by_key(|act| act.began);
    Cow::Owned(acts)
  }

  /// What waits for the operator, with each notice that the window still shows.
  pub fn desk(&self, notices: &[String]) -> Desk {
    let notices: Vec<&str> = notices.iter().map(String::as_str).collect();
    desk(&self.then(None), &notices)
  }

  /// The threads of a chain, with the workers of its rounds nested under the thread that started
  /// each, now or as they stood just after a moment. `read` is the place up to which the window
  /// read the chain, from which its runs of notes count the notes not yet read.
  pub fn threads(&self, chain: &str, at: Option<u64>, read: u64) -> Threads {
    let acts = self.then(at);
    let rows: Rows<'_> = acts.iter().map(|act| (act.id.as_str(), act)).collect();
    let all: Vec<&Act> = acts.iter().collect();
    let on: Vec<&Act> = acts.iter().filter(|act| act.chain == chain && act.id != chain).collect();
    let threads = chain_tree(chain, &all, &rows);
    let state =
      rows.get(chain).filter(|act| act.kind == Kind::Chain).map_or(State::Idle, |act| act.state);
    let (asides, notes) = (chain_asides(&on, &rows), chain_notes(&on, &rows, read));
    Threads { attention: attention(&threads), asides, notes, threads, state }
  }

  /// The threads of every chain of the life but those that `hidden` names, as one tree in the
  /// order of the list of the threads: each thread under the thread whose ladder made its prompt,
  /// the closed ones first and the open ones last, now or as they stood just after a moment.
  pub fn rail(&self, hidden: &[String], at: Option<u64>) -> Vec<ThreadNode> {
    let acts = self.then(at);
    let rows: Rows<'_> = acts.iter().map(|act| (act.id.as_str(), act)).collect();
    let mut chains: Vec<&str> = Vec::new();
    let mut on: HashMap<&str, Vec<&Act>> = HashMap::new();
    for act in acts.iter().filter(|act| act.id != act.chain) {
      let chain = act.chain.as_str();
      if hidden.iter().any(|one| one == chain) {
        continue;
      }
      on.entry(chain).or_insert_with(|| {
        chains.push(chain);
        Vec::new()
      });
      on.entry(chain).or_default().push(act);
    }
    let mut list: Vec<_> =
      chains.iter().flat_map(|chain| chain_threads(&on[chain], &rows)).collect();
    list.sort_by_key(|one| rows.get(one.prompt.as_str()).map_or(0, |act| act.began));
    order_threads(nest_threads(list, &rows), &rows)
  }

  /// The acts as they stood just after a moment.
  pub fn as_of(&self, at: u64) -> Vec<Act> {
    self.then(Some(at)).into_owned()
  }

  /// Each moment at which an act began or ended, of the life or of the acts of one chain.
  pub fn moments(&self, on: Option<&str>) -> Vec<u64> {
    match on {
      Some(chain) => {
        let acts: Vec<Act> = self.acts.iter().filter(|act| act.chain == chain).cloned().collect();
        moments_of(&acts)
      }
      None => moments_of(&self.acts),
    }
  }
}
