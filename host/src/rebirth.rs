//! The rebirth of a life: its actor writes its checkpoint, drops its engine and opens it again
//! from that checkpoint on the same thread, for the same readers. A life is born again when it
//! said [`TAIL`] facts since it opened while no command and no reply runs, so an open after a
//! crash replays a short tail, and the engine lets go of what its words left. A rebirth cuts
//! each word that runs, which the new open ends as interrupted, and each chain goes on as it
//! stood.
//!
//! A fault that stops the sandbox cuts the life with no checkpoint, and ends it: the record keeps
//! each fact up to the fault, and a later open replays them.

use std::collections::HashSet;

use furb::{Engine, Fault};

use crate::{
  event::Event,
  view::{Kind, Phase},
};

/// How many facts a life says after it opened before it writes its checkpoint and is born again,
/// at the first moment when no command and no reply runs, which a rebirth would cut.
pub(crate) const TAIL: u64 = 10_000;

/// What the actor watches of a life since it opened: the facts that it said, the commands and the
/// replies that run.
#[derive(Default)]
pub(crate) struct Watch {
  facts: u64,
  running: HashSet<String>,
}

impl Watch {
  /// One event that the life told.
  pub(crate) fn heard(&mut self, event: &Event) {
    match event {
      Event::Fact(_) => self.facts += 1,
      Event::Act(act) if matches!(act.kind, Kind::Bash | Kind::Reply) => {
        match act.phase {
          Phase::Done(_) => self.running.remove(&act.id),
          Phase::Asked | Phase::Started => self.running.insert(act.id.clone()),
        };
      }
      _ => {}
    }
  }

  /// Whether the life is born again now: it said [`TAIL`] facts since it opened and no command and
  /// no reply runs.
  pub(crate) fn due(&self) -> bool {
    self.facts >= TAIL && self.running.is_empty()
  }

  /// The life opened again, with no fact said and nothing that runs.
  pub(crate) fn reborn(&mut self) {
    self.facts = 0;
    self.running.clear();
  }
}

/// The engine of a life dropped, with its checkpoint when the life keeps its record, and the fault
/// of that checkpoint when it fails.
pub(crate) fn ended(engine: Engine, keeps: bool) -> Option<Fault> {
  keeps.then(|| engine.checkpoint()).and_then(Result::err)
}
