//! The places that the operator left, as a browser keeps its pages: each thread that he read, as
//! he read it, or the list with no thread open. The thread of an agent keeps the thread that it
//! came from and the agent that he went back from, so a step of the trail and a step between a
//! thread and its agent make one history.

use std::collections::HashMap;

use crate::{Client, Opened};

/// The most places that the trail keeps on each side, the oldest of which goes first: each holds a
/// whole thread as the operator read it.
const KEPT: usize = 50;

/// The places behind the one that the operator reads, the last left last, and those ahead of it
/// that he went back from, the nearest last.
#[derive(Debug, Default)]
pub struct Trail {
  behind: Vec<Option<Opened>>,
  ahead: Vec<Option<Opened>>,
}

impl Trail {
  /// Whether a place stands ahead.
  #[must_use]
  pub fn ahead(&self) -> bool {
    !self.ahead.is_empty()
  }
}

impl Client {
  /// The place that the operator leaves for another, kept behind, and no place ahead any more, as
  /// a new page in a browser drops the pages ahead. A new thread that no message made yet is no
  /// place to go back to.
  pub fn leave(&mut self) {
    let left = self.thread.take().filter(|one| !one.root().thread.key.2.is_empty());
    self.trail.ahead.clear();
    if left.is_none() && self.trail.behind.last().is_some_and(Option::is_none) {
      return;
    }
    kept(&mut self.trail.behind, left.map(asks_again));
  }

  /// A step of the trail back, or ahead: the place there opens as the operator left it, and the
  /// place that he reads goes to the other side. Whether a place stood there.
  pub fn walk_trail(&mut self, back: bool) -> bool {
    let (from, to) = if back {
      (&mut self.trail.behind, &mut self.trail.ahead)
    } else {
      (&mut self.trail.ahead, &mut self.trail.behind)
    };
    let Some(place) = from.pop() else { return false };
    let here = self.thread.take().filter(|one| !one.root().thread.key.2.is_empty());
    kept(to, here.map(asks_again));
    self.selected = place.as_ref().map(|one| one.root().thread.key.clone());
    if let Some(opened) = &place {
      let key = &opened.thread.key;
      self.hold((key.0, key.1.clone()), self.cwd(key.0, &key.1).map(Into::into), self.moved);
    }
    (self.thread, self.aim) = (place, None);
    true
  }

  /// A step between a thread and its agent: no place stands ahead of it any more.
  pub fn went_on(&mut self) {
    self.trail.ahead.clear();
  }
}

/// A place kept on a side of the trail, past which the oldest goes.
fn kept(side: &mut Vec<Option<Opened>>, place: Option<Opened>) {
  side.push(place);
  if side.len() > KEPT {
    side.remove(0);
  }
}

/// A thread that waits as the operator read it, and asks its end again when it opens again, since
/// the daemon tells a window the changes of one thread at a time.
fn asks_again(mut opened: Opened) -> Opened {
  opened.thread.asked = HashMap::new();
  opened.thread.again = true;
  opened
}
