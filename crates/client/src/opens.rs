//! The lives that a client holds, and how far the open of each stands.
//!
//! A client holds the life of each thread that it reads and of each session that it writes in: it
//! asks the daemon to open the life, and attaches it in the same step. An open that runs tells
//! each of its stages. A life that stood when the client asked it opens with nothing to show.

use std::time::Duration;

use furb_wire::Stage;

use crate::Client;

/// A life of the client: its machine and its record.
pub type Life = (usize, String);

/// Where the open of a life that the client holds stands with the daemon: to be asked, asked
/// under the id of its request, answered, so each verb of the life may go, or ended by a fault,
/// with why, so only its views answer: the client reads its threads and sends it nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ask {
  Due,
  Sent(u64),
  Answered,
  Failed(String),
}

/// A life that the client holds: the folder that it opens in when no life holds its record, the
/// actor that a new life takes, where its open stands with the daemon, when the client asked it,
/// whether its life stood then, and each stage that its open reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hold {
  pub cwd: Option<String>,
  pub actor: Option<String>,
  pub ask: Ask,
  pub since: Duration,
  pub stood: bool,
  pub steps: Vec<Step>,
}

/// A stage that an open reached: its work done and its work known, in the unit of the stage, and
/// when it started, as the client heard it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Step {
  pub stage: Stage,
  pub done: u64,
  pub of: u64,
  pub at: Duration,
}

impl Hold {
  /// When the open ended: the moment of its last stage once that stage is whole.
  pub fn ended(&self) -> Option<Duration> {
    let last = self.steps.last()?;
    (last.stage == Stage::Log && last.done == last.of).then_some(last.at)
  }

  /// Whether the open of the life runs: a life that a fault ended opens no more.
  pub fn runs(&self) -> bool {
    !self.stood && self.ended().is_none() && self.failed().is_none()
  }

  /// Why a fault ended the life, which then only reads.
  pub fn failed(&self) -> Option<&str> {
    match &self.ask {
      Ask::Failed(why) => Some(why),
      _ => None,
    }
  }

  /// The share of the stage that runs, while it counts work that is not done: a stage whose work
  /// is done waits for the next one, and has no share to show.
  pub fn share(&self) -> Option<u64> {
    self.steps.last().filter(|one| one.done < one.of).map(|one| one.done * 100 / one.of)
  }

  /// A stage that the open reached, or the next numbers of the stage that runs. A stage that comes
  /// before the last one starts a new open, as at a rebirth of the life.
  fn heard(&mut self, mut step: Step) {
    let again =
      self.ended().is_some() || self.steps.last().is_some_and(|last| step.stage < last.stage);
    if again {
      (self.steps, self.since, self.stood) = (Vec::new(), step.at, false);
    }
    // The first stage starts with the open, whatever the time that its frame took to come.
    if self.steps.is_empty() {
      step.at = self.since;
    }
    match self.steps.last_mut() {
      Some(last) if last.stage == step.stage => (last.done, last.of) = (step.done, step.of),
      _ => self.steps.push(step),
    }
  }
}

impl Client {
  /// The life of a record held by the client from now on: the client asks to open it, in a folder
  /// when no life holds the record. A life that the client holds stays as it is.
  pub fn hold(&mut self, key: Life, cwd: Option<String>, since: Duration) {
    if self.holds.contains_key(&key) {
      return;
    }
    let cards = self.machines.get(key.0).map(|told| &told.cards[..]).unwrap_or_default();
    let stood = cards.iter().any(|card| card.life == key.1 && card.held);
    let hold = Hold { cwd, actor: None, ask: Ask::Due, since, stood, steps: Vec::new() };
    self.holds.insert(key, hold);
  }

  /// The open of a life asked of its machine under the id of its request. A life of a machine that
  /// the client does not dial stays as it is.
  pub fn asked_open(&mut self, life: &Life, id: Option<u64>) {
    if let (Some(hold), Some(id)) = (self.holds.get_mut(life), id) {
      hold.ask = Ask::Sent(id);
    }
  }

  /// Whether the daemon answered the open of a life, so each verb of the life may go.
  pub fn stands(&self, at: usize, life: &str) -> bool {
    self.holds.get(&(at, life.to_owned())).is_some_and(|hold| hold.ask == Ask::Answered)
  }

  /// Whether the views of a life answer: it stands, or a fault ended it and it still shows.
  pub fn reads(&self, at: usize, life: &str) -> bool {
    let hold = self.holds.get(&(at, life.to_owned()));
    hold.is_some_and(|hold| hold.ask == Ask::Answered || hold.failed().is_some())
  }

  /// Why a life only reads: a fault ended it, or its card says why no life opens its record,
  /// which the client knows before any open.
  pub fn fault_of(&self, at: usize, life: &str) -> Option<&str> {
    let held = self.holds.get(&(at, life.to_owned())).and_then(Hold::failed);
    let card = || self.machines.get(at)?.cards.iter().find(|card| card.life == life);
    held.or_else(|| card()?.refused.as_deref())
  }

  /// The answer of the daemon to the open of a life under the id of its request: each verb of the
  /// life may go.
  pub fn answered_open(&mut self, at: usize, id: u64) {
    let asked = |(key, hold): &(&Life, &mut Hold)| key.0 == at && hold.ask == Ask::Sent(id);
    if let Some((_, hold)) = self.holds.iter_mut().find(asked) {
      hold.ask = Ask::Answered;
    }
  }

  /// A stage of the open of a life that a machine told, as the client hears it now: whether the
  /// window holds that life.
  pub fn opening(&mut self, at: usize, life: String, step: Step) -> bool {
    let Some(hold) = self.holds.get_mut(&(at, life)) else { return false };
    hold.heard(step);
    true
  }

  /// The life of a record let go, once its machine says that no life holds it: its life ended, and
  /// a later verb opens it again.
  pub fn let_go(&mut self, at: usize, life: &str) {
    let key = (at, life.to_owned());
    if self.holds.get(&key).is_some_and(|hold| hold.ask == Ask::Answered) {
      self.holds.remove(&key);
    }
  }
}
