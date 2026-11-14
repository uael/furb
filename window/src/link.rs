//! The connection of the window to the daemon of a machine: one task of tokio holds the stream, and
//! the thread of the app hears each frame.
//!
//! The task says the hello and the ask of the cards first, then each verb that the window asks, in
//! its order, and reads each frame of the daemon. The thread of the app takes the frames that wait,
//! tells them to the window, and only then takes more. So a window that draws slowly holds the
//! daemon back at the stream, as the daemon expects of a slow window, and no frame is dropped.
//!
//! When the connection ends, the task tells the window why, and dials the machine again, at a pace
//! that grows to [`SLOWEST`], until it answers: a new connection says the hello and the ask of the
//! cards again. A verb that the window asked of the connection that ended goes nowhere: the window
//! asks a machine only while it lists its cards.

use std::{
  convert::Infallible,
  pin::pin,
  sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
  },
  time::Duration,
};

use furb_host::protocol::{BOUND, Frame, Hello, Kind, PROTOCOL, Request, Verb};
use futures::{
  SinkExt, StreamExt,
  future::{self, Either},
};
use opentui::App;
use tokio::{
  sync::mpsc::{self, Receiver, Sender, UnboundedReceiver, UnboundedSender},
  time,
};
use tokio_util::codec::{Framed, LinesCodec};

use crate::{Error, Machine};

/// How many frames of the daemon wait for the thread of the app before the task reads no more.
const WAITING: usize = 256;

/// How long the task waits before it dials a machine again after the first loss, a wait that
/// doubles at each loss that follows, up to [`SLOWEST`].
const FIRST: Duration = Duration::from_millis(250);

/// The longest wait between two dials of a machine that does not answer.
const SLOWEST: Duration = Duration::from_secs(10);

/// What the task tells the thread of the app: a frame of the daemon, or why the connection ended.
pub(crate) type Heard = Result<Frame, Error>;

/// The daemon of a machine, which the window dials on the runtime of the app, and dials again each
/// time that the connection ends, and what hears each of its frames on the thread of the app. The
/// window asks each verb after the hello and the cards through the asker that it is given.
pub(crate) fn dialed(app: &App, machine: Machine, on: impl FnMut(&App, Heard) + 'static) -> Asker {
  let (tell, frames) = mpsc::channel(WAITING);
  let (requests, outgoing) = mpsc::unbounded_channel();
  let next = Arc::new(AtomicU64::new(0));
  let ids = Arc::clone(&next);
  // The task ends when the window lets the frames go, and with the runtime of the app.
  drop(app.runtime().spawn(async move {
    let (mut machine, mut outgoing, mut wait) = (machine, outgoing, FIRST);
    loop {
      let Err(end) = held(&mut machine, &mut outgoing, &tell, (&ids, &mut wait)).await;
      // A window that is gone hears no end.
      if tell.send(Err(end)).await.is_err() {
        return;
      }
      time::sleep(wait).await;
      wait = (wait * 2).min(SLOWEST);
    }
  }));
  hearing(app, frames, on);
  Asker { requests, next }
}

/// What sends the requests of the window to the daemon of a machine, each with the next id.
pub(crate) struct Asker {
  requests: UnboundedSender<Request>,
  next: Arc<AtomicU64>,
}

impl Asker {
  /// A verb asked of the daemon, and the id of its request, which its answer names.
  pub(crate) fn ask(&mut self, verb: Verb) -> u64 {
    let id = self.next.fetch_add(1, Ordering::Relaxed);
    // A task that ended takes no request, and the window heard why it ended.
    drop(self.requests.send(Request { id, verb }));
    id
  }
}

/// One connection held until it ends, and why it ended: it says the hello and the ask of the
/// cards, then writes each request of the window in order, and reads each frame of the daemon. A
/// request that waits from a connection before it goes nowhere, and the wait before the next dial
/// starts again once the machine answers. A window that is gone ends it.
async fn held(
  machine: &mut Machine,
  outgoing: &mut UnboundedReceiver<Request>,
  tell: &Sender<Heard>,
  (ids, wait): (&AtomicU64, &mut Duration),
) -> Result<Infallible, Error> {
  let mut lines = Framed::new(machine.reach().await?, LinesCodec::new_with_max_length(BOUND));
  *wait = FIRST;
  while outgoing.try_recv().is_ok() {}
  let hello = Hello { protocol: PROTOCOL, kind: Kind::Window, name: None, role: None };
  for verb in [Verb::Hello(hello), Verb::Cards] {
    let id = ids.fetch_add(1, Ordering::Relaxed);
    lines.send(serde_json::to_string(&Request { id, verb })?).await?;
  }
  loop {
    match future::select(pin!(outgoing.recv()), lines.next()).await {
      Either::Left((request, reading)) => {
        drop(reading);
        let request = request.ok_or(Error::Closed)?;
        lines.send(serde_json::to_string(&request)?).await?;
      }
      Either::Right((line, _)) => {
        let line = line.ok_or(Error::Closed)??;
        if tell.send(Ok(serde_json::from_str(&line)?)).await.is_err() {
          return Err(Error::Closed);
        }
      }
    }
  }
}

/// The frames that wait told to the window, then the next ones, until the connection ends.
fn hearing(app: &App, mut frames: Receiver<Heard>, mut on: impl FnMut(&App, Heard) + 'static) {
  let waiting = async move {
    let mut heard = Vec::new();
    frames.recv_many(&mut heard, WAITING).await;
    (heard, frames)
  };
  drop(app.spawn(waiting, move |cx, (heard, frames)| {
    // No frame waits once the task ended and the window heard why.
    if heard.is_empty() {
      return;
    }
    for one in heard {
      on(cx.app(), one);
    }
    hearing(cx.app(), frames, on);
  }));
}
