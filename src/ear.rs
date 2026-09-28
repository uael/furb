//! The ear: the one thing a life hears by.
//!
//! The contract lets `boot`, `act` and `drive` take any generator of one shape: it hears every fact of the life
//! and every question that no ear before it took, and it speaks by yielding a saying. The Kernel, the gate, the
//! record and each ear of the World are one, whatever language writes it: a generator of python, a generator of
//! TypeScript, or a coroutine of rust.
//!
//! An ear of rust is an [`Ear`]: it is resumed with what it heard and gives back what it does, which are the steps
//! of a generator of the engine. At its birth it hears nothing but the [`Voice`] its later work speaks with. It
//! says a saying and hears back the fact the bus made of it; it waits and hears the next fact. While it hears, it
//! calls a verb of the engine with [`call`], as an ear of the engine calls one, and the life that hears it answers
//! at once, since the sandbox takes a call of the host while it waits for the host.
//!
//! [`ear`] makes one from an async body, a coroutine of `genawaiter`, whose `co.yield_(step).await` is the `yield`
//! of python: [`hear`] and [`say`] read like the ears of the engine.
//!
//! The work an ear begins, a command, a wait, outlives the hearing that began it, and speaks later by its Voice,
//! under the name of the ear, from any thread: it says a saying, or a verb of the engine.

use std::{
  cell::Cell,
  collections::{HashSet, VecDeque},
  future::Future,
  pin::Pin,
  ptr::NonNull,
  sync::{Arc, Mutex},
  task::Waker,
};

use genawaiter::{GeneratorState, rc::Gen};

use crate::{
  fact::Fact,
  value::{Fault, Object},
};

/// What an ear is resumed with.
#[derive(Debug)]
pub enum Heard {
  /// Its birth, with the voice its later work speaks with, under the name the engine hears the ear by.
  Born(Voice),
  /// One fact: the next of the log, or the one the bus made of what the ear said.
  Fact(Fact),
}

/// What an ear does with what it heard.
#[derive(Debug)]
pub enum Step {
  /// One saying, which the bus makes whole and gives back as the next fact the ear hears.
  Say(Fact),
  /// Nothing more: the ear waits for the next fact.
  Wait,
  /// The ear is over, as a generator that returned, and hears nothing more.
  Over,
  /// The ear raised, as a generator that raised, and hears nothing more.
  Raised(Fault),
}

/// One verb of the engine as an ear calls it: its name and its words.
#[derive(Debug)]
pub struct Call {
  pub verb: String,
  pub args: Vec<Object>,
  pub kwargs: Vec<(String, Object)>,
}

impl Step {
  /// What a generator of a host yielded, as a step: nothing is a wait, and a list or a tuple is a saying.
  #[cfg_attr(not(any(feature = "python", feature = "typescript")), allow(dead_code))]
  pub(crate) fn of(yielded: &Object) -> Result<Step, Fault> {
    let got = yielded.as_ref();
    if got.type_name() == "NoneType" {
      return Ok(Step::Wait);
    }
    let items = got.items().ok_or_else(|| Fault::refused("an ear yields a saying or nothing"))?;
    let saying = Object::tuple(items.into_iter().map(|one| one.to_owned()));
    Fact::of(saying.as_ref())
      .map(Step::Say)
      .ok_or_else(|| Fault::refused("an ear says a kind and what it is about, then its words"))
  }
}

/// An ear: one generator of the life, resumed with what it heard.
pub trait Ear {
  /// What the ear does with what it heard.
  fn resume(&mut self, heard: Heard) -> Step;
}

/// The coroutine an ear of rust yields its steps through: `co.yield_(step).await` gives what it heard next.
pub type Co = genawaiter::rc::Co<Step, Heard>;

type Body = Pin<Box<dyn Future<Output = Result<(), Fault>>>>;

/// An ear of rust: its body, which is made at its birth, when the voice is there to give it.
struct Coroutine {
  body: Option<Box<dyn FnOnce(Co, Voice) -> Body>>,
  running: Option<Gen<Step, Heard, Body>>,
}

impl Ear for Coroutine {
  fn resume(&mut self, heard: Heard) -> Step {
    if let Heard::Born(voice) = heard {
      let Some(body) = self.body.take() else { return Step::Wait };
      let born = Heard::Born(voice.clone());
      self.running = Some(Gen::new(move |co| body(co, voice)));
      // A coroutine drops what its first resume gives, since nothing awaits it yet: the body has its birth already.
      return self.step(born);
    }
    self.step(heard)
  }
}

impl Coroutine {
  fn step(&mut self, heard: Heard) -> Step {
    let Some(running) = self.running.as_mut() else { return Step::Over };
    match running.resume_with(heard) {
      GeneratorState::Yielded(step) => step,
      GeneratorState::Complete(done) => {
        self.running = None;
        done.map_or_else(Step::Raised, |()| Step::Over)
      }
    }
  }
}

/// An ear of rust from its body: an async block given the coroutine it yields through and the voice its later work
/// speaks with, which yields [`Step::Say`] or [`Step::Wait`], and whose end is the end of the ear.
pub fn ear<F, B>(body: B) -> Box<dyn Ear>
where
  B: FnOnce(Co, Voice) -> F + 'static,
  F: Future<Output = Result<(), Fault>> + 'static,
{
  Box::new(Coroutine {
    body: Some(Box::new(move |co, voice| Box::pin(body(co, voice)) as Body)),
    running: None,
  })
}

/// The next fact the ear hears, which is `a = yield` of python.
pub async fn hear(co: &Co) -> Fact {
  loop {
    if let Heard::Fact(fact) = co.yield_(Step::Wait).await {
      return fact;
    }
  }
}

/// One saying, said, and the fact the bus made of it, which is `a = yield saying` of python.
pub async fn say(co: &Co, saying: Fact) -> Fact {
  let mut said = co.yield_(Step::Say(saying)).await;
  loop {
    if let Heard::Fact(fact) = said {
      return fact;
    }
    said = co.yield_(Step::Wait).await;
  }
}

/// What answers a verb that an ear calls while it hears: the life that hears the ear.
pub(crate) type Answers<'a> = dyn FnMut(Call) -> Result<Object, Fault> + 'a;

thread_local! {
  /// What answers the verbs of the ear that a life of this thread hears now, when it hears one, and nothing while one
  /// of those verbs runs.
  static ANSWERS: Cell<Option<NonNull<Answers<'static>>>> = const { Cell::new(None) };
}

/// What `step` gives, with `answers` as what answers each verb that an ear calls while `step` runs. What answered
/// before answers again once `step` is over, even when it panics.
pub(crate) fn heard<R>(answers: &mut Answers<'_>, step: impl FnOnce() -> R) -> R {
  struct Before(Option<NonNull<Answers<'static>>>);
  impl Drop for Before {
    fn drop(&mut self) {
      ANSWERS.set(self.0.take());
    }
  }
  // SAFETY: the pointer stands in the slot only while `step` runs, which `answers` outlives, and `call` alone reads
  // it, out of the slot, so no two borrows of `answers` live at once.
  let answering = unsafe {
    std::mem::transmute::<NonNull<Answers<'_>>, NonNull<Answers<'static>>>(NonNull::from(answers))
  };
  let _before = Before(ANSWERS.replace(Some(answering)));
  step()
}

/// One verb of the engine, called by its name with its words by the ear that a life of this thread hears now, and
/// what it gave or raised.
pub fn call(verb: &str, args: Vec<Object>, kwargs: Vec<(&str, Object)>) -> Result<Object, Fault> {
  let kwargs = kwargs.into_iter().map(|(key, one)| (key.to_owned(), one)).collect();
  let call = Call { verb: verb.to_owned(), args, kwargs };
  let Some(mut answers) = ANSWERS.take() else {
    return Err(Fault::refused(format!("{verb} is called while no life hears an ear")));
  };
  // SAFETY: the pointer stands in the slot only while the step that set it runs, and this borrow of it lives while
  // the pointer is out of the slot.
  let got = unsafe { answers.as_mut() }(call);
  ANSWERS.set(Some(answers));
  got
}

/// One thing that the work of an ear said once its hearing was over: who says it, the act the work is about, and what
/// it says.
pub(crate) struct Said {
  pub by: String,
  pub about: String,
  pub spoken: Spoken,
}

/// What the work of an ear says: a saying, or a verb of the engine with its words.
pub(crate) enum Spoken {
  Saying(Fact),
  Verb(Call),
}

/// What an engine drains of the voices of its ears: the sayings not yet heard, the acts whose work is hushed, and
/// the waker of whoever drives the engine.
#[derive(Default)]
struct Held {
  said: VecDeque<Said>,
  hushed: HashSet<(String, String)>,
  waker: Option<Waker>,
}

/// What the work of an ear speaks with, once the hearing that began it is over.
///
/// It says under the name of its ear. It is cheap to clone and may be carried to any thread, so a command speaks
/// from where it runs. What is said is heard in the order it was said, the next time the engine is driven, and
/// saying it wakes whoever drives.
#[derive(Clone)]
pub struct Voice {
  held: Arc<Mutex<Held>>,
  by: String,
}

impl Voice {
  /// The voices of one engine, which say under no name until [`Voice::of`] gives one.
  pub(crate) fn new() -> Voice {
    Voice { held: Arc::default(), by: String::new() }
  }

  /// The voice of the ear of this name.
  pub(crate) fn of(&self, by: &str) -> Voice {
    Voice { held: Arc::clone(&self.held), by: by.to_owned() }
  }

  /// One saying: its kind, the act it is about, and its words, which the bus makes whole under the name of the ear.
  pub fn say(&self, kind: &str, about: &str, words: impl IntoIterator<Item = Object>) {
    self.spoken(about, Spoken::Saying(Fact::says(kind, about, words)));
  }

  /// One verb of the engine, said with its words by the work of this ear about an act, under the name of the ear
  /// and in its turn among what the ear says, as the ear calls a verb while it hears. A control that the work of an
  /// ear must say before a done, such as a pause, is said so.
  pub fn call(&self, about: &str, verb: &str, args: Vec<Object>, kwargs: Vec<(&str, Object)>) {
    let kwargs = kwargs.into_iter().map(|(key, one)| (key.to_owned(), one)).collect();
    self.spoken(about, Spoken::Verb(Call { verb: verb.to_owned(), args, kwargs }));
  }

  /// What the work of this ear about an act says, held until the engine is driven, unless that work is hushed.
  fn spoken(&self, about: &str, spoken: Spoken) {
    let waker = {
      let Ok(mut held) = self.held.lock() else { return };
      if held.hushed.contains(&(self.by.clone(), about.to_owned())) {
        return;
      }
      held.said.push_back(Said { by: self.by.clone(), about: about.to_owned(), spoken });
      held.waker.take()
    };
    if let Some(waker) = waker {
      waker.wake();
    }
  }

  /// The work of this ear about an act says nothing more: what it said and is not yet heard is dropped, and so is
  /// what it says later. An ear that ended the work of an act at a control says so, so that no second done comes.
  pub fn hush(&self, about: &str) {
    let Ok(mut held) = self.held.lock() else { return };
    let by = self.by.clone();
    held.said.retain(|one| one.by != by || one.about != about);
    held.hushed.insert((by, about.to_owned()));
  }

  /// Everything said and not yet heard, and the waker to wake when more is said.
  pub(crate) fn drained(&self, waker: &Waker) -> Vec<Said> {
    let Ok(mut held) = self.held.lock() else { return Vec::new() };
    held.waker = Some(waker.clone());
    held.said.drain(..).collect()
  }
}

impl std::fmt::Debug for Voice {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_tuple("Voice").field(&self.by).finish()
  }
}
