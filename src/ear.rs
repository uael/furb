//! The ear: the one thing a life hears by.
//!
//! The contract lets `boot`, `act` and `drive` take any generator of one shape: it hears every fact of the life
//! and every question that no ear before it took, and it speaks by yielding a saying. The Kernel, the gate, the
//! record and each ear of the World are one, whatever language writes it: a generator of python, a generator of
//! TypeScript, or a coroutine of rust.
//!
//! An ear of rust is an [`Ear`]: it is resumed with what it heard and gives back what it does, which are the steps
//! of a generator of the engine. It says a saying and hears back the fact the bus made of it; it waits and hears
//! the next fact. While it hears, it calls a verb of the engine with [`call`], as an ear of the engine calls one,
//! and the life that hears it answers at once, since the sandbox takes a call of the host while it waits for the
//! host. It tells a verb with [`tell`] when it speaks of its own accord, which the life says in its turn.
//!
//! [`ear`] makes one from an async body: [`hear`] and [`say`] read like the ears of the engine. The body holds the
//! work it begins, a command, a wait, a call of a model, as futures of its own, and selects between what it hears
//! and what its work comes to. The life polls every ear each time it is driven, and whatever an ear waits for wakes
//! whoever drives; an ear that says something then says it as it would in answer to a fact.

use std::{
  cell::{Cell, RefCell},
  collections::VecDeque,
  future::{Future, poll_fn},
  pin::Pin,
  ptr::NonNull,
  rc::Rc,
  sync::OnceLock,
  task::{Context, Poll},
  thread,
};

use tokio::runtime::Handle;

use crate::{
  fact::Fact,
  value::{Fault, Object},
};

/// What an ear is resumed with.
#[derive(Debug)]
pub enum Heard {
  /// Its birth, which starts it.
  Born,
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

/// An ear: one generator of the life, resumed with what it heard, and polled for what it says on its own.
pub trait Ear {
  /// What the ear does with what it heard. Whatever it waits for then wakes the waker of `cx`.
  fn resume(&mut self, heard: Heard, cx: &mut Context<'_>) -> Step;

  /// What the ear says of its own accord, when its work came to something: a saying, which the bus makes whole and
  /// hands back as the next fact the ear hears, or a verb. A generator of a host says nothing but at a resume.
  fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Spoken> {
    let _ = cx;
    Poll::Pending
  }
}

/// What an ear says of its own accord: a saying, or a verb of the engine with its words, whose value goes nowhere.
#[derive(Debug)]
pub enum Spoken {
  Saying(Fact),
  Verb(Call),
}

/// What a coroutine of rust and its body share: the facts it heard and has not taken, and what it yields.
#[derive(Default)]
struct Mailbox {
  heard: VecDeque<Fact>,
  spoken: Option<Spoken>,
}

/// What the body of an ear of rust hears and says through.
#[derive(Clone, Default)]
pub struct Co(Rc<RefCell<Mailbox>>);

type Body = Pin<Box<dyn Future<Output = Result<(), Fault>>>>;

/// An ear of rust: its body, polled when it hears and when the life is driven, and how it ended, which the next
/// resume gives.
struct Coroutine {
  co: Co,
  body: Option<Body>,
  ended: Option<Step>,
}

impl Ear for Coroutine {
  fn resume(&mut self, heard: Heard, cx: &mut Context<'_>) -> Step {
    if let Heard::Fact(fact) = heard {
      self.co.0.borrow_mut().heard.push_back(fact);
    }
    loop {
      match self.poll(cx) {
        Poll::Ready(Spoken::Saying(saying)) => return Step::Say(saying),
        // A verb told while the ear hears is said at once, by the life that hears it.
        Poll::Ready(Spoken::Verb(Call { verb, args, kwargs })) => {
          let kwargs = kwargs.iter().map(|(key, one)| (key.as_str(), one.clone())).collect();
          if let Err(fault) = call(&verb, args, kwargs) {
            (self.body, self.ended) = (None, Some(Step::Over));
            return Step::Raised(fault);
          }
        }
        Poll::Pending => {
          return self.ended.take().map_or(Step::Wait, |ended| {
            self.ended = Some(Step::Over);
            ended
          });
        }
      }
    }
  }

  fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Spoken> {
    let Some(body) = self.body.as_mut() else { return Poll::Pending };
    if let Poll::Ready(done) = body.as_mut().poll(cx) {
      self.body = None;
      self.ended = Some(done.map_or_else(Step::Raised, |()| Step::Over));
    }
    self.co.0.borrow_mut().spoken.take().map_or(Poll::Pending, Poll::Ready)
  }
}

/// An ear of rust from its body: an async block given what it hears and says through, which runs until the ear is
/// over, and whose end is the end of the ear.
pub fn ear<F, B>(body: B) -> Box<dyn Ear>
where
  B: FnOnce(Co) -> F,
  F: Future<Output = Result<(), Fault>> + 'static,
{
  let co = Co::default();
  Box::new(Coroutine { body: Some(Box::pin(body(co.clone()))), co, ended: None })
}

/// The next fact the ear hears, which is `a = yield` of python. It takes no fact until it gives one, so a select
/// may drop it.
pub fn hear(co: &Co) -> impl Future<Output = Fact> + '_ {
  poll_fn(|_| co.0.borrow_mut().heard.pop_front().map_or(Poll::Pending, Poll::Ready))
}

/// One saying, said, and the fact the bus made of it, which is `a = yield saying` of python.
pub async fn say(co: &Co, saying: Fact) -> Fact {
  yielded(co, Spoken::Saying(saying)).await;
  hear(co).await
}

/// One verb of the engine, told with its words under the name of the ear, whose value goes nowhere. An ear that
/// speaks of its own accord tells a verb so, since no life hears it then: a control that it must say before a done,
/// such as a pause. While the ear hears, the verb is said at once.
pub async fn tell(co: &Co, verb: &str, args: Vec<Object>, kwargs: Vec<(&str, Object)>) {
  let kwargs = kwargs.into_iter().map(|(key, one)| (key.to_owned(), one)).collect();
  yielded(co, Spoken::Verb(Call { verb: verb.to_owned(), args, kwargs })).await;
}

/// What the ear yields, taken by what steps it.
async fn yielded(co: &Co, spoken: Spoken) {
  co.0.borrow_mut().spoken = Some(spoken);
  poll_fn(|_| if co.0.borrow().spoken.is_some() { Poll::Pending } else { Poll::Ready(()) }).await;
}

/// What answers a verb that an ear calls while it hears: the life that hears the ear.
pub(crate) type Answers<'a> = dyn FnMut(Call) -> Result<Object, Fault> + 'a;

thread_local! {
  /// What answers the verbs of the ear that a life of this thread hears now, when it hears one, and nothing while one
  /// of those verbs runs.
  static ANSWERS: Cell<Option<NonNull<Answers<'static>>>> = const { Cell::new(None) };
}

/// What `step` gives, with `answers` as what answers each verb that an ear calls while `step` runs, and inside the
/// reactor, which wakes what the ear waits for. What answered before answers again once `step` is over, even when it
/// panics. A life polls an ear inside the reactor alone, since no life hears it then.
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
  let _inside = reactor().enter();
  step()
}

/// Whether a life of this thread hears an ear now, which answers each verb that the ear calls.
pub fn hearing() -> bool {
  ANSWERS.get().is_some()
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

/// The reactor that wakes an ear when what it waits for is ready: a pipe, a timer, a socket. It is one runtime of
/// tokio, driven by a thread of its own for the process, and every ear runs on the thread of its life.
pub fn reactor() -> &'static Handle {
  static REACTOR: OnceLock<Handle> = OnceLock::new();
  REACTOR.get_or_init(|| {
    let runtime = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .expect("the machine gives the ears a reactor");
    let handle = runtime.handle().clone();
    thread::Builder::new()
      .name("furb-reactor".to_owned())
      .spawn(move || runtime.block_on(std::future::pending::<()>()))
      .expect("the machine gives the reactor a thread");
    handle
  })
}
