//! The sessions that run: each life that a machine holds, with whether its work runs and who holds
//! it, and what the operator does to one: a pause, which ends the life and keeps its open work for
//! the next open, a kill, which cancels that work first, or a pin, which keeps it running in the
//! background once no window holds it. The same three come when he leaves while a session that
//! the client holds lives: a thread of it works or asks him.
#![expect(clippy::indexing_slicing, reason = "debt: an index or a slice that can panic")]

use furb_wire::{
  card::Standing,
  protocol::{Card, Verb},
};

use crate::{Asked, Client};

/// The holder of the pin that keeps a session running in the background.
pub const BACKGROUND: &str = "background";

/// A session that a machine holds: its machine, its record, its name, its project, each thread
/// of it that lives, whether this client holds it, the operators whose window holds it, and
/// whether a pin keeps it in the background.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Running {
  pub at: usize,
  pub life: String,
  pub name: String,
  pub project: Option<String>,
  pub live: Vec<Live>,
  pub here: bool,
  pub viewers: Vec<String>,
  pub background: bool,
}

/// A thread of a session that lives while the life holds it: its title, and how it stands, as it
/// works or asks the operator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Live {
  pub title: String,
  pub standing: Standing,
}

impl Running {
  /// Whether a thread of the session works.
  pub fn works(&self) -> bool {
    self.live.iter().any(|one| one.standing.works())
  }
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
  /// The life ends, and its record and its files move to the trash of its project.
  Delete,
}

impl Client {
  /// Each session that a machine holds, by machine, the ones whose work runs first.
  pub fn running(&self) -> Vec<Running> {
    let mut all: Vec<Running> = Vec::new();
    for (at, told) in self.machines.iter().enumerate() {
      for card in told.cards.iter().filter(|card| card.held) {
        let place = all.iter().position(|one| one.at == at && one.life == card.life);
        let place = place.unwrap_or_else(|| {
          all.push(self.runs(at, card));
          all.len() - 1
        });
        let standing = &card.thread.standing;
        if standing.works() || matches!(standing, Standing::Asks { .. }) {
          let title = card.thread.title.clone();
          all[place].live.push(Live { title, standing: standing.clone() });
        }
      }
    }
    all.sort_by_key(|one| (one.at, !one.works()));
    all
  }

  fn runs(&self, at: usize, card: &Card) -> Running {
    let named = card.session.clone().filter(|name| !name.is_empty());
    Running {
      at,
      life: card.life.clone(),
      name: named.unwrap_or_else(|| card.thread.title.clone()),
      project: card.project.clone(),
      live: Vec::new(),
      here: self.holds.keys().any(|key| key.0 == at && key.1 == card.life),
      viewers: card.viewers.clone(),
      background: card.pins.iter().any(|one| one == BACKGROUND),
    }
  }

  /// The sessions that this client holds with a thread that lives, which a leave of the client
  /// asks about, while its window is their last view: another window that holds one keeps it as
  /// it is, and a login that only reads does nothing to it. The thread that the client started
  /// works too while its machine did not tell its card yet.
  pub fn leaving(&self) -> Vec<Running> {
    let last = |at: usize, life: &str| {
      let present = self.machines.get(at).and_then(|told| told.presence.get(life));
      present.is_none_or(|all| all.iter().map(|one| one.windows).sum::<usize>() <= 1)
    };
    let held = |one: &Running| {
      let controls = self.only_reads(one.at).is_none();
      one.here && !one.live.is_empty() && !one.background && controls && last(one.at, &one.life)
    };
    let mut all: Vec<Running> = self.running().into_iter().filter(held).collect();
    let started = self.thread.as_ref().filter(|one| one.thread.started).map(|one| &one.thread.key);
    if let Some((at, life, _)) = started
      && last(*at, life)
      && !all.iter().any(|one| one.at == *at && one.life == *life)
    {
      let title = "The thread that you started".to_owned();
      all.push(Running {
        at: *at,
        life: life.clone(),
        name: title.clone(),
        project: self.project(*at, life),
        live: vec![Live { title, standing: Standing::Thinks }],
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
      Control::Delete => Verb::Delete { life },
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
