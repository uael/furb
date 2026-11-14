//! The daemon of a machine, which holds each life by the path of its record (#983, section 8).
//!
//! A life lives while a holder holds it, each window that attached it and each pin, and for the
//! grace after its last holder left: then it writes its image and ends. Work that runs holds
//! nothing. The daemon listens before it opens a life, and each life opens on a thread of its own,
//! so the daemon answers each verb while a life opens, and a fault of one life ends that life
//! alone. Presence, who is attached now, is a verb and a frame of the daemon, never a fact of a
//! record.
//!
//! One task owns the lives and their holders, and each peer asks it through a [`Daemon`]. A daemon
//! of a new build takes the place of the one that runs only through [`upgrade`], never by a
//! takeover: the daemon that runs writes the checkpoint of each life, stops to listen, and gives
//! its lives and their pins, which the new one opens again. Each window then dials again and
//! attaches, with no loss. The handover waits for each life to end, however long it takes: a life
//! given up before its end would still hold the lease of its record and the facts that its record
//! does not hold yet, and the new daemon would open a second life on that record.
//!
//! When the operator turns it on, the daemon also listens on the address of the machine on the
//! tailnet, by TCP and by WebSocket, and lets in each peer whose login the access list names, with
//! the role that it gives (#983, D6). A web view reaches no socket of the machine, so the daemon
//! also opens a door of the windows, a WebSocket on loopback, which lets in the user alone: the
//! peer that gives the token that the daemon wrote in a file of its folder.

#![expect(
  clippy::too_many_arguments,
  reason = "debt: a function of more than 4 parameters takes a struct, rule 8 of the skill rust"
)]
use std::{
  collections::{HashMap, HashSet},
  fmt::Display,
  mem,
  net::{IpAddr, SocketAddr},
  path::{Path, PathBuf},
  pin::pin,
  sync::Arc,
  time::Duration,
};

use furb::{
  engine::OPERATOR,
  life::{Answer as Answered, Log, Opening},
  view::{self, WATCH},
  wire::parsed,
  world::{
    Members, Roster,
    codex::{self, SignIn},
    config::Change,
  },
};
use futures::{SinkExt, Stream, StreamExt, future};
use serde_json::Value;
use tokio::{
  net::{TcpListener, TcpStream},
  sync::{mpsc, oneshot, watch},
  task::{JoinSet, spawn_blocking},
  time::{Instant, Interval, MissedTickBehavior, interval, sleep_until},
};
use tokio_util::codec::{Framed, LinesCodec};

use crate::{
  access::{Access, Ports},
  answers::Answerer,
  cards::{self, Told},
  error::{DaemonError, ProtocolError, TransportError},
  event::{Event, Seq},
  holders::{Holders, Window},
  life::{Command, Life, LifeHandle, Reply},
  opens::Heard,
  processes::{self, Played},
  protocol::{
    Answer, BOUND, Card, Config, Frame, Handed, Hello, Kind, Listed, Mount, Open, PROTOCOL, Peer,
    Request, Verb,
  },
  rebirth::{Asks, Heap},
  records,
  serve::{Host, Outbox, fixed, serve},
  stall::{Quiet, Stall},
  transport::{
    Accepted, Dialer, Door, Identity, Listener, Local, Tailscale, Tcp, Ticket, own, upgraded,
  },
  views::{Skill, Views},
};

/// How long a life with no holder lives, unless the settings of the daemon say otherwise.
pub const GRACE: Duration = Duration::from_secs(30);

/// How many frames of the events of a life may wait for a window before the daemon reads the next
/// event: a window that reads slowly stands behind the life, and reads the log of the life again
/// once it reads on, in the place of a copy of each event in its outbox. That read holds a place
/// and a reference for each event of the rest of the log, and keeps each such event until the
/// window has it.
const WAITING: usize = 256;

/// How often the daemon reads its heap, when it watches one.
const WEIGH: Duration = Duration::from_secs(1);

/// What the daemon opens each life on.
#[derive(Clone)]
pub struct Settings {
  /// The opening of each life, whose record and folder the daemon sets. Its log takes the reports
  /// of the daemon, as a peer that the transport refused, and stderr takes them when it has none.
  pub opening: Opening,
  /// How long a life with no holder lives before it writes its image and ends.
  pub grace: Duration,
  /// How long a life that holds an open act says no fact before the daemon tells each of its
  /// windows and its root chain that it stalled.
  pub stall: Duration,
  /// The daemon on the tailnet, when the operator turns it on in `host.json`.
  pub tailnet: Option<Tailnet>,
  /// The file where the daemon writes the address and the token of its door of the windows, the
  /// WebSocket on loopback that a web view dials, when it opens one.
  pub door: Option<PathBuf>,
  /// The folder of the local transport, where the process of each skill that a life plays dials
  /// the daemon. With none, the daemon starts no process.
  pub host: Option<PathBuf>,
  /// The heap of the daemon, past whose share the life that weighs the most is born again. With
  /// none, only the tail of a life makes it be born again.
  pub heap: Option<Heap>,
}

/// The daemon on the tailnet: who each peer is, and the ports where it listens on the address of
/// the machine.
#[derive(Debug, Clone)]
pub struct Tailnet {
  pub identity: Tailscale,
  pub ports: Ports,
}

/// The daemon of the machine served on the transport where it listens, until a daemon of a new
/// build takes its place. Its identity says who each peer is and the role that it holds.
pub async fn run<I>(identity: &I, listener: Listener, settings: Settings) -> Result<(), DaemonError>
where
  I: Identity<Stream = Accepted> + Clone + Send + 'static,
{
  served(identity, listener, settings, Vec::new()).await
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
  let hello = Verb::Hello(Hello { protocol: PROTOCOL, kind: Kind::Daemon, name: None, role: None });
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
async fn served<I>(
  identity: &I,
  mut listener: Listener,
  settings: Settings,
  handed: Vec<Handed>,
) -> Result<(), DaemonError>
where
  I: Identity<Stream = Accepted> + Clone + Send + 'static,
{
  let log = settings.opening.log.clone();
  let doors = match &settings.tailnet {
    Some(tailnet) => Doors::listen(tailnet, log.as_ref()).await,
    None => None,
  };
  let door = match &settings.door {
    Some(file) => Some(Door::open(file).await?),
    None => None,
  };
  let (grace, lasting) = (settings.grace, doors.is_some());
  // A daemon that no peer reached yet waits for the first one, which the program that started it
  // sends.
  let mut met = !handed.is_empty();
  let (daemon, mut leave, mut empty) = Daemon::spawn(settings, handed);
  let mut peers = JoinSet::new();
  let mut quiet = None;
  let leaving = loop {
    tokio::select! {
      accepted = listener.accept() => match accepted {
        Ok(stream) => {
          met = true;
          peers.spawn(peer(daemon.clone(), identity.clone(), stream));
        }
        Err(no) => report(log.as_ref(), &no),
      },
      arrived = Doors::arrival(doors.as_ref()) => match arrived {
        Ok((identity, arrival)) => {
          peers.spawn(remote(daemon.clone(), identity, arrival));
        }
        Err(no) => report(log.as_ref(), &no),
      },
      knocked = knock(door.as_ref()) => match knocked {
        Ok((stream, ticket)) => {
          met = true;
          peers.spawn(window(daemon.clone(), ticket, stream));
        }
        Err(no) => report(log.as_ref(), &no),
      },
      Some(ended) = peers.join_next() => match ended {
        Ok(Ok(())) => {}
        Ok(Err(no)) => report(log.as_ref(), &no),
        Err(no) => report(log.as_ref(), &no),
      },
      Ok(()) = empty.changed() => {}
      () = until(quiet) => break None,
      leaving = &mut leave => break leaving.ok(),
    }
    // The daemon of this machine ends once it holds no peer and no life for its grace, so no
    // daemon outlives the windows that started it. A daemon on the tailnet serves its doors.
    let idle = met && !lasting && peers.is_empty() && *empty.borrow();
    quiet = if idle { quiet.or_else(|| Some(Instant::now() + grace)) } else { None };
  };
  drop((listener, doors, door));
  if let Some(Leaving { handed, asker, reply }) = leaving {
    reply.send(Ok(Answer::Handed(handed))).ok();
    asker.closed().await;
  }
  Ok(())
}

/// One peer of the transport, identified, then served until it leaves.
async fn peer<I: Identity<Stream = Accepted>>(
  daemon: Daemon,
  identity: I,
  stream: Accepted,
) -> Result<(), TransportError> {
  let who = identity.identify(&stream).await?;
  let granted = fixed(who.role);
  serve(&daemon, stream, who, granted).await
}

/// One peer of the tailnet, identified by Tailscale, upgraded when it came by WebSocket, then
/// served until it leaves.
async fn remote(
  daemon: Daemon,
  identity: Tailscale,
  arrival: Arrival,
) -> Result<(), TransportError> {
  match arrival {
    Arrival::Tcp(stream) => {
      let who = identity.identify(&stream).await?;
      let granted = identity.follow(&who);
      serve(&daemon, stream, who, granted).await
    }
    Arrival::WebSocket(stream) => {
      let who = identity.identify(&stream).await?;
      let granted = identity.follow(&who);
      serve(&daemon, upgraded(stream).await?, who, granted).await
    }
  }
}

/// The next peer of the door of the windows, or never with no door.
async fn knock(door: Option<&Door>) -> Result<(TcpStream, Ticket), TransportError> {
  match door {
    Some(door) => door.accept().await,
    None => future::pending().await,
  }
}

/// One peer of the door of the windows, let in once it gave the token, then served until it
/// leaves.
async fn window(daemon: Daemon, ticket: Ticket, stream: TcpStream) -> Result<(), TransportError> {
  let (stream, who) = ticket.admitted(stream).await?;
  let granted = fixed(who.role);
  serve(&daemon, stream, who, granted).await
}

/// The listeners of the daemon on the address of the machine on the tailnet, each where the
/// settings give a port.
struct Doors {
  identity: Tailscale,
  tcp: Option<TcpListener>,
  websocket: Option<TcpListener>,
}

/// A stream of a peer of the tailnet, by the transport that it came by.
enum Arrival {
  Tcp(TcpStream),
  WebSocket(TcpStream),
}

impl Doors {
  /// The doors listening on the first address of the machine on the tailnet, an address of IPv4
  /// before one of IPv6. A daemon that cannot listen there reports why, and serves the machine
  /// alone.
  async fn listen(tailnet: &Tailnet, log: Option<&Log>) -> Option<Self> {
    let opened = async {
      let mut ips = tailnet.identity.addresses().await?;
      ips.sort_by_key(IpAddr::is_ipv6);
      let ip = *ips.first().ok_or(TransportError::Tailless)?;
      let door = async |port: Option<u16>| match port {
        Some(port) => Tcp::new(SocketAddr::new(ip, port)).listen().await.map(Some),
        None => Ok(None),
      };
      let (tcp, websocket) = (door(tailnet.ports.tcp).await?, door(tailnet.ports.websocket).await?);
      for (name, one) in [("TCP", &tcp), ("WebSocket", &websocket)] {
        if let Some(at) = one.as_ref().and_then(|one| one.local_addr().ok()) {
          report(log, &format_args!("listens on the tailnet at {at} by {name}"));
        }
      }
      Ok::<_, TransportError>(Self { identity: tailnet.identity.clone(), tcp, websocket })
    };
    opened.await.inspect_err(|no| report(log, &format_args!("no door on the tailnet: {no}"))).ok()
  }

  /// The next peer of the tailnet, with the identity that says who it is, or never with no door.
  async fn arrival(doors: Option<&Self>) -> Result<(Tailscale, Arrival), TransportError> {
    let Some(doors) = doors else { return future::pending().await };
    let arrival = tokio::select! {
      stream = accepted(doors.tcp.as_ref()) => Arrival::Tcp(stream?),
      stream = accepted(doors.websocket.as_ref()) => Arrival::WebSocket(stream?),
    };
    Ok((doors.identity.clone(), arrival))
  }
}

async fn accepted(listener: Option<&TcpListener>) -> Result<TcpStream, TransportError> {
  match listener {
    Some(listener) => Ok(listener.accept().await.map_err(TransportError::Accept)?.0),
    None => future::pending().await,
  }
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

/// The door of a peer that lives in the process of the daemon, as a kit of a session does: it says
/// its verbs as a peer of a transport does, with an outbox of its own, and holds the task that owns
/// the lives no longer than the daemon does.
#[derive(Clone)]
pub(crate) struct Inside(mpsc::WeakUnboundedSender<Asked>);

impl Inside {
  /// The answer to a verb of the peer, or the refusal that the daemon stopped.
  pub(crate) async fn answer(
    &self,
    peer: &Peer,
    verb: Verb,
    outbox: &Outbox,
  ) -> Result<Answer, ProtocolError> {
    let asks = self.0.upgrade().ok_or(ProtocolError::Stopped)?;
    Daemon { asks }.answer(peer, verb, outbox).await
  }
}

/// What the task that owns the lives hears.
enum Asked {
  /// A verb of the daemon, from a peer.
  Verb { peer: Peer, verb: Box<Verb>, outbox: Outbox, reply: Replied },
  /// The handle of a life, for a verb of that life.
  Life { life: String, reply: oneshot::Sender<Result<(LifeHandle, Option<String>), ProtocolError>> },
  /// A window whose peer is gone.
  Left { life: String, outbox: Outbox },
  /// A life whose thread told its end, in one of its births.
  Ended { life: String, born: u64 },
  /// A life that told its cards, or a record whose index of the threads the daemon made.
  Carded { life: String },
  /// A life that stalled, in one of its births: each open act, and the notice.
  Stalled { life: String, born: u64, acts: Vec<Open>, notice: String },
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
  fn spawn(
    settings: Settings,
    handed: Vec<Handed>,
  ) -> (Self, oneshot::Receiver<Leaving>, watch::Receiver<bool>) {
    let (asks, heard) = mpsc::unbounded_channel();
    let (leave, leaving) = oneshot::channel();
    let (empty, emptied) = watch::channel(handed.is_empty());
    let changes = settings.tailnet.as_ref().map(|one| one.identity.changes());
    let mut weigh = interval(WEIGH);
    weigh.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let (index, records) = mpsc::unbounded_channel();
    let log = settings.opening.log.clone();
    tokio::spawn(indexed(records, asks.downgrade(), log));
    let mut lives = Lives {
      told: Told::default(),
      indexing: HashSet::new(),
      index,
      changes,
      weigh,
      rebirths: Asks::default(),
      members: Members::default(),
      settings,
      held: HashMap::new(),
      asks: asks.downgrade(),
      born: 0,
      leave,
      views: Views::default(),
      empty,
    };
    lives.members.set(lives.logins());
    lives.adopt(handed);
    tokio::spawn(lives.kept(heard));
    (Self { asks }, leaving, emptied)
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
        let command = match (peer.kind, &peer.name) {
          (Kind::Skill, Some(name)) => command.by(processes::speaker(name)),
          (Kind::Window, _) => command.by(format!("{OPERATOR}/{}", peer.login)),
          _ => command,
        };
        let (handle, config) = self.asked(|reply| Asked::Life { life, reply }).await??;
        if let Command::Configure(change) = &command {
          signed(change, config.as_deref().map(Path::new), outbox).await?;
        }
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
    Verb::Filled { life, act } => (life, Command::Filled { id: act }),
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
      (life, Command::Bash { command, stdin: if feed { Some(None) } else { stdin.map(Some) }, on })
    }
    Verb::Read { life, path, on } => (life, Command::Read { path, on }),
    Verb::Write { life, path, content, on } => (life, Command::Write { path, content, on }),
    Verb::Names { life, on } => (life, Command::Names { on }),
    Verb::Stand { life, roster: None } => (life, Command::Stand),
    Verb::Stand { life, roster: Some(names) } => (life, Command::Roster(names)),
    Verb::Config { life, config } => (life, configured(config)),
    Verb::Seen { life, about } => (life, Command::Seen { about }),
    Verb::Queue { life, entry, shape, message, on } => {
      (life, Command::Queue { entry, shape, message, on })
    }
    Verb::Unqueue { life, entry } => (life, Command::Unqueue { entry }),
    Verb::Queued { life, on } => (life, Command::Queued { on }),
    Verb::Desk { life, notices } => (life, Command::Desk { notices }),
    Verb::Threads { life, on, at, read } => (life, Command::Threads { on, at, read, site: None }),
    Verb::Rail { life, hidden, at } => (life, Command::Rail { hidden, at }),
    Verb::Ladder { life, on, at, opened, owed, taken, shown } => {
      (life, Command::Ladder { on, at, opened, owed, taken, shown })
    }
    Verb::AsOf { life, at } => (life, Command::AsOf { at }),
    Verb::Moments { life, on } => (life, Command::Moments { on }),
    other => return Routed::Daemon(other),
  };
  Routed::Life(life, command)
}

/// The command of a life that a verb on its configs comes to.
fn configured(config: Config) -> Command {
  match config {
    Config::Get => Command::Settings,
    Config::Set { key, value, layer } => {
      Command::Configure(Change { key, value, layer: layer.unwrap_or_default() })
    }
  }
}

impl From<Reply> for Answer {
  fn from(reply: Reply) -> Self {
    match reply {
      Reply::Done => Self::Done,
      Reply::Act(id) => Self::Act(id),
      Reply::Value(value) => Self::Value(value),
      Reply::Values(values) => Self::Values(values),
      Reply::Inspected { kind, repr, value } => Self::Inspected { kind, repr, value },
      Reply::Desk(desk) => Self::Desk(desk),
      Reply::Threads(threads) => Self::Threads(threads),
      Reply::Rail(threads) => Self::Rail(threads),
      Reply::Ladder(rungs) => Self::Ladder(rungs),
      Reply::Acts(acts) => Self::Acts(acts),
      Reply::Moments(moments) => Self::Moments(moments),
    }
  }
}

/// What an open names of the life that it makes: its folder, the actor, the roster, the
/// skills, the folder of the settings of the user and the claude command line that its peer
/// chose, and the peer that answers its models, when one does. A choice that is unsaid takes the
/// one of the settings of the daemon, and an actor or a roster that both leave unsaid takes the
/// one that the record of the life stood on last, as [`Opening`] says.
#[derive(Clone, Default)]
struct Birth {
  cwd: String,
  actor: Option<String>,
  roster: Option<Vec<String>>,
  skills: Option<bool>,
  config: Option<String>,
  claude: Option<String>,
  answerer: Option<Answerer>,
  holder: Option<Holder>,
}

/// The peer that attaches the life that it opens, and the place in the log that it hears from.
#[derive(Clone)]
struct Holder {
  peer: Peer,
  outbox: Outbox,
  from: Seq,
}

/// The lives of the daemon, each under the path of its record, which one task owns: the count of
/// the births of a life, where the task tells the listener of a handover, and the views that the
/// skills of the lives serve.
struct Lives {
  settings: Settings,
  /// The access list of the tailnet as it changes, when there is a tailnet.
  changes: Option<watch::Receiver<Access>>,
  /// The home of the members that each life shares, which a change of the access list sets.
  members: Members,
  held: HashMap<String, Held>,
  asks: mpsc::WeakUnboundedSender<Asked>,
  born: u64,
  leave: oneshot::Sender<Leaving>,
  views: Views,
  /// Whether the daemon holds no life, which the listener reads to end the daemon.
  empty: watch::Sender<bool>,
  /// The clock at which the daemon reads its heap, and each life that it asked to be born again
  /// since the heap passed its share.
  weigh: Interval,
  rebirths: Asks,
  /// The cards that the daemon told to the peers that asked them.
  told: Told,
  /// Each record that the daemon sent to the task that makes the index of its threads, so no
  /// record goes there twice: a record whose index cannot be made is not asked again.
  indexing: HashSet<String>,
  index: mpsc::UnboundedSender<String>,
}

/// The index of the threads of each record that the daemon asks, made one at a time on a thread
/// that may block, so the fold of a large record holds back no verb and no second fold, then the
/// daemon told, which tells the cards of the record again.
async fn indexed(
  mut records: mpsc::UnboundedReceiver<String>,
  asks: mpsc::WeakUnboundedSender<Asked>,
  log: Option<Log>,
) {
  while let Some(life) = records.recv().await {
    let record = PathBuf::from(&life);
    match spawn_blocking(move || cards::folded(&record)).await {
      Ok(Ok(())) => {}
      Ok(Err(no)) => {
        report(log.as_ref(), &format_args!("{life} has no index of its threads: {no}"));
      }
      Err(no) => report(log.as_ref(), &format_args!("{life} has no index of its threads: {no}")),
    }
    let Some(asks) = asks.upgrade() else { return };
    asks.send(Asked::Carded { life }).ok();
  }
}

/// The next change of the access list, or never when there is no list.
async fn changed(changes: Option<&mut watch::Receiver<Access>>) -> Option<()> {
  match changes {
    Some(changes) => changes.changed().await.ok(),
    None => future::pending().await,
  }
}

/// The sign-in of codex that a change of its provider asks, driven on this machine: a cancel when
/// the change turns codex off, and, when it turns codex on, a sign-in that waits for the browser,
/// whose state the window hears at each change. It ends with the sign-in, and is refused when the
/// operator cancels the sign-in or it fails.
async fn signed(
  change: &Change,
  config: Option<&Path>,
  outbox: &Outbox,
) -> Result<(), ProtocolError> {
  if change.key != ["providers", "codex"] {
    return Ok(());
  }
  match change.value {
    Some(Value::Bool(false)) => codex::cancel(config),
    Some(Value::Bool(true)) => return signing(config, outbox).await,
    _ => {}
  }
  Ok(())
}

/// A sign-in of codex, started unless one runs or the account is connected, told to the window
/// from the state that it shows now until the state ends it.
async fn signing(config: Option<&Path>, outbox: &Outbox) -> Result<(), ProtocolError> {
  let refused = |why: &str| ProtocolError::Refused { why: why.to_owned() };
  let told = |state: &SignIn| {
    let frame = Frame::SignIn {
      state: state.state().to_owned(),
      url: state.url().map(str::to_owned),
      message: state.message().map(str::to_owned),
    };
    outbox.send(frame).ok();
  };
  let mut states = codex::subscribe(config);
  // A start that fails publishes its failure in the watch, which the loop below reads.
  codex::refresh(config)
    .and_then(|state| match state {
      SignIn::Disconnected | SignIn::NeedsLogin(_) | SignIn::Failed(_) | SignIn::Canceled => {
        codex::begin(config)
      }
      running => Ok(running),
    })
    .ok();
  loop {
    let state = states.borrow_and_update().clone();
    told(&state);
    match state {
      SignIn::Connected => return Ok(()),
      SignIn::Waiting { .. } | SignIn::Connecting => {
        states.changed().await.or(Err(ProtocolError::Stopped))?;
      }
      SignIn::Canceled => return Err(refused("the sign-in of codex was canceled")),
      SignIn::NeedsLogin(why) | SignIn::Failed(why) => return Err(refused(&why)),
      SignIn::Disconnected => return Err(refused("the sign-in of codex ended with no account")),
    }
  }
}

/// A life of the daemon: its handle, its folder, the folder of its configs when it has one, its
/// birth, its stage, and the peer that answers its models, when one does.
struct Held {
  handle: LifeHandle,
  cwd: String,
  config: Option<String>,
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
        Some(()) = changed(self.changes.as_mut()) => {
          self.members.set(self.logins());
          self.restand();
          continue;
        }
        _ = self.weigh.tick(), if self.settings.heap.is_some() => {
          self.weighed();
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
      let empty = self.held.is_empty();
      self.empty.send_if_modified(|was| mem::replace(was, empty) != empty);
    }
  }

  /// The heap of the daemon weighed: past its share, the life that said the most facts since it
  /// opened, of those that the daemon did not ask yet, is asked to be born again, and the next one
  /// once that life is born again, ended, or did not in [`crate::rebirth::PATIENCE`] weighs, while
  /// the heap stays past the share. A life that holds work is born again once that work ends, as at
  /// its tail. Once the heap falls under the share, each life asked is spared.
  fn weighed(&mut self) {
    let Some(heap) = self.settings.heap else { return };
    let living = |held: &Held| matches!(held.stage, Stage::Living(_));
    if !heap.over() {
      for life in self.rebirths.spared() {
        if let Some(held) = self.held.get(&life).filter(|one| living(one)) {
          held.handle.reborn(false);
        }
      }
      return;
    }
    let weighed: Vec<(String, u64, u64)> = self
      .held
      .iter()
      .filter(|(_, held)| living(held))
      .map(|(life, held)| (life.clone(), held.handle.weight(), held.handle.births()))
      .collect();
    if let Some(held) = self.rebirths.next(&weighed).and_then(|life| self.held.get(&life)) {
      held.handle.reborn(true);
    }
  }

  /// Each life that lives told to stand again, so that its roster names the members that the
  /// access list names now.
  fn restand(&self) {
    for held in self.held.values() {
      if matches!(held.stage, Stage::Living(_)) {
        let handle = held.handle.clone();
        tokio::spawn(async move { handle.send(Command::Stand).await.ok() });
      }
    }
  }

  fn heard(&mut self, asked: Asked) {
    match asked {
      Asked::Verb { peer, verb, outbox, reply } => self.verb(peer, *verb, &outbox, reply),
      Asked::Life { life, reply } => {
        let found = self.held.get(&life).map(|held| (held.handle.clone(), held.config.clone()));
        reply.send(found.ok_or(ProtocolError::Closed { life })).ok();
      }
      Asked::Left { life, outbox } => {
        self.views.left(&life, &outbox);
        if let Ok((_, holders)) = living(&mut self.held, &life)
          && holders.detach(&outbox)
        {
          holders.present(&life);
          self.carded(&life);
        }
      }
      Asked::Carded { life } => self.carded(&life),
      Asked::Ended { life, born } => self.ended(&life, born),
      Asked::Stalled { life, born, acts, notice } => {
        if let Some(Held { born: now, stage: Stage::Living(holders), .. }) = self.held.get(&life)
          && *now == born
        {
          holders.tell(&Frame::Stalled { life, acts, notice });
        }
      }
    }
  }

  fn verb(&mut self, peer: Peer, verb: Verb, outbox: &Outbox, reply: Replied) {
    let answer = match verb {
      Verb::Open { life, cwd, actor, roster, skills, config, claude, answers, attach } => {
        let answerer = answers.then(|| Answerer::new(life.clone(), outbox.clone()));
        let holder = attach.map(|from| Holder { peer, outbox: outbox.clone(), from });
        let cwd = cwd.unwrap_or_default();
        let birth = Birth { cwd, actor, roster, skills, config, claude, answerer, holder };
        return self.open(life, birth, reply);
      }
      Verb::List { config } => Ok(Answer::Lives(self.listed(config))),
      Verb::Cards => Ok(Answer::Cards(self.cards(outbox))),
      Verb::Attach { life, from } => self.attach(&life, from, peer, outbox),
      Verb::Detach { life } => self.detach(&life, outbox),
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
        one.answered(outbox, ask, turn).map(|()| Answer::Done)
      }),
      Verb::Wrote { life, ask, text, thinking } => {
        let wrote = |one: &Answerer| one.wrote(outbox, ask, &text, &thinking);
        self.answerer(&life).and_then(wrote).map(|()| Answer::Done)
      }
      Verb::CancelSignIn { life } => match self.held.get(&life) {
        Some(held) => {
          codex::cancel(held.config.as_deref().map(Path::new));
          Ok(Answer::Done)
        }
        None => Err(ProtocolError::Closed { life }),
      },
      Verb::ViewMount(mount) => self.mount(mount, &peer, outbox),
      Verb::ViewEvent(event) => self.views.event(event, outbox).map(|()| Answer::Done),
      Verb::ViewDispose(dispose) => self.views.dispose(&dispose, outbox).map(|()| Answer::Done),
      Verb::ViewTree(tree) => self.views.tree(tree, outbox).map(|()| Answer::Done),
      Verb::ViewEffect(effect) => self.views.effect(effect, outbox).map(|()| Answer::Done),
      // The door refuses a second hello, and each verb of a life goes to its life.
      other => Err(ProtocolError::Refused { why: format!("the daemon holds no verb {other:?}") }),
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
    let holder = birth.holder.clone();
    let opened = match self.held.get_mut(&life).map(|held| &mut held.stage) {
      Some(Stage::Living(_)) => self.answers(&life, birth.answerer.as_ref()),
      Some(Stage::Ending(waiting)) => return waiting.push((birth, reply)),
      None => self.opened(&life, birth),
    };
    reply.send(opened.and_then(|()| self.held_by(&life, holder))).ok();
  }

  /// A later open with `answers` of a life that lives: its peer answers the models of the life in
  /// the place of a peer that is gone, and is refused while that one lives. The models of a life
  /// that no peer answers stay its models.
  fn answers(&self, life: &str, asked: Option<&Answerer>) -> Result<(), ProtocolError> {
    let held = self.held.get(life).and_then(|held| held.answerer.as_ref());
    match (asked, held) {
      (Some(asked), Some(held)) => held.taken_by(&asked.outbox()),
      _ => Ok(()),
    }
  }

  /// The life attached by the peer that opened it, when that peer asked to, before any end of
  /// the life can pass: a life with no holder ends after the grace, which may be none.
  fn held_by(&mut self, life: &str, holder: Option<Holder>) -> Result<Answer, ProtocolError> {
    match holder {
      Some(Holder { peer, outbox, from }) => self.attach(life, from, peer, &outbox),
      None => Ok(Answer::Done),
    }
  }

  /// The logins of the members of each life, whom a model asks as `operator/<login>`: the user of
  /// the daemon, then each login of the access list of the tailnet, each once.
  fn logins(&self) -> Vec<String> {
    let mut all: Vec<String> = own().ok().into_iter().collect();
    let listed = self.settings.tailnet.iter().flat_map(|one| one.identity.logins());
    for login in listed {
      if !all.contains(&login) {
        all.push(login);
      }
    }
    all
  }

  /// A life born on a thread of its own, with no holder yet, and watched for its end.
  fn opened(&mut self, life: &str, birth: Birth) -> Result<(), ProtocolError> {
    let Birth { cwd, actor, roster, skills, config, claude, answerer, holder: _ } = birth;
    let base = &self.settings.opening;
    let config = config.or_else(|| base.config.clone());
    let answer = answerer.as_ref().map(|one| Answered(one.hosted()));
    let opening = Opening {
      record: Some(life.to_owned()),
      directory: cwd.clone(),
      actor: actor.or_else(|| base.actor.clone()),
      // Each life takes a home of its own, so a change of its roster changes no other life.
      roster: roster.or_else(|| base.roster.as_ref().and_then(Roster::get)).map(Roster::from),
      skills: skills.or(base.skills),
      config: config.clone(),
      claude: claude.or_else(|| base.claude.clone()),
      answer: answer.or_else(|| base.answer.clone()),
      members: Some(self.members.clone()),
      ..base.clone()
    };
    let places = processes::places(&opening);
    let handle = Life::spawn(opening)?;
    self.born += 1;
    let watch = Watch {
      life: handle.clone(),
      record: life.to_owned(),
      born: self.born,
      stall: self.settings.stall,
      log: self.settings.opening.log.clone(),
    };
    tokio::spawn(watch.watched(self.asks.clone()));
    if let Some(host) = &self.settings.host {
      let played = Played {
        handle: handle.clone(),
        life: life.to_owned(),
        cwd: PathBuf::from(&cwd),
        host: host.clone(),
        inside: Inside(self.asks.clone()),
        places,
        log: self.settings.opening.log.clone(),
      };
      tokio::spawn(played.run());
    }
    let (mut told, asks, record) = (handle.cards(), self.asks.clone(), life.to_owned());
    tokio::spawn(async move {
      while told.changed().await.is_ok()
        && let Some(asks) = asks.upgrade()
      {
        asks.send(Asked::Carded { life: record.clone() }).ok();
      }
    });
    let stage = Stage::Living(Holders::new(self.settings.grace));
    self
      .held
      .insert(life.to_owned(), Held { handle, cwd, config, born: self.born, stage, answerer });
    // The life changes its record, so the index that the daemon made of it is asked again later.
    self.indexing.remove(life);
    self.carded(life);
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
      if let Err(no) = self.opened(&life, Birth { cwd, ..Birth::default() }) {
        report(self.settings.opening.log.as_ref(), &format_args!("{life} did not open: {no}"));
      } else if let Ok((_, holders)) = living(&mut self.held, &life) {
        for pin in pins {
          holders.pin(pin);
        }
      }
    }
  }

  /// A peer attached to a life, which hears each event of the life from a place in its log. A
  /// window holds the life, and a skill, under its name, serves views of it and holds nothing.
  fn attach(
    &mut self,
    life: &str,
    from: Seq,
    peer: Peer,
    outbox: &Outbox,
  ) -> Result<Answer, ProtocolError> {
    let skill = match peer.kind {
      Kind::Skill => Some(peer.name.clone().ok_or(ProtocolError::Nameless)?),
      Kind::Window | Kind::Daemon => None,
    };
    let (handle, holders) = living(&mut self.held, life)?;
    let opens = skill.is_none().then(|| handle.opens());
    let teller = Teller { life: life.to_owned(), opens, outbox: outbox.clone() };
    let told = tokio::spawn(teller.told(handle.events(from), self.asks.clone())).abort_handle();
    if let Some(name) = skill {
      let life = life.to_owned();
      self.views.attach(Skill { life, name, outbox: outbox.clone(), told });
    } else {
      holders.attach(Window { peer, outbox: outbox.clone(), told });
      holders.present(life);
      self.carded(life);
    }
    Ok(Answer::Done)
  }

  /// A peer that let go of a life: a window of its hold and of its views, and a skill of its views.
  fn detach(&mut self, life: &str, outbox: &Outbox) -> Result<Answer, ProtocolError> {
    let (_, holders) = living(&mut self.held, life)?;
    let was = holders.detach(outbox);
    if was {
      holders.present(life);
      self.carded(life);
    }
    let skill = self.views.left(life, outbox);
    (was || skill)
      .then_some(Answer::Done)
      .ok_or(ProtocolError::Unattached { life: life.to_owned() })
  }

  /// A view that a window mounts in a slot of a life that it holds.
  fn mount(&mut self, mount: Mount, peer: &Peer, outbox: &Outbox) -> Result<Answer, ProtocolError> {
    let (_, holders) = living(&mut self.held, &mount.life)?;
    if !holders.holds(outbox) {
      return Err(ProtocolError::Unattached { life: mount.life });
    }
    self.views.mount(mount, peer, outbox).map(|()| Answer::Done)
  }

  /// Each record of the workspaces of the list in a config folder, the config folder of the
  /// daemon when none is given, and each life of the daemon, by the path of its record, with
  /// whether a life holds it, whether that life lives or ends, whether a window holds it, and the
  /// name of its session.
  fn listed(&self, config: Option<String>) -> Vec<Listed> {
    let config = config.or_else(|| self.settings.opening.config.clone());
    let folder = processes::places(&Opening { config, ..Opening::default() }).config;
    let found = records::recorded(&folder);
    let mut listed: Vec<Listed> = Vec::new();
    for one in &found {
      for life in &one.records {
        let workspace = Some(one.workspace.clone());
        let (held, open, watched, name) = (false, false, false, None);
        listed.push(Listed { life: life.clone(), workspace, held, open, watched, name });
      }
    }
    for (life, held) in &self.held {
      let (open, name) = (matches!(held.stage, Stage::Living(_)), held.handle.name());
      let watched = matches!(&held.stage, Stage::Living(holders) if !holders.presence().is_empty());
      if let Some(one) = listed.iter_mut().find(|one| &one.life == life) {
        (one.held, one.open, one.watched, one.name) = (true, open, watched, name);
        continue;
      }
      // A record that a life opens stands in the list before the disk holds it.
      let at = Path::new(life).parent();
      let workspace = found.iter().find(|one| Some(one.folder.as_path()) == at);
      let workspace = workspace.map(|one| one.workspace.clone());
      listed.push(Listed { life: life.clone(), workspace, held: true, open, watched, name });
    }
    listed.sort_by(|one, other| one.life.cmp(&other.life));
    listed
  }

  /// The cards of the machine for a peer that asks them, which hears each change from now on:
  /// the cards of each record of the list of the daemon. A peer that heard them before hears what
  /// changed since.
  fn cards(&mut self, peer: &Outbox) -> Vec<Card> {
    self.told.watched();
    let mut all = Vec::new();
    for Listed { life, workspace, .. } in self.listed(None) {
      let cards = self.cards_of(&life, workspace.as_ref());
      all.extend(cards.iter().cloned());
      self.told.tell(&life, workspace, cards);
    }
    self.told.watch(peer.clone());
    all
  }

  /// The cards of one record told again to each peer that hears the cards, when they changed.
  fn carded(&mut self, life: &str) {
    if !self.told.watched() {
      return;
    }
    let project = match self.told.project(life) {
      Some(project) => project.clone(),
      None => {
        self.listed(None).into_iter().find(|one| one.life == life).and_then(|one| one.workspace)
      }
    };
    let cards = self.cards_of(life, project.as_ref());
    self.told.tell(life, project, cards);
  }

  /// The cards of one record of a project now: those that its life told, when one lives and
  /// told some, else those of the index beside it. A record with no index that agrees with it
  /// gives one card under the name of its file, and goes once to the task that makes its index,
  /// unless a life holds it, which tells its cards itself. So no list waits for the fold of a
  /// record.
  fn cards_of(&mut self, life: &str, project: Option<&String>) -> Vec<Card> {
    let record = Path::new(life);
    let (session, viewers) = match self.held.get(life) {
      Some(Held { handle, stage: Stage::Living(holders), .. }) => {
        (handle.session(), holders.presence().into_iter().map(|peer| peer.login).collect())
      }
      _ => (None, Vec::new()),
    };
    let held = self.held.contains_key(life);
    let card = |session: Option<String>, since, thread| Card {
      life: life.to_owned(),
      session,
      project: project.cloned(),
      held,
      viewers: viewers.clone(),
      since,
      thread,
    };
    if let Some(session) = session.or_else(|| cards::kept(record).map(Arc::new)) {
      let stands = |thread: view::Card| if held { thread } else { thread.at_rest() };
      let threads = session.threads.iter().cloned();
      return threads
        .map(|one| card(session.name.clone(), one.since, stands(one.thread)))
        .collect();
    }
    if !held && record.is_file() && self.indexing.insert(life.to_owned()) {
      // The task ends with the daemon, and then no peer hears a card.
      self.index.send(life.to_owned()).ok();
    }
    let name = record.file_stem().unwrap_or_default().to_string_lossy().into_owned();
    vec![card(None, cards::changed(record), cards::unfolded(name))]
  }

  /// When the first life with no holder ends.
  fn due(&self) -> Option<Instant> {
    let idle = self.held.values().filter_map(|held| match &held.stage {
      Stage::Living(holders) => holders.idle(),
      Stage::Ending(_) => None,
    });
    idle.min()
  }

  /// Each life whose grace passed, told to write its image and end.
  fn expire(&mut self) {
    let now = Instant::now();
    for held in self.held.values_mut() {
      if let Stage::Living(holders) = &held.stage
        && holders.idle().is_some_and(|at| at <= now)
      {
        held.stage = Stage::Ending(Vec::new());
        let handle = held.handle.clone();
        tokio::spawn(async move { end(&handle, Command::End).await });
      }
    }
  }

  /// A life that ended, in the birth that the daemon holds: gone, and opened again for each open
  /// that waited for its end.
  fn ended(&mut self, life: &str, born: u64) {
    if self.held.get(life).is_none_or(|held| held.born != born) {
      return;
    }
    self.views.ended(life);
    let gone = self.held.remove(life);
    self.carded(life);
    let Some(Held { stage: Stage::Ending(waiting), .. }) = gone else { return };
    let Some((first, _)) = waiting.first() else { return };
    let opened = self.opened(life, first.clone());
    for (birth, one) in waiting {
      one.send(opened.clone().and_then(|()| self.held_by(life, birth.holder))).ok();
    }
  }

  /// The handover: each window detached, each life told to write its checkpoint and end, then each
  /// life that lives, with its pins, given to the peer that asked once the listener stopped. A life
  /// that ends already ends here and goes to no one: the open that waited for it is refused, and
  /// the next open opens it from its record.
  fn handover(self, asker: Outbox, reply: Replied) {
    let (mut handed, mut lives) = (Vec::new(), Vec::new());
    for (life, held) in self.held {
      match held.stage {
        Stage::Living(holders) => {
          handed.push(Handed { life: life.clone(), cwd: held.cwd, pins: holders.release() });
        }
        Stage::Ending(waiting) => {
          for (_, one) in waiting {
            one.send(Err(ProtocolError::Stopped)).ok();
          }
        }
      }
      lives.push((life, held.handle));
    }
    let log = self.settings.opening.log.clone();
    tokio::spawn(gave(lives, log, self.leave, Leaving { handed, asker, reply }));
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

/// A life told to end, with its image, or with its checkpoint for a handover, once its commands
/// before ran, and the reply once it ended. A life whose record does not hold each fact that it
/// said yet, as on a full disk, keeps
/// the end and ends once the record holds them, so the daemon tells it once. A fault of that
/// checkpoint is the fault of the event `Ended` of the life, which its windows and its watch hear,
/// so the reply tells only the moment of the end.
async fn end(life: &LifeHandle, how: Command) {
  life.send(how).await.ok();
}

/// The watch of one birth of a life, which no word of the life runs: its handle, its record, its
/// birth, the bound of a stall, and the log of the reports of the daemon.
struct Watch {
  life: LifeHandle,
  record: String,
  born: u64,
  stall: Duration,
  log: Option<Log>,
}

impl Watch {
  /// The life watched until its end, which the daemon hears. Each stall goes to the daemon, which
  /// tells each window, and as a message of None to the chain that stalled, or to the root chain
  /// for a stall of the life, so the actor of that chain wakes.
  /// A life whose thread hangs takes the message late or never, and the watch goes on.
  async fn watched(self, asks: mpsc::WeakUnboundedSender<Asked>) {
    let mut events = pin!(self.life.events(0));
    let mut quiet = Quiet::new(self.stall);
    let mut end = None;
    loop {
      tokio::select! {
        biased;
        next = events.next() => match next {
          Some((_, event)) => {
            if let Event::Ended(fault) = &*event {
              end = Some(fault.clone());
            }
            quiet.heard(&event);
          }
          None => break,
        },
        () = until(quiet.due()) => {
          for Stall { acts, notice, chain } in quiet.stalled(Instant::now()) {
            let (life, born) = (self.record.clone(), self.born);
            if let Some(asks) = asks.upgrade() {
              asks.send(Asked::Stalled { life, born, acts, notice: notice.clone() }).ok();
            }
            let (handle, record, log) = (self.life.clone(), self.record.clone(), self.log.clone());
            tokio::spawn(woken(handle, record, notice, chain, log));
          }
        }
      }
    }
    let why = match end {
      Some(Some(fault)) => fault,
      Some(None) => "with no fault".to_owned(),
      None => "with no event of its end".to_owned(),
    };
    report(self.log.as_ref(), &format_args!("the life of {} ended: {why}", self.record));
    if let Some(asks) = asks.upgrade() {
      asks.send(Asked::Ended { life: self.record, born: self.born }).ok();
    }
  }
}

/// The message of a stall sent to a chain of a life, or to its root with no chain, as a prompt of
/// None, or the report of why the life refused it.
async fn woken(
  life: LifeHandle,
  record: String,
  notice: String,
  on: Option<String>,
  log: Option<Log>,
) {
  let to = on.clone().unwrap_or_else(|| "its root".to_owned());
  // The watch speaks for itself, so the threads of the operator read the notice as none of his.
  let message = Command::Prompt { shape: "None".to_owned(), message: notice, to: None, on }
    .by(WATCH.to_owned());
  if let Err(no) = life.send(message).await {
    report(log.as_ref(), &format_args!("{record} stalled, and {to} took no message: {no}"));
  }
}

/// What the daemon pushes of a life to a peer that attached it: its events, and, to a window, how
/// far its open stands.
struct Teller {
  life: String,
  opens: Option<Heard>,
  outbox: Outbox,
}

impl Teller {
  /// The events of the life pushed to the peer, no more than [`WAITING`] of them ahead of what the
  /// peer read, until the life ends or the peer is gone, which the daemon hears. The frames of an
  /// open go when [`Heard`] says that they are due, and the last of them before the next event, so
  /// the frame that ends an open comes before the first event that the open told.
  async fn told(
    self,
    events: impl Stream<Item = (Seq, Arc<Event>)>,
    asks: mpsc::WeakUnboundedSender<Asked>,
  ) {
    let Self { life, mut opens, outbox } = self;
    let mut events = pin!(events);
    loop {
      let event = tokio::select! {
        biased;
        () = outbox.closed() => break,
        () = due(opens.as_mut()) => None,
        next = events.next() => match next {
          Some(event) => Some(event),
          None => return,
        },
      };
      if event.is_some() {
        outbox.room(WAITING).await;
      }
      let paced = event.is_none();
      let due = |opens: &&mut Heard| paced || opens.ended();
      let opening = opens.iter_mut().filter(due).flat_map(|opens| opens.frames(&life));
      let event = event.map(|(seq, event)| Frame::Event {
        life: life.clone(),
        seq,
        event: Event::clone(&event),
      });
      if opening.chain(event).any(|frame| outbox.send(frame).is_err()) {
        break;
      }
    }
    if let Some(asks) = asks.upgrade() {
      asks.send(Asked::Left { life, outbox }).ok();
    }
  }
}

/// Once a window has a frame of an open to hear, and never for a peer that hears none.
async fn due(opens: Option<&mut Heard>) {
  match opens {
    Some(opens) => opens.due().await,
    None => future::pending().await,
  }
}

/// The handover once each life wrote its checkpoint and ended, however long that takes, given to
/// the listener, or refused when the listener is gone. The log says which lives it waits for.
async fn gave(
  lives: Vec<(String, LifeHandle)>,
  log: Option<Log>,
  leave: oneshot::Sender<Leaving>,
  leaving: Leaving,
) {
  if !lives.is_empty() {
    let records: Vec<&str> = lives.iter().map(|(record, _)| record.as_str()).collect();
    let waits = records.join(", ");
    report(log.as_ref(), &format_args!("the handover waits for each life to end: {waits}"));
  }
  future::join_all(lives.iter().map(|(_, life)| end(life, Command::Hand))).await;
  if let Err(Leaving { reply, .. }) = leave.send(leaving) {
    reply.send(Err(ProtocolError::Stopped)).ok();
  }
}
