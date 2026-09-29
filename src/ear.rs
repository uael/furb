//! The ear: the one thing a life hears by.
//!
//! The contract lets `boot`, `act` and `drive` take any generator of one shape: it hears every fact of the life
//! and every question that no ear before it took, and it speaks by yielding a saying. The Kernel, the gate, the
//! record and each ear of the World are one, whatever language writes it: a generator of python, a generator of
//! TypeScript, or a coroutine of rust.
//!
//! An ear of rust is an [`Ear`]: it is resumed with what it heard and gives back what it does, which are the steps
//! of a generator of the engine. It says a saying and hears back the fact the bus made of it; it calls a verb of the
//! engine and hears back what the verb gave; it waits and hears the next fact. The life answers each verb with the
//! ear in its place, so no ear reaches into a life that hears it. Code of a host that calls a verb while it runs,
//! as the ears of the engine do, calls it through the [`Life`] lent to it for as long as it runs.
//!
//! [`ear`] makes one from an async body: [`hear`] and [`say`] read like the ears of the engine. The work an ear
//! begins, a command, a wait, a call of a model, is a stream the ear gives its [`Co`] with [`Co::work`], and the ear
//! takes from [`Co::next`] what it heard and what its work came to, in turn. The life polls the work of every ear
//! each time it is driven, and never while the ear hears, so what a work comes to lands in the log after what the
//! life was doing; and whatever a work waits for wakes whoever drives.

use std::{
  collections::VecDeque,
  future::{Future, poll_fn},
  pin::Pin,
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
  engine::Life,
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
  /// What the verb the ear called gave, or what it raised.
  Answer(Result<Object, Fault>),
}

/// What an ear does with what it heard.
#[derive(Debug)]
pub enum Step {
  /// One saying, which the bus makes whole and gives back as the next fact the ear hears.
  Say(Fact),
  /// One verb of the engine, which the life says under the name of the ear and answers as the next thing it hears.
  Call(Call),
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
  /// What a generator of a host yielded, as a step: nothing is a wait, a map that names a verb under `call` is that
  /// verb, with its words under `args` and `kwargs`, and a list or a tuple is a saying.
  #[cfg_attr(not(any(feature = "python", feature = "typescript")), allow(dead_code))]
  pub(crate) fn of(yielded: &Object) -> Result<Step, Fault> {
    let got = yielded.as_ref();
    if got.type_name() == "NoneType" {
      return Ok(Step::Wait);
    }
    if let Some(pairs) = got.pairs() {
      let field =
        |key: &str| pairs.iter().find(|(name, _)| name.as_str() == Some(key)).map(|(_, one)| one);
      let verb = field("call").and_then(|one| one.as_str()).map(str::to_owned);
      let verb = verb
        .ok_or_else(|| Fault::refused("an ear yields a verb as a map that names it under call"))?;
      let args = field("args").and_then(|one| one.items()).unwrap_or_default();
      let kwargs = field("kwargs").and_then(|one| one.pairs()).unwrap_or_default();
      let kwargs =
        kwargs.iter().filter_map(|(key, one)| Some((key.as_str()?.to_owned(), one.to_owned())));
      let args = args.into_iter().map(|one| one.to_owned()).collect();
      return Ok(Step::Call(Call { verb, args, kwargs: kwargs.collect() }));
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
  /// fact before it spoke, and it hears its saying as every ear does. A generator of a host says nothing but at a
  /// resume.
  fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Spoken> {
    let _ = cx;
    Poll::Pending
  }

  /// What a verb that [`Ear::poll`] gave came to, which the ear takes at its next poll.
  fn answered(&mut self, got: Result<Object, Fault>) {
    let _ = got;
  }

  /// What the ear does with what it heard, as [`Ear::resume`] does, with the life lent to it until it is done: code
  /// of a host that calls a verb of the life while it runs calls it through the life. The life comes back with the
  /// step. An ear of rust calls a verb by yielding it, and needs no life.
  fn lent(&mut self, heard: Heard, cx: &mut Context<'_>, life: Life) -> (Step, Life) {
    (self.resume(heard, cx), life)
  }
}

/// What an ear says of its own accord: a saying, or a verb of the engine with its words, which the life answers.
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

/// What the coroutine of an ear tells its body: a fact heard, what a verb it called came to, or that the life drives
/// the ear, which lets its works go on until it hears again.
enum Event {
  Heard(Fact),
  Answer(Result<Object, Fault>),
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

  /// What the coroutine told since, taken, and what the verb the body called last came to, when it came. The coroutine
  /// polls the body each time it tells it something, so the body takes what it was told then, and waits for no event.
  fn told(&mut self) -> Option<Result<Object, Fault>> {
    let mut back = None;
    while let Some(Some(event)) = self.events.recv().now_or_never() {
      back = self.took(event).or(back);
    }
    back
  }

  /// One verb of the engine, called with its words under the name of the ear, and what it gave or raised: `a =
  /// verb(...)` of python.
  pub async fn call(
    &mut self,
    verb: &str,
    args: Vec<Object>,
    kwargs: Vec<(&str, Object)>,
  ) -> Result<Object, Fault> {
    let kwargs = kwargs.into_iter().map(|(key, one)| (key.to_owned(), one)).collect();
    self.spoke(Spoken::Verb(Call { verb: verb.to_owned(), args, kwargs }));
    poll_fn(|_| self.told().map_or(Poll::Pending, Poll::Ready)).await
  }

  /// One event, taken: a fact heard waits to be taken, and the life drives the ear until it hears again.
  fn took(&mut self, event: Event) -> Option<Result<Object, Fault>> {
    match event {
      Event::Heard(fact) => {
        self.driven = false;
        self.heard.push_back(fact);
      }
      Event::Answer(got) => return Some(got),
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
/// gives, and whether the next fact a resume gives is the one the bus made of what the ear said at a resume, which
/// goes nowhere, since the log gives the ear that fact later too.
struct Coroutine {
  body: Option<Body>,
  events: spsc::Sender<Event>,
  spoken: spsc::Receiver<Spoken>,
  ended: Option<Step>,
  given: bool,
}

impl Ear for Coroutine {
  fn resume(&mut self, heard: Heard, cx: &mut Context<'_>) -> Step {
    if let Heard::Answer(got) = heard {
      self.tell(Event::Answer(got));
    } else if let Heard::Fact(fact) = heard
      && !std::mem::take(&mut self.given)
    {
      self.tell(Event::Heard(fact));
    }
    let step = self.heard(cx);
    self.given = matches!(step, Step::Say(_));
    step
  }

  fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Spoken> {
    self.tell(Event::Driven);
    self.stepped(cx)
  }

  fn answered(&mut self, got: Result<Object, Fault>) {
    self.tell(Event::Answer(got));
  }
}

impl Coroutine {
  /// One event, told to the body.
  fn tell(&mut self, event: Event) {
    let _ = self.events.try_send(event);
  }

  /// What the ear does with what it heard: a saying, a verb, or a wait, or how it ended.
  fn heard(&mut self, cx: &mut Context<'_>) -> Step {
    match self.stepped(cx) {
      Poll::Ready(Spoken::Saying(saying)) => Step::Say(saying),
      Poll::Ready(Spoken::Verb(call)) => Step::Call(call),
      Poll::Pending => self.ended.take().map_or(Step::Wait, |ended| {
        self.ended = Some(Step::Over);
        ended
      }),
    }
  }

  /// What the body said and the coroutine has not taken, or what it says when it is polled once; and how it ended
  /// when it ended.
  fn stepped(&mut self, cx: &mut Context<'_>) -> Poll<Spoken> {
    if let Some(Some(spoken)) = self.spoken.recv().now_or_never() {
      return Poll::Ready(spoken);
    }
    let Some(body) = self.body.as_mut() else { return Poll::Pending };
    // What the body waits for, a pipe, a timer, a socket, the reactor wakes.
    let polled = {
      let _inside = reactor().enter();
      body.as_mut().poll(cx)
    };
    if let Poll::Ready(done) = polled {
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
  Box::new(Coroutine { body: Some(body), events: told, spoken: said, ended: None, given: false })
}

/// The next fact the ear hears, which is `a = yield` of python, for an ear with no work.
pub async fn hear(co: &mut Co) -> Fact {
  loop {
    if let Next::Heard(fact) = co.next().await {
      return fact;
    }
  }
}

/// One saying, said in its turn after what the ear said before. The ear hears it as every ear does, in the order of
/// the log, and it goes on hearing while the life says it.
pub fn say<W: 'static>(co: &mut Co<W>, saying: Fact) {
  co.spoke(Spoken::Saying(saying));
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
