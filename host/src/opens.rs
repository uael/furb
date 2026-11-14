//! How far the open of a life stands. The thread of a life boots its engine and serves its log, so
//! it tells no event while it opens: it writes each stage that the open passes here, and the task
//! that pushes the events of the life to a window reads them, on a thread of the daemon. The open
//! of a life tells its stages.

use std::{future, sync::Arc, time::Duration};

use furb::life::{Progress, Stage};
use tokio::{
  sync::watch,
  time::{Instant, sleep_until},
};

use crate::protocol::Frame;

/// The least time between two frames of a stage that goes on, so a long stage tells a window at
/// most four frames each second.
const PACE: Duration = Duration::from_millis(250);

/// A stage that an open reached, with its work done and its work known.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Step {
  stage: Stage,
  done: u64,
  of: u64,
}

/// The opens of a life: which open this is, counted from 1, and each stage that it reached, in
/// order. An open only goes forward: a stage never comes again, its numbers never go back, and
/// its work done never passes its work known.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Opens {
  born: u64,
  steps: Vec<Step>,
}

impl Opens {
  /// A stage told with its numbers, and whether it moved the open.
  fn at(&mut self, stage: Stage, done: u64, of: u64) -> bool {
    let before = self.steps.last().copied();
    match self.steps.last_mut() {
      Some(last) if last.stage == stage => {
        last.done = last.done.max(done);
        last.of = last.of.max(of).max(last.done);
      }
      Some(last) if last.stage > stage => {}
      _ => self.steps.push(Step { stage, done, of: of.max(done) }),
    }
    self.steps.last().copied() != before
  }

  /// Whether the open ended: the log of the life took each event that the open told.
  fn whole(&self) -> bool {
    matches!(self.steps.last(), Some(Step { stage: Stage::Log, done, of }) if done == of)
  }
}

/// Where the thread of a life writes how far its open stands.
pub(crate) struct Told(watch::Sender<Opens>);

impl Told {
  /// The place of the opens of a life, and what each window of it reads them from.
  pub(crate) fn new() -> (Self, watch::Receiver<Opens>) {
    let (told, heard) = watch::channel(Opens::default());
    (Self(told), heard)
  }

  /// A new open of the life, which reached no stage yet.
  pub(crate) fn born(&self) {
    self.0.send_modify(|opens| *opens = Opens { born: opens.born + 1, steps: Vec::new() });
  }

  /// The function that an open tells its stages to.
  pub(crate) fn progress(&self) -> Progress {
    let told = self.0.clone();
    Progress(Arc::new(move |stage, done, of| {
      told.send_if_modified(|opens| opens.at(stage, done, of));
    }))
  }
}

/// What one window heard of the opens of a life, and when it may hear the next frame of a stage
/// that goes on. A window that attaches while the life opens hears each stage that the open
/// reached, and one that attaches to a life that stands hears none.
pub(crate) struct Heard {
  opens: watch::Receiver<Opens>,
  sent: Opens,
  due: Instant,
}

impl Heard {
  pub(crate) fn new(opens: watch::Receiver<Opens>) -> Self {
    let now = opens.borrow().clone();
    let sent = if now.whole() { now } else { Opens { born: now.born, steps: Vec::new() } };
    Self { opens, sent, due: Instant::now() }
  }

  /// Once the window has a frame to hear: at once for a new stage, and after the pace for a stage
  /// that goes on. It waits for ever once the thread of the life is gone.
  pub(crate) async fn due(&mut self) {
    loop {
      let (moved, staged) = {
        let (now, sent) = (self.opens.borrow(), &self.sent);
        (*now != *sent, now.born != sent.born || now.steps.len() != sent.steps.len())
      };
      if staged || (moved && Instant::now() >= self.due) {
        return;
      }
      tokio::select! {
        changed = self.opens.changed() => if changed.is_err() {
          return future::pending().await;
        },
        () = sleep_until(self.due), if moved => {}
      }
    }
  }

  /// Whether the open ended, so each frame of it goes before the next event of the life.
  pub(crate) fn ended(&self) -> bool {
    self.opens.borrow().whole()
  }

  /// The frames that the window did not hear yet, one for each stage that moved, in order.
  pub(crate) fn frames(&mut self, life: &str) -> Vec<Frame> {
    let now = self.opens.borrow();
    if *now == self.sent {
      return Vec::new();
    }
    let known = if now.born == self.sent.born { self.sent.steps.as_slice() } else { &[] };
    let news = now.steps.iter().enumerate().filter(|(at, step)| known.get(*at) != Some(step));
    let frames = news.map(|(_, &Step { stage, done, of })| Frame::Opening {
      life: life.to_owned(),
      stage,
      done,
      of,
    });
    let frames = frames.collect();
    self.sent = now.clone();
    self.due = Instant::now() + PACE;
    frames
  }
}

#[cfg(test)]
#[path = "opens.test.rs"]
mod test;
