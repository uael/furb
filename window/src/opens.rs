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

use furb_client::{Ask, Hold, Life, Step};

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

/// What a stage counts.
enum Unit {
  Nothing,
  Bytes,
  Entries,
  /// Thousandths of the work, which a person reads as a percent.
  Thousandths,
}

/// The one table of the stages of an open: the words that say each, and what it counts.
fn said(stage: Stage) -> (&'static str, Unit) {
  match stage {
    Stage::Recall => ("reads what the record told", Unit::Thousandths),
    Stage::Find => ("reads the record", Unit::Bytes),
    Stage::Cross => ("moves the life to this build", Unit::Nothing),
    Stage::Read => ("reads the record again", Unit::Bytes),
    Stage::Seed => ("loads the engine", Unit::Nothing),
    Stage::Boot => ("replays the record", Unit::Entries),
    Stage::Log => ("prepares the view", Unit::Entries),
  }
}

impl Model {
  /// The open of a life that runs and that the window says, from [`QUIET`] on.
  pub(crate) fn shown_open(&self, at: usize, life: &str) -> Option<&Hold> {
    let hold = self.client.holds.get(&(at, life.to_owned()))?;
    (hold.runs() && self.client.moved.saturating_sub(hold.since) >= QUIET).then_some(hold)
  }

  /// The refusal of the daemon to open a life under the id of its request, and why: the window
  /// holds the life no more, its thread says why, and each message that waits comes back to the
  /// input with why. The text that the input holds then, when it changed.
  pub(crate) fn unopened(&mut self, at: usize, id: u64, why: String) -> Option<String> {
    let asked = |(key, hold): &(&Life, &Hold)| key.0 == at && hold.ask == Ask::Sent(id);
    let life = self.client.holds.iter().find(asked).map(|(key, _)| key.clone())?;
    self.client.holds.remove(&life);
    if let Some(opened) = self
      .client
      .thread
      .as_mut()
      .filter(|one| (one.thread.key.0, &one.thread.key.1) == (at, &life.1))
    {
      opened.thread.fault = Some(why.clone());
    }
    (!self.client.outgoing.is_empty()).then(|| self.refused(why))
  }

  /// The end of a life that a fault ended, which its machine tells a window that attached: the
  /// window still reads its threads, which say why, and sends it nothing, so a message that waited
  /// for it comes back to the input, and nothing waits for ever. The end can come before the answer
  /// to the open, which then leaves the life as it is, and after its machine said that no life
  /// holds the record, which let the life go: the window holds it again while its thread is open,
  /// so the thread shows the same whatever came first.
  pub(crate) fn failed(&mut self, at: usize, life: &str, fault: String) -> bool {
    let key = (at, life.to_owned());
    let open = self
      .client
      .thread
      .as_ref()
      .is_some_and(|one| (one.thread.key.0, &one.thread.key.1) == (at, &key.1));
    if !open && !self.client.holds.contains_key(&key) {
      return false;
    }
    let since = self.client.moved;
    let gone =
      || Hold { cwd: None, actor: None, ask: Ask::Due, since, stood: true, steps: Vec::new() };
    let hold = self.client.holds.entry(key).or_insert_with(gone);
    hold.ask = Ask::Failed(fault.clone());
    let actor = hold.actor.clone();
    if !self.client.outgoing.is_empty() {
      self.client.restore = Some(self.refused(fault));
      // The model that he chose for the session that did not open stays his choice.
      if let Some(actor) = actor {
        self.took_actor(actor);
      }
    }
    true
  }

  /// The open that the head of the open thread shows: one that runs, from [`QUIET`] on, until the
  /// page of the thread comes. An open that ended after the thread opened shows until then too, so
  /// the head and the page change in one frame.
  pub(crate) fn open_shown(&self) -> Option<&Hold> {
    let opened = self
      .client
      .thread
      .as_ref()
      .filter(|one| one.thread.above.is_none() && one.thread.fault.is_none())?;
    let hold = self.client.holds.get(&(opened.thread.key.0, opened.thread.key.1.clone()))?;
    let after = hold.ended().is_none_or(|ended| ended > opened.thread.since);
    let waited = self.client.moved.saturating_sub(hold.since) >= QUIET;
    (!hold.stood && after && waited).then_some(hold)
  }

  /// The rows of an open under the head of its thread: its stage with its share and its time, then
  /// each stage that it reached once it ran for [`STAGES`], and a blank row.
  pub(crate) fn open_rows(&self) -> Vec<Row> {
    let Some(hold) = self.open_shown() else { return Vec::new() };
    let t = &self.tokens;
    let took = hold.ended().unwrap_or(self.client.moved).saturating_sub(hold.since);
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
    let took = next.unwrap_or(self.client.moved).saturating_sub(step.at);
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
    Unit::Thousandths if whole => String::new(),
    Unit::Thousandths => format!("{}%", done * 100 / of.max(1)),
  }
}

#[cfg(test)]
#[path = "opens.test.rs"]
mod test;
