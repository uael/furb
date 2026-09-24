//! One life of the engine: the sandbox it runs in, the World and the ears it reaches, and the verbs a host says.
//!
//! A life is driven from one thread. A verb is one call: it runs in the sandbox, the engine does what it does,
//! every fact it says reaches the World and the ears, and what the verb gave comes back. An act is awaited: the
//! future drives the life until the act is done, which means it drains what the World said unasked, drives the
//! work the World gave back as futures, and says each result into the engine as the fact the engine waits for.
//! Nothing polls: what the World says wakes whoever awaits, and so does the work it finishes.
//!
//! The host reaches the sandbox as three objects the stand-in holds: the World, whose methods it calls as the
//! engine asks of a World; the gate, which reads a sheet; and the ears, which hear every other ear by name. A
//! call of a method of one of them comes here with the id of the object, and is answered on the thread of the
//! life, inside the call.

use std::{
  collections::HashMap,
  future::Future,
  marker::PhantomData,
  pin::Pin,
  task::{Context, Poll},
};

use monty_types::{MontyUuid, NamedValues, ResourceLimits};

use crate::{
  PREAMBLE, SHEET,
  ear::{Ears, Reply},
  fact::Fact,
  gate::checked,
  sand::{Sand, id, object},
  value::{Fault, Object, ObjectRef, entry, inward},
  world::{Later, Said, Voice, World},
};

/// The name the engine hears the World by, which is the name whose facts the journal keeps.
pub const WORLD: &str = "world";

/// The prefix of the name a callable of the host crosses in under: `call:` and its number.
const CALL: &str = "call:";

/// A callable of the host, as the sandbox may call it back.
type Callable = Box<dyn FnMut(Vec<Object>) -> Result<Object, Fault>>;

/// What a host does when an act is done, given what it came to.
type Watcher = Box<dyn FnMut(&Object)>;

/// One piece of work the World gave back as a future, and the fact its result becomes.
enum Work {
  /// The turn of a model, which answers an ask.
  Answer { rung: String, later: Later<Object> },
  /// A wait, done when its time is up.
  Wait { about: String, later: Later<()> },
  /// A prompt of the operator, closed with what the operator answered.
  Prompt { about: String, later: Later<Result<Object, Fault>> },
}

/// The three objects of the host the stand-in holds, by their ids.
mod objects {
  /// The World, whose methods the stand-in calls as the engine asks of a World.
  pub const WORLD: u8 = 1;
  /// The gate, which reads a sheet.
  pub const GATE: u8 = 2;
  /// The ears, which hear every other ear by name.
  pub const EARS: u8 = 3;
}

/// The World of a life: the typed one a Rust host writes, or none, when the host hands its World over as an ear
/// among its ears, which a host that is python does.
enum Worldly {
  Typed(Box<dyn World>),
  Heard,
}

/// The host, as the sandbox reaches it: the World, the ears, and everything in flight. The gate is the thread's.
struct Hosting {
  /// The source of the engine the life runs, which the gate reads a word on.
  source: String,
  world: Worldly,
  ears: Option<Box<dyn Ears>>,
  voice: Voice,
  later: Vec<Work>,
  calls: Vec<Callable>,
}

impl Hosting {
  /// One call of the sandbox, answered: a method of one of the objects of the host, or a callable of the host.
  fn called(
    &mut self,
    on: Option<MontyUuid>,
    name: &str,
    args: Vec<Object>,
  ) -> Result<Object, Fault> {
    let Some(on) = on else {
      let Some(n) = name.strip_prefix(CALL).and_then(|n| n.parse::<usize>().ok()) else {
        return Err(Fault::refused(format!("{name} is no function of the host")));
      };
      let Some(call) = self.calls.get_mut(n) else {
        return Err(Fault::refused(format!("{name} is no callable of the host")));
      };
      return call(args);
    };
    if on == id(objects::GATE) {
      let sheet = text(args.first().map(Object::as_ref));
      let found = checked(&sheet, &self.source)?;
      return Ok(Object::list(found.into_iter().map(|(line, why)| {
        Object::tuple([Object::int(i64::try_from(line).unwrap_or_default()), Object::string(why)])
      })));
    }
    if on == id(objects::EARS) {
      let Some(ears) = self.ears.as_mut() else {
        return Err(Fault::refused("no ears of the host hear"));
      };
      let ear = text(args.first().map(Object::as_ref));
      let reply = match name {
        "hears" => ears.hears(&ear, args.get(1).and_then(|one| Fact::of(one.as_ref())).as_ref()),
        "answered" => ears.answered(
          &ear,
          args.get(1).map_or_else(|| Object::none().as_ref().to_owned(), Clone::clone).as_ref(),
        ),
        "called" => {
          let held = args.get(1).and_then(|one| one.as_ref().items()).unwrap_or_default();
          let held = held.into_iter().map(|one| one.to_owned()).collect();
          let kwargs = args.get(2).and_then(|one| one.as_ref().pairs()).unwrap_or_default();
          let kwargs = kwargs
            .into_iter()
            .map(|(key, one)| (key.as_str().unwrap_or_default().to_owned(), one.to_owned()))
            .collect();
          return ears.called(&ear, held, kwargs);
        }
        _ => return Err(Fault::refused(format!("the ears of the host have no {name}"))),
      };
      return Ok(replied(reply));
    }
    if on == id(objects::WORLD) {
      let Hosting { world, later, .. } = self;
      let Worldly::Typed(world) = world else {
        return Err(Fault::refused("the World of the host is heard among its ears"));
      };
      return worldly(world.as_mut(), later, name, args);
    }
    Err(Fault::refused(format!("{on} is no object of the host")))
  }
}

/// One method of the World, as the stand-in calls it: what takes time joins the work in flight, and every other
/// fact goes to the World as an ear hears it.
fn worldly(
  world: &mut dyn World,
  later: &mut Vec<Work>,
  name: &str,
  args: Vec<Object>,
) -> Result<Object, Fault> {
  let at = |i: usize| args.get(i).map(Object::as_ref);
  let word = |i: usize| text(at(i));
  Ok(match name {
    "stand" => world.stand().object(),
    "clock" => Object::float(world.clock()),
    "chance" => Object::float(world.chance()),
    "kinds" => Object::list(world.kinds().into_iter().map(Object::string)),
    "hears" => match at(0).and_then(Fact::of) {
      Some(fact) => replied(world.hears(&fact)),
      None => Object::none(),
    },
    "answered" => {
      let none = Object::none();
      replied(world.answered(at(0).unwrap_or(none.as_ref())))
    }
    "keep" => {
      if let Some(entry) = at(0) {
        world.keep(entry);
      }
      Object::none()
    }
    "ask" => {
      let turns = at(3).map_or_else(|| Object::list([]), |one| one.to_owned());
      let answer = world.ask(&word(0), &word(1), &word(2), turns.as_ref());
      later.push(Work::Answer { rung: word(0), later: answer });
      Object::none()
    }
    "wait" => {
      let done = world.wait(at(1).and_then(number).unwrap_or_default());
      later.push(Work::Wait { about: word(0), later: done });
      Object::none()
    }
    "prompt" => {
      let closed = world.prompt(&word(0), &word(1), &word(2));
      later.push(Work::Prompt { about: word(0), later: closed });
      Object::none()
    }
    _ => return Err(Fault::refused(format!("the World of the host has no {name}"))),
  })
}

/// A reply, as the stand-in reads it.
fn replied(reply: Reply) -> Object {
  match reply {
    Reply::Nothing => Object::none(),
    Reply::Say(fact) => Object::tuple([Object::string("say"), fact.0]),
    Reply::Raised(fault) => Object::tuple([Object::string("raised"), fault.object()]),
    Reply::Over => Object::tuple([Object::string("over")]),
    Reply::Calls { name, args, kwargs } => Object::tuple([
      Object::string("calls"),
      Object::string(name),
      Object::list(args),
      Object::dict(kwargs.into_iter().map(|(key, one)| (Object::string(key), one))),
    ]),
  }
}

/// The text a word of a call is, or nothing.
fn text(got: Option<ObjectRef<'_>>) -> String {
  got.and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default()
}

/// The number a word of a call is, whether the engine said it whole or not.
fn number(got: ObjectRef<'_>) -> Option<f64> {
  got.as_float().or_else(|| got.as_int().map(|n| n as f64))
}

/// The sandbox and the host it reaches, which is what a life holds.
struct Inner {
  sand: Sand,
  host: Hosting,
  /// Who waits on an act, by its name, told once when it is done.
  watchers: HashMap<String, Vec<Watcher>>,
}

impl Inner {
  /// One piece of code of the stand-in, run in the sandbox with these names bound, each as it goes in; and then
  /// whoever watches an act that is done now is told.
  fn run(&mut self, code: &str, inputs: Vec<(&str, Object)>) -> Result<Object, Fault> {
    let got = self.ran(code, inputs)?;
    self.told()?;
    Ok(got)
  }

  fn ran(&mut self, code: &str, inputs: Vec<(&str, Object)>) -> Result<Object, Fault> {
    let Inner { sand, host, .. } = self;
    let mut named = NamedValues::new();
    for (name, value) in inputs {
      named.push(name, inward(&value));
    }
    sand.run(code, named, &mut |on, name, args| host.called(on, name, args))
  }

  /// What every watched act came to, in one reading, and each watcher told once.
  fn told(&mut self) -> Result<(), Fault> {
    if self.watchers.is_empty() {
      return Ok(());
    }
    let ids: Vec<String> = self.watchers.keys().cloned().collect();
    let got = self.ran(
      "outcomes_of(__engine, __ids)",
      vec![("__ids", Object::list(ids.iter().map(Object::string)))],
    )?;
    let got = got.as_ref();
    for (i, id) in ids.iter().enumerate() {
      if let Some(done) = entry(&got, i)
        && done.type_name() != "NoneType"
        && let Some(value) = entry(&done, 0)
      {
        let value = value.to_owned();
        for mut watcher in self.watchers.remove(id).unwrap_or_default() {
          watcher(&value);
        }
      }
    }
    Ok(())
  }

  /// One thing the World said, said into the engine.
  fn deliver(&mut self, said: &Said) -> Result<(), Fault> {
    self.run("said(__engine, __ears, __one)", vec![("__one", said.object())]).map(|_| ())
  }

  /// The life driven as far as it goes without the host: everything the World said unasked is said into the
  /// engine, and every piece of work the World finished is said as the fact the engine waits for.
  fn pump(&mut self, cx: &mut Context<'_>) -> Result<(), Fault> {
    loop {
      let mut moved = false;
      for said in self.host.voice.drained(cx.waker()) {
        self.deliver(&said)?;
        moved = true;
      }
      let mut i = 0;
      while i < self.host.later.len() {
        let done = match &mut self.host.later[i] {
          Work::Answer { rung, later } => match later.as_mut().poll(cx) {
            Poll::Ready(turn) => Some(Said::Fact(Fact::says("answer", rung, [turn]))),
            Poll::Pending => None,
          },
          Work::Wait { about, later } => match later.as_mut().poll(cx) {
            Poll::Ready(()) => Some(Said::Fact(Fact::says("done", about, [Object::none()]))),
            Poll::Pending => None,
          },
          Work::Prompt { about, later } => match later.as_mut().poll(cx) {
            Poll::Ready(Ok(value)) => Some(Said::Closed { id: about.clone(), value }),
            Poll::Ready(Err(fault)) => {
              Some(Said::Closed { id: about.clone(), value: fault.object() })
            }
            Poll::Pending => None,
          },
        };
        match done {
          Some(said) => {
            self.host.later.remove(i);
            self.deliver(&said)?;
            moved = true;
          }
          None => i += 1,
        }
      }
      if !moved {
        return Ok(());
      }
    }
  }
}

/// A life being opened: the World it runs on, and the ears the host gives it.
pub struct Opening {
  world: Worldly,
  ears: Option<Box<dyn Ears>>,
  names: Vec<String>,
  limits: ResourceLimits,
  words: Vec<String>,
  lives: Vec<String>,
}

impl Opening {
  /// The ears of the host, and the names they hear by, in the order the engine hears them. A host that has no
  /// typed World names `world` among them.
  #[must_use]
  pub fn ears(
    mut self,
    ears: impl Ears + 'static,
    names: impl IntoIterator<Item = impl Into<String>>,
  ) -> Self {
    self.ears = Some(Box::new(ears));
    self.names.extend(names.into_iter().map(Into::into));
    self
  }

  /// The limits of the sandbox: how much memory a life may hold, how long it may run.
  #[must_use]
  pub fn limits(mut self, limits: ResourceLimits) -> Self {
    self.limits = limits;
    self
  }

  /// The words of the extensions, which the module of the engine runs after the engine, so every chain binds their
  /// names from its birth, and the gate reads a word after them. A record that pins words gives its own instead, and
  /// a life whose record pins none pins these, when there are some, as the World, about the root.
  #[must_use]
  pub fn words(mut self, words: impl IntoIterator<Item = impl Into<String>>) -> Self {
    self.words.extend(words.into_iter().map(Into::into));
    self
  }

  /// The life words of the extensions, which the life plays as rungs, as the World, in every life on every chain
  /// without a source: once boot stands on its record, and at the birth of each such chain after.
  #[must_use]
  pub fn lives(mut self, lives: impl IntoIterator<Item = impl Into<String>>) -> Self {
    self.lives.extend(lives.into_iter().map(Into::into));
    self
  }

  /// The life, opened from what a World kept of the life before it.
  ///
  /// The record is the entries the World kept, each the fact, and for a query of a run what it was
  /// answered. The stand-in runs first in a module of its own, then the engine, and
  /// `boot` is given the Kernel of the crate and one generator for the World and for each ear.
  pub fn boot(self, record: impl IntoIterator<Item = Object>) -> Result<Life, Fault> {
    let Opening { mut world, ears, mut names, limits, words, lives } = self;
    let record: Vec<Object> = record.into_iter().collect();
    let pin = crate::extension::pinned(&record);
    let words = pin.clone().unwrap_or(words);
    let voice = Voice::default();
    if let Worldly::Typed(world) = &mut world {
      world.opened(voice.clone());
      names.insert(0, WORLD.to_owned());
    }
    let typed = matches!(world, Worldly::Typed(_));
    let source = crate::extension::source(&words);
    let host =
      Hosting { source: source.clone(), world, ears, voice, later: Vec::new(), calls: Vec::new() };
    let mut inner = Inner { sand: Sand::new(limits), host, watchers: HashMap::new() };
    inner.ran(PREAMBLE, vec![])?;
    // The three objects of the host and the two modules are bound as names of the session, which every later
    // piece of code of the stand-in reads.
    let opening = "__engine = module(__source, {**MODULE})\n__sheet = module(__sheet_source, {})\n__world, __gate, __ears = __given\n__root, __raised = opened(__engine, __sheet, __record, __world, __gate, __ears, __names)\n(__root, __raised)";
    let world = if typed { object("World", id(objects::WORLD)) } else { Object::none() };
    let got = inner.ran(
      opening,
      vec![
        ("__source", Object::string(source)),
        ("__sheet_source", Object::string(SHEET)),
        ("__record", Object::list(record)),
        (
          "__given",
          Object::tuple([
            world,
            object("Gate", id(objects::GATE)),
            object("Ears", id(objects::EARS)),
          ]),
        ),
        ("__names", Object::list(names.into_iter().map(Object::string))),
      ],
    )?;
    let got = got.as_ref();
    let root = entry(&got, 0).and_then(|one| one.as_str()).unwrap_or_default().to_owned();
    let raised = entry(&got, 1).and_then(Fault::of);
    if raised.is_none() && pin.is_none() && !words.is_empty() {
      inner.run(
        "pinning(__engine, __root, __words)",
        vec![("__words", Object::list(words.iter().map(Object::string)))],
      )?;
    }
    if raised.is_none() && !lives.is_empty() {
      inner.run(
        "played(__engine, __lives)",
        vec![("__lives", Object::list(lives.into_iter().map(Object::string)))],
      )?;
    }
    Ok(Life { held: inner, root, raised, words })
  }
}

/// One life of the engine, as a host holds it.
///
/// One host holds one life and says one thing at a time, as an operator does: its methods are the verbs of the
/// contract, and each takes the chain it is on, which is the operator's own when it is empty. An act borrows
/// the life while it is awaited, since awaiting it drives the life.
pub struct Life {
  held: Inner,
  root: String,
  raised: Option<Fault>,
  words: Vec<String>,
}

impl Life {
  /// A life being opened on this World.
  pub fn open(world: impl World + 'static) -> Opening {
    Opening {
      world: Worldly::Typed(Box::new(world)),
      ears: None,
      names: Vec::new(),
      limits: ResourceLimits::default(),
      words: Vec::new(),
      lives: Vec::new(),
    }
  }

  /// A life being opened on the ears of a host alone, the World among them by name.
  pub fn open_on(
    ears: impl Ears + 'static,
    names: impl IntoIterator<Item = impl Into<String>>,
  ) -> Opening {
    Opening {
      world: Worldly::Heard,
      ears: None,
      names: Vec::new(),
      limits: ResourceLimits::default(),
      words: Vec::new(),
      lives: Vec::new(),
    }
    .ears(ears, names)
  }

  /// A life on this World, opened from the record.
  pub fn boot(
    world: impl World + 'static,
    record: impl IntoIterator<Item = Object>,
  ) -> Result<Life, Fault> {
    Life::open(world).boot(record)
  }

  /// The root chain of the life, which is the first act of any record.
  pub fn root(&self) -> &str {
    &self.root
  }

  /// The words of the extensions the life runs after the engine: those its record pins, or else those it was
  /// opened with, which it pinned.
  pub fn words(&self) -> &[String] {
    &self.words
  }

  /// What boot raised, if it raised: a drift, which breaks the journal while the life goes on with nothing kept,
  /// or a refusal of the ears it was given.
  pub fn raised(&self) -> Option<&Fault> {
    self.raised.as_ref()
  }

  /// One word of the operator, run in the names of the engine with these values bound, and what it gave.
  ///
  /// This is what an operator that is python does; a host that is not says the verbs below instead, and keeps
  /// this for what they do not say.
  pub fn word(&mut self, word: &str, inputs: Vec<(&str, Object)>) -> Result<Object, Fault> {
    let mut bound = vec![("__word", Object::string(word))];
    bound.extend(inputs);
    self.held.run("eval(__word, __engine, dict(locals()))", bound)
  }

  /// A callable of the host, as a value a verb may be given: a show, a filter. The sandbox calls it back by
  /// name, and what it gives is what the call gave.
  pub fn callable(
    &mut self,
    call: impl FnMut(Vec<Object>) -> Result<Object, Fault> + 'static,
  ) -> Object {
    self.held.host.calls.push(Box::new(call));
    Object::function(format!("{CALL}{}", self.held.host.calls.len() - 1), None)
  }

  /// One callable the engine made, called back by the handle it crossed under, with these words, and what it
  /// gave. A callable of the engine that has a name crosses as its name instead, and needs no call back.
  pub fn made(
    &mut self,
    n: i64,
    args: Vec<Object>,
    kwargs: Vec<(&str, Object)>,
  ) -> Result<Object, Fault> {
    let kwargs = Object::dict(kwargs.into_iter().map(|(key, value)| (Object::string(key), value)));
    self.held.run(
      "made_called(__engine, __ears, __n, __args, __kwargs)",
      vec![("__n", Object::int(n)), ("__args", Object::list(args)), ("__kwargs", kwargs)],
    )
  }

  /// A callable the engine made, forgotten: the host holds its handle no more, so the sandbox holds it no more.
  pub fn forget(&mut self, n: i64) -> Result<(), Fault> {
    self.held.run("forgotten(__n)", vec![("__n", Object::int(n))]).map(|_| ())
  }

  /// One reading of a map of the life where it stands: `acts`, `asked`, `outcomes` or `modules`, under these
  /// keys, asked whether it holds the last key (`in`), for the value at it (`at`), for its keys (`keys`) or for
  /// how many (`len`).
  pub fn held(&mut self, name: &str, keys: Vec<Object>, ask: &str) -> Result<Object, Fault> {
    self.held.run(
      "held(__engine, __name, __keys, __ask)",
      vec![
        ("__name", Object::string(name)),
        ("__keys", Object::list(keys)),
        ("__ask", Object::string(ask)),
      ],
    )
  }

  /// Who speaks in the life, and who speaks from now on when a value is given: `site`, read and set where it
  /// stands.
  pub fn site(&mut self, value: Option<&str>) -> Result<String, Fault> {
    let value = value.map_or_else(Object::none, Object::string);
    let got = self.held.run("spoken(__engine, __value)", vec![("__value", value)])?;
    Ok(got.as_ref().as_str().unwrap_or_default().to_owned())
  }

  /// What an act came to, once it is done, and nothing while it lives.
  pub fn outcome(&mut self, id: &str) -> Result<Option<Object>, Fault> {
    let held = self.held("outcomes", vec![Object::string(id)], "in")?;
    if !held.as_ref().as_bool().unwrap_or_default() {
      return Ok(None);
    }
    self.held("outcomes", vec![Object::string(id)], "at").map(Some)
  }

  /// What to do when an act is done, given what it came to: told at once for an act that is done already, and
  /// once when it is, after the run in which it was. This is how a host that drives the life itself awaits.
  pub fn watch(&mut self, id: &str, then: impl FnMut(&Object) + 'static) -> Result<(), Fault> {
    self.held.watchers.entry(id.to_owned()).or_default().push(Box::new(then));
    self.held.told()
  }

  /// One verb of the engine by its name, said by the operator with these words, and what it gave.
  ///
  /// The verbs below are this, typed; this is for a host that binds the names of the contract itself.
  pub fn verb(
    &mut self,
    name: &str,
    args: Vec<Object>,
    kwargs: Vec<(&str, Object)>,
  ) -> Result<Object, Fault> {
    let kwargs = Object::dict(kwargs.into_iter().map(|(key, value)| (Object::string(key), value)));
    self.held.run(
      "called(__engine, __ears, __name, __args, __kwargs)",
      vec![("__name", Object::string(name)), ("__args", Object::list(args)), ("__kwargs", kwargs)],
    )
  }

  /// The life driven as far as it goes without the host: what the World said unasked and the work it finished,
  /// said into the engine. An awaited act drives the life itself; this is for a host that awaits nothing.
  pub async fn drive(&mut self) -> Result<(), Fault> {
    std::future::poll_fn(|cx| Poll::Ready(self.held.pump(cx))).await
  }

  /// One act of the life by its name, to await: an act a verb of an extension made, which a host says by `verb`
  /// and holds by its name.
  pub fn awaited(&mut self, id: &str) -> Act<'_, Object> {
    Act { life: self, id: id.to_owned(), came: PhantomData }
  }

  /// One act the operator made, to await.
  fn act<T>(&mut self, got: Object) -> Result<Act<'_, T>, Fault> {
    let id = got
      .as_ref()
      .as_str()
      .map(str::to_owned)
      .ok_or_else(|| Fault::refused("a verb gave no act"))?;
    Ok(Act { life: self, id, came: PhantomData })
  }

  /// The chain a verb is on, as its keyword: none for the operator's own.
  fn on(on: &str) -> Vec<(&str, Object)> {
    if on.is_empty() { vec![] } else { vec![("on", Object::string(on))] }
  }

  /// What an act came to, or nothing while it lives.
  pub fn peek(&mut self, at: &str, on: &str) -> Result<Object, Fault> {
    self.verb("peek", vec![Object::string(at)], Life::on(on))
  }

  /// The turns of a chain, as its model reads them.
  pub fn turns(&mut self, on: &str) -> Result<Object, Fault> {
    self.verb("turns", vec![], Life::on(on))
  }

  /// A reading of the clock.
  pub fn clock(&mut self, on: &str) -> Result<Object, Fault> {
    self.verb("clock", vec![], Life::on(on))
  }

  /// A number drawn.
  pub fn chance(&mut self, on: &str) -> Result<Object, Fault> {
    self.verb("chance", vec![], Life::on(on))
  }

  /// What the gate finds against a word on a chain.
  pub fn gate(&mut self, word: &str, on: &str) -> Result<Object, Fault> {
    self.verb("gate", vec![Object::string(word)], Life::on(on))
  }

  /// The fact of an act, by its name.
  pub fn get(&mut self, about: &str) -> Result<Fact, Fault> {
    let got = self.verb("get", vec![Object::string(about)], vec![])?;
    Fact::of(got.as_ref()).ok_or_else(|| Fault::refused(format!("{about} is no act")))
  }

  /// One act or chain, paused.
  pub fn pause(&mut self, id: &str) -> Result<(), Fault> {
    self.verb("pause", vec![Object::string(id)], vec![]).map(|_| ())
  }

  /// One act or chain, woken.
  pub fn wake(&mut self, id: &str) -> Result<(), Fault> {
    self.verb("wake", vec![Object::string(id)], vec![]).map(|_| ())
  }

  /// One act or chain, cancelled.
  pub fn cancel(&mut self, id: &str) -> Result<(), Fault> {
    self.verb("cancel", vec![Object::string(id)], vec![]).map(|_| ())
  }

  /// One act, closed with a value: the operator's answer to a prompt.
  pub fn close(&mut self, value: Object, id: &str) -> Result<(), Fault> {
    self.verb("close", vec![value], vec![("id", Object::string(id))]).map(|_| ())
  }

  /// A debug of the operator: each expression beside its value, told into the act that runs.
  pub fn debug(&mut self, pairs: Vec<(&str, Object)>) -> Result<(), Fault> {
    let pairs = Object::list(pairs.into_iter().map(|(e, v)| Object::tuple([Object::string(e), v])));
    self.held.run("debugged(__engine, __ears, __pairs)", vec![("__pairs", pairs)]).map(|_| ())
  }

  /// A wait of this many seconds on a chain.
  pub fn wait(&mut self, seconds: f64, on: &str) -> Result<Act<'_, ()>, Fault> {
    let got = self.verb("wait", vec![Object::float(seconds)], Life::on(on))?;
    self.act(got)
  }

  /// A rung on a chain: with a word, or with none for a rung the model writes.
  pub fn rung(
    &mut self,
    word: &str,
    retells: &str,
    actor: &str,
    on: &str,
  ) -> Result<Act<'_, Object>, Fault> {
    let args = [word, retells, actor].into_iter().map(Object::string).collect();
    let got = self.verb("rung", args, Life::on(on))?;
    self.act(got)
  }

  /// A prompt on a chain: of this shape, by its name, with a message, to an actor, or to the operator when empty.
  pub fn prompt(
    &mut self,
    shape: &str,
    message: &str,
    to: &str,
    on: &str,
  ) -> Result<Act<'_, Object>, Fault> {
    let args = [shape, message, to].into_iter().map(Object::string).collect();
    let got = self.verb("prompt", args, Life::on(on))?;
    self.act(got)
  }

  /// A chain, with a label, on a source, kept by a filter, which is a callable of the host or nothing.
  pub fn chain(
    &mut self,
    label: &str,
    source: &str,
    filter: Option<Object>,
    on: &str,
  ) -> Result<Act<'_, Object>, Fault> {
    let mut kwargs = Life::on(on);
    if let Some(filter) = filter {
      kwargs.push(("filter", filter));
    }
    let got = self.verb("chain", vec![Object::string(label), Object::string(source)], kwargs)?;
    self.act(got)
  }
}

impl std::fmt::Debug for Life {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Life").field("root", &self.root).finish_non_exhaustive()
  }
}

/// What an act comes to, read as a value of the host.
pub trait Came: Sized {
  /// The value the act came to, or the fault it completed with.
  fn came(got: ObjectRef<'_>) -> Result<Self, Fault>;
}

impl Came for Object {
  fn came(got: ObjectRef<'_>) -> Result<Self, Fault> {
    match Fault::of(got) {
      Some(fault) => Err(fault),
      None => Ok(got.to_owned()),
    }
  }
}

impl Came for () {
  fn came(got: ObjectRef<'_>) -> Result<Self, Fault> {
    Object::came(got).map(|_| ())
  }
}

/// One act of a life, awaited for what it comes to.
///
/// It borrows the life for as long as it is awaited, since awaiting it drives the life: what the World said and
/// the work it finished is said into the engine until the act is done. An act that completed with an exception
/// gives that fault. Dropping it leaves the act living; the life holds what it comes to under its name.
pub struct Act<'a, T> {
  life: &'a mut Life,
  id: String,
  came: PhantomData<T>,
}

impl<T> Act<'_, T> {
  /// The name of the act, which is what the engine knows it by.
  pub fn id(&self) -> &str {
    &self.id
  }
}

impl<T> std::fmt::Debug for Act<'_, T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_tuple("Act").field(&self.id).finish()
  }
}

impl<T: Came + Unpin> Future for Act<'_, T> {
  type Output = Result<T, Fault>;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let this = self.get_mut();
    if let Err(fault) = this.life.held.pump(cx) {
      return Poll::Ready(Err(fault));
    }
    match this.life.outcome(&this.id) {
      Ok(Some(got)) => Poll::Ready(T::came(got.as_ref())),
      Ok(None) => Poll::Pending,
      Err(fault) => Poll::Ready(Err(fault)),
    }
  }
}

#[cfg(test)]
#[path = "life.test.rs"]
mod test;
