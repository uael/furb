//! The connection of the window to the daemon of a machine: one task of tokio holds the stream, and
//! the thread of the app hears each frame.
//!
//! The task says the hello, then the verbs that the window asks at its dial, in their order, and
//! reads each frame of the daemon. The thread of the app takes the frames that wait, tells them to
//! the screen, and only then takes more. So a window that draws slowly holds the daemon back at the
//! stream, as the daemon expects of a slow window, and no frame is dropped.

use std::convert::Infallible;

use furb_host::protocol::{BOUND, Frame, Hello, Kind, PROTOCOL, Request, Verb};
use futures::{SinkExt, StreamExt};
use opentui::App;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio_util::codec::{Framed, LinesCodec};

use crate::{Error, Machine, Screen};

/// How many frames of the daemon wait for the thread of the app before the task reads no more.
const WAITING: usize = 256;

/// What the task tells the thread of the app: a frame of the daemon, or why the connection ended.
pub(crate) type Heard = Result<Frame, Error>;

/// A screen on the daemon of a machine, which the window dials on the runtime of the app. The
/// window asks the records of the machine at its dial.
pub(crate) fn dialed(app: &App, machine: Machine, screen: Screen) {
  let (tell, frames) = mpsc::channel(WAITING);
  // The task ends when the screen lets the frames go, and with the runtime of the app.
  drop(app.runtime().spawn(async move {
    let Err(end) = held(machine, [Verb::List { config: None }], &tell).await;
    // A screen that is gone hears no end.
    drop(tell.send(Err(end)).await);
  }));
  hearing(app, frames, screen);
}

/// The connection held until it ends, and why it ended. Each request takes the next id, from the
/// hello on.
async fn held(
  machine: Machine,
  asks: impl IntoIterator<Item = Verb>,
  tell: &Sender<Heard>,
) -> Result<Infallible, Error> {
  let mut lines = Framed::new(machine.reach().await?, LinesCodec::new_with_max_length(BOUND));
  let hello = Hello { protocol: PROTOCOL, kind: Kind::Window, name: None, role: None };
  for (id, verb) in (0..).zip([Verb::Hello(hello)].into_iter().chain(asks)) {
    lines.send(serde_json::to_string(&Request { id, verb })?).await?;
  }
  loop {
    let line = lines.next().await.ok_or(Error::Closed)??;
    if tell.send(Ok(serde_json::from_str(&line)?)).await.is_err() {
      return Err(Error::Closed);
    }
  }
}

/// The frames that wait told to the screen, then the next ones, until the connection ends.
fn hearing(app: &App, mut frames: Receiver<Heard>, mut screen: Screen) {
  let waiting = async move {
    let mut heard = Vec::new();
    frames.recv_many(&mut heard, WAITING).await;
    (heard, frames)
  };
  drop(app.spawn(waiting, move |cx, (heard, frames)| {
    // No frame waits once the task ended and the screen heard why.
    if heard.is_empty() {
      return;
    }
    for one in heard {
      screen.tell(|told| told.hear(one));
    }
    hearing(cx.app(), frames, screen);
  }));
}
