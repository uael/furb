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
//!
//! The daemon of this computer follows the build of furb: once the program that serves it holds a
//! build that the daemon does not run, as its welcome or its refusal of the protocol of the window
//! says, the window asks the handover of that program, once for each build, and dials the new
//! daemon, as [`Machine::upgrade`] says.

use std::{convert::Infallible, io, time::Duration};

use furb_client::greeting;
use furb_host::{
  ProtocolError,
  protocol::{Answer, BOUND, Frame, Kind, PROTOCOL, Request, Role},
};
use futures::{SinkExt, StreamExt};
use opentui::App;
use tokio::{
  sync::mpsc::{self, Receiver, Sender, UnboundedReceiver, UnboundedSender},
  time,
};
use tokio_util::codec::{Framed, LinesCodec};

use crate::{Error, Machine, machine::Looking};

/// How many frames of the daemon wait for the thread of the app before the task reads no more.
const WAITING: usize = 256;

/// How long the task waits before it dials a machine again after the first loss, a wait that
/// doubles at each loss that follows, up to [`SLOWEST`].
const FIRST: Duration = Duration::from_millis(250);

/// The longest wait between two dials of a machine that does not answer.
const SLOWEST: Duration = Duration::from_secs(10);

/// How long the window waits between two looks at the tailnet for a machine that came online.
const LOOK: Duration = Duration::from_secs(30);

/// How often the window looks at the program that serves the daemon of this computer.
const FOLLOW: Duration = Duration::from_secs(1);

/// What the task tells the thread of the app: a frame of the daemon, or why the connection ended.
pub(crate) type Heard = Result<Frame, Error>;

/// The daemon of a machine, which the window dials on the runtime of the app, and dials again each
/// time that the connection ends, and what hears each of its frames on the thread of the app. The
/// window asks each verb after the hello and the cards through the asker that it is given.
pub(crate) fn dialed(app: &App, machine: Machine, on: impl FnMut(&App, Heard) + 'static) -> Asker {
  let (tell, frames) = mpsc::channel(WAITING);
  let (requests, outgoing) = mpsc::unbounded_channel();
  // The task ends when the window lets the frames go, and with the runtime of the app.
  drop(app.runtime().spawn(async move {
    let (mut link, mut outgoing) =
      (Link { machine, wait: FIRST, seen: None, asked: None }, outgoing);
    loop {
      let Err(end) = held(&mut link, &mut outgoing, &tell).await;
      // A window that is gone hears no end.
      if tell.send(Err(end)).await.is_err() {
        return;
      }
      time::sleep(link.wait).await;
      link.wait = (link.wait * 2).min(SLOWEST);
    }
  }));
  hearing(app, frames, on);
  Asker { requests }
}

/// The window looks for the daemon of each machine of the tailnet after a wait, then again at each
/// [`LOOK`], and gives each one that it finds, once, to what hears it on the thread of the app.
pub(crate) fn looked(
  app: &App,
  mut looking: Looking,
  wait: Duration,
  mut on: impl FnMut(&App, Machine) + 'static,
) {
  let look = async move {
    time::sleep(wait).await;
    let found = looking.found().await;
    (looking, found)
  };
  drop(app.spawn(look, move |cx, (looking, found)| {
    for machine in found {
      on(cx.app(), machine);
    }
    looked(cx.app(), looking, LOOK, on);
  }));
}

/// What sends the requests of the window to the daemon of a machine, each under the id that the
/// client core gave it.
pub(crate) struct Asker {
  requests: UnboundedSender<Request>,
}

impl Asker {
  /// A request sent to the daemon.
  pub(crate) fn send(&mut self, request: Request) {
    // A task that ended takes no request, and the window heard why it ended.
    drop(self.requests.send(request));
  }
}

/// One connection held until it ends, and why it ended: it says the hello and the ask of the
/// cards, then writes each request of the window in order, and reads each frame of the daemon. A
/// request that waits from a connection before it goes nowhere, and the wait before the next dial
/// starts again once the machine answers. A window that is gone ends it.
async fn held(
  link: &mut Link,
  outgoing: &mut UnboundedReceiver<Request>,
  tell: &Sender<Heard>,
) -> Result<Infallible, Error> {
  let machine = &mut link.machine;
  let mut lines = Framed::new(machine.reach().await?, LinesCodec::new_with_max_length(BOUND));
  link.wait = FIRST;
  while outgoing.try_recv().is_ok() {}
  // A window that only reads on the machine asks its daemon for the role that reads.
  for request in greeting(Kind::Window, link.machine.reads.then_some(Role::Read)) {
    lines.send(serde_json::to_string(&request)?).await?;
  }
  // The program of the daemon, as its welcome names it, until the welcome comes.
  let mut daemon = None;
  let mut follow = time::interval(FOLLOW);
  loop {
    tokio::select! {
      request = outgoing.recv() => {
        let request = request.ok_or(Error::Closed)?;
        lines.send(serde_json::to_string(&request)?).await?;
      }
      read = lines.next() => {
        let frame: Frame = serde_json::from_str(&read.ok_or(Error::Closed)??)?;
        match &frame {
          Frame::Reply { answer: Answer::Welcome { program, .. }, .. } => {
            daemon = Some(program.clone());
          }
          // A daemon of an older protocol refuses the window and closes: its program is not the
          // one that serves it now.
          Frame::Refused { error: ProtocolError::Protocol { ours, .. }, .. } if *ours < PROTOCOL => {
            daemon = Some(None);
          }
          _ => {}
        }
        if tell.send(Ok(frame)).await.is_err() {
          return Err(Error::Closed);
        }
      }
      _ = follow.tick() => {}
    }
    if let Some(program) = &daemon {
      link.followed(program.as_deref()).map_err(Error::Upgrade)?;
    }
  }
}

/// The connection to one machine as the task holds it from one dial to the next: the machine, the
/// wait before its next dial, the program that serves its daemon at the last look, and the program
/// that the window last asked the handover of.
struct Link {
  machine: Machine,
  wait: Duration,
  seen: Option<String>,
  asked: Option<String>,
}

impl Link {
  /// The handover asked of the program that serves the daemon, once it holds a build that the
  /// daemon does not run and stood still since the last look, so the build wrote it whole. The
  /// window asks it once for each build, so a program that fails to take the place of the daemon
  /// is not asked again, and its failure ends the connection with its reason.
  fn followed(&mut self, daemon: Option<&str>) -> io::Result<()> {
    let now = self.machine.program();
    let still = now.is_some() && now == self.seen;
    let asks = still && now.as_deref() != daemon && now != self.asked;
    self.seen.clone_from(&now);
    if asks {
      self.asked = now;
      self.machine.upgrade()?;
    }
    Ok(())
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
