//! The rebirth of a life: its actor writes its checkpoint, drops its engine and opens it again
//! from that checkpoint on the same thread, for the same readers. A life is born again when it
//! said [`TAIL`] facts since it opened while it holds no act that a checkpoint does not keep, so an
//! open after a crash replays a short tail, and the engine lets go of what its words left. A
//! checkpoint keeps each chain, each rung and each open prompt, and the seed makes again each act
//! of a life word of the standing, under a rung whose word is that life word; each other act that
//! lives, as a command, a reply, a word that runs or an act with an ear of its own, holds the
//! rebirth until it ends, however long that takes, so a rebirth never cuts work. A life whose
//! record does not hold each fact that it said, as on a full disk, holds its rebirth until the
//! record takes them, since the drop of its engine would lose them.
//!
//! A fault that stops the sandbox cuts the life with no checkpoint, and ends it: the record keeps
//! each fact up to the fault, and a later open replays them.

use std::collections::HashSet;

use furb::{
  Engine, Fault,
  view::{Kind, Phase},
};

use serde_json::Value;

use crate::event::Event;

/// How many facts a life says after it opened before it writes its checkpoint and is born again,
/// at the first moment when it holds no act that a checkpoint does not keep.
pub(crate) const TAIL: u64 = 10_000;

/// What the actor watches of a life since it opened: the facts that it said, each act that lives
/// and that a checkpoint does not keep, the rungs that say a life word of the last standing, whose
/// acts the seed makes again, and those life words. The rungs go with the birth: the next birth
/// makes its own.
#[derive(Default)]
pub(crate) struct Watch {
  facts: u64,
  held: HashSet<String>,
  seeds: HashSet<String>,
  lives: HashSet<String>,
}

impl Watch {
  /// One event that the life told.
  pub(crate) fn heard(&mut self, event: &Event) {
    match event {
      Event::Fact(fact) => {
        self.facts += 1;
        let stood =
          fact[0] == "done" && fact[1].as_str().is_some_and(|one| one.starts_with("stand"));
        if let Some(extensions) = fact[3].get(4).and_then(Value::as_array).filter(|_| stood) {
          let words = extensions.iter().filter_map(|one| one.get(2)?.as_str());
          self.lives = words.filter(|one| !one.is_empty()).map(str::to_owned).collect();
        }
      }
      Event::Act(act) if matches!(act.phase, Phase::Done(_)) => {
        self.held.remove(&act.id);
      }
      Event::Act(act) if act.kind == Kind::Rung => {
        let word = act.words.first().and_then(Value::as_str).unwrap_or_default();
        if self.lives.contains(word) {
          self.seeds.insert(act.id.clone());
        }
      }
      Event::Act(act)
        if !matches!(act.kind, Kind::Chain | Kind::Prompt) && !self.seeds.contains(&act.by) =>
      {
        self.held.insert(act.id.clone());
      }
      _ => {}
    }
  }

  /// Whether the life is born again now: it said [`TAIL`] facts since it opened, and it holds no
  /// act that a checkpoint does not keep.
  pub(crate) fn due(&self) -> bool {
    self.facts >= TAIL && self.held.is_empty()
  }

  /// The life opened again, with no fact said, no act held and no rung of a life word: the seed of
  /// the new birth says them again.
  pub(crate) fn reborn(&mut self) {
    self.facts = 0;
    self.held.clear();
    self.seeds.clear();
  }
}

/// The engine of a life dropped, with its checkpoint when the life keeps its record, and the fault
/// of that checkpoint when it fails.
pub(crate) fn ended(engine: Engine, keeps: bool) -> Option<Fault> {
  keeps.then(|| engine.checkpoint()).and_then(Result::err)
}

#[cfg(test)]
#[path = "rebirth.test.rs"]
mod tests;
