//! The ear: the one thing a life hears by.
//!
//! The contract lets `boot`, `act` and `drive` take any generator of one shape: it hears the facts of the life
//! that it waits for and every question that no ear before it took, and every chain at its birth, and it speaks by yielding a saying. The Kernel, the gate, the
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
//! each time it is driven, and never while the ear hears, and it takes one thing that a work says at each poll, so
//! what a work comes to lands in the log alone, after all that the life was doing, as a later life says it again;
//! and whatever a work waits for wakes whoever drives.
//!
//! What an ear says or calls of its own accord, from what its work came to, the life says while no ear hears, so
//! the facts that it makes reach the ears at once, this ear among them. The body of an ear takes nothing more of its
//! work and calls no verb until the life took all that it said before, so what it says of its own accord lands
//! while it stands at its next yield, where it takes each act that the saying makes. A verb is otherwise: while the
//! life answers it, the body waits in that call and takes no act, and the life refuses an act that no ear takes. So
//! the law of an ear of the host is this: it calls of its own accord no verb whose hearing can reach an act that it
//! takes itself, and it says a fact in its place, which the ear that does that work hears from the log. The provider
//! says each statement that a model finished as the fact writing, which the Kernel hears and runs, and not as an
//! act that it awaits, since a statement that prompts makes a chain ask the provider for a reply.
//! Each verb that an ear of the crate calls of its own accord keeps the law: the provider calls the views
//! `unquoted` and `paused`, the gate through `against`, and `pause`, which makes no act; the store calls `prompt` to
//! the operator, and takes no prompt; and the commands, the time and the web call none. The extensions, memory and
//! compaction do no work, so they call each verb while they hear, where the life answers it in their place.

use std::{
  cell::Cell,
  collections::VecDeque,
  future::{Future, poll_fn},
  pin::Pin,
  rc::Rc,
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
  /// Nothing more: the ear waits for the next fact about one of the names and kinds it gives, as a set, or for the
  /// next fact of any kind when it gives none.
  Wait(Option<Object>),
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
      return Ok(Step::Wait(None));
    }
    if matches!(got.type_name(), "set" | "frozenset") {
      return Ok(Step::Wait(Some(yielded.clone())));
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

  /// What the ear does at the end of each run of the sandbox, once the run said all that it says: the ear that keeps
  /// the record syncs its store there. A fault that it gives is what the run gives, when the run raised nothing.
  fn ran(&mut self) -> Result<(), Fault> {
    Ok(())
  }

  /// Whether the ear starts again after it raised: a fresh ear stands in its place and hears from its birth. An ear
  /// that [`lasting`] made does, and any other ear stays over.
  fn again(&mut self) -> bool {
    false
  }

  /// Whether the ear holds a fact that it writes beside the record and could not write yet, as on a full disk: a
  /// snapshot of the life then would hold more than the files that it covers, so the life writes none.
  fn behind(&self) -> bool {
    false
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
/// what it says to the coroutine, how much of that the coroutine has not given the life yet, its works, the facts it
/// heard and has not taken, whether the life drives it, and whether it began a work while it heard. `W` is what each
/// of its works gives.
pub struct Co<W = ()> {
  events: spsc::Receiver<Event>,
  spoken: spsc::Sender<Spoken>,
  unsaid: Rc<Cell<usize>>,
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
  /// the life drives it and once the life took all that the ear said, so the ear stands here when it lands.
  pub async fn next(&mut self) -> Next<W> {
    poll_fn(|cx| {
      self.told();
      if let Some(fact) = self.heard.pop_front() {
        return Poll::Ready(Next::Heard(fact));
      }
      if std::mem::take(&mut self.began) {
        cx.waker().wake_by_ref();
      }
      // The life polls an ear again until it says no more, so the ear goes on once what it said landed.
      if !self.driven || self.unsaid.get() > 0 {
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
  /// verb(...)` of python. The verb waits until the life took all that the ear said before it, so a saying of its
  /// own accord never lands while the ear waits in a call and cannot take an act that the saying makes.
  pub async fn call(
    &mut self,
    verb: &str,
    args: Vec<Object>,
    kwargs: Vec<(&str, Object)>,
  ) -> Result<Object, Fault> {
    poll_fn(|_| if self.unsaid.get() > 0 { Poll::Pending } else { Poll::Ready(()) }).await;
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

  /// What the ear says or tells, to the coroutine, which counts it until it gives it to the life.
  fn spoke(&mut self, spoken: Spoken) {
    self.unsaid.set(self.unsaid.get() + 1);
    let _ = self.spoken.try_send(spoken);
  }
}

type Body = Pin<Box<dyn Future<Output = Result<(), Fault>>>>;

/// An ear of rust: its body, what it tells the body, what the body says and how much of it the life has not taken,
/// what its work said that the life has not taken yet, what the verb its work called came to, which the body takes
/// at the next poll, how the body ended, which the next resume gives, and whether the next fact a resume gives is the
/// one the bus made of what the ear said at a resume, which goes nowhere, since the log gives the ear that fact later
/// too.
struct Coroutine {
  body: Option<Body>,
  /// The names and kinds the ear waits for, as a set, or nothing for every fact.
  hears: Option<Object>,
  events: spsc::Sender<Event>,
  spoken: spsc::Receiver<Spoken>,
  unsaid: Rc<Cell<usize>>,
  accord: VecDeque<Spoken>,
  answer: Option<Result<Object, Fault>>,
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

  /// What the work of the ear said, one at a time. A body can say more than one thing in one poll, and what it said
  /// past the first waits here for the next poll, so a resume never gives it and the work of an ear never speaks
  /// while the ear hears: each thing it says lands in the log alone, as a later life says it again.
  fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Spoken> {
    self.tell(Event::Driven);
    if let Some(spoken) = self.accord.pop_front() {
      return Poll::Ready(self.given(spoken));
    }
    if let Some(got) = self.answer.take() {
      self.tell(Event::Answer(got));
    }
    let got = self.stepped(cx).map(|spoken| self.given(spoken));
    while let Some(Some(more)) = self.spoken.recv().now_or_never() {
      self.accord.push_back(more);
    }
    got
  }

  fn answered(&mut self, got: Result<Object, Fault>) {
    self.answer = Some(got);
  }
}

impl Coroutine {
  /// One event, told to the body.
  fn tell(&mut self, event: Event) {
    let _ = self.events.try_send(event);
  }

  /// What the body said, given to the life, which says it before it polls the body again.
  fn given(&mut self, spoken: Spoken) -> Spoken {
    self.unsaid.set(self.unsaid.get().saturating_sub(1));
    spoken
  }

  /// What the ear does with what it heard: a saying, a verb, or a wait, or how it ended.
  fn heard(&mut self, cx: &mut Context<'_>) -> Step {
    match self.stepped(cx).map(|spoken| self.given(spoken)) {
      Poll::Ready(Spoken::Saying(saying)) => Step::Say(saying),
      Poll::Ready(Spoken::Verb(call)) => Step::Call(call),
      Poll::Pending => self.ended.take().map_or_else(
        || Step::Wait(self.hears.clone()),
        |ended| {
          self.ended = Some(Step::Over);
          ended
        },
      ),
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
  coroutine(None, body)
}

/// An ear of rust from its body, as [`ear`] makes one, that hears only the facts of these kinds and the facts about
/// an act of these kinds, besides the controls that every ear hears.
pub fn hearing<W, F, B>(kinds: &[&str], body: B) -> Box<dyn Ear>
where
  W: 'static,
  B: FnOnce(Co<W>) -> F,
  F: Future<Output = Result<(), Fault>> + 'static,
{
  coroutine(Some(Object::set(kinds.iter().map(|one| Object::string(*one)))), body)
}

/// An ear that starts again after it raises: first the ear it is given, and after each raise a fresh one that `make`
/// gives, which the life starts in its place, so a fault of one act ends no ear of the crate.
pub fn lasting(first: Box<dyn Ear>, make: impl FnMut() -> Box<dyn Ear> + 'static) -> Box<dyn Ear> {
  Box::new(Lasting { ear: first, make: Box::new(make) })
}

/// The ear that [`lasting`] makes: the ear that hears now, and how to make the next one.
struct Lasting {
  ear: Box<dyn Ear>,
  make: Box<dyn FnMut() -> Box<dyn Ear>>,
}

impl Ear for Lasting {
  fn resume(&mut self, heard: Heard, cx: &mut Context<'_>) -> Step {
    self.ear.resume(heard, cx)
  }

  fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Spoken> {
    self.ear.poll(cx)
  }

  fn answered(&mut self, got: Result<Object, Fault>) {
    self.ear.answered(got);
  }

  fn lent(&mut self, heard: Heard, cx: &mut Context<'_>, life: Life) -> (Step, Life) {
    self.ear.lent(heard, cx, life)
  }

  fn ran(&mut self) -> Result<(), Fault> {
    self.ear.ran()
  }

  fn again(&mut self) -> bool {
    self.ear = (self.make)();
    true
  }

  fn behind(&self) -> bool {
    self.ear.behind()
  }
}

/// An ear of rust from its body and what it waits for.
fn coroutine<W, F, B>(hears: Option<Object>, body: B) -> Box<dyn Ear>
where
  W: 'static,
  B: FnOnce(Co<W>) -> F,
  F: Future<Output = Result<(), Fault>> + 'static,
{
  let (told, events) = spsc::unbounded();
  let (spoken, said) = spsc::unbounded();
  let unsaid = Rc::new(Cell::new(0));
  let co = Co {
    events,
    spoken,
    unsaid: Rc::clone(&unsaid),
    works: SelectAll::new(),
    heard: VecDeque::new(),
    driven: false,
    began: false,
  };
  let body = Box::pin(body(co));
  Box::new(Coroutine {
    body: Some(body),
    hears,
    events: told,
    spoken: said,
    unsaid,
    accord: VecDeque::new(),
    answer: None,
    ended: None,
    given: false,
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

/// One saying, said in its turn after what the ear said before in the same step: a saying of the ear while it hears
/// lands at once, and a saying of its work waits for the next poll, so it lands after what the ear says while it
/// hears. The ear hears it as every ear does, in the order of the log, and it goes on hearing while the life says it.
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
