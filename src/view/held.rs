//! The acts of a life as a window last heard them, which the life keeps as it tells them, so a
//! window asks the life for the meaning that it draws: the desk, the threads of a chain, the acts
//! as they stood at a moment, and the moments.

#![expect(
  clippy::too_many_arguments,
  reason = "debt: a function of more than 4 parameters takes a struct, rule 8 of the skill rust"
)]
use std::{
  collections::{BTreeSet, HashMap},
  mem,
  sync::Arc,
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::engine::OPERATOR;

use super::{
  act::{Act, Kind},
  desk::{Desk, desk},
  ladder::{Index, Rung, Shown, folded, lives},
  state::State,
  threads::{
    Attention, ChainAside, Lookup, NoteRun, Rows, ThreadNode, attention, chain_asides, chain_notes,
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

/// Each act of a life, as the last event of it told it, in the order the acts were first told. An
/// act is shared, so a host that keeps the event of an act beside it holds the act once.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Held {
  acts: Vec<Arc<Act>>,
  at: HashMap<String, usize>,
  /// The places of the acts of each maker, and of each chain, in the order they were first told.
  made: HashMap<String, Vec<usize>>,
  on: HashMap<String, Vec<usize>>,
  /// The places of the acts that live: not done, or a rung whose word runs.
  live: BTreeSet<usize>,
  /// Whether an act was told that began before the last one held, so the order of the acts is not
  /// the order they began.
  unordered: bool,
  /// The place of each act or chain at which each operator last said that he saw it, by the site
  /// that said it: `operator` alone, or `operator/<login>` for the window of a member. A seen that
  /// a later life says again as the journal names no site, so it stands for each of them.
  seen: HashMap<String, HashMap<String, u64>>,
}

impl<'a> Lookup<'a> for &'a Held {
  fn act(&self, id: &str) -> Option<&'a Act> {
    self.at.get(id).map(|at| &*self.acts[*at])
  }
}

impl Held {
  /// A fact of the life at its place: what the operator says that he saw, which marks the threads
  /// that he started and that ended after it.
  pub fn noted(&mut self, fact: &serde_json::Value, at: u64) {
    let text = |n: usize| fact.get(n).and_then(serde_json::Value::as_str).unwrap_or_default();
    if text(0) == "seen" && (super::act::operator(text(2)) || text(2) == "journal") {
      self.seen.entry(text(2).to_owned()).or_default().insert(text(1).to_owned(), at);
    }
  }

  /// An act of the life heard as a host tells it: it takes the place of the earlier one of the same
  /// id.
  pub fn heard(&mut self, act: &Act) {
    self.took(act.clone());
  }

  /// An act of the life heard as a host tells it, which the life gives up or shares with it: the
  /// fold of a record tells some hundred thousand of them, each with its words and its value, so it
  /// takes no copy of each. It gives back the act of the same id that it held before.
  pub fn took(&mut self, act: impl Into<Arc<Act>>) -> Option<Arc<Act>> {
    let act = act.into();
    let live = lives(&act);
    let (at, before) = if let Some(&at) = self.at.get(&act.id) {
      (at, Some(mem::replace(&mut self.acts[at], act)))
    } else {
      let at = self.acts.len();
      // A read of now takes the acts in place as the order they began, which a host and a record
      // tell them in. A window that makes acts of its own may tell one that began before the last,
      // and then the reads sort a copy.
      if self.acts.last().is_some_and(|last| last.began > act.began) {
        self.unordered = true;
      }
      self.at.insert(act.id.clone(), at);
      self.made.entry(act.by.clone()).or_default().push(at);
      self.on.entry(act.chain.clone()).or_default().push(at);
      self.acts.push(act);
      (at, None)
    };
    if live {
      self.live.insert(at);
    } else {
      self.live.remove(&at);
    }
    before
  }

  /// The act of an id as the last event of it told it.
  pub fn shared(&self, id: &str) -> Option<&Arc<Act>> {
    self.at.get(id).map(|at| &self.acts[*at])
  }

  /// The ladder of a chain, now or as it stood just after a moment, with the folds that the window
  /// opened, the prompts that the operator owes, and the only acts at its top when `taken` names
  /// them.
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
    for act in self.as_of(at) {
      then.took(act);
    }
    then.ladder(chain, None, opened, owed, taken, shown)
  }

  /// The acts now, or as they stood just after a moment, in the order they began, as `read` takes
  /// them. The acts now are read in place, and the order of a life that was told an act out of
  /// order is a sort of references: a read of the threads, the rail, the desk or the cards takes
  /// no copy of the life, which holds some hundred thousand acts.
  pub(super) fn then<T>(&self, at: Option<u64>, read: impl FnOnce(&[&Act]) -> T) -> T {
    let past;
    let mut acts: Vec<&Act> = match at {
      None => self.acts.iter().map(|act| &**act).collect(),
      Some(at) => {
        past = as_of(self.acts.iter().map(|act| &**act), at);
        past.iter().collect()
      }
    };
    if self.unordered || at.is_some() {
      acts.sort_by_key(|act| act.began);
    }
    read(&acts)
  }

  /// What waits for the operator, with each notice that the window still shows.
  pub fn desk(&self, notices: &[String]) -> Desk {
    let notices: Vec<&str> = notices.iter().map(String::as_str).collect();
    self.then(None, |acts| desk(acts.iter().copied(), &notices))
  }

  /// The threads of a chain, with the workers of its rounds nested under the thread that started
  /// each, now or as they stood just after a moment. `read` is the place up to which the window
  /// read the chain, from which its runs of notes count the notes not yet read.
  pub fn threads(&self, chain: &str, at: Option<u64>, read: u64) -> Threads {
    self.threads_as(OPERATOR, chain, at, read)
  }

  /// The acts that an act or a person made, in the order they were first told.
  pub(super) fn made_by<'a>(&'a self, id: &str) -> impl Iterator<Item = &'a Act> + use<'a> {
    self.made.get(id).into_iter().flatten().map(|at| &*self.acts[*at])
  }

  /// The acts of a chain, in the order they were first told.
  pub(super) fn on_chain<'a>(&'a self, chain: &str) -> impl Iterator<Item = &'a Act> + use<'a> {
    self.on.get(chain).into_iter().flatten().map(|at| &*self.acts[*at])
  }

  /// Each site that said what it saw, and the operator with no login, who saw what the journal
  /// says again.
  pub(super) fn sites(&self) -> impl Iterator<Item = &str> {
    let said =
      self.seen.keys().map(String::as_str).filter(|site| !matches!(*site, "journal" | OPERATOR));
    [OPERATOR].into_iter().chain(said)
  }

  /// What one site saw: its own seen and those of the journal, the later for each act, so a seen
  /// of another site clears nothing of its marks.
  pub(super) fn seen_by(&self, site: &str) -> HashMap<String, u64> {
    let mut seen = self.seen.get("journal").cloned().unwrap_or_default();
    for (id, at) in self.seen.get(site).into_iter().flatten() {
      let place = seen.entry(id.clone()).or_default();
      *place = (*place).max(*at);
    }
    seen
  }

  /// The threads of a chain as the window of a site reads them: the mark of what it has not seen
  /// counts the seen of that site alone.
  pub fn threads_as(&self, site: &str, chain: &str, at: Option<u64>, read: u64) -> Threads {
    self.then(at, |acts| {
      let rows: Rows<'_> = acts.iter().map(|act| (act.id.as_str(), *act)).collect();
      let on: Vec<&Act> =
        acts.iter().copied().filter(|act| act.chain == chain && act.id != chain).collect();
      let threads = chain_tree(chain, acts, &rows);
      let state =
        rows.get(chain).filter(|act| act.kind == Kind::Chain).map_or(State::Idle, |act| act.state);
      let (asides, notes) = (chain_asides(&on, &rows), chain_notes(&on, &rows, read));
      Threads {
        attention: attention(&threads, &rows, &self.seen_by(site)),
        asides,
        notes,
        threads,
        state,
      }
    })
  }

  /// The threads of every chain of the life but those that `hidden` names, as one tree in the
  /// order of the list of the threads: each thread under the thread whose ladder made its prompt,
  /// the closed ones first and the open ones last, now or as they stood just after a moment.
  pub fn rail(&self, hidden: &[String], at: Option<u64>) -> Vec<ThreadNode> {
    self.then(at, |acts| {
      let rows: Rows<'_> = acts.iter().map(|act| (act.id.as_str(), *act)).collect();
      let mut chains: Vec<&str> = Vec::new();
      let mut on: HashMap<&str, Vec<&Act>> = HashMap::new();
      for act in acts.iter().copied().filter(|act| act.id != act.chain) {
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
    })
  }

  /// The acts as they stood just after a moment, in the order they began.
  pub fn as_of(&self, at: u64) -> Vec<Act> {
    let mut acts = as_of(self.acts.iter().map(|act| &**act), at);
    acts.sort_by_key(|act| act.began);
    acts
  }

  /// Each moment at which an act began or ended, of the life or of the acts of one chain.
  pub fn moments(&self, on: Option<&str>) -> Vec<u64> {
    moments_of(
      self.acts.iter().map(|act| &**act).filter(|act| on.is_none_or(|chain| act.chain == chain)),
    )
  }
}
