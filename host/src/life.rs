//! A life as the host holds it: a thread of its own, with a runtime of tokio and a set of local
//! tasks, since the engine is not `Send`. The thread owns the engine. It awaits the open, then one
//! local task pumps the engine while it waits for the next command, and keeps the log of the events
//! of the life. The thread opens the life again from its checkpoint at each rebirth, as
//! [`crate::rebirth`] says, for the same handles and the same log. Once the life ends, the thread
//! holds its log for the readers until each handle of the life is gone.

use std::{
  collections::{HashSet, VecDeque},
  future::poll_fn,
  hash::{DefaultHasher, Hash, Hasher},
  path::PathBuf,
  sync::Arc,
  task::Poll,
  thread,
};

use furb::{
  Engine, Fact, Fault, Object, Text, checkpoint,
  life::{Log, Opening, Stream},
  verbs,
  wire::{outward, parsed},
  world::{self, Shape, answered},
};
use futures::stream::{self, Stream as Streams};
use serde_json::{Value, json};
use tokio::{
  runtime,
  sync::{
    broadcast::{self, error::RecvError},
    mpsc::{self, UnboundedReceiver, UnboundedSender},
    oneshot,
  },
  task::LocalSet,
};

use crate::{
  changes::{Journal, journal},
  ears::{ears, json_of},
  error::LifeError,
  event::{Event, Seq},
  queue::{FollowUp, QUEUE, Queue},
  rebirth::{self, Watch},
};

/// How many events a reader may stand behind the life before it reads them again from the log of
/// the life.
const BEHIND: usize = 256;

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
  /// What the gate finds in a word on a chain, which runs nothing.
  Gate { word: String, on: Option<String> },
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
  /// The fact that the operator read a chain or a thread up to now.
  Seen { about: String },
  /// A follow-up that the operator queued on a chain, under an entry that a second queue finds,
  /// which the life sends as a prompt of its shape and its message once the chain rests.
  Queue { entry: String, shape: String, message: String, on: Option<String> },
  /// A follow-up taken out of its queue before the life sent it.
  Unqueue { entry: String },
  /// The follow-ups that wait on a chain, in order.
  Queued { on: Option<String> },
  /// The end of the life, which replies and then ends the life.
  End,
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
  Inspected { kind: String, repr: String, value: Value },
}

/// One event under its place in the log.
type Logged = (Seq, Arc<Event>);

/// Where the thread of a life replies to a command.
type Replier = oneshot::Sender<Result<Reply, LifeError>>;

/// What a handle says to the thread of its life.
enum Message {
  Command(Command, Replier),
  /// The events of the log from a place, for a reader that stands behind the life.
  Since(Seq, oneshot::Sender<Vec<Logged>>),
}

/// The log of a life on its thread: each event that it told, the readers that hear each next one,
/// what the ears and the provider told that is not in the log yet, the digest of each fact that it
/// told, and what the actor watches to know when the life is born again.
pub struct Life {
  log: Vec<Arc<Event>>,
  readers: broadcast::Sender<Logged>,
  heard: UnboundedReceiver<Event>,
  facts: HashSet<u64>,
  watch: Watch,
  queue: Queue,
}

/// The handle of a life, which any thread holds: it says a command to the life and reads its
/// events.
#[derive(Clone)]
pub struct LifeHandle {
  inbox: UnboundedSender<Message>,
  readers: broadcast::Sender<Logged>,
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
    thread::Builder::new()
      .name("furb-life".to_owned())
      .spawn(move || lived(opening, received, told))
      .map_err(LifeError::Thread)?;
    Ok(LifeHandle { inbox, readers })
  }

  /// One event put in the log, and sent to each reader, then the follow-ups of a chain when the
  /// event changed them.
  fn put(&mut self, event: Event) {
    if let Event::Fact(fact) = &event {
      self.facts.insert(digest(fact));
    }
    self.watch.heard(&event);
    let queued = self.queue.heard(&event);
    let (seq, event) = (self.log.len() as Seq, Arc::new(event));
    self.log.push(Arc::clone(&event));
    self.readers.send((seq, event)).ok();
    if let Some(queued) = queued {
      self.put(queued);
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
        event => self.put(event),
      }
    }
    for chain in changed {
      self.put(Event::Changed(chain));
    }
  }

  /// What an open told, less each fact that the log told before it: an open after a rebirth says
  /// again each fact that the checkpoint restates, which each reader already heard.
  fn opened(&mut self) {
    let mut told = Vec::new();
    while let Ok(event) = self.heard.try_recv() {
      told.push(event);
    }
    let heard =
      |event: &Event| matches!(event, Event::Fact(fact) if self.facts.contains(&digest(fact)));
    let told: Vec<Event> = told.into_iter().filter(|event| !heard(event)).collect();
    for event in told {
      self.told(Some(event));
    }
  }

  /// The facts of a record before its last checkpoint, told first: the life boots on that
  /// checkpoint and replays none of them, so each reader hears the life whole, as after a rebirth,
  /// and the open tells no fact again that the checkpoint restates. A record that the store cannot
  /// read tells nothing here, and its open says why.
  fn recalled(&mut self, record: &str) {
    let Some(at) = world::kept(record).ok().and_then(|entries| checkpoint::last(&entries)) else {
      return;
    };
    let Ok(told) = world::retold(record) else { return };
    let mut facts = Vec::new();
    if told.before(at + 1, &mut |one| facts.extend(Fact::of(one.as_ref()))).is_err() {
      return;
    }
    // Each birth that a checkpoint opened said again the facts that its checkpoint restates, which
    // the log of that birth told no reader twice, and the log of this one tells none twice either.
    for fact in facts.iter().filter(|one| one.kind() != checkpoint::CHECKPOINT) {
      let fact = json_of(fact);
      if !self.facts.contains(&digest(&fact)) {
        self.put(Event::Fact(fact));
      }
    }
    self.watch.reborn();
  }

  /// The life opened again, after each event that the engine before it told.
  fn reborn(&mut self, fault: Option<String>) {
    self.told(None);
    self.put(Event::Reborn(fault));
    self.watch.reborn();
  }

  /// The events of the log from a place.
  fn since(&self, from: Seq) -> Vec<Logged> {
    let from = usize::try_from(from).unwrap_or(usize::MAX);
    let since = self.log.iter().enumerate().skip(from);
    since.map(|(at, one)| (at as Seq, Arc::clone(one))).collect()
  }
}

/// The digest of a fact, by which the log knows a fact that it told.
fn digest(fact: &Value) -> u64 {
  let mut hasher = DefaultHasher::new();
  fact.to_string().hash(&mut hasher);
  hasher.finish()
}

/// The thread of a life, until each handle of the life is gone.
#[expect(
  clippy::disallowed_methods,
  reason = "the thread of each life blocks on its own current_thread runtime by design (#983 section 2)"
)]
fn lived(
  opening: Opening,
  received: UnboundedReceiver<Message>,
  readers: broadcast::Sender<Logged>,
) {
  match runtime::Builder::new_current_thread().enable_all().build() {
    Ok(runtime) => LocalSet::new().block_on(&runtime, run(opening, received, readers)),
    Err(no) => {
      let ended = Event::Ended(Some(format!("the life has no runtime: {no}")));
      readers.send((0, Arc::new(ended))).ok();
    }
  }
}

/// The life: opened, then driven until it ends, and its log held for the readers after its end.
async fn run(
  mut opening: Opening,
  mut received: UnboundedReceiver<Message>,
  readers: broadcast::Sender<Logged>,
) {
  let (told, heard) = mpsc::unbounded_channel();
  let mut life = Life {
    log: Vec::new(),
    readers,
    heard,
    facts: HashSet::new(),
    watch: Watch::default(),
    queue: Queue::default(),
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
  let record = opening.record.clone().filter(|_| opening.keeps());
  let folder = PathBuf::from(&opening.directory);
  let journal = record.map(|one| Journal::spawn(folder, journal(&one), told.clone()));
  let end = lives(opening, &mut life, &mut received, &told, journal.as_ref()).await;
  let fault = flushed(end.fault, journal.as_ref()).await;
  if let Some(asked) = end.asked {
    asked.send(fault.clone().map_or(Ok(Reply::Done), |one| Err(LifeError::Refused(one)))).ok();
  }
  life.told(None);
  life.put(Event::Ended(fault));
  while let Some(message) = received.recv().await {
    match message {
      Message::Since(from, reply) => reply.send(life.since(from)).ok(),
      Message::Command(_, reply) => reply.send(Err(LifeError::Ended)).ok(),
    };
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

/// How a life ended: the fault that ended it, when one did, and the client that ended it, which
/// hears the reply once the life is over.
struct End {
  fault: Option<String>,
  asked: Option<Replier>,
}

/// The life opened, driven, and opened again from its checkpoint at each rebirth, until it ends,
/// each birth on the one journal of its changes, and how it ended.
async fn lives(
  opening: Opening,
  life: &mut Life,
  received: &mut UnboundedReceiver<Message>,
  told: &UnboundedSender<Event>,
  journal: Option<&Journal>,
) -> End {
  let record = opening.record.clone().filter(|_| opening.keeps());
  if let Some(one) = record.as_deref() {
    life.recalled(one);
  }
  loop {
    let engine = match opening.clone().open(ears(told.clone(), journal)).await {
      Ok((engine, _)) => engine,
      Err(fault) => return End { fault: Some(fault.to_string()), asked: None },
    };
    life.opened();
    match driven(engine, life, received, record.as_deref()).await {
      Drove::Ended(fault, asked) => return End { fault: fault.map(|one| one.to_string()), asked },
      Drove::Reborn(fault) => life.reborn(fault.map(|one| one.to_string())),
    }
  }
}

/// How the engine of a life went: the life ended, with the client that ended it, or it is born
/// again, each with the fault that stopped it, when one did.
enum Drove {
  Ended(Option<Fault>, Option<Replier>),
  Reborn(Option<Fault>),
}

/// What woke the thread of a life.
enum Woke {
  Told(Event),
  Stopped(Fault),
  Said(Option<Message>),
}

/// The engine driven while it hears each command, until a client ends it, each handle is gone, a
/// fault stops it, or the life is due to be born again. A life that keeps a record ends with its
/// checkpoint.
async fn driven(
  mut engine: Engine,
  life: &mut Life,
  received: &mut UnboundedReceiver<Message>,
  record: Option<&str>,
) -> Drove {
  loop {
    let woke = tokio::select! {
      biased;
      Some(event) = life.heard.recv() => Woke::Told(event),
      fault = pumped(&mut engine) => Woke::Stopped(fault),
      message = received.recv() => Woke::Said(message),
    };
    match woke {
      Woke::Told(event) => life.told(Some(event)),
      Woke::Stopped(fault) => {
        drop(engine);
        return stopped(fault, record).await;
      }
      Woke::Said(None) => return Drove::Ended(rebirth::ended(engine, record.is_some()), None),
      Woke::Said(Some(Message::Since(from, reply))) => {
        life.told(None);
        reply.send(life.since(from)).ok();
      }
      Woke::Said(Some(Message::Command(Command::End, reply))) => {
        return Drove::Ended(rebirth::ended(engine, record.is_some()), Some(reply));
      }
      Woke::Said(Some(Message::Command(command, reply))) => {
        reply.send(commanded(&mut engine, &life.queue, command)).ok();
      }
    }
    drained(&mut engine, life);
    if let Some(fault) = engine.stopped().cloned() {
      drop(engine);
      return stopped(fault, record).await;
    }
    if record.is_some() && life.watch.due() {
      return match engine.checkpoint() {
        Ok(()) => Drove::Reborn(None),
        Err(fault) => stopped(fault, record).await,
      };
    }
  }
}

/// What a fault that stopped the sandbox or its checkpoint comes to, once the engine is gone: a
/// rebirth from the tail carried as data, when the life keeps a record that holds a checkpoint,
/// and else the end of the life.
async fn stopped(fault: Fault, record: Option<&str>) -> Drove {
  let Some(record) = record else { return Drove::Ended(Some(fault), None) };
  match rebirth::carried(record.to_owned()).await {
    Ok(true) => Drove::Reborn(Some(fault)),
    Ok(false) => Drove::Ended(Some(fault), None),
    Err(no) => Drove::Ended(Some(Fault::refused(format!("{fault}, and then {no}"))), None),
  }
}

/// The engine pumped until a fault stops it.
async fn pumped(engine: &mut Engine) -> Fault {
  poll_fn(|cx| match engine.pump(cx.waker()) {
    Ok(()) => Poll::Pending,
    Err(fault) => Poll::Ready(fault),
  })
  .await
}

/// What a command comes to: on the queue of the life for a command of the queue, and else on the
/// engine.
fn commanded(engine: &mut Engine, queue: &Queue, command: Command) -> Result<Reply, LifeError> {
  let root = engine.root().to_owned();
  let chain = |on: Option<String>| on.unwrap_or_else(|| root.clone());
  match command {
    Command::Queue { entry, shape, message, on } => {
      let chain = chain(on);
      if !queue.holds(&entry) {
        let words = ["add", &entry, &shape, &message].map(Object::string);
        engine.say(QUEUE, &chain, words.into())?;
      }
      Ok(Reply::Done)
    }
    Command::Unqueue { entry } => {
      let follow = queue.takes(&entry);
      let follow =
        follow.ok_or_else(|| LifeError::Refused(format!("No follow-up {entry} waits.")))?;
      engine.say(QUEUE, &follow.chain, ["drop", &entry].map(Object::string).into())?;
      Ok(Reply::Done)
    }
    Command::Queued { on } => {
      let waiting = queue.waiting(&chain(on));
      let waiting = waiting.iter().map(serde_json::to_value).collect::<Result<_, _>>();
      Ok(Reply::Values(waiting.map_err(|no| LifeError::Refused(no.to_string()))?))
    }
    other => answered_by(engine, other),
  }
}

/// Each follow-up that is due sent as a prompt on its chain, or why its send failed, which holds
/// the queue of its chain and stands in the record.
fn drained(engine: &mut Engine, life: &mut Life) {
  for follow in life.queue.due() {
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
    life.queue.sent(&follow.entry, made);
  }
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
    Command::Gate { word, on: chain } => {
      let found = engine.verb("gate", vec![Object::string(word)], named(chain))?;
      Reply::Value(outward(found.as_ref()))
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
  /// The reply of the life to a command, once the life ran it, in the order that the life heard
  /// each command.
  pub async fn send(&self, command: Command) -> Result<Reply, LifeError> {
    let (reply, replied) = oneshot::channel();
    if self.inbox.send(Message::Command(command, reply)).is_err() {
      return Err(LifeError::Ended);
    }
    replied.await.unwrap_or(Err(LifeError::Ended))
  }

  /// The events of the life from a place in its log, each once, in order and with no gap, until the
  /// event that ends the life. A reader that stands too far behind the life reads again from the
  /// log of the life.
  pub fn events(&self, from: Seq) -> impl Streams<Item = Logged> + Send + 'static {
    let reader = Reader {
      life: self.clone(),
      receiver: self.readers.subscribe(),
      next: from,
      read: VecDeque::new(),
      behind: true,
      over: false,
    };
    stream::unfold(reader, Reader::next)
  }

  /// The events of the log of the life from a place, or none once the thread of the life is gone.
  async fn since(&self, from: Seq) -> Option<Vec<Logged>> {
    let (reply, replied) = oneshot::channel();
    self.inbox.send(Message::Since(from, reply)).ok()?;
    replied.await.ok()
  }
}

/// A reader of the events of a life: the place of the next event that it gives, the events that it
/// read from the log and did not give yet, whether it stands behind the life, and whether it gave
/// the end.
struct Reader {
  life: LifeHandle,
  receiver: broadcast::Receiver<Logged>,
  next: Seq,
  read: VecDeque<Logged>,
  behind: bool,
  over: bool,
}

impl Reader {
  /// The next event, and the reader after it.
  async fn next(mut self) -> Option<(Logged, Reader)> {
    while !self.over {
      if let Some(logged) = self.read.pop_front() {
        if logged.0 == self.next {
          return Some(self.given(logged));
        }
      } else if self.behind {
        self.behind = false;
        self.read = self.life.since(self.next).await?.into();
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
  fn given(mut self, logged: Logged) -> (Logged, Reader) {
    self.next = logged.0 + 1;
    self.over = matches!(*logged.1, Event::Ended(_));
    (logged, self)
  }
}
