//! A life as the host holds it: a thread of its own, with a runtime of tokio and a set of local
//! tasks, since the engine is not `Send`. The thread owns the engine. It awaits the open, and
//! tells how far it stands as [`crate::opens`] says, then one local task pumps the engine while it
//! waits for the next command, and keeps the log of the events of the life. A command that a
//! handle says while the life opens waits, and the life answers it once it stands. A life that
//! keeps its record writes its image as it goes and at its end, as [`crate::rebirth`] says, which
//! holds what the life keeps of the events that it told, and the next life of the record opens
//! from it.
//! Once the life ends, the thread holds its log for the readers
//! until each handle of the life is gone.

#![expect(
  clippy::too_many_arguments,
  reason = "debt: a function of more than 4 parameters takes a struct, rule 8 of the skill rust"
)]
use std::{
  borrow::Cow,
  collections::{BTreeMap, HashMap, VecDeque},
  fs::File,
  future::poll_fn,
  hash::{DefaultHasher, Hash, Hasher},
  io::{BufRead, BufReader},
  iter, mem,
  ops::Range,
  path::{Path, PathBuf},
  sync::{Arc, Mutex},
  task::Poll,
  thread,
  time::Duration,
};

use furb::{
  Engine, Fact, Fault, Object, Text, checkpoint, image,
  life::{Log, Opening, Progress, Stage, Stream},
  verbs,
  view::{
    Act, Desk, Fold, Held, Named, Page, Phase, Reach, Rung, SHOWN, Shown, Streamed, ThreadNode,
    Threads,
  },
  wire::{outward, parsed},
  world::{
    self, Shape, answered,
    config::{Change, Layers},
  },
};
use futures::stream::{self, Stream as Streams};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::{
  runtime,
  sync::{
    broadcast::{self, error::RecvError},
    mpsc::{self, UnboundedReceiver, UnboundedSender},
    oneshot, watch,
  },
  task::{LocalSet, spawn_blocking},
  time::{Instant, sleep_until},
};

use crate::{
  backlog::{self, Backlog},
  cards::{self, Session, Times},
  changes::{Journal, journal},
  ears::{ears, json_of},
  error::LifeError,
  event::{Event, Seq},
  opens::{Heard, Opens, Told},
  queue::{FollowUp, QUEUE, Queue},
  rebirth::{self, Watch},
};

/// How many events a reader may stand behind the life before it reads them again from the log of
/// the life.
const BEHIND: usize = 256;

/// The least time between two tells of the cards of a life.
const CARDS: Duration = Duration::from_millis(250);

/// What a client says to a life. A command that names no chain says it on the root.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
  /// A prompt on a chain, of a shape, with a message, to an actor or to the actor of the chain: the
  /// act.
  Prompt { shape: String, message: String, to: Option<String>, on: Option<String> },
  /// A word that runs on a chain as a turn of the operator: the act of its rung.
  Rung { word: String, on: Option<String> },
  /// A prompt to the operator, closed with a line that the operator wrote, read as the shape that
  /// the prompt wants.
  Answer { id: String, line: String },
  /// A pause of an act or of a chain, with a cause.
  Pause { id: String, why: Option<String> },
  /// A wake of an act or of a chain.
  Wake { id: String },
  /// A cancel of an act or of a chain, with a cause.
  Cancel { id: String, why: Option<String> },
  /// The turns of a chain, each as JSON.
  Turns { on: Option<String> },
  /// The words of a chain that run, by the rung that gave each one.
  Program { on: Option<String> },
  /// The standing of the life.
  Standing,
  /// The share of the window that the answer of a reply filled.
  Filled { id: String },
  /// The facts of a chain, of one kind or of each kind.
  Transcript { on: Option<String>, of: Option<String> },
  /// One name that the module of a chain binds.
  Inspect { name: String, on: Option<String> },
  /// The facts of an act, or null when no act has its name.
  Get { id: String },
  /// What an act came to, refused while it lives.
  Peek { id: String },
  /// The folder of a chain.
  Cwd { on: Option<String> },
  /// A chain made on a chain, under a label: the act of the chain.
  Chain { label: Option<String>, source: Option<String>, on: Option<String> },
  /// The acts that wait for a wake, each as its name and its kind.
  Pending,
  /// A close of an act with a value, as the record writes a value: `2.0` stays a float, and
  /// `{"is": "Refused", "args": ["why"]}` is a refusal.
  Close { id: String, value: String },
  /// A ceiling of the spend of a chain, in dollars or in a share of its window: the act.
  Grant { usd: Option<f64>, share: Option<f64>, on: Option<String> },
  /// A command that runs on a chain: the act.
  ///
  /// Its stdin is the text given, the default of the engine when it is none, and open for a feed
  /// when it is `None` within.
  Bash { command: String, stdin: Option<Option<String>>, on: Option<String> },
  /// The text at a path, read where a chain stands, as `{"path", "content"}`.
  Read { path: String, on: Option<String> },
  /// A text written at a path, where a chain stands.
  Write { path: String, content: String, on: Option<String> },
  /// The names that the module of a chain binds.
  Names { on: Option<String> },
  /// A stand of the life on its root, which reads the roster and the actor again: the act.
  Stand,
  /// Each key of the configs of the folder of the life, with the value that wins for it and its
  /// layer.
  Settings,
  /// A change of a key of a layer of the configs, which the World keeps only when the catalog
  /// reads it, then a stand of the life on them: the act of the stand.
  Configure(Change),
  /// The names of the models of the roster of the life put in place of the ones it held, then a
  /// stand of the life, which reads the catalog again and offers them: the act of the stand. The
  /// life goes on, and each later birth of it keeps them.
  Roster(Vec<String>),
  /// The fact that the operator read a chain or a thread up to now.
  Seen { about: String },
  /// A follow-up that the operator queued on a chain, under an entry that a second queue finds,
  /// which the life sends as a prompt of its shape and its message once the chain rests.
  Queue { entry: String, shape: String, message: String, on: Option<String> },
  /// A follow-up taken out of its queue before the life sent it.
  Unqueue { entry: String },
  /// The follow-ups that wait on a chain, in order.
  Queued { on: Option<String> },
  /// What waits for the operator, as a [`furb::view::Desk`], with each notice that the window
  /// still shows.
  Desk { notices: Vec<String> },
  /// The threads of a chain, as [`furb::view::Threads`], now or as they stood just after a
  /// moment. `read` is the place up to which the window read the chain, from which the runs of
  /// notes count the ones not yet read; none reads nothing.
  /// The threads of a chain as the window of `site` reads them: the mark of what it has not seen
  /// counts the seen of that site alone, and of the operator alone when none is given.
  Threads { on: Option<String>, at: Option<u64>, read: Option<u64>, site: Option<String> },
  /// The threads of every chain of the life but those it names, as one tree in the order of the
  /// list of the threads, now or as they stood just after a moment.
  Rail { hidden: Vec<String>, at: Option<u64> },
  /// The ladder of a chain, as [`furb::view::Held::ladder`] folds it, now or as it stood just
  /// after a moment.
  Ladder {
    on: Option<String>,
    at: Option<u64>,
    opened: Vec<String>,
    owed: Vec<String>,
    taken: Option<Vec<String>>,
    shown: Option<Shown>,
  },
  /// The acts as they stood just after a moment, each with its meaning then.
  AsOf { at: u64 },
  /// A page of a thread, as [`furb::view::Held::page`] gives it.
  Thread { thread: String, reach: Reach },
  /// Each moment at which an act began or ended, in order, of the life or of the acts of one
  /// chain.
  Moments { on: Option<String> },
  /// The end of the life, which replies and then ends the life. A life whose record does not hold
  /// each fact that it said yet, as on a full disk, keeps the end, tells why in a notice, and ends
  /// once the record holds them: the reply comes then.
  End,
  /// A command of the engine said by a speaker other than the life itself, a skill as
  /// `skill/<name>` or a window as `operator/<login>`: each act and each fact that the command
  /// says is of that speaker, so a fact seen names the person who read.
  By { speaker: String, command: Box<Self> },
}

impl Command {
  /// The command said by a speaker, when it says acts or facts on the engine. A query, and a
  /// command of the configs, of the queue or of the meaning of the acts, stays as it is.
  #[must_use]
  pub fn by(self, speaker: String) -> Self {
    match self {
      Self::Prompt { .. }
      | Self::Rung { .. }
      | Self::Answer { .. }
      | Self::Pause { .. }
      | Self::Wake { .. }
      | Self::Cancel { .. }
      | Self::Chain { .. }
      | Self::Close { .. }
      | Self::Grant { .. }
      | Self::Bash { .. }
      | Self::Read { .. }
      | Self::Write { .. }
      | Self::Seen { .. }
      | Self::Stand => Self::By { speaker, command: Box::new(self) },
      // A read of the threads tells the window of the speaker what it has not seen.
      Self::Threads { on, at, read, .. } => Self::Threads { on, at, read, site: Some(speaker) },
      other => other,
    }
  }
}

/// What a life replies to a command.
#[derive(Debug, Clone, PartialEq)]
pub enum Reply {
  /// The command is done.
  Done,
  /// The name of the act that the command made.
  Act(String),
  /// What a query gives, as JSON.
  Value(Value),
  /// The items that a query gives, each as JSON.
  Values(Vec<Value>),
  /// A name of a module: the name of its type, its representation, and its value as JSON.
  Inspected {
    kind: String,
    repr: String,
    value: Value,
  },
  /// What waits for the operator.
  Desk(Desk),
  /// The threads of a chain.
  Threads(Threads),
  /// The threads of every chain of the life, as one tree.
  Rail(Vec<ThreadNode>),
  Ladder(Vec<Rung>),
  /// The acts as they stood at a moment.
  Acts(Vec<Act>),
  /// Each moment at which an act began or ended.
  Moments(Vec<u64>),
  /// A page of a thread.
  Page(Page),
}

/// One event under its place in the log.
pub(crate) type Logged = (Seq, Arc<Event>);

/// Where the thread of a life replies to a command.
type Replier = oneshot::Sender<Result<Reply, LifeError>>;

/// What a handle says to the thread of its life.
enum Message {
  Command(Command, Replier),
  /// What a reader of the log asks.
  Read(Read),
}

/// What a reader of the log asks of the thread of a life.
enum Read {
  /// The events of the log from a place, for a reader that stands behind the life.
  Since(Seq, oneshot::Sender<Since>),
  /// The reply of a command, with the place in the log where the life gave it.
  After(Command, oneshot::Sender<(Result<Reply, LifeError>, Seq)>),
}

impl Read {
  /// The reply, once the log took each event that the ears told, so each fact that the life said
  /// before it stands before the place that it gives. A life that ended answers no command.
  async fn answered(self, engine: Option<&mut Engine>, life: &mut Life) {
    match self {
      Self::Since(from, reply) => {
        life.told(None);
        reply.send(life.since(from)).ok();
      }
      Self::After(command, reply) => {
        let replied = match engine {
          Some(engine) => commanded(engine, life, command).await,
          None => Err(LifeError::Ended),
        };
        life.told(None);
        reply.send((replied, life.since(Seq::MAX).tip)).ok();
      }
    }
  }
}

/// A life on its thread: what it keeps of the events that it told, the readers that hear each next
/// one, and what the ears and the provider told that is not in the log yet.
pub struct Life {
  kept: Kept,
  readers: broadcast::Sender<Logged>,
  heard: UnboundedReceiver<Event>,
  configs: Layers,
  /// The home of the names of the roster that each birth of the life reads at a stand.
  roster: world::Roster,
  /// Where the handle reads the name of the session.
  name: Arc<Mutex<Option<String>>>,
  /// The record that the life keeps.
  record: Option<String>,
  /// How far the open of the life stands, which each window reads while the thread opens.
  opens: Told,
  /// The cards of the life as the daemon reads them, and what makes them: see [`Life::carded`].
  cards: watch::Sender<Option<Arc<Session>>>,
  times: Times,
  /// Whether an event changed what the cards read since the life last told them, and when it may
  /// tell them next.
  stale: bool,
  due: Instant,
  /// The time of each card that the life tells first: when its record last changed.
  first: Option<u64>,
}

/// What a life keeps of the events that it told, which is the part of the host in the image of the
/// life, so a later life of the record stands where this one stood and reads no fact of its past:
/// the log, which is each event that it told, less those that a later event replaced (see
/// `Spent`), how many facts it told and the place of each by its digest, the place in the log of
/// each fact that the engine of this birth heard, what the actor watches to know when the life
/// writes its image, the queue of the follow-ups, the acts with their meaning, the work that the
/// life holds for its record, the name of the session, folded from each fact of the life, the facts
/// that the log let go since the record holds them, as [`Life::let_go`] says, and what the models
/// wrote and thought and when each act that lives began, which no fact holds and a page of a
/// thread reads. The image holds it as a [`Part`].
#[derive(Default, Serialize, Deserialize)]
struct Kept {
  log: Vec<Option<Slot>>,
  spent: Spent,
  said: Seq,
  facts: HashMap<u64, Seq>,
  births: Vec<Seq>,
  watch: Watch,
  queue: Queue,
  held: Held,
  backlog: Backlog,
  named: Named,
  gone: Gone,
  streams: Streamed,
}

/// A place of the log that holds an event. The place of the last event of an act names the act that
/// `Held` holds, which is that event, so the life and its image hold each act once. When a later
/// event of the act comes, the place takes the act as `Held` held it, as an event told.
#[derive(Serialize, Deserialize)]
enum Slot {
  Told(Arc<Event>),
  Held(String),
}

impl Slot {
  /// The fact that the place holds.
  fn fact(&self) -> Option<&Value> {
    match self {
      Self::Told(event) => match &**event {
        Event::Fact(fact) => Some(fact),
        _ => None,
      },
      Self::Held(_) => None,
    }
  }
}

/// The part of the host in the image of a life: what the life keeps, under the digest of the files
/// that give it its shape, which `build.rs` computes. A host of another build of them takes no
/// such part, so the life opens by its record.
#[derive(Serialize, Deserialize)]
struct Part<K> {
  build: Cow<'static, str>,
  kept: K,
}

/// The digest of the files that give their shape to what a host of this build keeps.
const BUILD: &str = env!("FURB_KEPT");

/// The facts that the log of a life let go: each one before the last checkpoint of the life, which
/// the record holds, and each fact that an open told but a fact of keep, which the file of the log
/// of the record holds, so a log holds the facts of one birth and not those of a whole life, and
/// no fact that an open says again. The log keeps the place and the digest of each, and a reader
/// behind them reads them again from the record or from that file, as [`Retell`] says.
#[derive(Default, Serialize, Deserialize)]
struct Gone {
  /// The place in the log of each fact let go that the record holds, in order, with its digest.
  places: Vec<(Seq, Digest)>,
  /// The place in the log of each fact let go that an open told, in order, with its digest.
  logged: Vec<(Seq, Digest)>,
  /// The length of the log when it last let go, from which the next let go reads.
  scanned: usize,
  /// The places of the log of what each open told, whose facts of keep the log keeps: the journal
  /// says them again at the opening of a birth, and the file of the record does not hold them.
  opened: Vec<Range<usize>>,
}

/// The places of the log whose event a later event replaced, which the log drops once the life told
/// [`BEHIND`] events after the event that replaced them, so a reader that stands no further behind,
/// and reads the log, still hears each event: an event of an act replaces each earlier event of
/// that act, and the act of a rung that has its word, or is done, replaces each part that a model
/// wrote for it. A life that holds an act for each fact, and a part
/// for each few words that a model streams, would else hold every state of each act and every part
/// of each reply for its whole life.
#[derive(Default, Serialize, Deserialize)]
struct Spent {
  /// The place of the last event of each act.
  acts: HashMap<String, Seq>,
  /// The places of the parts that a model wrote for each rung that has no word yet.
  parts: HashMap<String, Vec<Seq>>,
  /// Each place that a later event replaced, under the place of that event, in order.
  replaced: VecDeque<(Seq, Seq)>,
}

impl Spent {
  /// An event heard at a place of the log, and the places that the log may drop now.
  fn heard(&mut self, seq: Seq, event: &Event) -> Vec<Seq> {
    match event {
      Event::Act(act) => {
        if let Some(before) = self.acts.insert(act.id.clone(), seq) {
          self.replaced.push_back((seq, before));
        }
        if act.word.is_some() || matches!(act.phase, Phase::Done(_)) {
          let parts = self.parts.remove(&act.id).unwrap_or_default();
          self.replaced.extend(parts.into_iter().map(|part| (seq, part)));
        }
      }
      Event::Text { rung, .. } => self.parts.entry(rung.clone()).or_default().push(seq),
      _ => {}
    }
    let mut dropped = Vec::new();
    while let Some(&(by, at)) = self.replaced.front()
      && by + BEHIND as Seq <= seq
    {
      self.replaced.pop_front();
      dropped.push(at);
    }
    dropped
  }
}

/// The handle of a life, which any thread holds: it says a command to the life and reads its
/// events.
#[derive(Clone)]
pub struct LifeHandle {
  inbox: UnboundedSender<Message>,
  readers: broadcast::Sender<Logged>,
  name: Arc<Mutex<Option<String>>>,
  opens: watch::Receiver<Opens>,
  cards: watch::Receiver<Option<Arc<Session>>>,
}

impl Life {
  /// A life opened on a thread of its own, on the ears of the host and then on the ears of the
  /// crate. The host tells what the models write and what the life reports as events, in the place
  /// of the `stream` and the `log` of the opening. A life that does not open ends at once, with its
  /// fault.
  pub fn spawn(opening: Opening) -> Result<LifeHandle, LifeError> {
    let (inbox, received) = mpsc::unbounded_channel();
    let (readers, _) = broadcast::channel(BEHIND);
    let told = readers.clone();
    let name = Arc::new(Mutex::new(None));
    let named = Arc::clone(&name);
    let (opened, opens) = Told::new();
    let (carded, cards) = watch::channel(None);
    let told = Tells { readers: told, name: named, opens: opened, cards: carded };
    thread::Builder::new()
      .name("furb-life".to_owned())
      .spawn(move || lived(opening, received, told))
      .map_err(LifeError::Thread)?;
    Ok(LifeHandle { inbox, readers, name, opens, cards })
  }

  /// One event put in the log, and sent to each reader, then the follow-ups of a chain when the
  /// event changed them.
  fn put(&mut self, mut event: Event) {
    if let Event::Fact(fact) = &event {
      self.kept.said += 1;
      self.kept.held.noted(fact, self.kept.said);
      self.stale |= fact[0] == "seen";
      self.kept.facts.entry(digest(fact)).or_insert(self.kept.said);
      if self.kept.named.hear(fact, false) {
        self.named();
        self.stale = true;
      }
    }
    self.kept.watch.heard(&event);
    let queued = self.kept.queue.heard(&event);
    match &event {
      Event::Act(act) => {
        self.kept.streams.heard(act, cards::now());
        let before = self.kept.held.took(Arc::clone(act)).map(|one| Event::Act(one).into());
        // Each reader hears the act as the life holds it, which keeps who made it through a birth.
        let held = self.kept.held.shared(&act.id).map(Arc::clone);
        let place = self.kept.spent.acts.get(&act.id).and_then(|at| usize::try_from(*at).ok());
        if let (Some(before), Some(slot)) = (before, place.and_then(|at| self.kept.log.get_mut(at)))
        {
          *slot = Some(Slot::Told(before));
        }
        self.stale = true;
        if let Some(held) = held {
          event = Event::Act(held);
        }
      }
      Event::Text { rung, text, thinking, .. } => {
        self.kept.streams.wrote(rung, [text, thinking], cards::now());
      }
      _ => {}
    }
    let seq = self.kept.log.len() as Seq;
    for at in self.kept.spent.heard(seq, &event) {
      if let Some(slot) = usize::try_from(at).ok().and_then(|at| self.kept.log.get_mut(at)) {
        *slot = None;
      }
    }
    let event = Arc::new(event);
    self.kept.log.push(Some(match &*event {
      Event::Act(act) => Slot::Held(act.id.clone()),
      _ => Slot::Told(Arc::clone(&event)),
    }));
    self.readers.send((seq, event)).ok();
    if let Some(queued) = queued {
      self.put(queued);
    }
  }

  /// The name of the session, as the facts of the life fold it, told to each handle.
  fn named(&self) {
    if let Ok(mut name) = self.name.lock() {
      *name = self.kept.named.name().map(str::to_owned);
    }
  }

  /// The cards of the life told to the daemon, when an event changed what they read and they are
  /// not the cards told before. A card is a fold of each act of the life, so the life tells its
  /// cards once in [`CARDS`] at most, whatever the count of its events.
  fn carded(&mut self) {
    if !mem::take(&mut self.stale) {
      return;
    }
    let at = self.first.take().unwrap_or_else(cards::now);
    let name = self.kept.named.name().map(str::to_owned);
    let session = Session { name, threads: self.times.timed(self.kept.held.cards(), at) };
    self.cards.send_if_modified(|told| {
      let changed = told.as_deref() != Some(&session);
      if changed {
        *told = Some(Arc::new(session));
      }
      changed
    });
    self.due = Instant::now() + CARDS;
  }

  /// The index of the threads of the record written from the cards of the life, at its end, unless
  /// the index of the record says them already.
  fn indexed(&mut self) {
    self.stale = true;
    self.carded();
    let told = self.cards.borrow().clone();
    if let (Some(record), Some(session)) = (&self.record, told)
      && cards::kept(Path::new(record)).as_ref() != Some(&*session)
      && let Err(no) = cards::keep(Path::new(record), &session)
    {
      self.put(Event::Notice(format!("The index of the threads of {record} is not written: {no}")));
    }
  }

  /// An event, and each event told after it that the log lacks, put in the log in turn, with each
  /// chain that they changed told once, after the last of them.
  fn told(&mut self, mut first: Option<Event>) {
    let mut changed: Vec<String> = Vec::new();
    while let Some(event) = first.take().or_else(|| self.heard.try_recv().ok()) {
      match event {
        Event::Changed(chain) if !changed.contains(&chain) => changed.push(chain),
        Event::Changed(_) => {}
        Event::Fact(fact) => {
          self.put(Event::Fact(fact));
          self.kept.births.push(self.kept.said);
        }
        Event::Act(act) => {
          let act = self.placed(act);
          self.put(Event::Act(act));
        }
        event => self.put(event),
      }
    }
    for chain in changed {
      self.put(Event::Changed(chain));
    }
  }

  /// What an open told, less each fact of its opening that restates one that the log told before
  /// it: an open on a checkpoint says again what the checkpoint restates, which each reader
  /// already heard, then goes on with facts of its own, which the log tells whole.
  fn opened(&mut self, told: Vec<Event>, progress: &Progress) {
    self.kept.births.clear();
    let start = self.kept.log.len();
    let (mut opening, of) = (true, told.len() as u64);
    (progress.0)(Stage::Log, 0, of);
    for (at, event) in (1..).zip(told) {
      match &event {
        Event::Fact(fact) if self.restates(&mut opening, fact) => {
          let place = self.kept.facts.get(&digest(fact)).copied().unwrap_or(self.kept.said);
          self.kept.births.push(place);
        }
        _ => self.told(Some(event)),
      }
      (progress.0)(Stage::Log, at, of);
    }
    let told = start..self.kept.log.len();
    if self.record.is_some() {
      for (at, slot) in self.kept.log.iter_mut().enumerate().skip(start) {
        let Some(fact) = slot.as_ref().and_then(Slot::fact).filter(|fact| fact[0] != "keep") else {
          continue;
        };
        self.kept.gone.logged.push((at as Seq, Digest::of(fact)));
        *slot = None;
      }
    }
    self.kept.gone.opened.push(told);
  }

  /// The life stands where the life of its image stood, on what that life kept: no open told a
  /// fact, so the log takes none, and each reader reads the log of the life before. The fresh ears
  /// of the life take again the acts that the ears before held, and tell what they say as each
  /// later event.
  fn imaged(&mut self, kept: Kept, progress: &Progress) {
    self.kept = kept;
    self.named();
    self.stale = true;
    (progress.0)(Stage::Log, 0, 0);
  }

  /// An act that the engine of this birth told, at its place in the log. The fold of the engine
  /// counts the facts that it heard, and the log holds each fact of the life once, from its first
  /// birth on, so a reader reads the place of each act in one time for the whole life.
  fn placed(&self, mut act: Arc<Act>) -> Arc<Act> {
    let placed = Arc::make_mut(&mut act);
    placed.began = self.place(placed.began);
    placed.ended = placed.ended.map(|ended| self.place(ended));
    act
  }

  /// The place in the log of a fact that the engine of this birth heard, by its count in the fold.
  fn place(&self, clock: Seq) -> Seq {
    let at = clock.checked_sub(1).and_then(|one| usize::try_from(one).ok());
    at.and_then(|one| self.kept.births.get(one)).copied().unwrap_or(clock)
  }

  /// Whether a fact of an opening restates a fact that the log told, which ends the opening when it
  /// is the stand of its boot: the facts after it are those of the life, each occurrence of an
  /// equal fact among them.
  fn restates(&self, opening: &mut bool, fact: &Value) -> bool {
    let word = |at: usize| fact.get(at).and_then(Value::as_str);
    *opening &=
      !checkpoint::opened(word(0).unwrap_or_default(), word(2).unwrap_or_default(), word(3));
    *opening && self.kept.facts.contains_key(&digest(fact))
  }

  /// The facts of a record before its last checkpoint, told first, in order, each with the acts
  /// that it changed, then the birth on that checkpoint: the life boots on it and replays none of
  /// them, so each reader hears the life whole, and the watch of a stall holds
  /// only the acts that the open tells again. No engine reads them, so no pause covers their acts.
  /// Each birth on a checkpoint before the last opened by saying again what that checkpoint
  /// restates, which its log told no reader twice, and this one tells none twice either; each other
  /// fact is told, though it equals one before it, as a second pause of a chain. A record that the
  /// store cannot read tells nothing here, and its open says why.
  fn recalled(&mut self, record: &str, told: &Progress) {
    if !Path::new(record).exists() {
      return;
    }
    (told.0)(Stage::Recall, 0, 0);
    // One pass over the heads of the lines gives the place of each checkpoint, and a record with
    // none tells nothing here.
    let Ok(checkpoints) = world::checkpoints(record) else { return };
    let marks: Vec<usize> = checkpoints.into_iter().map(|at| at + 1).collect();
    let Some(&last) = marks.last() else { return };
    let Ok(retold) = world::retold(record) else { return };
    let (mut passed, mut opening) = (0, false);
    let mut fold = Fold::default();
    // Each fact is folded as the read gives it and let go at once, since the record holds it: the
    // life holds the acts of its past, and no list of its facts.
    let mut take = |kept: usize, one: Object| {
      let Some(a) = Fact::of(one.as_ref()).filter(|a| a.kind() != checkpoint::CHECKPOINT) else {
        return;
      };
      (told.0)(Stage::Recall, kept as u64, last as u64);
      while marks.get(passed).is_some_and(|mark| kept >= *mark) {
        (passed, opening) = (passed + 1, true);
      }
      let fact = json_of(&a);
      if !self.restates(&mut opening, &fact) {
        let acts = fold.hear(&fact, &[], None);
        self.put(Event::Fact(fact));
        for act in acts {
          self.put(Event::Act(Arc::new(act)));
        }
        self.let_go();
      }
    };
    // A read that fails after it told a fact leaves the facts before it told, and the life is
    // born after them, as after a past that the read gave whole.
    if retold.before_kept(last, &mut take).is_ok() || self.kept.said > 0 {
      (told.0)(Stage::Recall, last as u64, last as u64);
      self.reborn();
    }
  }

  /// Each fact of the log let go, since the record holds it: those that a checkpoint of the record
  /// passed. The log keeps an empty place for each, which [`Life::since`] fills from the record.
  /// A fact that kept a checkpoint or a fact made stays, since the log of the record holds that
  /// entry as its number alone.
  fn let_go(&mut self) {
    let opened = mem::take(&mut self.kept.gone.opened);
    let kept = |at: usize| opened.iter().any(|one| one.contains(&at));
    for (at, slot) in self.kept.log.iter_mut().enumerate().skip(self.kept.gone.scanned) {
      let fact = slot.as_ref().and_then(Slot::fact);
      let Some(fact) = fact.filter(|fact| !kept(at) && !kept_checkpoint(fact)) else { continue };
      self.kept.gone.places.push((at as Seq, Digest::of(fact)));
      *slot = None;
    }
    self.kept.gone.scanned = self.kept.log.len();
  }

  /// The birth after the past of the record: each event told so far, then each fact let go, since
  /// the record holds it, then the event of the birth, on no image.
  fn reborn(&mut self) {
    self.told(None);
    self.let_go();
    self.put(Event::Reborn(None));
    self.kept.watch.reborn();
  }

  /// The events of the log from a place, and the read of the facts among them that the log let
  /// go, which the reader reads from the record.
  fn since(&self, from: Seq) -> Since {
    let start = usize::try_from(from).unwrap_or(usize::MAX);
    let since = self.kept.log.iter().enumerate().skip(start);
    let logged = since.filter_map(|(at, one)| {
      let event = match one.as_ref()? {
        Slot::Told(event) => Arc::clone(event),
        Slot::Held(id) => Event::Act(Arc::clone(self.kept.held.shared(id)?)).into(),
      };
      Some((at as Seq, event))
    });
    let wanted =
      |places: &[(Seq, Digest)]| places[places.partition_point(|(at, _)| *at < from)..].to_vec();
    let retells = self.record.as_deref().map_or_else(Vec::new, |path| {
      let gone = &self.kept.gone;
      let record = Retell { from: Source::Record(path.to_owned()), wanted: wanted(&gone.places) };
      let log = Retell { from: Source::Log(format!("{path}.log")), wanted: wanted(&gone.logged) };
      [record, log].into_iter().filter(|one| !one.wanted.is_empty()).collect()
    });
    Since { logged: logged.collect(), retells, tip: self.kept.log.len() as Seq }
  }
}

/// What a fact that the log let go was: a fact of its digest, or the fact `keep` by which the
/// journal, or the operator for a checkpoint, told that the record kept an entry, which holds the
/// fact of its digest, with that speaker. The file of the
/// record holds that entry as its number alone, so a reader makes the fact again from the fact
/// that the entry holds.
#[derive(Clone, Serialize, Deserialize)]
enum Digest {
  Fact(u64),
  Keep(u64, String),
}

impl Digest {
  /// What a fact of the log is.
  fn of(fact: &Value) -> Self {
    match fact.as_array().map(Vec::as_slice) {
      Some([kind, about, by, Value::Array(entry)])
        if kind == "keep" && about == "" && entry.len() == 1 =>
      {
        Self::Keep(digest(&entry[0]), by.as_str().unwrap_or_default().to_owned())
      }
      _ => Self::Fact(digest(fact)),
    }
  }
}

/// The events of the log from a place, and the reads of the facts among them that the log let go.
struct Since {
  logged: Vec<Logged>,
  retells: Vec<Retell>,
  tip: Seq,
}

/// What holds the facts that the log let go: the record, or the file of the log of the record,
/// which holds each fact that the life said, as `json_of` writes it, so each of its lines has the
/// digest of its fact.
enum Source {
  Record(String),
  Log(String),
}

/// The read of the facts that the log let go from a place on: their source, and the place and
/// digest of each. A source holds the facts in the order that the engine said them, which is not
/// always the order in which the life told them, so a fact of the source takes the first place of
/// its digest that no fact took yet, and the read gives each in the order of the places, as soon as
/// each place before it is taken. A place is a gap only when the read of the source ends with no
/// fact for it, as when the source does not read: a reader never loses a fact that it holds.
struct Retell {
  from: Source,
  wanted: Vec<(Seq, Digest)>,
}

/// The places that wait for a fact of the record, by the kind and the digest of the fact, each with
/// the speaker of the entry that a `keep` fact says.
type Places = HashMap<(bool, u64), VecDeque<(Seq, Option<String>)>>;

/// The facts of each read, given to a reader in the order of their places, one ahead of what the
/// reader read: a fact that is read again is a copy, and the facts that an open told are large.
/// Each read gives its own facts in order.
fn retold(retells: Vec<Retell>) -> Option<mpsc::Receiver<Logged>> {
  let mut reads: Vec<mpsc::Receiver<Logged>> = retells.into_iter().map(Retell::spawned).collect();
  if reads.len() < 2 {
    return reads.pop();
  }
  let (sender, receiver) = mpsc::channel(1);
  tokio::spawn(async move {
    let mut heads: Vec<Option<Logged>> = vec![None; reads.len()];
    loop {
      for (read, head) in reads.iter_mut().zip(&mut heads) {
        if head.is_none() {
          *head = read.recv().await;
        }
      }
      let first = heads
        .iter_mut()
        .filter(|one| one.is_some())
        .min_by_key(|one| one.as_ref().map(|one| one.0));
      let Some(one) = first.and_then(Option::take) else { return };
      if sender.send(one).await.is_err() {
        return;
      }
    }
  });
  Some(receiver)
}

impl Retell {
  /// The facts, read on a thread that may block, sent in the order of their places to a reader,
  /// one of them in the channel ahead of what the reader read.
  fn spawned(self) -> mpsc::Receiver<Logged> {
    let (sender, receiver) = mpsc::channel(1);
    spawn_blocking(move || self.told(&sender));
    receiver
  }

  /// Each fact read and sent, until the source ends, each place is filled, or the reader is gone.
  fn told(self, sender: &mpsc::Sender<Logged>) {
    let mut places: Places = HashMap::new();
    for (at, one) in &self.wanted {
      let (key, by) = match one {
        Digest::Fact(one) => ((false, *one), None),
        Digest::Keep(one, by) => ((true, *one), Some(by.clone())),
      };
      places.entry(key).or_default().push_back((*at, by));
    }
    let mut order: VecDeque<Seq> = self.wanted.iter().map(|one| one.0).collect();
    // The facts that wait for a place before them. The journal tells the keep of each fact of a
    // run at the end of that run, so the keeps of a whole run wait here while the read passes the
    // run. No bound cuts the wait, since a place that is given up before the read reaches its
    // fact is a fact lost. At its worst, when the source lacks the fact of the first place, each
    // fact that is retold waits here until the read ends: the facts that the log let go, and no
    // more, since a fact with no place is not held.
    let mut ready: BTreeMap<Seq, Value> = BTreeMap::new();
    let give = |at: Seq, fact: Value| {
      sender.blocking_send((at, Arc::new(Event::Fact(fact)))).ok();
    };
    let first = order.front().copied();
    // A fact of the source by its digest, which the read makes only for a place that takes it,
    // and whether the read goes on.
    let mut take = |one: u64, fact: &dyn Fn() -> Value| {
      if let Some((at, by)) = places.get_mut(&(true, one)).and_then(VecDeque::pop_front) {
        ready.insert(at, json!(["keep", "", by, [fact()]]));
      }
      if let Some((at, _)) = places.get_mut(&(false, one)).and_then(VecDeque::pop_front) {
        ready.insert(at, fact());
      }
      while let Some(&at) = order.front() {
        let Some(fact) = ready.remove(&at) else { break };
        give(at, fact);
        order.pop_front();
      }
      !order.is_empty() && !sender.is_closed()
    };
    let (read, name) = match &self.from {
      Source::Record(record) => (read_record(record, &mut take), record),
      Source::Log(log) => (read_log(log, &mut take), log),
    };
    if read.is_none()
      && let Some(at) = first
    {
      let why =
        format!("The file {name} does not read, so the facts that the log let go are gaps here.");
      sender.blocking_send((at, Arc::new(Event::Notice(why)))).ok();
    }
    // The read ended: each place that has no fact is a gap, and each fact that waited behind one
    // goes in the order of the places.
    for (at, fact) in ready {
      give(at, fact);
    }
  }
}

/// A fact by its digest, and whether the read goes on.
type Take<'a> = dyn FnMut(u64, &dyn Fn() -> Value) -> bool + 'a;

/// Each fact of the whole record: a fact that a life said as it wrote its checkpoint may stand
/// after that checkpoint in the record. Once each place is filled, the rest of
/// the record is read and left.
fn read_record(record: &str, take: &mut Take<'_>) -> Option<()> {
  let mut goes = true;
  let each = &mut |one: Object| {
    if let Some(fact) = Fact::of(one.as_ref()).map(|a| json_of(&a)).filter(|_| goes) {
      goes = take(digest(&fact), &|| fact.clone());
    }
  };
  world::retold_live(record).ok()?.each(&mut |one| each(one)).ok()
}

/// Each line of the file of the log, until each place is filled: a line is a fact as JSON writes
/// it, so its digest is the digest of its text, and only a line that a place takes is parsed.
fn read_log(log: &str, take: &mut Take<'_>) -> Option<()> {
  for line in BufReader::new(File::open(log).ok()?).lines() {
    let line = line.ok()?;
    if !take(digested(&line), &|| serde_json::from_str(&line).unwrap_or_default()) {
      break;
    }
  }
  Some(())
}

/// Whether a fact tells that the record kept a checkpoint, or a fact made, which a life keeps
/// after its checkpoint and the store at the birth of a record.
fn kept_checkpoint(fact: &Value) -> bool {
  let entry = fact.get(3).and_then(|one| one.get(0)).and_then(|one| one.get(0));
  fact.get(0).and_then(Value::as_str) == Some("keep")
    && entry
      .and_then(Value::as_str)
      .is_some_and(|kind| [checkpoint::CHECKPOINT, world::MADE].contains(&kind))
}

/// The digest of a fact, by which the log knows a fact that it told.
fn digest(fact: &Value) -> u64 {
  digested(&fact.to_string())
}

/// The digest of a fact as JSON writes it.
fn digested(text: &str) -> u64 {
  let mut hasher = DefaultHasher::new();
  text.hash(&mut hasher);
  hasher.finish()
}

/// The thread of a life, until each handle of the life is gone.
#[expect(
  clippy::disallowed_methods,
  reason = "the thread of each life blocks on its own current_thread runtime by design (#983 section 2)"
)]
fn lived(opening: Opening, received: UnboundedReceiver<Message>, tells: Tells) {
  match runtime::Builder::new_current_thread().enable_all().build() {
    Ok(runtime) => LocalSet::new().block_on(&runtime, run(opening, received, tells)),
    Err(no) => {
      let ended = Event::Ended(Some(format!("the life has no runtime: {no}")));
      tells.readers.send((0, Arc::new(ended))).ok();
    }
  }
}

/// Where the thread of a life tells what each handle of the life reads: its events, the name of
/// its session, how far its open stands, and its cards.
struct Tells {
  readers: broadcast::Sender<Logged>,
  name: Arc<Mutex<Option<String>>>,
  opens: Told,
  cards: watch::Sender<Option<Arc<Session>>>,
}

/// The life: opened, then driven until it ends, and its log held for the readers after its end.
async fn run(mut opening: Opening, mut received: UnboundedReceiver<Message>, tells: Tells) {
  let Tells { readers, name, opens, cards } = tells;
  let record = opening.record.clone().filter(|_| opening.keeps());
  let (told, heard) = mpsc::unbounded_channel();
  let configs = Layers::new(&opening.directory, opening.config.as_ref().map(PathBuf::from))
    .claude(opening.claude.as_ref().map(PathBuf::from));
  let mut life = Life {
    kept: Kept::default(),
    readers,
    heard,
    configs,
    roster: opening.roster.get_or_insert_default().clone(),
    name,
    first: record.as_deref().map(|one| cards::changed(Path::new(one))),
    times: Times::kept(record.as_deref()),
    record,
    opens,
    cards,
    stale: false,
    due: Instant::now(),
  };
  let writes = told.clone();
  opening.stream = Some(Stream(Arc::new(move |rung, chain, text, thinking| {
    let (rung, chain) = (rung.to_owned(), chain.to_owned());
    let (text, thinking) = (text.to_owned(), thinking.to_owned());
    writes.send(Event::Text { rung, chain, text, thinking }).ok();
  })));
  let reports = told.clone();
  opening.log = Some(Log(Arc::new(move |line| {
    reports.send(Event::Notice(line.to_owned())).ok();
  })));
  let folder = PathBuf::from(&opening.directory);
  let journal = life.record.clone().map(|one| Journal::spawn(folder, journal(&one), told.clone()));
  let end = lives(opening, &mut life, &mut received, &told, journal.as_ref()).await;
  let fault = flushed(end.fault, journal.as_ref()).await;
  for asked in end.asked {
    asked.send(fault.clone().map_or(Ok(Reply::Done), |one| Err(LifeError::Refused(one)))).ok();
  }
  life.told(None);
  if end.stood {
    life.indexed();
  }
  life.put(Event::Ended(fault.clone()));
  // A verb that waited for an open that failed hears why, and not only that the life ended.
  while let Some(message) = received.recv().await {
    match message {
      Message::Read(read) => read.answered(None, &mut life).await,
      Message::Command(_, reply) => {
        drop(reply.send(Err(fault.clone().map_or(LifeError::Ended, LifeError::Refused))));
      }
    }
  }
}

/// The fault that ended a life, with what its journal lost, once each change of its files is on the
/// disk: the end of a life waits for it, so a client that hears the end finds the journal whole.
async fn flushed(fault: Option<String>, journal: Option<&Journal>) -> Option<String> {
  let Some(journal) = journal else { return fault };
  match (fault, journal.flushed().await) {
    (fault, Ok(())) => fault,
    (None, Err(lost)) => Some(lost.to_string()),
    (Some(fault), Err(lost)) => Some(format!("{fault}, and {lost}")),
  }
}

/// How a life ended: the fault that ended it, when one did, each client that ended it, which
/// hears the reply once the life is over, and whether the life stood. A life that never opened
/// holds no act of the tail of its record, so it writes no index of its threads.
struct End {
  fault: Option<String>,
  asked: Vec<Replier>,
  stood: bool,
}

/// The life opened and driven until it ends, on the one journal of its changes, and how it ended.
///
/// A life whose image agrees with its record opens from it: the host stands on what the life of
/// the image kept, reads no fact of the record before the image, and hears what the engine says
/// of each entry after it, as after an image at rest. Each other life tells the past of its record
/// first, as [`Life::recalled`] says, and opens by the record. A life whose image does not load
/// opens by the record once, and tells its past after that open and before its birth.
async fn lives(
  opening: Opening,
  life: &mut Life,
  received: &mut UnboundedReceiver<Message>,
  told: &UnboundedSender<Event>,
  journal: Option<&Journal>,
) -> End {
  let record = opening.record.clone().filter(|_| opening.keeps());
  let progress = life.opens.progress();
  life.opens.born();
  let images = record.as_deref().is_some_and(|one| image::agrees(Path::new(one)));
  if let Some(one) = record.as_deref().filter(|_| !images) {
    life.recalled(one, &progress);
  }
  let ears = ears(told.clone(), journal);
  let mut kept = None::<Kept>;
  // A life that told its past stands on it, and on no image that came after.
  let mut stands = |part: &[u8]| {
    let read = serde_json::from_slice::<Part<Kept>>(part).ok();
    kept = read.filter(|one| images && one.build == BUILD).map(|one| one.kept);
    kept.is_some()
  };
  let engine = match opening.open(ears, progress.clone(), &mut stands).await {
    Ok((engine, _)) => engine,
    Err(fault) => return End { fault: Some(fault.to_string()), asked: Vec::new(), stood: false },
  };
  match (kept, images, record.as_deref()) {
    (Some(kept), ..) => life.imaged(kept, &progress),
    (None, images, record) => {
      let birth = iter::from_fn(|| life.heard.try_recv().ok()).collect();
      // The image named the tip of the record and did not load, as one whose body is torn, so
      // the life opened by its record before it told its past: it tells its past, then the
      // birth that its record already keeps.
      if let (true, Some(one)) = (images, record) {
        life.recalled(one, &progress);
      }
      life.opened(birth, &progress);
    }
  }
  let (fault, asked) = driven(engine, life, received, record.as_deref()).await;
  End { fault: fault.map(|one| one.to_string()), asked, stood: true }
}

/// What woke the thread of a life.
enum Woke {
  Told(Event),
  Stopped(Fault),
  Said(Option<Message>),
  /// The record holds each fact that the life waited for it to hold, or the life told its cards,
  /// which were due.
  Kept,
}

/// The engine driven while it hears each command, until a client ends it, each handle is gone, or
/// a fault stops it, with that fault and each client that ended it. A life that keeps a record
/// ends with its image, as [`rebirth::ended`] says.
///
/// While the record holds no place for facts that the life said, as on a full disk, the engine
/// holds them in memory, and a drop of the engine loses them: the life keeps each end that it is
/// told and outlives its handles until the record takes each of them. It wakes
/// on the outcome of each write of the record, which the ear of the record tries again on a clock
/// of its own, since no event tells that a disk has room again.
async fn driven(
  mut engine: Engine,
  life: &mut Life,
  received: &mut UnboundedReceiver<Message>,
  record: Option<&str>,
) -> (Option<Fault>, Vec<Replier>) {
  let (mut gone, mut asked) = (false, Vec::new());
  // A life that opened on no image may owe one before anything wakes it.
  owed_image(&mut engine, life, record);
  loop {
    let ends = gone || !asked.is_empty();
    let owed = life.kept.watch.images(life.kept.said);
    let waits =
      (ends || life.kept.backlog.holds() || record.is_some() && owed) && engine.unkept() > 0;
    let woke = tokio::select! {
      biased;
      Some(event) = life.heard.recv() => Woke::Told(event),
      () = sleep_until(life.due), if life.stale => { life.carded(); Woke::Kept }
      woke = pumped(&mut engine, waits) => woke,
      message = received.recv(), if !gone => Woke::Said(message),
    };
    match woke {
      Woke::Told(event) => life.told(Some(event)),
      Woke::Kept => {}
      Woke::Stopped(fault) => {
        drop(engine);
        return (Some(fault), asked);
      }
      Woke::Said(None) if engine.unkept() > 0 => gone = true,
      Woke::Said(None) => return (ended(engine, life), asked),
      Woke::Said(Some(Message::Read(read))) => read.answered(Some(&mut engine), life).await,
      Woke::Said(Some(Message::Command(Command::End, reply))) => {
        asked.push(reply);
        if engine.unkept() == 0 {
          return (ended(engine, life), asked);
        }
        life.put(Event::Notice(backlog::unkept(engine.unkept())));
      }
      Woke::Said(Some(Message::Command(command, reply))) => {
        let replied = commanded(&mut engine, life, command);
        reply.send(replied.await).ok();
        if let Some(fault) = engine.stopped().cloned() {
          drop(engine);
          return (Some(fault), asked);
        }
      }
    }
    drained(&mut engine, life);
    weighed(&mut engine, life, record);
    if let Some(fault) = engine.stopped().cloned() {
      drop(engine);
      return (Some(fault), asked);
    }
    if (gone || !asked.is_empty()) && engine.unkept() == 0 {
      return (ended(engine, life), asked);
    }
    owed_image(&mut engine, life, record);
  }
}

/// The engine of a life dropped at the end of the life, as [`rebirth::ended`] says, with what the
/// life keeps once the log took each event that the ears told.
fn ended(engine: Engine, life: &mut Life) -> Option<Fault> {
  life.told(None);
  // The index stands before the image: a daemon that ends between them leaves a record whose
  // next daemon reads its cards from the index, where none would fold the whole record for them.
  life.indexed();
  // The next life opens from this image, so it stands on one.
  life.kept.watch.imaged();
  let record = life.record.as_deref().map(Path::new);
  let part = Part { build: BUILD.into(), kept: &life.kept };
  rebirth::ended(engine, record, &part)
}

/// The image of a life that keeps its record and passed its tail, as [`rebirth::TAIL`] says,
/// written beside its record once the record holds each fact. The life goes on, with no
/// checkpoint: a later open goes on from the image and feeds what the record took after it. A life
/// that cannot write an image tries again past its next tail.
fn owed_image(engine: &mut Engine, life: &mut Life, record: Option<&str>) {
  let owed = life.kept.watch.images(life.kept.said) && engine.unkept() == 0;
  let Some(record) = record.filter(|_| owed) else { return };
  life.told(None);
  life.kept.watch.imaged();
  let part = Part { build: BUILD.into(), kept: &life.kept };
  if let Err(no) = image::keep(engine, Path::new(record), &part) {
    life.put(Event::Notice(format!("The life wrote no image at its rest: {no}")));
  }
}

/// The work of a life paused or woken as its record lacks facts or holds each again, as
/// [`backlog`] says, with the notice of each change.
fn weighed(engine: &mut Engine, life: &mut Life, record: Option<&str>) {
  match record.map(|one| life.kept.backlog.weighed(engine, one)) {
    Some(Ok(Some(notice))) => life.put(Event::Notice(notice)),
    Some(Err(no)) => life.put(Event::Notice(format!("The life cannot hold its work: {no}"))),
    Some(Ok(None)) | None => {}
  }
}

/// What the engine comes to, pumped: a fault that stops it, or, when the life waits for its record,
/// the moment the record holds each fact that the life said. Each pump lets the ear of the record
/// write what waits, so the life hears the outcome of each write.
async fn pumped(engine: &mut Engine, waits: bool) -> Woke {
  poll_fn(|cx| match engine.pump(cx.waker()) {
    Err(fault) => Poll::Ready(Woke::Stopped(fault)),
    Ok(()) if waits && engine.unkept() == 0 => Poll::Ready(Woke::Kept),
    Ok(()) => Poll::Pending,
  })
  .await
}

/// What a command comes to: on the configs of the life for a command of its configs, on the home of
/// its roster for a change of the roster, on the queue of the life for a command of the queue, on
/// the acts that the life told for a query of their meaning, and else on the engine.
async fn commanded(engine: &mut Engine, life: &Life, command: Command) -> Result<Reply, LifeError> {
  let root = engine.root().to_owned();
  let chain = |on: Option<String>| on.unwrap_or_else(|| root.clone());
  match command {
    Command::By { speaker, command } => spoken(engine, &speaker, *command),
    Command::Queue { entry, shape, message, on } => {
      let chain = chain(on);
      if !life.kept.queue.holds(&entry) {
        let words = ["add", &entry, &shape, &message].map(Object::string);
        engine.say(QUEUE, &chain, words.into())?;
      }
      Ok(Reply::Done)
    }
    Command::Unqueue { entry } => {
      let follow = life.kept.queue.takes(&entry);
      let follow =
        follow.ok_or_else(|| LifeError::Refused(format!("No follow-up {entry} waits.")))?;
      engine.say(QUEUE, &follow.chain, ["drop", &entry].map(Object::string).into())?;
      Ok(Reply::Done)
    }
    Command::Queued { on } => {
      let waiting = life.kept.queue.waiting(&chain(on));
      let waiting = waiting.iter().map(serde_json::to_value).collect::<Result<_, _>>();
      Ok(Reply::Values(waiting.map_err(|no| LifeError::Refused(no.to_string()))?))
    }
    command @ (Command::Desk { .. }
    | Command::Threads { .. }
    | Command::Rail { .. }
    | Command::Ladder { .. }
    | Command::AsOf { .. }
    | Command::Moments { .. }
    | Command::Thread { .. }) => viewed(life, &root, command),
    Command::Settings => {
      let settings = life.configs.get()?.into_iter().map(|one| json!(one));
      Ok(Reply::Values(settings.collect()))
    }
    Command::Configure(change) => {
      life.configs.set(change).await?;
      answered_by(engine, Command::Stand)
    }
    Command::Roster(names) => {
      life.roster.set(Some(names));
      answered_by(engine, Command::Stand)
    }
    other => answered_by(engine, other),
  }
}

/// What a query of the meaning of the acts that the life told comes to, on its chain or on the
/// root.
fn viewed(life: &Life, root: &str, command: Command) -> Result<Reply, LifeError> {
  let chain = |on: Option<String>| on.unwrap_or_else(|| root.to_owned());
  match command {
    Command::Desk { notices } => Ok(Reply::Desk(life.kept.held.desk(&notices))),
    Command::Threads { on, at, read, site } => {
      let site = site.as_deref().unwrap_or("operator");
      Ok(Reply::Threads(life.kept.held.threads_as(site, &chain(on), at, read.unwrap_or_default())))
    }
    Command::Rail { hidden, at } => Ok(Reply::Rail(life.kept.held.rail(&hidden, at))),
    Command::Ladder { on, at, opened, owed, taken, shown } => {
      let shown = shown.unwrap_or(SHOWN);
      Ok(Reply::Ladder(life.kept.held.ladder(
        &chain(on),
        at,
        &opened,
        &owed,
        taken.as_deref(),
        shown,
      )))
    }
    Command::AsOf { at } => Ok(Reply::Acts(life.kept.held.as_of(at))),
    Command::Moments { on } => Ok(Reply::Moments(life.kept.held.moments(on.as_deref()))),
    Command::Thread { thread, reach } => {
      Ok(Reply::Page(life.kept.held.page(&life.kept.streams, &thread, reach)))
    }
    other => Err(LifeError::Refused(format!("{other:?} is no view"))),
  }
}

/// Each follow-up that is due sent as a prompt on its chain, or why its send failed, which holds
/// the queue of its chain and stands in the record.
fn drained(engine: &mut Engine, life: &mut Life) {
  for follow in life.kept.queue.due() {
    let made = sent(engine, &follow).map_err(|no| no.to_string());
    if let Err(why) = &made {
      let words = ["failed", &follow.entry, why].map(Object::string);
      if let Err(no) = engine.say(QUEUE, &follow.chain, words.into()) {
        life.put(Event::Notice(format!(
          "The failure of the follow-up {} is not kept: {no}",
          follow.entry
        )));
      }
    }
    life.kept.queue.sent(&follow.entry, made);
  }
}

/// What a command of the engine comes to, said by a speaker: the life speaks as that speaker for
/// the command alone, then as before, whatever the command came to.
fn spoken(engine: &mut Engine, speaker: &str, command: Command) -> Result<Reply, LifeError> {
  let before = engine.site(Some(speaker))?;
  let reply = answered_by(engine, command);
  engine.site(Some(&before))?;
  reply
}

/// What a command comes to on the engine.
fn answered_by(engine: &mut Engine, command: Command) -> Result<Reply, LifeError> {
  let root = engine.root().to_owned();
  let on = |chain: Option<String>| Some(chain.unwrap_or_else(|| root.clone()));
  let id = |made: &str| Reply::Act(made.to_owned());
  Ok(match command {
    Command::Prompt { shape, message, to, on: chain } => {
      let with = verbs::Prompt { message: Some(message), to, on: on(chain) };
      id(engine.prompt(Shape::object(&shape), with)?.id())
    }
    Command::Close { id, value } => {
      engine.close(parsed(&value)?, verbs::Close { id: Some(id) })?;
      Reply::Done
    }
    Command::Grant { usd, share, on: chain } => {
      id(engine.grant(verbs::Grant { usd, share, on: on(chain) })?.id())
    }
    Command::Bash { command, stdin, on: chain } => {
      let with = verbs::Bash { stdin, on: on(chain), ..verbs::Bash::default() };
      id(engine.bash(&command, with)?.id())
    }
    Command::Read { path, on: chain } => {
      let text = engine.read(&path, verbs::Read { on: on(chain), ..verbs::Read::default() })?;
      Reply::Value(json!({ "path": text.path, "content": text.content }))
    }
    Command::Write { path, content, on: chain } => {
      engine.write(&Text { path, content }, verbs::Write { on: on(chain) })?;
      Reply::Done
    }
    Command::Stand => id(engine.stand(verbs::Stand { on: on(None) })?.id()),
    Command::Seen { about } => {
      engine.say("seen", &about, vec![])?;
      Reply::Done
    }
    Command::Rung { word, on: chain } => {
      let with = verbs::Rung { word: Some(word), on: on(chain), ..verbs::Rung::default() };
      id(engine.rung(with)?.id())
    }
    Command::Answer { id, line } => {
      let shape = engine.get(&id)?.filter(|one| one.kind() == "prompt");
      let shape = shape.and_then(|one| one.word(1).and_then(|w| w.as_str().map(str::to_owned)));
      let shape = shape.ok_or_else(|| LifeError::Refused(format!("No prompt is named {id}.")))?;
      engine.close(answered(&shape, &line)?, verbs::Close { id: Some(id) })?;
      Reply::Done
    }
    control @ (Command::Pause { .. } | Command::Wake { .. } | Command::Cancel { .. }) => {
      controlled(engine, control)?
    }
    Command::Chain { label, source, on: chain } => {
      let with = verbs::Chain { label, source, on: on(chain), ..verbs::Chain::default() };
      id(engine.chain(with)?.id())
    }
    Command::End => Reply::Done,
    query => queried(engine, query, &root)?,
  })
}

/// What a pause, a wake or a cancel comes to on the engine.
fn controlled(engine: &mut Engine, control: Command) -> Result<Reply, LifeError> {
  Ok(match control {
    Command::Pause { id, why } => {
      engine.pause(&id, verbs::Pause { why: why.map(Object::string) }).map(|()| Reply::Done)?
    }
    Command::Wake { id } => engine.wake(&id).map(|()| Reply::Done)?,
    Command::Cancel { id, why } => {
      engine.cancel(&id, verbs::Cancel { why: why.map(Object::string) }).map(|()| Reply::Done)?
    }
    other => return Err(LifeError::Refused(format!("{other:?} is no control"))),
  })
}

/// What a query gives, read on the engine.
fn queried(engine: &mut Engine, query: Command, root: &str) -> Result<Reply, LifeError> {
  let on = |chain: Option<String>| chain.unwrap_or_else(|| root.to_owned());
  let named = |chain: Option<String>| vec![("on", Object::string(on(chain)))];
  Ok(match query {
    Command::Turns { on: chain } => {
      let turns = engine.turns(verbs::Turns { on: Some(on(chain)) })?;
      Reply::Values(turns.iter().map(|one| outward(one.as_ref())).collect())
    }
    Command::Transcript { on: chain, of } => {
      let facts = engine.transcript(verbs::Transcript { on: Some(on(chain)), of })?;
      Reply::Values(facts.iter().map(|one| outward(one.0.as_ref())).collect())
    }
    Command::Program { on: chain } => {
      Reply::Value(outward(engine.verb("program", vec![], named(chain))?.as_ref()))
    }
    Command::Standing => Reply::Value(outward(engine.standing()?.as_ref())),
    Command::Filled { id } => {
      Reply::Value(outward(engine.verb("filled", vec![Object::string(id)], vec![])?.as_ref()))
    }
    Command::Inspect { name, on: chain } => {
      let (kind, repr, value) = engine.inspect(&name, Some(&on(chain)))?;
      Reply::Inspected { kind, repr, value: outward(value.as_ref()) }
    }
    Command::Get { id } => {
      Reply::Value(engine.get(&id)?.map_or(Value::Null, |made| outward(made.0.as_ref())))
    }
    Command::Peek { id } => match engine.outcome(&id)? {
      Some(got) => Reply::Value(outward(got.as_ref())),
      None => return Err(LifeError::Refused(format!("No act named {id} is done."))),
    },
    Command::Cwd { on: chain } => {
      Reply::Value(Value::String(engine.cwd(verbs::Cwd { on: Some(on(chain)) })?))
    }
    Command::Pending => {
      let pending = engine.pending()?.into_iter().map(|(id, kind)| json!([id, kind]));
      Reply::Values(pending.collect())
    }
    Command::Names { on: chain } => {
      Reply::Values(engine.names(Some(&on(chain)))?.into_iter().map(Value::String).collect())
    }
    acted => return Err(LifeError::Refused(format!("{acted:?} is no query"))),
  })
}

/// The act of the prompt that a follow-up went as on its chain, between the facts that begin and
/// end its send.
fn sent(engine: &mut Engine, follow: &FollowUp) -> Result<String, LifeError> {
  let chain = follow.chain.as_str();
  engine.say(QUEUE, chain, ["begin", &follow.entry].map(Object::string).into())?;
  let with =
    verbs::Prompt { message: Some(follow.message.clone()), to: None, on: Some(chain.to_owned()) };
  let made = engine.prompt(Shape::object(&follow.shape), with)?.id().to_owned();
  engine.say(QUEUE, chain, ["sent", &follow.entry, &made].map(Object::string).into())?;
  Ok(made)
}

impl LifeHandle {
  /// The name of the session of the life, as its facts fold it: the one that the operator said last
  /// on the root, which wins, else the one that the skill naming gave, and nothing while it
  /// holds neither.
  pub(crate) fn cards(&self) -> watch::Receiver<Option<Arc<Session>>> {
    self.cards.clone()
  }

  /// The cards that the life told last, and none before it told any.
  pub(crate) fn session(&self) -> Option<Arc<Session>> {
    self.cards.borrow().clone()
  }

  pub fn name(&self) -> Option<String> {
    self.name.lock().ok().and_then(|name| name.clone())
  }

  /// How far the open of the life stands, as one window hears it: each stage of the open that
  /// runs now.
  pub(crate) fn opens(&self) -> Heard {
    Heard::new(self.opens.clone())
  }

  /// Each event that the life tells from now on, with its place in the log, which a reader that
  /// stands too far behind misses.
  pub(crate) fn told(&self) -> broadcast::Receiver<Logged> {
    self.readers.subscribe()
  }

  /// The reply of the life to a command, once the life ran it, in the order that the life heard
  /// each command.
  pub async fn send(&self, command: Command) -> Result<Reply, LifeError> {
    let (reply, replied) = oneshot::channel();
    if self.inbox.send(Message::Command(command, reply)).is_err() {
      return Err(LifeError::Ended);
    }
    replied.await.unwrap_or(Err(LifeError::Ended))
  }

  /// The events of the life from a place in its log, each once and in order, until the event that
  /// ends the life. A reader that stands too far behind the life reads again from the log of the
  /// life, which holds no event that a later one replaced: of each act its last event, and no part
  /// that a model wrote for a rung that has its word. Such a reader may find a gap in the places.
  pub fn events(&self, from: Seq) -> impl Streams<Item = Logged> + Send + 'static {
    stream::unfold(self.reader(self.readers.subscribe(), from, true), Reader::next)
  }

  /// The events that the log of the life holds from a place, as [`LifeHandle::events`] gives
  /// them, with no fact that the log let go: a reader that needs the acts that live and no fact of
  /// the past reads nothing of the record.
  pub fn held(&self, from: Seq) -> impl Streams<Item = Logged> + Send + 'static {
    stream::unfold(self.reader(self.readers.subscribe(), from, false), Reader::next)
  }

  /// The events of the life from its tip at the call on, as [`LifeHandle::events`] gives them,
  /// or none once the thread of the life is gone.
  pub async fn tipped(&self) -> Option<impl Streams<Item = Logged> + Send + 'static> {
    let receiver = self.readers.subscribe();
    let tip = self.since(Seq::MAX).await?.tip;
    Some(stream::unfold(self.reader(receiver, tip, true), Reader::next))
  }

  /// The reply of a command, and the events of the life from the place where the life gave that
  /// reply, as [`LifeHandle::events`] gives them, or none once the thread of the life is gone.
  pub async fn after(
    &self,
    command: Command,
  ) -> Option<(Result<Reply, LifeError>, impl Streams<Item = Logged> + Send + 'static)> {
    let receiver = self.readers.subscribe();
    let (reply, replied) = oneshot::channel();
    self.inbox.send(Message::Read(Read::After(command, reply))).ok()?;
    let (got, tip) = replied.await.ok()?;
    Some((got, stream::unfold(self.reader(receiver, tip, true), Reader::next)))
  }

  /// A reader of the events of the life from a place, which reads again from the record or the
  /// file of its log the facts that the log let go when it `retells`.
  fn reader(&self, receiver: broadcast::Receiver<Logged>, from: Seq, retells: bool) -> Reader {
    Reader {
      life: self.clone(),
      receiver,
      next: from,
      read: VecDeque::new(),
      retold: None,
      ahead: None,
      behind: true,
      over: false,
      retells,
    }
  }

  /// The events of the log of the life from a place, or none once the thread of the life is gone.
  async fn since(&self, from: Seq) -> Option<Since> {
    let (reply, replied) = oneshot::channel();
    self.inbox.send(Message::Read(Read::Since(from, reply))).ok()?;
    replied.await.ok()
  }
}

/// A reader of the events of a life: the place of the next event that it gives, the events that it
/// read from the log and did not give yet, the facts among them that the log let go as the record
/// gives them again, with the next of them, whether it stands behind the life, whether it gave
/// the end, and whether it reads those facts again.
struct Reader {
  life: LifeHandle,
  receiver: broadcast::Receiver<Logged>,
  next: Seq,
  read: VecDeque<Logged>,
  retold: Option<mpsc::Receiver<Logged>>,
  retells: bool,
  ahead: Option<Logged>,
  behind: bool,
  over: bool,
}

impl Reader {
  /// The next event, and the reader after it: of the events read from the log and the facts that
  /// the record gives again, the one of the first place.
  async fn next(mut self) -> Option<(Logged, Self)> {
    while !self.over {
      if self.ahead.is_none()
        && let Some(retold) = &mut self.retold
      {
        match retold.recv().await {
          Some(one) => self.ahead = Some(one),
          None => self.retold = None,
        }
        continue;
      }
      let first = match (&self.ahead, self.read.front()) {
        (Some(fact), Some(logged)) => fact.0 < logged.0,
        (Some(_), None) => true,
        (None, _) => false,
      };
      if first {
        if let Some(fact) = self.ahead.take()
          && fact.0 >= self.next
        {
          return Some(self.given(fact));
        }
      } else if let Some(logged) = self.read.pop_front() {
        if logged.0 >= self.next {
          return Some(self.given(logged));
        }
      } else if self.behind {
        self.behind = false;
        let Since { logged, retells, .. } = self.life.since(self.next).await?;
        self.read = logged.into();
        self.retold = if self.retells { retold(retells) } else { None };
        self.ahead = None;
      } else {
        match self.receiver.recv().await {
          Ok(logged) if logged.0 == self.next => return Some(self.given(logged)),
          Ok(logged) if logged.0 < self.next => {}
          Ok(_) | Err(RecvError::Lagged(_)) => self.behind = true,
          Err(RecvError::Closed) => return None,
        }
      }
    }
    None
  }

  /// One event given, and the reader after it.
  fn given(mut self, logged: Logged) -> (Logged, Self) {
    self.next = logged.0 + 1;
    self.over = matches!(*logged.1, Event::Ended(_));
    (logged, self)
  }
}
