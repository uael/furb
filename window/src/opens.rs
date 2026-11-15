//! The lives that the window holds, and how far the open of each stands, which the head of a thread
//! and its card show.
//!
//! The window holds the life of each thread that it reads and of each session that it writes in:
//! it asks the daemon to open the life, and attaches it in the same step. An open that runs tells
//! each of its stages. The head of the thread shows them from [`QUIET`] on, and its card says that
//! it opens. A life that stood when the window asked it opens with nothing to show.

use std::time::Duration;

use furb::{
  life::Stage,
  view::{elapsed, grouped, size},
};
use opentui::style::Style;

use crate::{
  line::{Line, run, spread},
  model::Model,
  thread::Row,
  tokens::{Loader, Role},
};

/// How long an open runs before the window says that it opens: a shorter wait shows nothing.
pub(crate) const QUIET: Duration = Duration::from_millis(400);

/// How long an open runs before the head of its thread shows each of its stages.
const STAGES: Duration = Duration::from_secs(5);

/// How long an open may take: past it, its row takes the color of attention.
const BOUND: Duration = Duration::from_secs(15);

/// The cells of the bar of the share of a stage.
const BAR: usize = 12;

/// A life of the window: its machine and its record.
pub(crate) type Life = (usize, String);

/// Where the open of a life that the window holds stands with the daemon: to be asked, asked
/// under the id of its request, answered, so each verb of the life may go, or ended by a fault,
/// with why, so only its views answer: the window reads its threads and sends it nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Ask {
  Due,
  Sent(u64),
  Answered,
  Failed(String),
}

/// A life that the window holds: the folder that it opens in when no life holds its record, where
/// its open stands with the daemon, when the window asked it, whether its life stood then, and
/// each stage that its open reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Hold {
  pub(crate) cwd: Option<String>,
  pub(crate) ask: Ask,
  pub(crate) since: Duration,
  pub(crate) stood: bool,
  pub(crate) steps: Vec<Step>,
}

/// A stage that an open reached: its work done and its work known, in the unit of the stage, and
/// when it started, as the window heard it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Step {
  pub(crate) stage: Stage,
  pub(crate) done: u64,
  pub(crate) of: u64,
  pub(crate) at: Duration,
}

/// What a stage counts.
enum Unit {
  Nothing,
  Bytes,
  Entries,
}

/// The one table of the stages of an open: the words that say each, and what it counts.
fn said(stage: Stage) -> (&'static str, Unit) {
  match stage {
    Stage::Recall => ("reads what the record told", Unit::Entries),
    Stage::Find => ("reads the record", Unit::Bytes),
    Stage::Cross => ("moves the life to this build", Unit::Nothing),
    Stage::Read => ("reads the record again", Unit::Bytes),
    Stage::Seed => ("loads the engine", Unit::Nothing),
    Stage::Boot => ("replays the record", Unit::Entries),
    Stage::Log => ("prepares the view", Unit::Entries),
  }
}

impl Hold {
  /// When the open ended: the moment of its last stage once that stage is whole.
  pub(crate) fn ended(&self) -> Option<Duration> {
    let last = self.steps.last()?;
    (last.stage == Stage::Log && last.done == last.of).then_some(last.at)
  }

  /// Whether the open of the life runs: a life that a fault ended opens no more.
  pub(crate) fn runs(&self) -> bool {
    !self.stood && self.ended().is_none() && self.failed().is_none()
  }

  /// Why a fault ended the life, which then only reads.
  pub(crate) fn failed(&self) -> Option<&str> {
    match &self.ask {
      Ask::Failed(why) => Some(why),
      _ => None,
    }
  }

  /// The share of the stage that runs, while it counts work that is not done: a stage whose work
  /// is done waits for the next one, and has no share to show.
  pub(crate) fn share(&self) -> Option<u64> {
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

impl Model {
  /// The life of a record held by the window from now on: the window asks to open it, in a folder
  /// when no life holds the record. A life that the window holds stays as it is.
  pub(crate) fn hold(&mut self, at: usize, life: &str, cwd: Option<String>) {
    let key = (at, life.to_owned());
    if self.holds.contains_key(&key) {
      return;
    }
    let cards = self.machines.get(at).map(|told| &told.cards[..]).unwrap_or_default();
    let stood = cards.iter().any(|card| card.life == life && card.held);
    let hold = Hold { cwd, ask: Ask::Due, since: self.moved, stood, steps: Vec::new() };
    self.holds.insert(key, hold);
  }

  /// The open of a life asked of its machine under the id of its request. A life of a machine that
  /// the window does not dial stays as it is.
  pub(crate) fn asked_open(&mut self, life: &Life, id: Option<u64>) {
    if let (Some(hold), Some(id)) = (self.holds.get_mut(life), id) {
      hold.ask = Ask::Sent(id);
    }
  }

  /// Whether the daemon answered the open of a life, so each verb of the life may go.
  pub(crate) fn stands(&self, at: usize, life: &str) -> bool {
    self.holds.get(&(at, life.to_owned())).is_some_and(|hold| hold.ask == Ask::Answered)
  }

  /// Whether the views of a life answer: it stands, or a fault ended it and it still shows.
  pub(crate) fn reads(&self, at: usize, life: &str) -> bool {
    let hold = self.holds.get(&(at, life.to_owned()));
    hold.is_some_and(|hold| hold.ask == Ask::Answered || hold.failed().is_some())
  }

  /// Why a fault ended a life that the window holds, which then only reads.
  pub(crate) fn fault_of(&self, at: usize, life: &str) -> Option<&str> {
    self.holds.get(&(at, life.to_owned()))?.failed()
  }

  /// The open of a life that runs and that the window says, from [`QUIET`] on.
  pub(crate) fn shown_open(&self, at: usize, life: &str) -> Option<&Hold> {
    let hold = self.holds.get(&(at, life.to_owned()))?;
    (hold.runs() && self.moved.saturating_sub(hold.since) >= QUIET).then_some(hold)
  }

  /// The answer of the daemon to the open of a life under the id of its request: each verb of the
  /// life may go.
  pub(crate) fn answered_open(&mut self, at: usize, id: u64) {
    let asked = |(key, hold): &(&Life, &mut Hold)| key.0 == at && hold.ask == Ask::Sent(id);
    if let Some((_, hold)) = self.holds.iter_mut().find(asked) {
      hold.ask = Ask::Answered;
    }
  }

  /// The refusal of the daemon to open a life under the id of its request, and why: the window
  /// holds the life no more, its thread says why, and each message that waits comes back to the
  /// input with why. The text that the input holds then, when it changed.
  pub(crate) fn unopened(&mut self, at: usize, id: u64, why: String) -> Option<String> {
    let asked = |(key, hold): &(&Life, &Hold)| key.0 == at && hold.ask == Ask::Sent(id);
    let life = self.holds.iter().find(asked).map(|(key, _)| key.clone())?;
    self.holds.remove(&life);
    if let Some(opened) =
      self.thread.as_mut().filter(|one| (one.key.0, &one.key.1) == (at, &life.1))
    {
      opened.fault = Some(why.clone());
    }
    (!self.outgoing.is_empty()).then(|| self.refused(why))
  }

  /// The end of a life that a fault ended, which its machine tells a window that attached: the
  /// window still reads its threads, which say why, and sends it nothing, so a message that waited
  /// for it comes back to the input, and nothing waits for ever. The end can come before the answer
  /// to the open, which then leaves the life as it is, and after its machine said that no life
  /// holds the record, which let the life go: the window holds it again while its thread is open,
  /// so the thread shows the same whatever came first.
  pub(crate) fn failed(&mut self, at: usize, life: &str, fault: String) -> bool {
    let key = (at, life.to_owned());
    let open = self.thread.as_ref().is_some_and(|one| (one.key.0, &one.key.1) == (at, &key.1));
    if !open && !self.holds.contains_key(&key) {
      return false;
    }
    let since = self.moved;
    let gone = || Hold { cwd: None, ask: Ask::Due, since, stood: true, steps: Vec::new() };
    self.holds.entry(key).or_insert_with(gone).ask = Ask::Failed(fault.clone());
    if !self.outgoing.is_empty() {
      self.restore = Some(self.refused(fault));
    }
    true
  }

  /// A stage of the open of a life that a machine told, as the window hears it now: whether the
  /// window holds that life.
  pub(crate) fn opening(&mut self, at: usize, life: String, step: Step) -> bool {
    let Some(hold) = self.holds.get_mut(&(at, life)) else { return false };
    hold.heard(step);
    true
  }

  /// The life of a record let go, once its machine says that no life holds it: its life ended, and
  /// a later verb opens it again.
  pub(crate) fn ended(&mut self, at: usize, life: &str) {
    let key = (at, life.to_owned());
    if self.holds.get(&key).is_some_and(|hold| hold.ask == Ask::Answered) {
      self.holds.remove(&key);
    }
  }

  /// The open that the head of the open thread shows: one that runs, from [`QUIET`] on, until the
  /// page of the thread comes. An open that ended after the thread opened shows until then too, so
  /// the head and the page change in one frame.
  pub(crate) fn open_shown(&self) -> Option<&Hold> {
    let opened = self.thread.as_ref().filter(|one| one.above.is_none() && one.fault.is_none())?;
    let hold = self.holds.get(&(opened.key.0, opened.key.1.clone()))?;
    let after = hold.ended().is_none_or(|ended| ended > opened.since);
    let waited = self.moved.saturating_sub(hold.since) >= QUIET;
    (!hold.stood && after && waited).then_some(hold)
  }

  /// The rows of an open under the head of its thread: its stage with its share and its time, then
  /// each stage that it reached once it ran for [`STAGES`], and a blank row.
  pub(crate) fn open_rows(&self) -> Vec<Row> {
    let Some(hold) = self.open_shown() else { return Vec::new() };
    let t = &self.tokens;
    let took = hold.ended().unwrap_or(self.moved).saturating_sub(hold.since);
    let over = took > BOUND;
    let color = if over { t.attention } else { t.accent };
    let words = hold.steps.last().map_or("starts", |one| said(one.stage).0);
    let left =
      vec![run("  Opening", Style::new().fg(color)), run(format!(" · {words}"), self.prose(over))];
    // The bar fills in the color of the open, and its room stands in the color of a border.
    let share = hold.share().map(|share| {
      let full = usize::try_from(share).unwrap_or_default() * BAR / 100;
      let room = format!("{}  ", "━".repeat(BAR - full));
      [run("━".repeat(full), Style::new().fg(color)), run(room, Style::new().fg(t.border))]
    });
    let time =
      if over { format!("{}, over {}", elapsed(took), elapsed(BOUND)) } else { elapsed(took) };
    let mut right: Line = share.into_iter().flatten().collect();
    right.push(run(time, self.prose(over)));
    let mut rows = vec![Row::of(spread(left, right, self.wide().saturating_sub(2), Style::new()))];
    if took >= STAGES {
      rows.extend(hold.steps.iter().enumerate().map(|(at, step)| self.stage_row(hold, at, step)));
    }
    rows.push(Row::default());
    rows
  }

  /// The look of the text of the row of an open: the text, or the color of attention past the
  /// bound.
  fn prose(&self, over: bool) -> Style {
    if over { Style::new().fg(self.tokens.attention) } else { self.tokens.style(Role::Prose) }
  }

  /// The row of a stage: a mark that it is done, or the loader of the stage that runs, its words,
  /// its count and its time.
  fn stage_row(&self, hold: &Hold, at: usize, step: &Step) -> Row {
    let next = hold.steps.get(at + 1).map(|one| one.at);
    let done = next.is_some() || hold.ended().is_some();
    let took = next.unwrap_or(self.moved).saturating_sub(step.at);
    let (words, unit) = said(step.stage);
    let style = self.tokens.style(if done { Role::Muted } else { Role::Prose });
    let mark = if done { "✓" } else { " " };
    let left = vec![run(format!("    {mark} {words}"), style)];
    let count = counted(&unit, step.done, step.of, done || step.done == step.of);
    let right = vec![run(format!("{count:>20}   {:>7}", elapsed(took)), style)];
    let line: Line = spread(left, right, self.wide().saturating_sub(2), Style::new());
    Row { loader: (!done).then_some((4, Loader::Model)), ..Row::of(line) }
  }
}

/// The count of a stage as a person reads it: what a stage that is done passed, or the work done
/// of the work known, with nothing for a stage that counts nothing.
fn counted(unit: &Unit, done: u64, of: u64, whole: bool) -> String {
  match unit {
    Unit::Nothing => String::new(),
    Unit::Bytes if whole => size(of),
    Unit::Bytes => format!("{} of {}", size(done), size(of)),
    Unit::Entries if whole => grouped(of),
    Unit::Entries => format!("{} of {}", grouped(done), grouped(of)),
  }
}

#[cfg(test)]
#[path = "opens.test.rs"]
mod test;
