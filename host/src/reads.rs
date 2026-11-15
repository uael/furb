//! The thread that each peer reads, told again to it at each change.
//!
//! A peer that reads the end of a thread hears a frame `thread` with each exchange of the end that
//! is new or changed since it last heard it, with the agents and the context as they stand. A task
//! of the daemon waits for an event of the life that a page reads, an act or what a model wrote,
//! takes each event that waits with it, asks the life the page of the end again at once and tells
//! the difference. So the life does no work for a thread that no peer reads, and a page costs the
//! life the time of the thread, as `furb::view::Held::page` says, at most once in [`MOMENT`].

use std::time::Duration;

use furb::view::{self, Page, Reach};
use tokio::{
  sync::broadcast::{
    Receiver,
    error::{RecvError, TryRecvError},
  },
  task::AbortHandle,
  time::sleep,
};

use crate::{
  event::Event,
  life::{Command, LifeHandle, Logged, Reply},
  protocol::Frame,
  serve::Outbox,
};

/// How long a task waits after a page before it asks the next one: the line that a model writes
/// moves at most at this pace, 30 times a second.
const MOMENT: Duration = Duration::from_millis(33);

/// The thread that a peer reads, which the task tells.
pub(crate) struct Read {
  pub(crate) handle: LifeHandle,
  /// Each event of the life from before the page that the peer was given.
  pub(crate) heard: Receiver<Logged>,
  pub(crate) outbox: Outbox,
  pub(crate) life: String,
  pub(crate) thread: String,
  pub(crate) limit: usize,
}

/// The task of each peer that reads a thread. A peer reads one thread at a time, so a new read of a
/// peer ends its read before.
#[derive(Default)]
pub(crate) struct Readers {
  tasks: Vec<(Outbox, AbortHandle)>,
}

impl Readers {
  /// The task of the read of a peer, in the place of its read before.
  pub(crate) fn read(&mut self, outbox: Outbox, task: AbortHandle) {
    self.tasks.retain(|(one, before)| {
      let stays = !one.is_closed() && !one.same_channel(&outbox);
      if !stays {
        before.abort();
      }
      stays
    });
    self.tasks.push((outbox, task));
  }
}

impl Read {
  /// The read of a thread, from the page that the peer was given, told until the peer or the life
  /// is gone.
  pub(crate) async fn told(mut self, mut told: Page) {
    loop {
      match self.heard.recv().await {
        Ok((_, event)) if matches!(*event, Event::Ended(_)) => return,
        Ok((_, event)) if !moves(&event) => continue,
        Ok(_) | Err(RecvError::Lagged(_)) => {}
        Err(RecvError::Closed) => return,
      }
      if !self.gathered() {
        return;
      }
      let reach = Reach { before: None, limit: self.limit };
      let command = Command::Thread { thread: self.thread.clone(), reach, at: None };
      let Ok(Reply::Page(page)) = self.handle.send(command).await else { return };
      let changed: Vec<_> =
        page.exchanges.iter().filter(|one| !told.exchanges.contains(one)).cloned().collect();
      // What a page holds beside its exchanges, which a frame tells whole.
      let moved =
        page.above != told.above || page.agents != told.agents || page.context != told.context;
      if !changed.is_empty() || moved {
        let (agents, context) = (page.agents.clone(), page.context.clone());
        let page = Page { exchanges: changed, above: page.above, agents, context };
        let frame = Frame::Thread { life: self.life.clone(), thread: self.thread.clone(), page };
        if self.outbox.send(frame).is_err() {
          return;
        }
      }
      told = page;
      // The events of the pause wait in the channel, so the next page answers each of them.
      sleep(MOMENT).await;
    }
  }

  /// Each event that waits, taken so that one page answers all of them: false once the life is
  /// gone.
  fn gathered(&mut self) -> bool {
    loop {
      match self.heard.try_recv() {
        Ok((_, event)) if matches!(*event, Event::Ended(_)) => return false,
        Ok(_) | Err(TryRecvError::Lagged(_)) => {}
        Err(TryRecvError::Empty) => return true,
        Err(TryRecvError::Closed) => return false,
      }
    }
  }
}

/// Whether an event can change a page: an act, what a model writes, a retry of its call, and what
/// a command writes.
fn moves(event: &Event) -> bool {
  match event {
    Event::Act(_) | Event::Text { .. } | Event::Retried { .. } => true,
    Event::Fact(fact) => view::wrote(fact),
    _ => false,
  }
}
