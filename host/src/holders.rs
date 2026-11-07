//! The holders of a life (D2 of #983): each window that attached it and each pin. A life with no
//! holder lives for the grace of the daemon, then writes its checkpoint and ends. Work that runs
//! in the life holds nothing.

use std::{collections::BTreeSet, time::Duration};

use tokio::{task::AbortHandle, time::Instant};

use crate::{
  protocol::{Frame, Peer},
  serve::Outbox,
};

/// A window that attached a life: its peer, the outbox of that peer, and the task that pushes the
/// events of the life to it.
pub(crate) struct Window {
  pub(crate) peer: Peer,
  pub(crate) outbox: Outbox,
  pub(crate) told: AbortHandle,
}

/// The holders of a life, and the moment when it ends while it has none.
pub(crate) struct Holders {
  windows: Vec<Window>,
  pins: BTreeSet<String>,
  grace: Duration,
  idle: Option<Instant>,
}

impl Holders {
  /// No holder yet: a life that no window attaches and no pin holds ends after the grace.
  pub(crate) fn new(grace: Duration) -> Self {
    let idle = Some(Instant::now() + grace);
    Self { windows: Vec::new(), pins: BTreeSet::new(), grace, idle }
  }

  /// When the life ends, while it has no holder.
  pub(crate) fn idle(&self) -> Option<Instant> {
    self.idle
  }

  /// A window attached, in the place of an earlier attach of the same peer.
  pub(crate) fn attach(&mut self, window: Window) {
    self.detach(&window.outbox);
    self.windows.push(window);
    self.settle();
  }

  /// Whether the peer of an outbox attached the life as a window.
  pub(crate) fn holds(&self, outbox: &Outbox) -> bool {
    self.windows.iter().any(|one| one.outbox.same_channel(outbox))
  }

  /// The window of a peer detached, which hears no event of the life more: whether it was attached.
  pub(crate) fn detach(&mut self, outbox: &Outbox) -> bool {
    let Some(at) = self.windows.iter().position(|one| one.outbox.same_channel(outbox)) else {
      return false;
    };
    self.windows.remove(at).told.abort();
    self.settle();
    true
  }

  pub(crate) fn pin(&mut self, holder: String) {
    self.pins.insert(holder);
    self.settle();
  }

  /// A pin let go: whether it held the life.
  pub(crate) fn unpin(&mut self, holder: &str) -> bool {
    let held = self.pins.remove(holder);
    self.settle();
    held
  }

  /// The peer of each window, in the order that they attached.
  pub(crate) fn presence(&self) -> Vec<Peer> {
    self.windows.iter().map(|one| one.peer.clone()).collect()
  }

  /// Who is attached now, pushed to each window of the life. A peer that is gone takes no frame,
  /// and the task that pushes its events tells the daemon that it left.
  pub(crate) fn present(&self, life: &str) {
    self.tell(&Frame::Presence { life: life.to_owned(), peers: self.presence() });
  }

  /// A frame pushed to each window of the life. A peer that is gone takes no frame.
  pub(crate) fn tell(&self, frame: &Frame) {
    for one in &self.windows {
      one.outbox.send(frame.clone()).ok();
    }
  }

  /// The holders let go at a handover: each window hears no event more, and the pins go with the
  /// life to the daemon that takes the place of this one.
  pub(crate) fn release(self) -> Vec<String> {
    for one in &self.windows {
      one.told.abort();
    }
    self.pins.into_iter().collect()
  }

  /// The moment when the life ends, kept from the time that its last holder left.
  fn settle(&mut self) {
    let held = !self.windows.is_empty() || !self.pins.is_empty();
    self.idle = if held { None } else { self.idle.or_else(|| Some(Instant::now() + self.grace)) };
  }
}
