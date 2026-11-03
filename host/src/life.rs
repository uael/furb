//! A life as the host holds it: a thread of its own, with a runtime of tokio and a set of local
//! tasks, since the engine is not `Send`. The thread owns the engine. It awaits the open, then one
//! local task pumps the engine while it waits for the next command, and keeps the log of the events
//! of the life. Once the life ends, the thread holds its log for the readers until each handle of
//! the life is gone.

use std::{collections::VecDeque, future::poll_fn, sync::Arc, task::Poll, thread};

use furb::{
  Engine, Fault, Object,
  life::{Log, Opening, Stream},
  verbs,
  wire::outward,
  world::answered,
};
use futures::stream::{self, Stream as Streams};
use serde_json::Value;
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
  ears::ears,
  error::LifeError,
  event::{Event, Seq},
};

/// How many events a reader may stand behind the life before it reads them again from the log of
/// the life.
const BEHIND: usize = 256;

/// What a client says to a life. A command that names no chain says it on the root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
  /// A prompt on a chain, of a shape, with a message, to an actor or to the actor of the chain: the
  /// act.
  Prompt { shape: String, message: String, to: Option<String>, on: Option<String> },
  /// A word that runs on a chain as a turn of the operator: the act of its rung.
  Rung { word: String, on: Option<String> },
  /// A prompt to the operator, closed with a line that the operator wrote, read as the shape that
  /// the prompt wants.
  Answer { id: String, line: String },
  /// A pause of an act or of a chain.
  Pause { id: String },
  /// A wake of an act or of a chain.
  Wake { id: String },
  /// A cancel of an act or of a chain.
  Cancel { id: String },
  /// The turns of a chain, each as JSON.
  Turns { on: Option<String> },
  /// The words of a chain that run, by the rung that gave each one.
  Program { on: Option<String> },
  /// The standing of the life.
  Standing,
  /// The facts of a chain, of one kind or of each kind.
  Transcript { on: Option<String>, of: Option<String> },
  /// One name that the module of a chain binds.
  Inspect { name: String, on: Option<String> },
  /// What the gate finds in a word on a chain, which runs nothing.
  Gate { word: String, on: Option<String> },
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

/// What a handle says to the thread of its life.
enum Message {
  Command(Command, oneshot::Sender<Result<Reply, LifeError>>),
  /// The events of the log from a place, for a reader that stands behind the life.
  Since(Seq, oneshot::Sender<Vec<Logged>>),
}

/// The log of a life on its thread: each event that it told, the readers that hear each next one,
/// and what the ears and the provider told that is not in the log yet.
pub struct Life {
  log: Vec<Arc<Event>>,
  readers: broadcast::Sender<Logged>,
  heard: UnboundedReceiver<Event>,
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

  /// One event put in the log, and sent to each reader.
  fn put(&mut self, event: Event) {
    let (seq, event) = (self.log.len() as Seq, Arc::new(event));
    self.log.push(Arc::clone(&event));
    self.readers.send((seq, event)).ok();
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

  /// The events of the log from a place.
  fn since(&self, from: Seq) -> Vec<Logged> {
    let from = usize::try_from(from).unwrap_or(usize::MAX);
    let since = self.log.iter().enumerate().skip(from);
    since.map(|(at, one)| (at as Seq, Arc::clone(one))).collect()
  }
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
  let mut life = Life { log: Vec::new(), readers, heard };
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
  let ended = match opening.open(ears(told)).await {
    Ok((engine, _)) => driven(engine, &mut life, &mut received).await,
    Err(fault) => Some(fault.to_string()),
  };
  life.told(None);
  life.put(Event::Ended(ended));
  while let Some(message) = received.recv().await {
    match message {
      Message::Since(from, reply) => reply.send(life.since(from)).ok(),
      Message::Command(_, reply) => reply.send(Err(LifeError::Ended)).ok(),
    };
  }
}

/// What woke the thread of a life.
enum Woke {
  Told(Event),
  Stopped(Fault),
  Said(Option<Message>),
}

/// The engine driven while it hears each command, until a client ends it, each handle is gone, or a
/// fault stops it, and the fault, when one did.
async fn driven(
  mut engine: Engine,
  life: &mut Life,
  received: &mut UnboundedReceiver<Message>,
) -> Option<String> {
  loop {
    let woke = tokio::select! {
      biased;
      Some(event) = life.heard.recv() => Woke::Told(event),
      fault = pumped(&mut engine) => Woke::Stopped(fault),
      message = received.recv() => Woke::Said(message),
    };
    match woke {
      Woke::Told(event) => life.told(Some(event)),
      Woke::Stopped(fault) => return Some(fault.to_string()),
      Woke::Said(None) => return None,
      Woke::Said(Some(Message::Since(from, reply))) => {
        life.told(None);
        reply.send(life.since(from)).ok();
      }
      Woke::Said(Some(Message::Command(Command::End, reply))) => {
        reply.send(Ok(Reply::Done)).ok();
        return None;
      }
      Woke::Said(Some(Message::Command(command, reply))) => {
        reply.send(answered_by(&mut engine, command)).ok();
        if let Some(fault) = engine.stopped() {
          return Some(fault.to_string());
        }
      }
    }
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

/// What a command comes to on the engine.
fn answered_by(engine: &mut Engine, command: Command) -> Result<Reply, LifeError> {
  let root = engine.root().to_owned();
  let on = |chain: Option<String>| Some(chain.unwrap_or_else(|| root.clone()));
  let id = |made: &str| Reply::Act(made.to_owned());
  Ok(match command {
    Command::Prompt { shape, message, to, on: chain } => {
      let shape = if shape == "None" { Object::none() } else { Object::string(shape) };
      let with = verbs::Prompt { message: Some(message), to, on: on(chain) };
      id(engine.prompt(shape, with)?.id())
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
    Command::Pause { id } => engine.pause(&id, verbs::Pause::default()).map(|()| Reply::Done)?,
    Command::Wake { id } => engine.wake(&id).map(|()| Reply::Done)?,
    Command::Cancel { id } => engine.cancel(&id, verbs::Cancel::default()).map(|()| Reply::Done)?,
    Command::End => Reply::Done,
    query => queried(engine, query, &root)?,
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
    Command::Inspect { name, on: chain } => {
      let (kind, repr, value) = engine.inspect(&name, Some(&on(chain)))?;
      Reply::Inspected { kind, repr, value: outward(value.as_ref()) }
    }
    Command::Gate { word, on: chain } => {
      let found = engine.verb("gate", vec![Object::string(word)], named(chain))?;
      Reply::Value(outward(found.as_ref()))
    }
    acted => return Err(LifeError::Refused(format!("{acted:?} is no query"))),
  })
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
