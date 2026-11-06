//! The rebirth of a life: its actor writes its checkpoint, drops its engine and opens it again
//! from that checkpoint on the same thread, for the same readers. A life is born again when it
//! said [`TAIL`] facts since it opened while no command, no reply and no word runs, so an open
//! after a crash replays a short tail, and the engine lets go of what its words left. A word that
//! runs, as one that awaits a wait or a prompt, holds the rebirth until [`BOUND`] facts, and the
//! rebirth then cuts it, which the new open ends as interrupted. The rebirth also cuts each act
//! whose ear no checkpoint keeps. The new open tells the root each act that it cut, by name, in a
//! prompt of None, so the one who runs the root starts again the work that it still needs, and
//! each chain goes on as it stood.
//!
//! A fault that stops the sandbox cuts the life with no checkpoint, and ends it: the record keeps
//! each fact up to the fault, and a later open replays them.

use std::collections::{BTreeMap, HashSet};

use furb::{Engine, Fault, verbs, world::Shape};

use crate::{
  event::Event,
  view::{Act, Kind, Phase},
};

/// How many facts a life says after it opened before it writes its checkpoint and is born again,
/// at the first moment when no command, no reply and no word runs, which a rebirth would cut.
pub(crate) const TAIL: u64 = 10_000;

/// How many facts a life says after it opened before a word that runs no longer holds its rebirth,
/// which then cuts that word.
pub(crate) const BOUND: u64 = 2 * TAIL;

/// The kinds of the acts that a rebirth never names as cut: the chains and the rungs, which the
/// checkpoint carries as chains and ladders, and the acts that a word or a command makes for
/// itself, which go with it.
const CARRIED: [&str; 6] = ["chain", "rung", "wants", "merged", "gate", "stand"];

/// What the actor watches of a life since it opened: the facts that it said, the commands and the
/// replies that run, the words that run, and each act that lives, which a rebirth may cut.
#[derive(Default)]
pub(crate) struct Watch {
  facts: u64,
  running: HashSet<String>,
  words: HashSet<String>,
  open: BTreeMap<String, Act>,
  before: BTreeMap<String, Act>,
}

impl Watch {
  /// One event that the life told.
  pub(crate) fn heard(&mut self, event: &Event) {
    match event {
      Event::Fact(_) => self.facts += 1,
      Event::Act(act) => {
        let done = matches!(act.phase, Phase::Done(_));
        let held = match act.kind {
          Kind::Bash | Kind::Reply => Some(&mut self.running),
          Kind::Run => Some(&mut self.words),
          _ => None,
        };
        if let Some(held) = held {
          if done {
            held.remove(&act.id)
          } else {
            held.insert(act.id.clone())
          };
        }
        if done || CARRIED.contains(&act.kind.as_str()) {
          self.open.remove(&act.id);
        } else {
          self.open.insert(act.id.clone(), (**act).clone());
        }
      }
      _ => {}
    }
  }

  /// Whether the life is born again now: it said [`TAIL`] facts since it opened, no command and no
  /// reply runs, and no word runs, or it said [`BOUND`] facts.
  pub(crate) fn due(&self) -> bool {
    self.facts >= TAIL && self.running.is_empty() && (self.words.is_empty() || self.facts >= BOUND)
  }

  /// The life opened again, with no fact said and nothing that runs: each act that lived before it
  /// waits for [`Watch::cut`].
  pub(crate) fn reborn(&mut self) {
    self.facts = 0;
    self.running.clear();
    self.words.clear();
    self.before = std::mem::take(&mut self.open);
  }

  /// Each act that lived before the rebirth and that the new open does not hold open, in order, but
  /// an act that another of them made, which goes with its maker.
  fn cut(&mut self, mut lives: impl FnMut(&str) -> bool) -> Vec<Act> {
    let before = std::mem::take(&mut self.before);
    let gone: Vec<Act> = before.into_values().filter(|one| !lives(&one.id)).collect();
    let makers: HashSet<&str> = gone
      .iter()
      .flat_map(|one| {
        let rung =
          one.words.first().and_then(|word| word.as_str()).filter(|_| one.kind == Kind::Run);
        [Some(one.id.as_str()), rung]
      })
      .flatten()
      .collect();
    gone.iter().filter(|one| !makers.contains(one.by.as_str())).cloned().collect()
  }
}

/// The engine of a life dropped, with its checkpoint when the life keeps its record, and the fault
/// of that checkpoint when it fails.
pub(crate) fn ended(engine: Engine, keeps: bool) -> Option<Fault> {
  keeps.then(|| engine.checkpoint()).and_then(Result::err)
}

/// The prompt of None on the root that names each act that the rebirth cut, when it cut one, or the
/// fault of the engine that refused it.
pub(crate) fn told(engine: &mut Engine, watch: &mut Watch) -> Result<(), Fault> {
  let cut =
    watch.cut(|id| matches!(engine.get(id), Ok(Some(_))) && matches!(engine.outcome(id), Ok(None)));
  if cut.is_empty() {
    return Ok(());
  }
  let line = |one: &Act| match (&one.kind, one.words.first().and_then(|word| word.as_str())) {
    (Kind::Run, Some(rung)) => format!("{} on {} (the word of {rung})", one.id, one.chain),
    _ => format!("{} on {}", one.id, one.chain),
  };
  let lines: Vec<String> = cut.iter().map(line).collect();
  let message = format!(
    "The life was born again from its checkpoint, and it cut each of these acts, which runs no more: {}. Start again in a turn that shows each one that the work still needs.",
    lines.join(", ")
  );
  let root = engine.root().to_owned();
  let with = verbs::Prompt { message: Some(message), to: None, on: Some(root) };
  engine.prompt(Shape::object("None"), with).map(|_| ())
}
