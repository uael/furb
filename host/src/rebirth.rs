//! The rebirth of a life: its actor writes its checkpoint, drops its engine and opens it again
//! from that checkpoint on the same thread, for the same readers. A life is born again when it
//! said [`TAIL`] facts since it opened while it holds no act that a checkpoint does not keep, so an
//! open after a crash replays a short tail, and the engine lets go of what its words left. A
//! checkpoint keeps each chain, each rung and each open prompt; each other act that lives, as a
//! command, a reply, a word that runs or an act with an ear of its own, holds the rebirth until it
//! ends, however long that takes, so a rebirth never cuts work. A life whose record does not hold
//! each fact that it said, as on a full disk, holds its rebirth until the record takes them, since
//! the drop of its engine would lose them.
//!
//! A fault that stops the sandbox cuts the life with no checkpoint, and ends it: the record keeps
//! each fact up to the fault, and a later open replays them.

use std::collections::HashSet;

use furb::{
  Engine, Fault,
  view::{Kind, Phase},
};

use crate::event::Event;

/// How many facts a life says after it opened before it writes its checkpoint and is born again,
/// at the first moment when it holds no act that a checkpoint does not keep.
pub(crate) const TAIL: u64 = 10_000;

/// What the actor watches of a life since it opened: the facts that it said, and each act that
/// lives and that a checkpoint does not keep.
#[derive(Default)]
pub(crate) struct Watch {
  facts: u64,
  held: HashSet<String>,
}

impl Watch {
  /// One event that the life told.
  pub(crate) fn heard(&mut self, event: &Event) {
    match event {
      Event::Fact(_) => self.facts += 1,
      Event::Act(act) if matches!(act.phase, Phase::Done(_)) => {
        self.held.remove(&act.id);
      }
      Event::Act(act) if !matches!(act.kind, Kind::Chain | Kind::Rung | Kind::Prompt) => {
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

  /// The life opened again, with no fact said and no act held.
  pub(crate) fn reborn(&mut self) {
    self.facts = 0;
    self.held.clear();
  }
}

/// The engine of a life dropped, with its checkpoint when the life keeps its record, and the fault
/// of that checkpoint when it fails.
pub(crate) fn ended(engine: Engine, keeps: bool) -> Option<Fault> {
  keeps.then(|| engine.checkpoint()).and_then(Result::err)
}
