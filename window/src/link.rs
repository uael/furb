//! The connection of the window to the daemon of a machine: one task of tokio holds the stream, and
//! the thread of the app hears each frame.
//!
//! The task says each verb that the window asks, in its order, the hello and the cards first, and
//! reads each frame of the daemon. The thread of the app takes the frames that wait, tells them to
//! the window, and only then takes more. So a window that draws slowly holds the daemon back at the
//! stream, as the daemon expects of a slow window, and no frame is dropped.

use std::{convert::Infallible, pin::pin};

use furb_host::protocol::{BOUND, Frame, Hello, Kind, PROTOCOL, Request, Verb};
use futures::{
  SinkExt, StreamExt,
  future::{self, Either},
};
use opentui::App;
use tokio::sync::mpsc::{self, Receiver, Sender, UnboundedReceiver, UnboundedSender};
use tokio_util::codec::{Framed, LinesCodec};

use crate::{Error, Machine};

/// How many frames of the daemon wait for the thread of the app before the task reads no more.
const WAITING: usize = 256;

/// What the task tells the thread of the app: a frame of the daemon, or why the connection ended.
pub(crate) type Heard = Result<Frame, Error>;

/// The daemon of a machine, which the window dials on the runtime of the app, and what hears each
/// of its frames on the thread of the app. The window says the hello and asks the cards of the
/// machine at its dial, and asks each later verb through the asker that it is given.
pub(crate) fn dialed(app: &App, machine: Machine, on: impl FnMut(&App, Heard) + 'static) -> Asker {
  let (tell, frames) = mpsc::channel(WAITING);
  let (requests, outgoing) = mpsc::unbounded_channel();
  let mut asker = Asker { requests, next: 0 };
  let hello = Hello { protocol: PROTOCOL, kind: Kind::Window, name: None, role: None };
  asker.ask(Verb::Hello(hello));
  asker.ask(Verb::Cards);
  // The task ends when the window lets the frames go, and with the runtime of the app.
  drop(app.runtime().spawn(async move {
    let Err(end) = held(machine, outgoing, &tell).await;
    // A window that is gone hears no end.
    drop(tell.send(Err(end)).await);
  }));
  hearing(app, frames, on);
  asker
}

/// What sends the requests of the window to the daemon of a machine, each with the next id.
pub(crate) struct Asker {
  requests: UnboundedSender<Request>,
  next: u64,
}

impl Asker {
  /// A verb asked of the daemon, and the id of its request, which its answer names.
  pub(crate) fn ask(&mut self, verb: Verb) -> u64 {
    let id = self.next;
    self.next += 1;
    // A connection that ended takes no request, and the window heard why it ended.
    drop(self.requests.send(Request { id, verb }));
    id
  }
}

/// The connection held until it ends, and why it ended: it writes each request of the window in
/// order, and reads each frame of the daemon. A window that is gone ends it.
async fn held(
  machine: Machine,
  mut outgoing: UnboundedReceiver<Request>,
  tell: &Sender<Heard>,
) -> Result<Infallible, Error> {
  let mut lines = Framed::new(machine.reach().await?, LinesCodec::new_with_max_length(BOUND));
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
