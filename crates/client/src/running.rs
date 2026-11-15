//! The sessions that run: each life that a machine holds, with whether its work runs and who holds
//! it, and what the operator does to one: a pause, which ends the life and keeps its open work for
//! the next open, a kill, which cancels that work first, or a pin, which keeps it running in the
//! background once no window holds it. The same three come when he leaves while a session that
//! the client holds works.

use furb_wire::protocol::{Card, Verb};

use crate::{Asked, Client};

/// The holder of the pin that keeps a session running in the background.
pub const BACKGROUND: &str = "background";

/// A session that a machine holds: its machine, its record, its name, its project, whether a
/// thread of it works, whether this client holds it, the operators whose window holds it, and
/// whether a pin keeps it in the background.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Running {
  pub at: usize,
  pub life: String,
  pub name: String,
  pub project: Option<String>,
  pub works: bool,
  pub here: bool,
  pub viewers: Vec<String>,
  pub background: bool,
}

/// What the operator does to a session that runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
  /// The life ends, and its open work waits for the next open of the session.
  Pause,
  /// The open work of the life is cancelled, and the life ends.
  Kill,
  /// A pin keeps the life running with no window.
  Background,
}

impl Client {
  /// Each session that a machine holds, by machine, the ones whose work runs first.
  pub fn running(&self) -> Vec<Running> {
    let mut all: Vec<Running> = Vec::new();
    for (at, told) in self.machines.iter().enumerate() {
      for card in told.cards.iter().filter(|card| card.held) {
        match all.iter_mut().find(|one| one.at == at && one.life == card.life) {
          Some(one) => one.works |= card.thread.works(),
          None => all.push(self.runs(at, card)),
        }
      }
    }
    all.sort_by_key(|one| (one.at, !one.works));
    all
  }

  fn runs(&self, at: usize, card: &Card) -> Running {
    let named = card.session.clone().filter(|name| !name.is_empty());
    Running {
      at,
      life: card.life.clone(),
      name: named.unwrap_or_else(|| card.thread.title.clone()),
      project: card.project.clone(),
      works: card.thread.works(),
      here: self.holds.keys().any(|key| key.0 == at && key.1 == card.life),
      viewers: card.viewers.clone(),
      background: card.pins.iter().any(|one| one == BACKGROUND),
    }
  }

  /// The sessions that this client holds whose work runs, which a leave of the client asks about:
  /// the thread that the client started works too while its machine did not tell its card yet.
  pub fn leaving(&self) -> Vec<Running> {
    let mut all: Vec<Running> =
      self.running().into_iter().filter(|one| one.here && one.works && !one.background).collect();
    let started = self.thread.as_ref().filter(|one| one.thread.started).map(|one| &one.thread.key);
    if let Some((at, life, _)) = started
      && !all.iter().any(|one| one.at == *at && one.life == *life)
    {
      all.push(Running {
        at: *at,
        life: life.clone(),
        name: "The thread that you started".to_owned(),
        project: self.cwd(*at, life),
        works: true,
        here: true,
        viewers: Vec::new(),
        background: false,
      });
    }
    all
  }

  /// What the operator does to a session, which goes to its machine.
  pub fn control(&mut self, at: usize, life: &str, control: Control) {
    let life = life.to_owned();
    let verb = match control {
      Control::Pause => Verb::End { life, cancel: false },
      Control::Kill => Verb::End { life, cancel: true },
      Control::Background => Verb::Pin { life, holder: BACKGROUND.to_owned() },
    };
    self.controls.push((at, verb));
  }

  /// The verbs of what the operator did to sessions, each with its machine, once each.
  pub(crate) fn controls_due(&mut self) -> Vec<(usize, Verb, Asked)> {
    self.controls.drain(..).map(|(at, verb)| (at, verb, Asked::Running)).collect()
  }

  /// Whether a verb of what the operator did to a session still waits for its answer.
  pub fn controlling(&self) -> bool {
    !self.controls.is_empty() || self.asked.values().any(|one| *one == Asked::Running)
  }
}
