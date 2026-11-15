//! The time of the open thread: while the operator holds its bar, he steps through each moment at
//! which an act that its page shows began or ended, and the thread shows what it was just after
//! that moment, page by page, as the daemon folds it. A play steps forward on its own until now.

use std::time::Duration;

use furb_wire::protocol::{Answer, Frame, Verb};

use crate::{Asked, Client, Key};

/// How long a play shows each moment.
const STEP: Duration = Duration::from_millis(120);

/// The bar of the time of a thread: the moments that its machine gave, the one that the thread
/// shows, none for now, whether they changed since the machine gave them, and when the play last
/// stepped while it plays.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Time {
  pub moments: Vec<u64>,
  pub at: Option<usize>,
  pub stale: bool,
  pub played: Option<Duration>,
}

impl Time {
  /// The moment that the thread shows, none for now. The last moment is now: nothing changed
  /// after it.
  pub fn moment(&self) -> Option<u64> {
    self.at.and_then(|at| self.moments.get(at).copied())
  }

  /// The place of a moment by its place among them: the last is now.
  fn placed(&self, at: usize) -> Option<usize> {
    (at + 1 < self.moments.len()).then_some(at)
  }

  /// The place a count of moments later, back for a negative count: never before the first, and
  /// now past the last.
  fn stepped(&self, by: isize) -> Option<usize> {
    let last = self.moments.len().saturating_sub(1);
    let from = self.at.unwrap_or(last);
    self.placed(from.saturating_add_signed(by).min(last))
  }
}

impl Client {
  /// The bar of the time of the open thread opened at now, or closed back to now.
  pub fn time(&mut self) {
    let Some(opened) = self.thread.as_mut() else { return };
    let thread = &mut opened.thread;
    if thread.time.take().is_some_and(|time| time.at.is_some()) {
      thread.moved();
    } else if !thread.key.2.is_empty() {
      thread.time = Some(Time { stale: true, ..Time::default() });
    }
  }

  /// The open thread a count of moments later, back for a negative count. The play stops.
  pub fn step(&mut self, by: isize) {
    self.move_to(|time| time.stepped(by));
  }

  /// The open thread at a moment by its place among them, as a click on its bar gives it: the
  /// last is now. The play stops.
  pub fn reach(&mut self, at: usize) {
    self.move_to(|time| time.placed(at));
  }

  fn move_to(&mut self, to: impl FnOnce(&Time) -> Option<usize>) {
    let Some(thread) = self.thread.as_mut().map(|one| &mut one.thread) else { return };
    let Some(time) = thread.time.as_mut() else { return };
    let at = to(time);
    time.played = None;
    if at != time.at {
      time.at = at;
      thread.moved();
    }
  }

  /// The play of the open thread started from where it shows, from the first moment when it shows
  /// now, or stopped.
  pub fn play(&mut self) {
    let moved = self.moved;
    let Some(thread) = self.thread.as_mut().map(|one| &mut one.thread) else { return };
    let Some(time) = thread.time.as_mut() else { return };
    if time.played.take().is_some() {
      return;
    }
    time.played = Some(moved);
    if time.at.is_none() && time.moments.len() > 1 {
      time.at = Some(0);
      thread.moved();
    }
  }

  /// The play of the open thread one moment on when its step passed; at now it stops.
  pub fn played(&mut self) {
    let moved = self.moved;
    let Some(thread) = self.thread.as_mut().map(|one| &mut one.thread) else { return };
    let Some(time) = thread.time.as_mut() else { return };
    let Some(last) = time.played.filter(|last| moved.saturating_sub(*last) >= STEP) else { return };
    time.at = time.stepped(1);
    time.played = time.at.map(|_| last + STEP);
    thread.moved();
  }

  /// Whether the open thread plays its time.
  pub fn plays(&self) -> bool {
    let time = self.thread.as_ref().and_then(|one| one.thread.time.as_ref());
    time.is_some_and(|time| time.played.is_some())
  }

  /// The ask of the moments of the open thread, while its bar shows and they changed since its
  /// machine gave them.
  pub(crate) fn moments_due(&self) -> Option<(usize, Verb, Asked)> {
    let thread = &self.thread.as_ref()?.thread;
    let asked = Asked::Moments(thread.key.clone());
    let asking = self.asked.values().any(|one| *one == asked);
    let (life, on) = (thread.key.1.clone(), Some(thread.key.2.clone()));
    (thread.time.as_ref()?.stale && !asking)
      .then(|| (thread.key.0, Verb::Moments { life, on }, asked))
  }

  /// What a machine said to the ask of the moments of the bar of a thread. A refusal or a lost
  /// machine gives none, so the bar stays at now.
  pub fn timed(&mut self, key: &Key, heard: Option<Frame>) {
    let Some(opened) = self.thread.as_mut().filter(|one| one.thread.key == *key) else { return };
    let Some(time) = opened.thread.time.as_mut() else { return };
    time.moments = match heard {
      Some(Frame::Reply { answer: Answer::Moments(moments), .. }) => moments,
      _ => Vec::new(),
    };
    time.stale = false;
    time.at = time.at.and_then(|at| time.placed(at));
  }
}
