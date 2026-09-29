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
//! [`ear`] makes one from an async body: [`hear`] and [`say`] read like the ears of the engine. The work an ear
//! begins, a command, a wait, a call of a model, is a stream the ear gives its [`Co`] with [`Co::work`], and the ear
//! takes from [`Co::next`] what it heard and what its work came to, in turn. The life polls the work of every ear
//! each time it is driven, and never while the ear hears, so what a work comes to lands in the log after what the
//! life was doing; and whatever a work waits for wakes whoever drives.

use std::{
  cell::Cell,
  collections::VecDeque,
  future::{Future, poll_fn},
  pin::Pin,
  ptr::NonNull,
  sync::OnceLock,
  task::{Context, Poll},
  thread,
};

use futures::{
  FutureExt, Stream, StreamExt,
  stream::{LocalBoxStream, SelectAll},
};
use tokio::runtime::Handle;
use unsync::spsc;

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

/// An ear: one generator of the life, resumed with what it heard, and polled for what it says of its own accord.
pub trait Ear {
  /// What the ear does with what it heard. Whatever it waits for then wakes the waker of `cx`.
  fn resume(&mut self, heard: Heard, cx: &mut Context<'_>) -> Step;

  /// What the ear says of its own accord, when its work came to something: a saying, or a verb. The ear heard every
  /// fact before it spoke, and whoever spoke hears first what it said, so the next fact a resume gives after a
  /// saying is the one the bus made of it. A generator of a host says nothing but at a resume.
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

/// What an ear of rust takes next: a fact it heard, or what one of its works came to.
#[derive(Debug)]
pub enum Next<W> {
  Heard(Fact),
  Worked(W),
}

/// What the coroutine of an ear tells its body: a fact heard, the fact given back for what the body said, or that
/// the life drives the ear, which lets its works go on until it hears again.
enum Event {
  Heard(Fact),
  Given(Fact),
  Driven,
}

/// What the body of an ear of rust hears, says and works through, which the body owns: what the coroutine tells it,
/// what it says to the coroutine, its works, the facts it heard and has not taken, whether the life drives it, and
/// whether it began a work while it heard. `W` is what each of its works gives.
pub struct Co<W = ()> {
  events: spsc::Receiver<Event>,
  spoken: spsc::Sender<Spoken>,
  works: SelectAll<LocalBoxStream<'static, W>>,
  heard: VecDeque<Fact>,
  driven: bool,
  began: bool,
}

impl<W: 'static> Co<W> {
  /// One work of the ear, whose every item [`Co::next`] gives, and which ends at the end of its stream. A work goes
  /// on only while the life drives the ear, never while the ear hears; one begun while the ear hears wakes whoever
  /// drives, so it begins.
  pub fn work(&mut self, work: impl Stream<Item = W> + 'static) {
    self.began |= !self.driven;
    self.works.push(work.boxed_local());
  }

  /// What the ear takes next: the next fact it heard, and when it heard all, what one of its works came to, while
  /// the life drives it.
  pub async fn next(&mut self) -> Next<W> {
    poll_fn(|cx| {
      self.told();
      if let Some(fact) = self.heard.pop_front() {
        return Poll::Ready(Next::Heard(fact));
      }
      if std::mem::take(&mut self.began) {
        cx.waker().wake_by_ref();
      }
      if !self.driven {
        return Poll::Pending;
      }
      match self.works.poll_next_unpin(cx) {
        Poll::Ready(Some(worked)) => Poll::Ready(Next::Worked(worked)),
        _ => Poll::Pending,
      }
    })
    .await
  }

  /// What the coroutine told since, taken. The coroutine polls the body each time it tells it something, so the body
  /// takes what it was told then, and waits for no event.
  fn told(&mut self) -> Option<Fact> {
    let mut given = None;
    while let Some(Some(event)) = self.events.recv().now_or_never() {
      given = self.took(event).or(given);
    }
    given
  }

  /// One event, taken: a fact heard waits to be taken, and the life drives the ear until it hears again.
  fn took(&mut self, event: Event) -> Option<Fact> {
    match event {
      Event::Heard(fact) => {
        self.driven = false;
        self.heard.push_back(fact);
      }
      Event::Given(fact) => return Some(fact),
      Event::Driven => self.driven = true,
    }
    None
  }

  /// What the ear says or tells, to the coroutine.
  fn spoke(&mut self, spoken: Spoken) {
    let _ = self.spoken.try_send(spoken);
  }
}

type Body = Pin<Box<dyn Future<Output = Result<(), Fault>>>>;

/// An ear of rust: its body, what it tells the body, what the body says, how the body ended, which the next resume
/// gives, and what the next fact a resume gives is: the one given back for what the ear answered, or its own fact as
/// the log gives it, after it said something of its own accord.
struct Coroutine {
  body: Option<Body>,
  events: spsc::Sender<Event>,
  spoken: spsc::Receiver<Spoken>,
  ended: Option<Step>,
  next: Given,
}

/// What the next fact that a resume gives is, after what the ear said.
#[derive(Clone, Copy, PartialEq)]
enum Given {
  /// The next fact of the log.
  Heard,
  /// The fact the bus made of what the ear answered, given back, which the log gives the ear later too.
  Back,
  /// The fact the bus made of what the ear said of its own accord, which the log gives the ear once: it is both what
  /// the saying gives and a fact the ear hears.
  Both,
}

impl Ear for Coroutine {
  fn resume(&mut self, heard: Heard, cx: &mut Context<'_>) -> Step {
    if let Heard::Fact(fact) = heard {
      match std::mem::replace(&mut self.next, Given::Heard) {
        Given::Heard => self.tell(Event::Heard(fact)),
        Given::Back => self.tell(Event::Given(fact)),
        Given::Both => {
          self.tell(Event::Given(fact.clone()));
          self.tell(Event::Heard(fact));
        }
      }
    }
    let step = self.heard(cx);
    if matches!(step, Step::Say(_)) {
      self.next = Given::Back;
    }
    step
  }

  fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Spoken> {
    self.tell(Event::Driven);
    let spoken = self.stepped(cx);
    if let Poll::Ready(Spoken::Saying(_)) = spoken {
      self.next = Given::Both;
    }
    spoken
  }
}

impl Coroutine {
  /// One event, told to the body.
  fn tell(&mut self, event: Event) {
    let _ = self.events.try_send(event);
  }

  /// What the ear does with what it heard: what it says, each verb it tells said at once, until it waits.
  fn heard(&mut self, cx: &mut Context<'_>) -> Step {
    loop {
      match self.stepped(cx) {
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

  /// What the body said and the coroutine has not taken, or what it says when it is polled once; and how it ended
  /// when it ended.
  fn stepped(&mut self, cx: &mut Context<'_>) -> Poll<Spoken> {
    if let Some(Some(spoken)) = self.spoken.recv().now_or_never() {
      return Poll::Ready(spoken);
    }
    let Some(body) = self.body.as_mut() else { return Poll::Pending };
    if let Poll::Ready(done) = body.as_mut().poll(cx) {
      self.body = None;
      self.ended = Some(done.map_or_else(Step::Raised, |()| Step::Over));
    }
    match self.spoken.recv().now_or_never() {
      Some(Some(spoken)) => Poll::Ready(spoken),
      _ => Poll::Pending,
    }
  }
}

/// An ear of rust from its body: an async block given what it hears, says and works through, which runs until the
/// ear is over, and whose end is the end of the ear.
pub fn ear<W, F, B>(body: B) -> Box<dyn Ear>
where
  W: 'static,
  B: FnOnce(Co<W>) -> F,
  F: Future<Output = Result<(), Fault>> + 'static,
{
  let (told, events) = spsc::unbounded();
  let (spoken, said) = spsc::unbounded();
  let co = Co {
    events,
    spoken,
    works: SelectAll::new(),
    heard: VecDeque::new(),
    driven: false,
    began: false,
  };
  let body = Box::pin(body(co));
  Box::new(Coroutine {
    body: Some(body),
    events: told,
    spoken: said,
    ended: None,
    next: Given::Heard,
  })
}

/// The next fact the ear hears, which is `a = yield` of python, for an ear with no work.
pub async fn hear(co: &mut Co) -> Fact {
  loop {
    if let Next::Heard(fact) = co.next().await {
      return fact;
    }
  }
}

/// One saying, said, and the fact the bus made of it, given back, which is `a = yield saying` of python.
pub async fn say<W: 'static>(co: &mut Co<W>, saying: Fact) -> Fact {
  co.spoke(Spoken::Saying(saying));
  poll_fn(|_| co.told().map_or(Poll::Pending, Poll::Ready)).await
}

/// One verb of the engine, told with its words under the name of the ear, whose value goes nowhere. An ear that
/// speaks of its own accord tells a verb so, since no life hears it then: a control that it must say before a done,
/// such as a pause. While the ear hears, the verb is said at once, and in either case before what the ear says next.
pub fn tell<W: 'static>(
  co: &mut Co<W>,
  verb: &str,
  args: Vec<Object>,
  kwargs: Vec<(&str, Object)>,
) {
  let kwargs = kwargs.into_iter().map(|(key, one)| (key.to_owned(), one)).collect();
  co.spoke(Spoken::Verb(Call { verb: verb.to_owned(), args, kwargs }));
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
