//! The engine: one life of `engine.py` in the sandbox, the ears it hears, and the verbs a host says.
//!
//! An engine is driven from one thread. A verb is one call: it runs in the sandbox, the engine does what it does,
//! every fact it says reaches its ears, and what the verb gave comes back. An act is awaited: the future drives the
//! engine until the act is done, which means it says what the voices of its ears said since, each under the name of
//! its ear. Nothing polls: a voice wakes whoever awaits.
//!
//! The host reaches the sandbox as two objects the stand-in holds, the gate, which reads a sheet, and the ears,
//! which hear every ear of the host by name, and as the functions it gave, a show or a filter. A call of one of them
//! comes here, and is answered on the thread of the engine, inside the call.

use std::{
  cell::RefCell,
  collections::{HashMap, HashSet},
  future::Future,
  marker::PhantomData,
  pin::Pin,
  rc::Rc,
  task::{Context, Poll, Waker},
};

use monty_types::{MontyUuid, NamedValues};

use crate::{
  ENGINE, KERNEL, PREAMBLE, SHEET,
  ear::{Ear, Heard, Step, Voice},
  fact::Fact,
  gate::checked,
  sand::{Sand, id, object},
  value::{Exit, Fault, Object, ObjectRef, Text, entry, inward, marked},
};

/// The prefix of the name a function of the host crosses in under: `call:` and its number.
const CALL: &str = "call:";

/// The prefix of the name an ear of the host is heard under when a verb is given it: `ear:` and its number.
const EAR: &str = "ear:";

/// A function of the host, as the sandbox may call it back.
type Callable = Box<dyn FnMut(Vec<Object>) -> Result<Object, Fault>>;

/// What a host does when an act is done, given what it came to.
type Watcher = Box<dyn FnMut(&Object)>;

/// The id of the gate, which reads a sheet, one of the two objects of the host the stand-in holds.
const GATE: u8 = 2;

/// The id of the ears, which hear every ear of the host by name, the other object of the host.
const EARS: u8 = 3;

/// The kinds of act that are work an operator waits for: a prompt, a rung, a command and a wait.
const WORK: [&str; 4] = ["prompt", "rung", "bash", "wait"];

/// Whether this life took up an act that the record shows started and not done: the outside says it started, which
/// it does once a wake that this life says puts the act to it, or the act is done.
const TOOK: &str =
  "peek(__id, ...) is not ... or ('started', __id) in [x[:2] for x in transcript(scope(__id))]";

/// Whether an act waits: it is not done, and no pause holds it.
const WAITS: &str = "peek(__id, ...) is ... and not paused(__id)";

/// The ears and the functions of the host, by the names the sandbox calls them by.
///
/// A door adds to them while an ear speaks or a function runs, since a function of the host may give a generator,
/// so they are shared, and each is taken out while it runs.
#[derive(Clone, Default)]
pub(crate) struct Hosted {
  ears: Rc<RefCell<Ears>>,
  calls: Rc<RefCell<Vec<Option<Callable>>>>,
}

/// The ears of the host by name, and how many a door added, which names the next.
#[derive(Default)]
struct Ears {
  named: HashMap<String, Box<dyn Ear>>,
  added: usize,
}

impl Hosted {
  /// An ear of the host, heard under this name.
  fn named(&self, name: String, ear: Box<dyn Ear>) -> Result<(), Fault> {
    match self.ears.borrow_mut().named.insert(name.clone(), ear) {
      Some(_) => Err(Fault::refused(format!("{name} hears"))),
      None => Ok(()),
    }
  }

  /// An ear of the host, heard from now on under a name of its own, as the value a verb is given: the generator
  /// that stands in for it in the sandbox, which stands at a yield when the host started the ear before it crossed.
  pub(crate) fn ear(&self, ear: Box<dyn Ear>, started: bool) -> Object {
    let name = {
      let mut held = self.ears.borrow_mut();
      held.added += 1;
      let name = format!("{EAR}{}", held.added);
      held.named.insert(name.clone(), ear);
      name
    };
    marked("ear", [("name", Object::string(name)), ("started", Object::bool(started))])
  }

  /// Every ear and every function of the host goes, with what each holds, as a command it runs or a record it keeps.
  #[cfg_attr(not(any(feature = "python", feature = "typescript")), allow(dead_code))]
  pub(crate) fn clear(&self) {
    let ears = std::mem::take(&mut self.ears.borrow_mut().named);
    let calls = std::mem::take(&mut *self.calls.borrow_mut());
    drop((ears, calls));
  }

  /// A function of the host, as a value a verb may be given.
  pub(crate) fn callable(&self, call: Callable) -> Object {
    let mut calls = self.calls.borrow_mut();
    calls.push(Some(call));
    Object::function(format!("{CALL}{}", calls.len() - 1), None)
  }

  /// What the ear of this name does with what it heard.
  fn resume(&self, name: &str, heard: Heard) -> Result<Step, Fault> {
    let taken = self.ears.borrow_mut().named.remove(name);
    let Some(mut ear) = taken else {
      return Err(Fault::refused(format!("no ear of the host is named {name}")));
    };
    let step = ear.resume(heard);
    self.ears.borrow_mut().named.insert(name.to_owned(), ear);
    Ok(step)
  }

  /// What the function of this name gave, called with these words.
  fn call(&self, name: &str, args: Vec<Object>) -> Result<Object, Fault> {
    let missing = || Fault::refused(format!("{name} is no function of the host"));
    let n = name.strip_prefix(CALL).and_then(|n| n.parse::<usize>().ok()).ok_or_else(missing)?;
    let taken = self.calls.borrow_mut().get_mut(n).and_then(Option::take);
    let mut call = taken.ok_or_else(missing)?;
    let got = call(args);
    self.calls.borrow_mut()[n] = Some(call);
    got
  }
}

impl Hosted {
  /// One call of the sandbox, answered: a method of one of the objects of the host, or a function of the host. The
  /// gate is the thread's, and an ear is born with the voice of the engine.
  fn called(
    &self,
    voice: &Voice,
    on: Option<MontyUuid>,
    name: &str,
    args: Vec<Object>,
  ) -> Result<Object, Fault> {
    let Some(on) = on else { return self.call(name, args) };
    let at = |i: usize| args.get(i).map(Object::as_ref);
    if on == id(GATE) {
      let found = checked(&text(at(0)))?;
      return Ok(Object::list(found.into_iter().map(|(line, why)| {
        Object::tuple([Object::int(i64::try_from(line).unwrap_or_default()), Object::string(why)])
      })));
    }
    if on != id(EARS) {
      return Err(Fault::refused(format!("{on} is no object of the host")));
    }
    let heard = match name {
      "born" => Heard::Born(voice.of(&text(at(1)))),
      "hears" => Heard::Fact(
        at(1).and_then(Fact::of).ok_or_else(|| Fault::refused("an ear hears a fact or nothing"))?,
      ),
      "answered" => {
        let got = at(1).map(|one| one.items().unwrap_or_default()).unwrap_or_default();
        match (got.first().and_then(|one| one.as_str()), got.get(1)) {
          (Some("value"), Some(value)) => Heard::Value(value.to_owned()),
          (Some("raised"), Some(no)) => Heard::Raised(
            Fault::of(*no).unwrap_or_else(|| Fault::refused(format!("{} raised", no.py_repr()))),
          ),
          _ => return Err(Fault::refused("a verb gives a value or raises")),
        }
      }
      _ => return Err(Fault::refused(format!("the ears of the host have no {name}"))),
    };
    Ok(replied(self.resume(&text(at(0)), heard)?))
  }
}

/// The acts that the entries of a record show started and not done, by name, in the order of the record: work that
/// the outside took in an earlier life. The record keeps the started of the outside alone, since a later life says
/// again what an act said.
fn left(record: &[Object]) -> Vec<String> {
  let facts: Vec<Fact> =
    record.iter().filter_map(|one| entry(&one.as_ref(), 0).and_then(Fact::of)).collect();
  let done: HashSet<&str> = facts.iter().filter(|a| a.kind() == "done").map(Fact::about).collect();
  let started = facts.iter().filter(|a| a.kind() == "started" && !done.contains(a.about()));
  started.map(|a| a.about().to_owned()).collect()
}

/// A step of an ear, as the stand-in reads it.
fn replied(step: Step) -> Object {
  match step {
    Step::Wait => Object::none(),
    Step::Say(saying) => Object::tuple([Object::string("say"), saying.0]),
    Step::Raised(fault) => Object::tuple([Object::string("raised"), fault.object()]),
    Step::Over => Object::tuple([Object::string("over")]),
    Step::Call(call) => Object::tuple([
      Object::string("calls"),
      Object::string(call.verb),
      Object::list(call.args),
      Object::dict(call.kwargs.into_iter().map(|(key, one)| (Object::string(key), one))),
    ]),
  }
}

/// The text a word of a call is, or nothing.
fn text(got: Option<ObjectRef<'_>>) -> String {
  got.and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default()
}

/// A template string, as a host says one: each expression beside its value.
fn templated(pairs: &[(&str, Object)]) -> Object {
  crate::value::templated(
    pairs.iter().map(|(expression, one)| (one.clone(), Object::string(*expression))),
  )
}

/// The words of a verb that have a name, as the map the stand-in reads.
fn named(kwargs: Vec<(&str, Object)>) -> Object {
  Object::dict(kwargs.into_iter().map(|(key, value)| (Object::string(key), value)))
}

/// One life of the engine, as a host holds it.
///
/// One host holds one engine and says one thing at a time, as an operator does: its methods are the verbs of the
/// contract, and an act borrows the engine while it is awaited, since awaiting it drives the engine.
pub struct Engine {
  sand: Sand,
  hosted: Hosted,
  voice: Voice,
  /// Who waits on an act, by its name, told once when it is done.
  watchers: HashMap<String, Vec<Watcher>>,
  root: String,
  raised: Option<Fault>,
  /// The acts that the record showed started and not done, which the journal holds from the outside and which this
  /// life has not taken up yet, in the order of the record.
  holding: Vec<String>,
}

impl Engine {
  /// One piece of code of the stand-in, run in the sandbox with these names bound, each as it goes in; and then
  /// whoever watches an act that is done now is told, even when the code raised.
  fn run(&mut self, code: &str, inputs: Vec<(&str, Object)>) -> Result<Object, Fault> {
    match self.fed(code, inputs) {
      Ok(got) => Ok(got),
      Err(fault) => self.told().and(Err(fault)),
    }
  }

  /// Whoever watches an act that is done now is told.
  fn told(&mut self) -> Result<(), Fault> {
    if self.watchers.is_empty() {
      return Ok(());
    }
    self.fed("None", vec![]).map(drop)
  }

  /// One feed of the sandbox: the code, then what every watched act came to, read in the same feed, since the
  /// session holds a name for each feed until the life ends. Each watcher of an act that is done is told once.
  fn fed(&mut self, code: &str, mut inputs: Vec<(&str, Object)>) -> Result<Object, Fault> {
    let ids: Vec<String> = self.watchers.keys().cloned().collect();
    inputs.push(("__watched", Object::list(ids.iter().map(Object::string))));
    let got = self.ran(&format!("({code}, outcomes_of(__engine, __watched))"), inputs)?;
    let got = got.as_ref();
    let outcomes = entry(&got, 1).expect("a feed gives what the watched acts came to");
    for (i, id) in ids.iter().enumerate() {
      if let Some(done) = entry(&outcomes, i)
        && done.type_name() != "NoneType"
        && let Some(value) = entry(&done, 0)
      {
        let value = value.to_owned();
        for mut watcher in self.watchers.remove(id).unwrap_or_default() {
          watcher(&value);
        }
      }
    }
    Ok(entry(&got, 0).expect("a feed gives what its code gave").to_owned())
  }

  fn ran(&mut self, code: &str, inputs: Vec<(&str, Object)>) -> Result<Object, Fault> {
    let Engine { sand, hosted, voice, .. } = self;
    let mut named = NamedValues::new();
    for (name, value) in inputs {
      named.push(name, inward(&value));
    }
    sand.run(code, named, &mut |on, name, args| hosted.called(voice, on, name, args))
  }

  /// What the voices said since the engine was last driven, said into it in one run, each under the name of its
  /// ear, until they say nothing more; and the waker to wake when they do. An awaited act drives the engine so,
  /// and a door that awaits in the loop of its own language drives it so too.
  pub(crate) fn pump(&mut self, waker: &Waker) -> Result<(), Fault> {
    loop {
      let said = self.voice.drained(waker);
      if said.is_empty() {
        return Ok(());
      }
      let said = said
        .into_iter()
        .map(|one| Object::tuple([Object::string(one.by), replied(one.spoken.into())]));
      self.run("said(__engine, __ears, __said)", vec![("__said", Object::list(said))])?;
    }
  }

  /// An engine, opened from the record, on these ears, each under the name the engine hears it by, in the order
  /// the engine offers them a question.
  ///
  /// The record is the entries the store kept, each one fact. The stand-in runs first in a module of its own, then
  /// the engine, and `boot` is given the Kernel and the gate of the crate before the ears of the host.
  pub fn boot<N: Into<String>>(
    record: impl IntoIterator<Item = Object>,
    ears: impl IntoIterator<Item = (N, Box<dyn Ear>)>,
  ) -> Result<Engine, Fault> {
    Engine::open(Hosted::default(), record, ears)
  }

  /// An engine opened on the ears and the functions of a host that a door already added to.
  pub(crate) fn open<N: Into<String>>(
    hosted: Hosted,
    record: impl IntoIterator<Item = Object>,
    ears: impl IntoIterator<Item = (N, Box<dyn Ear>)>,
  ) -> Result<Engine, Fault> {
    let mut names = Vec::new();
    for (name, ear) in ears {
      let name = name.into();
      hosted.named(name.clone(), ear)?;
      names.push(name);
    }
    let record: Vec<Object> = record.into_iter().collect();
    let (sand, voice, watchers, holding) =
      (Sand::new(), Voice::new(), HashMap::new(), left(&record));
    let (root, raised) = (String::new(), None);
    let mut engine = Engine { sand, hosted, voice, watchers, root, raised, holding };
    engine.ran(PREAMBLE, vec![])?;
    // The two objects of the host and the two modules are bound as names of the session, which every later piece
    // of code of the stand-in reads.
    let opening = "__engine = loaded(__source, {**MODULE})\n__sheet = loaded(__sheet_source, {})\n__kernel = loaded(__kernel_source, {})\n__gate, __ears = __given\n__root, __raised = opened(__engine, __sheet, __kernel, __record, __gate, __ears, __names)\n(__root, __raised)";
    let got = engine.ran(
      opening,
      vec![
        ("__source", Object::string(ENGINE)),
        ("__sheet_source", Object::string(SHEET)),
        ("__kernel_source", Object::string(KERNEL)),
        ("__record", Object::list(record)),
        ("__given", Object::tuple([object("Gate", id(GATE)), object("Ears", id(EARS))])),
        ("__names", Object::list(names.into_iter().map(Object::string))),
      ],
    )?;
    let got = got.as_ref();
    engine.root = entry(&got, 0).and_then(|one| one.as_str()).unwrap_or_default().to_owned();
    engine.raised = entry(&got, 1).and_then(Fault::of);
    Ok(engine)
  }

  /// The root chain of the life, which is the first act of any record.
  pub fn root(&self) -> &str {
    &self.root
  }

  /// What boot raised, if it raised: a drift, which breaks the journal while the life goes on with nothing kept,
  /// or a refusal of the ears it was given.
  pub fn raised(&self) -> Option<&Fault> {
    self.raised.as_ref()
  }

  /// A function of the host, as a value a verb may be given: a show, a filter. The sandbox calls it back, and what
  /// it gives is what the call gave.
  pub fn callable(
    &mut self,
    call: impl FnMut(Vec<Object>) -> Result<Object, Fault> + 'static,
  ) -> Object {
    self.hosted.callable(Box::new(call))
  }

  /// The ears and the functions of the host, which a door adds to as it carries a value of its language in.
  #[cfg_attr(not(any(feature = "python", feature = "typescript")), allow(dead_code))]
  pub(crate) fn hosted(&self) -> Hosted {
    self.hosted.clone()
  }

  /// One verb of the engine by its name, said with these words, and what it gave.
  pub(crate) fn verb(
    &mut self,
    name: &str,
    args: Vec<Object>,
    kwargs: Vec<(&str, Object)>,
  ) -> Result<Object, Fault> {
    self.run(
      "called(__engine, __ears, __name, __args, __kwargs)",
      vec![
        ("__name", Object::string(name)),
        ("__args", Object::list(args)),
        ("__kwargs", named(kwargs)),
      ],
    )
  }

  /// One word run in the names of the engine with these values bound, and what it gave, which is how a door reads
  /// what no verb reads.
  pub(crate) fn word(&mut self, word: &str, inputs: Vec<(&str, Object)>) -> Result<Object, Fault> {
    let mut bound = vec![("__word", Object::string(word))];
    bound.extend(inputs);
    self.run("eval(__word, __engine, dict(locals()))", bound)
  }

  /// One callable the engine made, called back by the handle it crossed under, with these words, and what it gave.
  #[cfg_attr(not(feature = "python"), allow(dead_code))]
  pub(crate) fn made(
    &mut self,
    n: i64,
    args: Vec<Object>,
    kwargs: Vec<(&str, Object)>,
  ) -> Result<Object, Fault> {
    self.run(
      "made_called(__engine, __ears, __n, __args, __kwargs)",
      vec![("__n", Object::int(n)), ("__args", Object::list(args)), ("__kwargs", named(kwargs))],
    )
  }

  /// A callable the engine made, forgotten: the host holds its handle no more, so the sandbox holds it no more.
  #[cfg_attr(not(feature = "python"), allow(dead_code))]
  pub(crate) fn forget(&mut self, n: i64) -> Result<(), Fault> {
    self.run("forgotten(__n)", vec![("__n", Object::int(n))]).map(|_| ())
  }

  /// Who speaks in the life, and who speaks from now on when a value is given: `site`, read and set where it stands.
  #[cfg_attr(not(any(feature = "python", feature = "typescript")), allow(dead_code))]
  pub(crate) fn site(&mut self, value: Option<&str>) -> Result<String, Fault> {
    let value = value.map_or_else(Object::none, Object::string);
    let got = self.run("spoken(__engine, __value)", vec![("__value", value)])?;
    Ok(got.as_ref().as_str().unwrap_or_default().to_owned())
  }

  /// The work that an earlier life left, which waits for a wake that this life says, in the order of the record, each
  /// act by its name and its kind. The record shows each act that the outside started and did not end, and the
  /// journal holds it from the outside until a wake that this life says puts it to the outside again. Such an act is
  /// pending while it is not done and no pause holds it, and so is each act above it that made it, directly or not,
  /// when that act is a prompt, a rung, a command or a wait that is not done and that no pause holds.
  pub fn pending(&mut self) -> Result<Vec<(String, String)>, Fault> {
    let mut holding = Vec::new();
    for id in std::mem::take(&mut self.holding) {
      if !self.whether(TOOK, &id)? {
        holding.push(id);
      }
    }
    self.holding = holding.clone();
    let mut pending = Vec::new();
    for id in holding {
      if !self.whether(WAITS, &id)? {
        continue;
      }
      let (mut made, mut at) = (Vec::new(), id.clone());
      while let Some(act) = self.get(&at)? {
        if WORK.contains(&act.kind()) && self.whether(WAITS, &at)? {
          made.push((at, act.kind().to_owned()));
        }
        at = act.by().to_owned();
      }
      for one in made.into_iter().rev() {
        if !pending.contains(&one) {
          pending.push(one);
        }
      }
    }
    Ok(pending)
  }

  /// Whether a word of the names of the engine is true of an act.
  fn whether(&mut self, word: &str, id: &str) -> Result<bool, Fault> {
    let got = self.word(word, vec![("__id", Object::string(id))])?;
    Ok(got.as_ref().as_bool().unwrap_or_default())
  }

  /// What an act came to, once it is done, and nothing while it lives.
  pub fn outcome(&mut self, id: &str) -> Result<Option<Object>, Fault> {
    let got = self.run("outcomes_of(__engine, [__id])[0]", vec![("__id", Object::string(id))])?;
    Ok(entry(&got.as_ref(), 0).map(|one| one.to_owned()))
  }

  /// What to do when an act is done, given what it came to: told at once for an act that is done already, and once
  /// when it is, after the run in which it was. This is how a door that drives the engine itself awaits.
  #[cfg_attr(not(any(feature = "python", feature = "typescript")), allow(dead_code))]
  pub(crate) fn watch(
    &mut self,
    id: &str,
    then: impl FnMut(&Object) + 'static,
  ) -> Result<(), Fault> {
    self.watchers.entry(id.to_owned()).or_default().push(Box::new(then));
    self.told()
  }

  /// One act a verb made, to await.
  fn awaited<T>(&mut self, got: Object) -> Result<Act<'_, T>, Fault> {
    let id = got
      .as_ref()
      .as_str()
      .map(str::to_owned)
      .ok_or_else(|| Fault::refused("a verb gave no act"))?;
    Ok(Act { engine: self, id, came: PhantomData })
  }
}

// The verbs of the contract, one method each, and its constants that are a number or a text, which the build makes
// from the contract.
include!(concat!(env!("OUT_DIR"), "/methods.rs"));
include!(concat!(env!("OUT_DIR"), "/constants.rs"));

impl std::fmt::Debug for Engine {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Engine").field("root", &self.root).finish_non_exhaustive()
  }
}

/// The struct of each verb that takes words with a default: each is none until a host gives it, and none leaves
/// the default of the engine.
pub mod verbs {
  include!(concat!(env!("OUT_DIR"), "/verbs.rs"));
}

/// What a verb gave or an act came to, read as the value a host of rust is given.
pub trait Plain: Sized {
  /// The value, or why it is not one.
  fn plain(got: Object) -> Result<Self, Fault>;
}

impl Plain for Object {
  fn plain(got: Object) -> Result<Self, Fault> {
    Ok(got)
  }
}

impl Plain for () {
  fn plain(_: Object) -> Result<Self, Fault> {
    Ok(())
  }
}

impl Plain for String {
  fn plain(got: Object) -> Result<Self, Fault> {
    got
      .as_ref()
      .as_str()
      .map(str::to_owned)
      .ok_or_else(|| Fault::refused(format!("{} is no text", got.py_repr())))
  }
}

impl Plain for f64 {
  fn plain(got: Object) -> Result<Self, Fault> {
    let one = got.as_ref();
    one
      .as_float()
      .or_else(|| one.as_int().map(|n| n as f64))
      .ok_or_else(|| Fault::refused(format!("{} is no number", got.py_repr())))
  }
}

/// A value read as a type of the crate that it makes again, or why it is none.
macro_rules! made {
  ($($kind:ty => $what:literal),*) => {$(
    impl Plain for $kind {
      fn plain(got: Object) -> Result<Self, Fault> {
        let no = || Fault::refused(format!("{} is no {}", got.py_repr(), $what));
        <$kind>::of(got.as_ref()).ok_or_else(no)
      }
    }
  )*};
}

made!(Text => "text", Exit => "exit", Fact => "fact");

impl Plain for Option<Fact> {
  fn plain(got: Object) -> Result<Self, Fault> {
    if got.as_ref().type_name() == "NoneType" { Ok(None) } else { Fact::plain(got).map(Some) }
  }
}

impl<T: Plain> Plain for Vec<T> {
  fn plain(got: Object) -> Result<Self, Fault> {
    let items = got
      .as_ref()
      .items()
      .ok_or_else(|| Fault::refused(format!("{} is no list", got.py_repr())))?;
    items.into_iter().map(|one| T::plain(one.to_owned())).collect()
  }
}

/// One act of an engine, awaited for what it comes to: `Act` of the contract, a name that is awaitable.
///
/// It borrows the engine for as long as it is awaited, since awaiting it drives the engine: what the voices said
/// is said into it until the act is done. An act that completed with an exception gives that fault. Dropping it
/// leaves the act living; the engine holds what it comes to under its name.
pub struct Act<'a, T> {
  engine: &'a mut Engine,
  id: String,
  came: PhantomData<T>,
}

impl<'a, T> Act<'a, T> {
  /// The act of this name, to await.
  pub fn of(engine: &'a mut Engine, id: impl Into<String>) -> Self {
    Act { engine, id: id.into(), came: PhantomData }
  }

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

impl<T: Plain + Unpin> Future for Act<'_, T> {
  type Output = Result<T, Fault>;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let this = self.get_mut();
    if let Err(fault) = this.engine.pump(cx.waker()) {
      return Poll::Ready(Err(fault));
    }
    match this.engine.outcome(&this.id) {
      Ok(Some(got)) => Poll::Ready(Fault::of(got.as_ref()).map_or_else(|| T::plain(got), Err)),
      Ok(None) => Poll::Pending,
      Err(fault) => Poll::Ready(Err(fault)),
    }
  }
}

#[cfg(test)]
#[path = "engine.test.rs"]
mod test;
