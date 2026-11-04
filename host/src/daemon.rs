//! The daemon of a machine, which holds each life by the path of its record (#983, section 8).
//!
//! A life lives while a holder holds it, each window that attached it and each pin, and for the
//! grace after its last holder left: then it writes its checkpoint and ends. Work that runs holds
//! nothing. The daemon listens before it opens a life, and each life opens on a thread of its own,
//! so the daemon answers each verb while a life opens, and a fault of one life ends that life
//! alone. Presence, who is attached now, is a verb and a frame of the daemon, never a fact of a
//! record.
//!
//! One task owns the lives and their holders, and each peer asks it through a [`Daemon`]. A daemon
//! of a new build takes the place of the one that runs only through [`upgrade`], never by a
//! takeover: the daemon that runs writes the checkpoint of each life, stops to listen, and gives
//! its lives and their pins, which the new one opens again. Each window then dials again and
//! attaches, with no loss.

use std::{
  collections::HashMap, fmt::Display, iter, path::Path, pin::pin, sync::Arc, time::Duration,
};

use furb::{
  life::{Log, Opening},
  wire::parsed,
};
use futures::{SinkExt, Stream, StreamExt, future};
use tokio::{
  sync::{mpsc, oneshot},
  task::JoinSet,
  time::{Instant, sleep_until},
};
use tokio_util::codec::{Framed, LinesCodec};

use crate::{
  answers::Answerer,
  error::{DaemonError, ProtocolError, TransportError},
  event::{Event, Seq},
  holders::{Holders, Window},
  life::{Command, Life, LifeHandle, Reply},
  protocol::{Answer, BOUND, Frame, Handed, Hello, Kind, Listed, PROTOCOL, Peer, Request, Verb},
  serve::{Host, Outbox, serve},
  transport::{Accepted, Dialer, Identity, Listener, Local},
};

/// How long a life with no holder lives, unless the settings of the daemon say otherwise.
pub const GRACE: Duration = Duration::from_secs(30);

/// What the daemon opens each life on.
#[derive(Clone)]
pub struct Settings {
  /// The opening of each life, whose record and folder the daemon sets. Its log takes the reports
  /// of the daemon, as a peer that the transport refused, and stderr takes them when it has none.
  pub opening: Opening,
  /// How long a life with no holder lives before it writes its checkpoint and ends.
  pub grace: Duration,
}

/// The daemon of the machine served on the transport where it listens, until a daemon of a new
/// build takes its place.
pub async fn run(local: &Local, listener: Listener, settings: Settings) -> Result<(), DaemonError> {
  served(local, listener, settings, Vec::new()).await
}

/// This daemon in the place of the daemon that runs on the transport: that one writes the
/// checkpoint of each life, stops to listen and gives its lives and their pins, which this one
/// opens again. When no daemon answers, this one starts as [`run`] does.
pub async fn upgrade(local: &Local, settings: Settings) -> Result<(), DaemonError> {
  let (handed, asker) = match local.dial().await {
    Ok(stream) => handover(stream).await.map(|(handed, asker)| (handed, Some(asker)))?,
    Err(TransportError::Dial { .. }) => (Vec::new(), None),
    Err(no) => return Err(no.into()),
  };
  let listener =
    if asker.is_some() { local.listen_after_handover().await? } else { local.listen().await? };
  // The daemon that ran closes its windows once this peer is gone, so each window dials again
  // when this daemon listens.
  drop(asker);
  served(local, listener, settings, handed).await
}

/// The stream of the peer that asks the handover.
type Asker = Framed<<Local as Dialer>::Stream, LinesCodec>;

/// The lives that the daemon of a stream gave at its handover, and the stream.
async fn handover(stream: <Local as Dialer>::Stream) -> Result<(Vec<Handed>, Asker), DaemonError> {
  let mut lines = Framed::new(stream, LinesCodec::new_with_max_length(BOUND));
  let hello = Verb::Hello(Hello { protocol: PROTOCOL, kind: Kind::Daemon, name: None });
  for (id, verb) in [(0, hello), (1, Verb::Handover)] {
    let request = serde_json::to_string(&Request { id, verb }).map_err(TransportError::from)?;
    lines.send(request).await.map_err(TransportError::from)?;
    loop {
      let heard = lines.next().await.ok_or(DaemonError::Unanswered)?;
      let heard = heard.map_err(TransportError::from)?;
      match serde_json::from_str::<Frame>(&heard).map_err(DaemonError::Unread)? {
        Frame::Reply { answer: Answer::Handed(given), .. } => return Ok((given, lines)),
        Frame::Reply { id: got, .. } if got == id => break,
        Frame::Refused { error, .. } => return Err(error.into()),
        _ => {}
      }
    }
  }
  Err(DaemonError::Unanswered)
}

/// The daemon served from a listener, with the lives that a daemon before it gave: each peer that
/// the transport accepts, until a handover. The listener stops first, then the peer that asked the
/// handover gets the lives, and each other peer closes once that peer is gone.
async fn served(
  local: &Local,
  mut listener: Listener,
  settings: Settings,
  handed: Vec<Handed>,
) -> Result<(), DaemonError> {
  let log = settings.opening.log.clone();
  let (daemon, mut leave) = Daemon::spawn(settings, handed);
  let mut peers = JoinSet::new();
  let leaving = loop {
    tokio::select! {
      accepted = listener.accept() => match accepted {
        Ok(stream) => {
          peers.spawn(peer(daemon.clone(), local.clone(), stream));
        }
        Err(no) => report(log.as_ref(), &no),
      },
      Some(ended) = peers.join_next() => match ended {
        Ok(Ok(())) => {}
        Ok(Err(no)) => report(log.as_ref(), &no),
        Err(no) => report(log.as_ref(), &no),
      },
      leaving = &mut leave => break leaving,
    }
  };
  drop(listener);
  if let Ok(Leaving { handed, asker, reply }) = leaving {
    reply.send(Ok(Answer::Handed(handed))).ok();
    asker.closed().await;
  }
  Ok(())
}

/// One peer of the local transport, identified, then served until it leaves.
async fn peer(daemon: Daemon, local: Local, stream: Accepted) -> Result<(), TransportError> {
  let who = local.identify(&stream)?;
  serve(&daemon, stream, who).await
}

/// A report of the daemon that no model reads, to the log of the opening, or to stderr.
fn report(log: Option<&Log>, what: &dyn Display) {
  match log {
    Some(Log(tell)) => tell(&format!("furb host: {what}")),
    None => eprintln!("furb host: {what}"),
  }
}

/// The door of a peer to the task that owns the lives.
#[derive(Clone)]
struct Daemon {
  asks: mpsc::UnboundedSender<Asked>,
}

type Replied = oneshot::Sender<Result<Answer, ProtocolError>>;

/// What the task that owns the lives hears.
enum Asked {
  /// A verb of the daemon, from a peer.
  Verb { peer: Peer, verb: Box<Verb>, outbox: Outbox, reply: Replied },
  /// The handle of a life, for a verb of that life.
  Life { life: String, reply: oneshot::Sender<Result<LifeHandle, ProtocolError>> },
  /// A window whose peer is gone.
  Left { life: String, outbox: Outbox },
  /// A life whose thread told its end, in one of its births.
  Ended { life: String, born: u64 },
}

/// The handover as the listener takes it: the lives to give, the peer that asked them, and its
/// reply.
struct Leaving {
  handed: Vec<Handed>,
  asker: Outbox,
  reply: Replied,
}

impl Daemon {
  /// The task that owns the lives started, with the lives that a daemon before it gave, and what
  /// it says to the listener at a handover.
  fn spawn(settings: Settings, handed: Vec<Handed>) -> (Daemon, oneshot::Receiver<Leaving>) {
    let (asks, heard) = mpsc::unbounded_channel();
    let (leave, leaving) = oneshot::channel();
    let mut lives =
      Lives { settings, held: HashMap::new(), asks: asks.downgrade(), born: 0, leave };
    lives.adopt(handed);
    tokio::spawn(lives.kept(heard));
    (Daemon { asks }, leaving)
  }

  /// What the task that owns the lives answers.
  async fn asked<T>(
    &self,
    asked: impl FnOnce(oneshot::Sender<T>) -> Asked,
  ) -> Result<T, ProtocolError> {
    let (reply, replied) = oneshot::channel();
    if self.asks.send(asked(reply)).is_err() {
      return Err(ProtocolError::Stopped);
    }
    replied.await.or(Err(ProtocolError::Stopped))
  }
}

impl Host for Daemon {
  async fn answer(
    &self,
    peer: &Peer,
    verb: Verb,
    outbox: &Outbox,
  ) -> Result<Answer, ProtocolError> {
    match routed(verb) {
      Routed::Life(life, command) => {
        let handle = self.asked(|reply| Asked::Life { life, reply }).await??;
        Ok(handle.send(command).await?.into())
      }
      Routed::Daemon(verb) => {
        let (peer, outbox, verb) = (peer.clone(), outbox.clone(), Box::new(verb));
        self.asked(|reply| Asked::Verb { peer, verb, outbox, reply }).await?
      }
    }
  }
}

/// Where a verb goes: to a life, as the command that the life runs, or to the daemon.
enum Routed {
  Life(String, Command),
  Daemon(Verb),
}

fn routed(verb: Verb) -> Routed {
  let (life, command) = match verb {
    Verb::Get { life, act } => (life, Command::Get { id: act }),
    Verb::Peek { life, act } => (life, Command::Peek { id: act }),
    Verb::Transcript { life, on, of } => (life, Command::Transcript { on, of }),
    Verb::Turns { life, on } => (life, Command::Turns { on }),
    Verb::Program { life, on } => (life, Command::Program { on }),
    Verb::Standing { life } => (life, Command::Standing),
    Verb::Cwd { life, on } => (life, Command::Cwd { on }),
    Verb::Inspect { life, name, on } => (life, Command::Inspect { name, on }),
    Verb::Pending { life } => (life, Command::Pending),
    Verb::Prompt { life, shape, message, to, on } => {
      (life, Command::Prompt { shape, message, to, on })
    }
    Verb::Answer { life, act, line } => (life, Command::Answer { id: act, line }),
    Verb::Pause { life, act, why } => (life, Command::Pause { id: act, why }),
    Verb::Wake { life, act } => (life, Command::Wake { id: act }),
    Verb::Cancel { life, act, why } => (life, Command::Cancel { id: act, why }),
    Verb::Chain { life, label, source, on } => (life, Command::Chain { label, source, on }),
    Verb::Run { life, word, on } => (life, Command::Rung { word, on }),
    Verb::Close { life, act, value } => (life, Command::Close { id: act, value }),
    Verb::Grant { life, usd, share, on } => (life, Command::Grant { usd, share, on }),
    Verb::Bash { life, command, stdin, feed, on } => {
      (life, Command::Bash { command, stdin: if feed { None } else { Some(stdin) }, on })
    }
    Verb::Read { life, path, on } => (life, Command::Read { path, on }),
    Verb::Write { life, path, content, on } => (life, Command::Write { path, content, on }),
    Verb::Names { life, on } => (life, Command::Names { on }),
    Verb::Gate { life, word, on } => (life, Command::Gate { word, on }),
    Verb::Stand { life } => (life, Command::Stand),
    Verb::Seen { life, about } => (life, Command::Seen { about }),
    Verb::Queue { life, entry, hash, shape, message, on } => {
      (life, Command::Queue { entry, hash, shape, message, on })
    }
    other => return Routed::Daemon(other),
  };
  Routed::Life(life, command)
}

impl From<Reply> for Answer {
  fn from(reply: Reply) -> Self {
    match reply {
      Reply::Done => Answer::Done,
      Reply::Act(id) => Answer::Act(id),
      Reply::Value(value) => Answer::Value(value),
      Reply::Values(values) => Answer::Values(values),
      Reply::Inspected { kind, repr, value } => Answer::Inspected { kind, repr, value },
    }
  }
}

/// What an open names of the life that it makes: its folder, the actor, the roster and the
/// extensions that its peer chose, and the peer that answers its models, when one does. A choice
/// that is unsaid takes the one of the settings of the daemon.
#[derive(Clone, Default)]
struct Birth {
  cwd: String,
  actor: Option<String>,
  roster: Option<Vec<String>>,
  extensions: Option<bool>,
  answerer: Option<Answerer>,
}

/// The lives of the daemon, each under the path of its record, which one task owns: the count of
/// the births of a life, and where the task tells the listener of a handover.
struct Lives {
  settings: Settings,
  held: HashMap<String, Held>,
  asks: mpsc::WeakUnboundedSender<Asked>,
  born: u64,
  leave: oneshot::Sender<Leaving>,
}

/// A life of the daemon: its handle, its folder, its birth, its stage, and the peer that answers
/// its models, when one does.
struct Held {
  handle: LifeHandle,
  cwd: String,
  born: u64,
  stage: Stage,
  answerer: Option<Answerer>,
}

/// A life lives under its holders, or ends, with each open that waits for its end to open it
/// again, in its folder.
enum Stage {
  Living(Holders),
  Ending(Vec<(Birth, Replied)>),
}

impl Lives {
  /// The task: each ask in turn, and the end of each life whose grace passed, until a handover or
  /// until each door of the daemon is gone.
  async fn kept(mut self, mut heard: mpsc::UnboundedReceiver<Asked>) {
    loop {
      let asked = tokio::select! {
        asked = heard.recv() => asked,
        () = until(self.due()) => {
          self.expire();
          continue;
        }
      };
      match asked {
        None => return,
        Some(Asked::Verb { verb, outbox, reply, .. }) if *verb == Verb::Handover => {
          return self.handover(outbox, reply);
        }
        Some(asked) => self.heard(asked),
      }
    }
  }

  fn heard(&mut self, asked: Asked) {
    match asked {
      Asked::Verb { peer, verb, outbox, reply } => self.verb(peer, *verb, &outbox, reply),
      Asked::Life { life, reply } => {
        let found = self.held.get(&life).map(|held| held.handle.clone());
        reply.send(found.ok_or(ProtocolError::Closed { life })).ok();
      }
      Asked::Left { life, outbox } => {
        if let Ok((_, holders)) = living(&mut self.held, &life)
          && holders.detach(&outbox)
        {
          holders.present(&life);
        }
      }
      Asked::Ended { life, born } => self.ended(life, born),
    }
  }

  fn verb(&mut self, peer: Peer, verb: Verb, outbox: &Outbox, reply: Replied) {
    let answer = match verb {
      Verb::Open { life, cwd, actor, roster, extensions, answers } => {
        let answerer = answers.then(|| Answerer::new(life.clone(), outbox.clone()));
        let cwd = cwd.unwrap_or_default();
        return self.open(life, Birth { cwd, actor, roster, extensions, answerer }, reply);
      }
      Verb::List => Ok(Answer::Lives(self.listed())),
      Verb::Attach { life, from } => self.attach(&life, from, peer, outbox),
      Verb::Detach { life } => living(&mut self.held, &life).and_then(|(_, holders)| {
        holders
          .detach(outbox)
          .then(|| holders.present(&life))
          .map(|()| Answer::Done)
          .ok_or(ProtocolError::Unattached { life: life.clone() })
      }),
      Verb::Pin { life, holder } => living(&mut self.held, &life).map(|(_, holders)| {
        holders.pin(holder);
        Answer::Done
      }),
      Verb::Unpin { life, holder } => living(&mut self.held, &life).and_then(|(_, holders)| {
        let unpinned = holders.unpin(&holder);
        unpinned
          .then_some(Answer::Done)
          .ok_or(ProtocolError::Unpinned { life: life.clone(), holder })
      }),
      Verb::Presence { life } => {
        living(&mut self.held, &life).map(|(_, holders)| Answer::Presence(holders.presence()))
      }
      Verb::Answered { life, ask, turn, why } => self.answerer(&life).and_then(|one| {
        let turn = match turn {
          Some(turn) => parsed(&turn.to_string()).map_err(|fault| fault.to_string()),
          None => Err(why.unwrap_or_else(|| "the window gave no turn".to_owned())),
        };
        one.answered(ask, turn).map(|()| Answer::Done)
      }),
      Verb::Wrote { life, ask, text, thinking } => {
        self.answerer(&life).and_then(|one| one.wrote(ask, &text, &thinking).map(|()| Answer::Done))
      }
      // The door refuses a second hello, and each verb of a life goes to its life, so each other
      // verb is a frame of a view.
      _ => Err(ProtocolError::Viewless),
    };
    reply.send(answer).ok();
  }

  /// The life of a record opened in a folder, the folder of its record when none is given, unless
  /// it lives. An open of a life that ends waits for its end, then opens it again.
  fn open(&mut self, life: String, mut birth: Birth, reply: Replied) {
    let record = Path::new(&life);
    if !record.is_absolute() {
      reply.send(Err(ProtocolError::Relative { life })).ok();
      return;
    }
    if birth.cwd.is_empty() {
      birth.cwd = record.parent().map_or_else(String::new, |one| one.display().to_string());
    }
    let opened = match self.held.get_mut(&life).map(|held| &mut held.stage) {
      Some(Stage::Living(_)) => Ok(Answer::Done),
      Some(Stage::Ending(waiting)) => return waiting.push((birth, reply)),
      None => self.opened(life, birth).map(|()| Answer::Done),
    };
    reply.send(opened).ok();
  }

  /// A life born on a thread of its own, with no holder yet, and watched for its end.
  fn opened(&mut self, life: String, birth: Birth) -> Result<(), ProtocolError> {
    let Birth { cwd, actor, roster, extensions, answerer } = birth;
    let base = &self.settings.opening;
    let answer = answerer.as_ref().map(|one| furb::life::Answer(one.hosted()));
    let opening = Opening {
      record: Some(life.clone()),
      directory: cwd.clone(),
      actor: actor.or_else(|| base.actor.clone()),
      roster: roster.or_else(|| base.roster.clone()),
      extensions: extensions.or(base.extensions),
      answer: answer.or_else(|| base.answer.clone()),
      ..base.clone()
    };
    let handle = Life::spawn(opening)?;
    self.born += 1;
    tokio::spawn(watched(handle.clone(), life.clone(), self.born, self.asks.clone()));
    let stage = Stage::Living(Holders::new(self.settings.grace));
    self.held.insert(life, Held { handle, cwd, born: self.born, stage, answerer });
    Ok(())
  }

  /// The peer that answers the models of a life, or the refusal that none does.
  fn answerer(&self, life: &str) -> Result<&Answerer, ProtocolError> {
    match self.held.get(life) {
      Some(Held { answerer: Some(one), .. }) => Ok(one),
      Some(_) => Err(ProtocolError::Answerless { life: life.to_owned() }),
      None => Err(ProtocolError::Closed { life: life.to_owned() }),
    }
  }

  /// The lives that a daemon before this one gave, opened again, each with its pins.
  fn adopt(&mut self, handed: Vec<Handed>) {
    for Handed { life, cwd, pins } in handed {
      if let Err(no) = self.opened(life.clone(), Birth { cwd, ..Birth::default() }) {
        report(self.settings.opening.log.as_ref(), &format_args!("{life} did not open: {no}"));
      } else if let Ok((_, holders)) = living(&mut self.held, &life) {
        for pin in pins {
          holders.pin(pin);
        }
      }
    }
  }

  /// A window attached to a life, which hears each event of the life from a place in its log.
  fn attach(
    &mut self,
    life: &str,
    from: Seq,
    peer: Peer,
    outbox: &Outbox,
  ) -> Result<Answer, ProtocolError> {
    let (handle, holders) = living(&mut self.held, life)?;
    let teller = told(life.to_owned(), handle.events(from), outbox.clone(), self.asks.clone());
    let told = tokio::spawn(teller).abort_handle();
    holders.attach(Window { peer, outbox: outbox.clone(), told });
    holders.present(life);
    Ok(Answer::Done)
  }

  /// Each life of the daemon, by the path of its record, and whether it lives or ends.
  fn listed(&self) -> Vec<Listed> {
    let mut listed: Vec<Listed> = self
      .held
      .iter()
      .map(|(life, held)| Listed {
        life: life.clone(),
        open: matches!(held.stage, Stage::Living(_)),
      })
      .collect();
    listed.sort_by(|one, other| one.life.cmp(&other.life));
    listed
  }

  /// When the first life with no holder ends.
  fn due(&self) -> Option<Instant> {
    let idle = self.held.values().filter_map(|held| match &held.stage {
      Stage::Living(holders) => holders.idle(),
      Stage::Ending(_) => None,
    });
    idle.min()
  }

  /// Each life whose grace passed, told to write its checkpoint and end.
  fn expire(&mut self) {
    let now = Instant::now();
    for held in self.held.values_mut() {
      if let Stage::Living(holders) = &held.stage
        && holders.idle().is_some_and(|at| at <= now)
      {
        held.stage = Stage::Ending(Vec::new());
        let handle = held.handle.clone();
        tokio::spawn(async move { end(&handle).await });
      }
    }
  }

  /// A life that ended, in the birth that the daemon holds: gone, and opened again for each open
  /// that waited for its end.
  fn ended(&mut self, life: String, born: u64) {
    if self.held.get(&life).is_none_or(|held| held.born != born) {
      return;
    }
    let Some(Held { stage: Stage::Ending(waiting), .. }) = self.held.remove(&life) else { return };
    let mut waiting = waiting.into_iter();
    if let Some((birth, first)) = waiting.next() {
      let opened = self.opened(life, birth).map(|()| Answer::Done);
      for one in iter::once(first).chain(waiting.map(|(_, one)| one)) {
        one.send(opened.clone()).ok();
      }
    }
  }

  /// The handover: each window detached, each life told to write its checkpoint and end, then the
  /// lives that lived, with their pins, given to the peer that asked once the listener stopped.
  fn handover(self, asker: Outbox, reply: Replied) {
    let (mut handed, mut lives) = (Vec::new(), Vec::new());
    for (life, held) in self.held {
      match held.stage {
        Stage::Living(holders) => {
          handed.push(Handed { life, cwd: held.cwd, pins: holders.release() });
        }
        Stage::Ending(waiting) => {
          for (_, one) in waiting {
            one.send(Err(ProtocolError::Stopped)).ok();
          }
        }
      }
      lives.push(held.handle);
    }
    tokio::spawn(gave(lives, self.leave, Leaving { handed, asker, reply }));
  }
}

/// The handle and the holders of a life that lives, or the refusal that none lives on its record.
fn living<'a>(
  held: &'a mut HashMap<String, Held>,
  life: &str,
) -> Result<(&'a LifeHandle, &'a mut Holders), ProtocolError> {
  match held.get_mut(life) {
    Some(Held { handle, stage: Stage::Living(holders), .. }) => Ok((handle, holders)),
    _ => Err(ProtocolError::Closed { life: life.to_owned() }),
  }
}

/// The moment when a life with no holder ends, or never.
async fn until(due: Option<Instant>) {
  match due {
    Some(at) => sleep_until(at).await,
    None => future::pending().await,
  }
}

/// A life told to write its checkpoint and end, once its commands before ran. A fault of that
/// checkpoint is the fault of the event `Ended` of the life, which its windows and its watch hear.
async fn end(life: &LifeHandle) {
  life.send(Command::End).await.ok();
}

/// The end of a life told to the daemon, once the life told it.
async fn watched(
  life: LifeHandle,
  record: String,
  born: u64,
  asks: mpsc::WeakUnboundedSender<Asked>,
) {
  let mut events = pin!(life.events(0));
  while events.next().await.is_some() {}
  if let Some(asks) = asks.upgrade() {
    asks.send(Asked::Ended { life: record, born }).ok();
  }
}

/// The events of a life pushed to the peer of a window, until the life ends or the peer is gone,
/// which the daemon hears.
async fn told(
  life: String,
  events: impl Stream<Item = (Seq, Arc<Event>)>,
  outbox: Outbox,
  asks: mpsc::WeakUnboundedSender<Asked>,
) {
  let mut events = pin!(events);
  loop {
    tokio::select! {
      () = outbox.closed() => break,
      next = events.next() => match next {
        Some((seq, event)) => {
          let frame = Frame::Event { life: life.clone(), seq, event: Event::clone(&event) };
          if outbox.send(frame).is_err() {
            break;
          }
        }
        None => return,
      },
    }
  }
  if let Some(asks) = asks.upgrade() {
    asks.send(Asked::Left { life, outbox }).ok();
  }
}

/// The handover once each life wrote its checkpoint and ended, given to the listener, or refused
/// when the listener is gone.
async fn gave(lives: Vec<LifeHandle>, leave: oneshot::Sender<Leaving>, leaving: Leaving) {
  future::join_all(lives.iter().map(end)).await;
  if let Err(Leaving { reply, .. }) = leave.send(leaving) {
    reply.send(Err(ProtocolError::Stopped)).ok();
  }
}
