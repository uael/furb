//! The thread that each peer reads, told again to it at each change.
//!
//! A peer that reads the end of a thread hears a frame `thread` with each exchange of the end that
//! is new or changed since it last heard it. A task of the daemon waits for an event of the life
//! that a page reads, an act or what a model wrote, gathers the events of a moment, asks the life
//! the page of the end again and tells the difference. So the life does no work for a thread that
//! no peer reads, and a page costs the life the time of the thread, as `furb::view::Held::page`
//! says, at most once in [`MOMENT`].

use std::time::Duration;

use furb::view::{Page, Reach};
use tokio::{sync::broadcast::error::RecvError, task::AbortHandle, time::sleep};

use crate::{
  event::Event,
  life::{Command, LifeHandle, Reply},
  protocol::Frame,
  serve::Outbox,
};

/// How long a task gathers the events of the life before it asks a page again: the line that a
/// model writes moves at this pace.
const MOMENT: Duration = Duration::from_millis(100);

/// The thread that a peer reads, which the task tells.
pub(crate) struct Read {
  pub(crate) handle: LifeHandle,
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
  pub(crate) async fn told(self, mut told: Page) {
    let mut heard = self.handle.told();
    loop {
      match heard.recv().await {
        Ok((_, event)) if matches!(*event, Event::Ended(_)) => return,
        Ok((_, event)) if !matches!(*event, Event::Act(_) | Event::Text { .. }) => continue,
        Ok(_) | Err(RecvError::Lagged(_)) => {}
        Err(RecvError::Closed) => return,
      }
      sleep(MOMENT).await;
      let reach = Reach { before: None, limit: self.limit };
      let command = Command::Thread { thread: self.thread.clone(), reach };
      let Ok(Reply::Page(page)) = self.handle.send(command).await else { return };
      let changed: Vec<_> =
        page.exchanges.iter().filter(|one| !told.exchanges.contains(one)).cloned().collect();
      if !changed.is_empty() || page.above != told.above || page.agents != told.agents {
        let page = Page { exchanges: changed, above: page.above, agents: page.agents.clone() };
        let frame = Frame::Thread { life: self.life.clone(), thread: self.thread.clone(), page };
        if self.outbox.send(frame).is_err() {
          return;
        }
      }
      told = page;
    }
  }
}
