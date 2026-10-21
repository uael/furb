//! The life that the app shows, on a thread of its own: the engine is of the thread that opens it, so the life lives
//! there, and the app speaks to it with commands and hears its facts as events.
//!
//! The app hears every fact that the life says, in the order of the log, as JSON on the wire, from the first fact of
//! the record on. Its views are a function of those facts alone, so a fact that the app heard once shows the life as
//! it was then, which is how the app travels in time.

use std::{
  cell::RefCell,
  collections::{HashMap, HashSet},
  future::{Future, poll_fn},
  path::PathBuf,
  pin::Pin,
  rc::Rc,
  sync::{Arc, mpsc},
  task::{Context, Poll},
  thread,
  time::Duration,
};

use furb::{
  Act, Engine, Fact, Fault, Object, Text,
  ear::{Co, Next, ear, reactor, say},
  engine::verbs,
  life::{Opening, Stream},
  wire,
  world::{SHAPES, answered},
};
use futures::{StreamExt, channel::mpsc as channel};
use serde_json::Value;

use crate::{demo, places};

/// What a life opens on: the demo, a record to inspect, or a live life on this machine.
#[derive(Debug, Clone)]
pub enum Source {
  /// The demo project, whose models answer from a script; `pace` is how long a model of it thinks.
  Demo { pace: Duration },
  /// A record, made again to be read: the life keeps nothing, asks no model and runs no command.
  Record(PathBuf),
  /// A life in a project directory, on the record it keeps there, on the models of the catalog.
  Live { directory: PathBuf, record: PathBuf },
}

/// A breakpoint: the app pauses a chain when the life reaches it.
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "lowercase")]
pub enum Break {
  /// Each new rung of the chain, which is each step of it.
  Chain(String),
  /// Each act of this kind, on any chain: the chain of the act pauses.
  Kind(String),
  /// The dollars that the replies of a chain cost, once they reach this sum.
  Cost(f64),
}

/// What the app tells the life to do, as the UI sends it: its kind, and what it carries under value.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "lowercase")]
pub enum Command {
  Pause(String),
  Wake(String),
  /// Wake a chain, and pause it again at its next rung.
  Step(String),
  Cancel(String),
  /// Close a prompt of the operator with a line, read as its shape.
  Answer {
    id: String,
    shape: String,
    line: String,
  },
  /// Write the door of a prompt with the program of its ladder.
  Door {
    prompt: String,
    on: String,
    program: String,
  },
  /// A prompt of the operator to the default actor of a chain.
  Prompt {
    on: String,
    shape: String,
    message: String,
  },
  /// A word of python that runs as a rung on a chain.
  Word {
    on: String,
    word: String,
  },
  Breaks(Vec<Break>),
}

/// What the life tells the app.
#[derive(Debug, Clone)]
pub enum Event {
  /// The life is open: its root, its directory, and whether it does work.
  Opened { root: String, directory: String, works: bool },
  /// The facts that the life said since the last event, in order.
  Facts(Vec<Value>),
  /// What a model wrote for a rung, as it writes it: its text and its thought.
  Writing { rung: String, text: String, thinking: String },
  /// A report for the operator: a refusal, or why the life did not open.
  Said(String),
}

/// A life as the app holds it: the way to tell it commands. What it tells comes on the receiver that opened it.
pub struct Life {
  commands: channel::UnboundedSender<Command>,
}

impl Life {
  /// A life on its own thread, and what it tells. `woke` is called after each event, so the app draws again.
  pub fn open(
    source: Source,
    woke: impl Fn() + Send + Sync + 'static,
  ) -> (Life, mpsc::Receiver<Event>) {
    let (commands, heard) = channel::unbounded();
    let (events, told) = mpsc::channel();
    let woke: Arc<dyn Fn() + Send + Sync> = Arc::new(woke);
    let spawned = thread::Builder::new().name("furb-life".to_owned()).spawn(move || {
      let tell = Teller { events, woke };
      if let Err(no) = lived(source, heard, tell.clone()) {
        tell.tell(Event::Said(no));
      }
    });
    if let Err(no) = spawned {
      eprintln!("furb-gui: the life has no thread: {no}");
    }
    (Life { commands }, told)
  }

  /// The way to send the life commands, from any thread.
  pub fn commands(&self) -> channel::UnboundedSender<Command> {
    self.commands.clone()
  }
}

/// The way to tell the app an event, and to wake it.
#[derive(Clone)]
struct Teller {
  events: mpsc::Sender<Event>,
  woke: Arc<dyn Fn() + Send + Sync>,
}

impl Teller {
  fn tell(&self, event: Event) {
    let _ = self.events.send(event);
    (self.woke)();
  }
}

/// The life, from its opening to the end of the commands of the app.
fn lived(
  source: Source,
  mut commands: channel::UnboundedReceiver<Command>,
  tell: Teller,
) -> Result<(), String> {
  let heard: Rc<RefCell<Vec<Value>>> = Rc::default();
  let (mut opening, demo) = match &source {
    Source::Demo { pace } => {
      let directory = demo::directory()?;
      (demo::opening(&directory, *pace), true)
    }
    Source::Record(path) => (
      Opening {
        directory: places::project_of(path).display().to_string(),
        record: Some(path.display().to_string()),
        inspecting: Some(true),
        ..Opening::default()
      },
      false,
    ),
    Source::Live { directory, record } => (
      Opening {
        directory: directory.display().to_string(),
        record: Some(record.display().to_string()),
        ..Opening::default()
      },
      false,
    ),
  };
  let works = !matches!(source, Source::Record(_));
  let writer = tell.clone();
  opening.stream =
    Some(Stream(Arc::new(move |rung: &str, _: &str, text: &str, thinking: &str| {
      if !text.is_empty() || !thinking.is_empty() {
        let (rung, text, thinking) = (rung.to_owned(), text.to_owned(), thinking.to_owned());
        writer.tell(Event::Writing { rung, text, thinking });
      }
    })));
  let directory = opening.directory.clone();
  let ears =
    [("observer", observer(heard.clone())), ("console", if works { console() } else { silent() })];
  let (engine, _) = opening.boot(ears).map_err(|no| no.to_string())?;
  let root = engine.root().to_owned();
  tell.tell(Event::Opened { root: root.clone(), directory, works });
  let mut held = Held {
    engine,
    root,
    heard,
    tell,
    breaks: Vec::new(),
    stepping: HashSet::new(),
    costs: Watch::default(),
  };
  held.flush();
  if demo {
    let root = held.root.clone();
    let seeded = demo::seed(&mut held.engine, &root);
    held.failed(seeded);
    held.flush();
  }
  reactor().block_on(async move {
    loop {
      tokio::select! {
        biased;
        () = poll_fn(|cx| held.driven(cx)) => {}
        command = commands.next() => match command {
          Some(command) => held.obey(command),
          None => return,
        },
      }
    }
  });
  Ok(())
}

/// The ear that hears every fact of the life, from the first fact of its record, and keeps each as JSON.
///
/// It is an ear of the outside, so it hears no act that an ear before it took, as a rung, a prompt or a chain, which
/// take their own acts. Each such act it reads by its name at the first fact about it, and keeps it before that fact,
/// so the app holds every act at its place in the log, or just after it.
fn observer(heard: Rc<RefCell<Vec<Value>>>) -> Box<dyn furb::Ear> {
  ear(move |mut co: Co<()>| async move {
    let mut known: HashSet<String> = HashSet::new();
    loop {
      let Next::Heard(a) = co.next().await else { continue };
      let about = a.about().to_owned();
      if a.question() {
        known.insert(about.clone());
      } else if named(&about) && !known.contains(&about) {
        known.insert(about.clone());
        let got = co.call("get", vec![Object::string(&about)], vec![]).await;
        if let Ok(act) = got.as_ref().map(Object::as_ref)
          && act.items().is_some()
        {
          heard.borrow_mut().push(wire::outward(act));
        }
      }
      heard.borrow_mut().push(wire::outward(a.0.as_ref()));
    }
  })
}

/// Whether a name is the name of an act: a kind, then its number.
fn named(name: &str) -> bool {
  let kind = name.trim_end_matches(|one: char| one.is_ascii_digit());
  kind.len() < name.len() && !kind.is_empty() && kind.bytes().all(|one| one.is_ascii_lowercase())
}

/// The console of the operator in the app: it takes each prompt put to the operator, and the app closes it with a
/// command. A prompt of a shape that the operator answers not is closed at once with the refusal of that shape, as
/// every console of furb does.
fn console() -> Box<dyn furb::Ear> {
  ear(|mut co: Co<()>| async move {
    loop {
      let Next::Heard(a) = co.next().await else { continue };
      if a.kind() != "prompt" || !a.question() {
        continue;
      }
      say(&mut co, Fact::says("started", a.about(), []));
      let shape = a.word(1).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default();
      if !SHAPES.contains(&shape.as_str()) {
        let no = answered(&shape, "").unwrap_or_else(|no| no.object());
        co.call("close", vec![no], vec![("id", Object::string(a.about()))]).await?;
      }
    }
  })
}

/// The console of a life that only reads its record: it hears each fact under the name of the console, and takes
/// nothing, so no question waits for the operator.
fn silent() -> Box<dyn furb::Ear> {
  ear(|mut co: Co<()>| async move {
    loop {
      let _ = co.next().await;
    }
  })
}

/// What the cost breakpoints watch: the chain of each reply, the dollars of each chain, and the chains that reached
/// their sum already.
#[derive(Default)]
struct Watch {
  replies: HashMap<String, String>,
  spent: HashMap<String, f64>,
  reached: HashSet<String>,
}

/// The life as its thread holds it.
struct Held {
  engine: Engine,
  root: String,
  heard: Rc<RefCell<Vec<Value>>>,
  tell: Teller,
  breaks: Vec<Break>,
  stepping: HashSet<String>,
  costs: Watch,
}

impl Held {
  /// The life driven: what its ears say of their own accord is said into it, and the app hears each new fact. It is
  /// never over, since the root never completes.
  fn driven(&mut self, cx: &mut Context<'_>) -> Poll<()> {
    let mut root = Act::<Object>::of(&mut self.engine, self.root.clone());
    if let Poll::Ready(Err(no)) = Pin::new(&mut root).poll(cx) {
      self.tell.tell(Event::Said(no.to_string()));
    }
    self.flush();
    Poll::Pending
  }

  /// Each new fact, told to the app, and each chain that a breakpoint reached, paused.
  fn flush(&mut self) {
    loop {
      let facts = std::mem::take(&mut *self.heard.borrow_mut());
      if facts.is_empty() {
        return;
      }
      let halts: Vec<String> = facts.iter().filter_map(|one| self.halts(one)).collect();
      self.tell.tell(Event::Facts(facts));
      for chain in halts {
        let paused = self.engine.pause(&chain);
        self.failed(paused);
      }
    }
  }

  /// The chain that a fact makes a breakpoint pause, if any.
  fn halts(&mut self, fact: &Value) -> Option<String> {
    let word = |at: usize| fact.get(at).and_then(Value::as_str).unwrap_or_default();
    let (kind, about) = (word(0), word(1));
    let question = about
      .strip_prefix(kind)
      .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
    if question && kind != "chain" {
      let on = word(3).to_owned();
      if kind == "reply" {
        self.costs.replies.insert(about.to_owned(), on.clone());
      }
      if kind == "rung" && self.stepping.remove(&on) {
        return Some(on);
      }
      let hit = self.breaks.iter().any(|one| match one {
        Break::Kind(wanted) => wanted == kind,
        Break::Chain(chain) => kind == "rung" && *chain == on,
        Break::Cost(_) => false,
      });
      return hit.then_some(on);
    }
    let on = self.costs.replies.get(about).filter(|_| kind == "done")?.clone();
    let cost = fact.get(3)?.get(2)?.get(4)?.as_f64()?;
    let spent = self.costs.spent.entry(on.clone()).or_default();
    *spent += cost;
    let sum = *spent;
    let reached = self.breaks.iter().any(|one| matches!(one, Break::Cost(most) if sum >= *most));
    (reached && self.costs.reached.insert(on.clone())).then_some(on)
  }

  /// A refusal of the life, told to the app.
  fn failed<T>(&self, got: Result<T, Fault>) {
    if let Err(no) = got {
      self.tell.tell(Event::Said(no.to_string()));
    }
  }

  /// One command of the app, done.
  fn obey(&mut self, command: Command) {
    let engine = &mut self.engine;
    let done: Result<(), Fault> = match command {
      Command::Pause(id) => engine.pause(&id),
      Command::Wake(id) => engine.wake(&id),
      Command::Step(chain) => {
        self.stepping.insert(chain.clone());
        engine.wake(&chain)
      }
      Command::Cancel(id) => engine.cancel(&id),
      Command::Answer { id, shape, line } => {
        let value = answered(&shape, &line).unwrap_or_else(|no| no.object());
        engine.close(value, verbs::Close { id: Some(id) })
      }
      Command::Door { prompt, on, program } => {
        engine.write(&Text::new(prompt, program), verbs::Write { on: Some(on) }).map(|_| ())
      }
      Command::Prompt { on, shape, message } => {
        let shape = if shape == "None" { Object::none() } else { Object::string(shape) };
        let with = verbs::Prompt { message: Some(message), on: Some(on), ..Default::default() };
        engine.prompt(shape, with).map(|_| ())
      }
      Command::Word { on, word } => engine
        .rung(verbs::Rung { word: Some(word), on: Some(on), ..Default::default() })
        .map(|_| ()),
      Command::Breaks(breaks) => {
        self.breaks = breaks;
        Ok(())
      }
    };
    self.failed(done);
    self.flush();
  }
}
