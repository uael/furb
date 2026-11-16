//! The ear: the one thing a life hears by.
//!
//! The contract lets `boot`, `act` and `drive` take any generator of one shape: it hears the facts of the life
//! that it waits for and every question that no ear before it took, and every chain at its birth,
//! and it speaks by yielding a saying. The Kernel, the record and each ear of the World are one,
//! whatever language writes it: a generator of python, a generator of TypeScript, or a coroutine of
//! rust.
//!
//! An ear of rust is an [`Ear`]: it is resumed with what it heard and gives back what it does, which are the steps
//! of a generator of the engine. It says a saying and hears back the fact the bus made of it; it calls a verb of the
//! engine and hears back what the verb gave; it waits and hears the next fact. The life answers each verb with the
//! ear in its place, so no ear reaches into a life that hears it. Code of a host that calls a verb while it runs,
//! as the ears of the engine do, calls it through the [`Life`] lent to it for as long as it runs.
//!
//! [`ear`] makes one from an async body: [`hear`] and [`say`] read like the ears of the engine.
//! The work an ear begins, a command, a wait, a call of a model, is a stream the ear gives its
//! [`Co`] with [`Co::work`], or with [`Co::take`] when it is the work of an act, which ends at the
//! first done of that act. The ear takes from [`Co::next`] what it heard and what its work came
//! to, in turn, and an act that an earlier life took and did not end, which a wake puts to it
//! again, as [`Next::Resumed`], so each ear says what it does with an act whose work may have run.
//! The life polls the work of every ear each time it is driven, and never while the ear hears, nor
//! while the record lacks an entry that the life said, and it takes one thing that a work says at
//! each poll, so what a work comes to lands in the log alone, after all that the life was doing, as
//! a later life says it again; and whatever a work waits for wakes whoever drives.
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
//! `unquoted` and `paused`, and `pause`, which makes no act; the store calls `prompt` to
//! the operator, and takes no prompt; and the commands, the time and the web call none.
//! The extensions and memory do no work, so they call each verb while they hear, where the life
//! answers it in their place.
#![expect(clippy::indexing_slicing, reason = "debt: an index or a slice that can panic")]

use std::{
  any::Any,
  collections::{HashMap, HashSet, VecDeque},
  future::{Future, poll_fn},
  pin::Pin,
  sync::{
    OnceLock,
    mpsc::{self, Receiver, Sender},
  },
  task::{Context, Poll},
  thread,
};

use futures::{
  Stream, StreamExt,
  stream::{LocalBoxStream, SelectAll},
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use tokio::{
  runtime::Handle,
  task::{AbortHandle, JoinError},
};
use unsync::oneshot;

use crate::{
  engine::Life,
  fact::Fact,
  value::{Fault, Given, Object, ObjectRef},
};
use std::{future, mem};
use tokio::runtime::Builder;

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
  #[cfg_attr(
    not(any(feature = "python", feature = "typescript")),
    expect(dead_code, reason = "only a door calls it")
  )]
  pub(crate) fn of(yielded: &Object) -> Result<Self, Fault> {
    let got = yielded.as_ref();
    if got.is_none() {
      return Ok(Self::Wait(None));
    }
    if matches!(got.type_name(), "set" | "frozenset") {
      return Ok(Self::Wait(Some(yielded.clone())));
    }
    if let Some(pairs) = got.pairs() {
      let field =
        |key: &str| pairs.iter().find(|(name, _)| name.as_str() == Some(key)).map(|(_, one)| one);
      let verb = field("call").and_then(ObjectRef::as_str).map(str::to_owned);
      let verb = verb
        .ok_or_else(|| Fault::refused("an ear yields a verb as a map that names it under call"))?;
      let args = field("args").and_then(ObjectRef::items).unwrap_or_default();
      let kwargs = field("kwargs").and_then(ObjectRef::pairs).unwrap_or_default();
      let kwargs =
        kwargs.iter().filter_map(|(key, one)| Some((key.as_str()?.to_owned(), one.to_owned())));
      let args = args.into_iter().map(|one| one.to_owned()).collect();
      return Ok(Self::Call(Call { verb, args, kwargs: kwargs.collect() }));
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
  fn poll(&mut self, _cx: &mut Context<'_>) -> Poll<Spoken> {
    Poll::Pending
  }

  /// What a verb that [`Ear::poll`] gave came to, which the ear takes at its next poll.
  fn answered(&mut self, _got: Result<Object, Fault>) {}

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

  /// How many entries the ear holds that it could not keep yet: the ear that keeps the record
  /// holds each entry that its store refused, as on a full disk, until the store takes it, and
  /// any other ear holds none.
  fn unkept(&self) -> usize {
    0
  }

  /// Whether the ear starts again after it raised: a fresh ear stands in its place and hears from its birth. An ear
  /// that [`lasting`] made does, and any other ear stays over.
  fn restart(&mut self) -> bool {
    false
  }

  /// What a fresh ear of the same make needs to stand where this ear stands, for the image of its
  /// life: the acts that it took and did not end, and what it keeps of the facts that it heard.
  /// Nothing when the ear cannot say it, as a generator of a host, and a life with such an ear
  /// writes no image.
  fn image(&self) -> Option<Value> {
    None
  }

  /// What [`Ear::image`] gave of an ear of an earlier process, given to this fresh ear before it
  /// hears anything: it stands where that ear stood, takes each act that the ear held as an act
  /// that an earlier life took, and reads no fact of the record.
  fn imaged(&mut self, _part: Value) {}
}

/// What an ear adds to the ear that [`With`] wraps: its end of a run, its restart and what it
/// holds unkept, the three methods that a wrapper of an ear gives its own. Each is lent the ear
/// that the wrapper holds, and by default does what that ear does, except `restart`, which leaves
/// the ear over.
pub trait Hooks {
  /// What the wrapper does at the end of each run, as [`Ear::ran`] does.
  fn ran(&mut self, ear: &mut dyn Ear) -> Result<(), Fault> {
    ear.ran()
  }

  /// Whether the wrapper starts again after it raised, as [`Ear::restart`] says: it may put a
  /// fresh ear in the place of `ear`, which hears from its birth.
  fn restart(&mut self, _ear: &mut Box<dyn Ear>) -> bool {
    false
  }

  /// How many entries the wrapper holds unkept, as [`Ear::unkept`] says.
  fn unkept(&self, ear: &dyn Ear) -> usize {
    ear.unkept()
  }
}

/// An ear with hooks: the ear that it wraps hears, speaks and is lent the life as before, and the
/// hooks give the end of each run and the debt.
pub struct With<H> {
  ear: Box<dyn Ear>,
  hooks: H,
}

impl<H: Hooks + 'static> With<H> {
  /// The ear that wraps `ear` with `hooks`.
  pub fn ear(ear: Box<dyn Ear>, hooks: H) -> Box<dyn Ear> {
    Box::new(Self { ear, hooks })
  }
}

impl<H: Hooks> Ear for With<H> {
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
    self.hooks.ran(&mut *self.ear)
  }

  fn restart(&mut self) -> bool {
    self.hooks.restart(&mut self.ear)
  }

  fn unkept(&self) -> usize {
    self.hooks.unkept(&*self.ear)
  }

  fn image(&self) -> Option<Value> {
    self.ear.image()
  }

  fn imaged(&mut self, part: Value) {
    self.ear.imaged(part);
  }
}

/// What an ear says of its own accord: a saying, or a verb of the engine with its words, which the life answers.
#[derive(Debug)]
pub enum Spoken {
  Saying(Fact),
  Verb(Call),
}

/// What an ear of rust takes next: a fact it heard, an act that an earlier life took and did not
/// end, which the journal said started and a wake puts to the ear again, or what one of its works
/// came to.
#[derive(Debug)]
pub enum Next<W> {
  Heard(Fact),
  Resumed(Fact),
  Worked(W),
}

/// What the coroutine of an ear tells its body: a fact heard, the question of an act that an earlier life took and
/// that a wake puts to it again, what a verb it called came to, that the life drives the ear, that the life took one
/// thing that the body said, what the body keeps, handed back before the body runs, and what an image gave of it.
enum Event {
  Heard(Fact),
  Resumed(Fact),
  Answer(Result<Object, Fault>),
  Driven,
  Taken,
  Kept(Box<dyn Kept>),
  Given(Value),
}

/// What the body of an ear gives its coroutine: what it says, with the question whose act a started of it takes,
/// and what it keeps, which the coroutine holds while the body waits.
enum Gave {
  Said(Spoken, Option<Fact>),
  Kept(Box<dyn Kept>),
}

/// What an ear of rust keeps across its steps, beside its acts in flight: its coroutine holds it while the body
/// waits, where the life reads what the image holds of it, ends a run with it and counts what it holds unkept.
pub trait Kept: Any {
  /// What the image of the life holds of it, or nothing when it cannot say it.
  fn image(&self) -> Option<Value>;

  /// What the ear does at the end of each run, as [`Ear::ran`] says.
  fn ran(&mut self) -> Result<(), Fault> {
    Ok(())
  }

  /// How many entries it holds unkept, as [`Ear::unkept`] says.
  fn unkept(&self) -> usize {
    0
  }
}

/// A value that serde writes is kept as its JSON.
impl<T: Serialize + 'static> Kept for T {
  fn image(&self) -> Option<Value> {
    serde_json::to_value(self).ok()
  }
}

/// What the coroutine of an ear of rust holds for the image of its life: the question of each act that the ear
/// started and that is not done, those acts, which are its acts in flight, and the acts that an earlier life took and
/// that no wake put to it yet.
///
/// A question goes to each ear in turn until one owns it, but its done goes only to the ears that wait for its kind
/// or for every fact. So the ear keeps a question only when its body starts the act of the question that it hears:
/// else it would keep each question that another ear answered, for the whole life.
#[derive(Default)]
struct Imaged {
  asked: HashMap<String, Fact>,
  started: Vec<String>,
  resumed: HashSet<String>,
}

impl Imaged {
  /// A fact told to the body: a done forgets its act, a started that the journal says again is of an act that an
  /// earlier life took, and the question of such an act is resumed, which this gives.
  fn told(&mut self, fact: &Fact) -> bool {
    match fact.kind() {
      "done" => {
        self.resumed.remove(fact.about());
        self.asked.remove(fact.about());
        self.started.retain(|one| one != fact.about());
      }
      "started" if fact.by() == "journal" => {
        self.resumed.insert(fact.about().to_owned());
      }
      _ if fact.question() && self.resumed.contains(fact.about()) => {
        self.asked.insert(fact.about().to_owned(), fact.clone());
        return true;
      }
      _ => {}
    }
    false
  }

  /// What the body said: a started makes its act one in flight, with the question that the body heard of it.
  fn said(&mut self, spoken: &Spoken, question: Option<Fact>) {
    if let Spoken::Saying(saying) = spoken
      && saying.kind() == "started"
    {
      if !self.started.iter().any(|one| one == saying.about()) {
        self.started.push(saying.about().to_owned());
      }
      if let Some(question) = question {
        self.asked.insert(question.about().to_owned(), question);
      }
    }
  }
}

/// What the body of an ear of rust hears, says and works through, which the body owns: what the coroutine tells it,
/// what it gives the coroutine, how much of what it said the life has not taken yet, its works, the stop of the work
/// of each act it took, the question that it hears now, the facts it heard and has not taken, the answer to the verb
/// it called, what it keeps while it runs, what an image gave of that, whether the life drives it, and whether it
/// began a work while it heard. `W` is what each of its works gives.
pub struct Co<W = ()> {
  events: Receiver<Event>,
  gave: Sender<Gave>,
  unsaid: usize,
  works: SelectAll<LocalBoxStream<'static, W>>,
  taken: HashMap<String, oneshot::Sender<()>>,
  hearing: Option<Fact>,
  heard: VecDeque<(Fact, bool)>,
  answer: Option<Result<Object, Fault>>,
  kept: Option<Box<dyn Kept>>,
  given: Option<Value>,
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

  /// The work of the act `id`, which ends at the first done of that act, whoever says it, or when
  /// the ear takes the act again with another work: a work whose act a control ended gives
  /// nothing more. The ear says the started of the act itself, once.
  pub fn take(&mut self, id: &str, work: impl Stream<Item = W> + 'static) {
    let (stop, stopped) = oneshot::channel();
    self.taken.insert(id.to_owned(), stop);
    self.work(work.take_until(stopped));
  }

  /// What the ear takes next: the next fact it heard, and when it heard all, what one of its works came to, while
  /// the life drives it and once the life took all that the ear said, so the ear stands here when it lands.
  pub async fn next(&mut self) -> Next<W> {
    poll_fn(|cx| {
      self.told();
      if let Some((fact, resumed)) = self.heard.pop_front() {
        return Poll::Ready(self.noted(fact, resumed));
      }
      if mem::take(&mut self.began) {
        cx.waker().wake_by_ref();
      }
      // The life polls an ear again until it says no more, so the ear goes on once what it said landed.
      if self.driven
        && self.unsaid == 0
        && let Poll::Ready(Some(worked)) = self.works.poll_next_unpin(cx)
      {
        return Poll::Ready(Next::Worked(worked));
      }
      self.waits()
    })
    .await
  }

  /// A fact heard, as the body takes it: a done ends the work of its act, the question of an act that an earlier
  /// life took is resumed, and the body hears any other question until it starts its act or hears the next.
  fn noted(&mut self, fact: Fact, resumed: bool) -> Next<W> {
    if fact.kind() == "done" {
      self.taken.remove(fact.about());
    }
    if resumed {
      return Next::Resumed(fact);
    }
    if fact.question() {
      self.hearing = Some(fact.clone());
    }
    Next::Heard(fact)
  }

  /// What an image of the life gave of what this ear keeps, read once as `T`: nothing for an ear that no image
  /// made.
  pub fn given<T: DeserializeOwned>(&mut self) -> Result<Option<T>, Fault> {
    self.told();
    let read = |given| serde_json::from_value(given);
    let fault = |no: serde_json::Error| {
      Fault::refused(format!("the image holds another state of the ear: {no}"))
    };
    self.given.take().map(read).transpose().map_err(fault)
  }

  /// What the ear keeps from now on: what an image of the life gave of it to this fresh ear, or else `first`.
  pub fn keeps<K: Kept + DeserializeOwned>(&mut self, first: K) -> Result<(), Fault> {
    let kept = self.given()?.unwrap_or(first);
    self.keep(kept);
    Ok(())
  }

  /// What the ear keeps, taken while it steps, which [`Co::keep`] gives back before it waits again. The image of a
  /// life that reads it meanwhile reads the empty value in its place.
  pub fn held<K: Kept + Default>(&mut self) -> Result<K, Fault> {
    self.kept().map(mem::take)
  }

  /// What the ear keeps from now on, which its coroutine holds while it waits, where the life reads it between two
  /// runs.
  pub fn keep<K: Kept>(&mut self, kept: K) {
    self.kept = Some(Box::new(kept));
  }

  /// What the ear keeps, while it runs, as the value that it gave [`Co::keep`].
  pub fn kept<K: Kept>(&mut self) -> Result<&mut K, Fault> {
    self.told();
    let kept: Option<&mut dyn Any> = self.kept.as_deref_mut().map(|one| one as &mut dyn Any);
    let kept = kept.ok_or_else(|| Fault::refused("the ear keeps nothing yet"))?;
    kept.downcast_mut().ok_or_else(|| Fault::refused("the ear keeps a value of another type"))
  }

  /// What the coroutine told since, taken: the facts wait to be taken, and the life drives the ear until it hears
  /// again.
  fn told(&mut self) {
    while let Ok(event) = self.events.try_recv() {
      match event {
        Event::Heard(fact) => {
          self.driven = false;
          self.heard.push_back((fact, false));
        }
        Event::Resumed(fact) => {
          self.driven = false;
          self.heard.push_back((fact, true));
        }
        Event::Answer(got) => self.answer = Some(got),
        Event::Driven => self.driven = true,
        Event::Taken => self.unsaid = self.unsaid.saturating_sub(1),
        Event::Kept(kept) => self.kept = Some(kept),
        Event::Given(given) => self.given = Some(given),
      }
    }
  }

  /// The body waits: what it keeps goes to its coroutine until it runs again.
  fn waits<T>(&mut self) -> Poll<T> {
    if let Some(kept) = self.kept.take() {
      self.gives(Gave::Kept(kept));
    }
    Poll::Pending
  }

  /// One thing given to the coroutine, which holds the ear and its body, so it stands while the body runs.
  fn gives(&self, gave: Gave) {
    self.gave.send(gave).ok();
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
    poll_fn(|_| {
      self.told();
      if self.unsaid > 0 { self.waits() } else { Poll::Ready(()) }
    })
    .await;
    let kwargs = kwargs.into_iter().map(|(key, one)| (key.to_owned(), one)).collect();
    self.spoke(Spoken::Verb(Call { verb: verb.to_owned(), args, kwargs }));
    poll_fn(|_| {
      self.told();
      self.answer.take().map_or_else(|| self.waits(), Poll::Ready)
    })
    .await
  }

  /// What the ear says or tells, to the coroutine, which counts it until it gives it to the life. A started takes
  /// the question that the ear hears, when it is of the same act.
  fn spoke(&mut self, spoken: Spoken) {
    let question = match &spoken {
      Spoken::Saying(saying) if saying.kind() == "started" => {
        self.hearing.take_if(|one| one.about() == saying.about())
      }
      _ => None,
    };
    self.unsaid += 1;
    self.gives(Gave::Said(spoken, question));
  }
}

/// A body that ends gives back what it kept, which the image of the life still reads.
impl<W> Drop for Co<W> {
  fn drop(&mut self) {
    if let Some(kept) = self.kept.take() {
      self.gave.send(Gave::Kept(kept)).ok();
    }
  }
}

type Body = Pin<Box<dyn Future<Output = Result<(), Fault>>>>;

/// An ear of rust: its body, what it tells the body and what the body gives it, what the body said and the life has
/// not taken, what its work said past the first thing of a poll, what the verb its work called came to, which the
/// body takes at the next poll, how the body ended, which the next resume gives, whether the next fact a resume
/// gives is the one the bus made of what the ear said at a resume, which goes nowhere, since the log gives the ear
/// that fact later too, what it holds for the image, and what the body keeps while it waits, once it keeps a thing.
struct Coroutine {
  body: Option<Body>,
  /// The names and kinds the ear waits for, as a set, or nothing for every fact.
  hears: Option<Object>,
  events: Sender<Event>,
  gave: Receiver<Gave>,
  spoken: VecDeque<Spoken>,
  accord: VecDeque<Spoken>,
  answer: Option<Result<Object, Fault>>,
  ended: Option<Step>,
  given: bool,
  imaged: Imaged,
  kept: Option<Box<dyn Kept>>,
  keeps: bool,
}

impl Ear for Coroutine {
  fn resume(&mut self, heard: Heard, cx: &mut Context<'_>) -> Step {
    if let Heard::Answer(got) = heard {
      self.tell(Event::Answer(got));
    } else if let Heard::Fact(fact) = heard
      && !mem::take(&mut self.given)
    {
      let event = if self.imaged.told(&fact) { Event::Resumed(fact) } else { Event::Heard(fact) };
      self.tell(event);
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
    self.accord.extend(self.spoken.drain(..));
    got
  }

  fn answered(&mut self, got: Result<Object, Fault>) {
    self.answer = Some(got);
  }

  fn ran(&mut self) -> Result<(), Fault> {
    self.kept.as_deref_mut().map_or(Ok(()), Kept::ran)
  }

  fn unkept(&self) -> usize {
    self.kept.as_deref().map_or(0, Kept::unkept)
  }

  /// Each act in flight goes out as the question that the ear heard, with what the body keeps; nothing while the
  /// body holds what it keeps.
  fn image(&self) -> Option<Value> {
    let imaged = &self.imaged;
    let asked = imaged.started.iter().filter_map(|id| imaged.asked.get(id));
    let flight =
      asked.map(|fact| serde_json::to_value(&fact.0).ok()).collect::<Option<Vec<_>>>()?;
    let resumed: Vec<&String> = imaged.resumed.iter().collect();
    let kept = match self.kept.as_deref() {
      Some(kept) => kept.image()?,
      None if self.keeps => return None,
      None => Value::Null,
    };
    Some(json!({ "flight": flight, "resumed": resumed, "kept": kept }))
  }

  /// The body is not polled yet, so it finds its state where it asks for what it keeps. It holds
  /// each act in flight as the ear before it did, each an act that an earlier life took, which a
  /// wake puts to it again, as [`Engine::flight`] says.
  fn imaged(&mut self, mut part: Value) {
    let names = |part: &Value| -> Vec<String> {
      let held = part.as_array().into_iter().flatten();
      held.filter_map(|one| one.as_str().map(str::to_owned)).collect()
    };
    let imaged = &mut self.imaged;
    imaged.resumed.extend(names(&part["resumed"]));
    let flight = part["flight"].take();
    let flight = serde_json::from_value::<Vec<Object>>(flight).unwrap_or_default();
    for fact in flight.into_iter().filter_map(Fact::owned) {
      imaged.resumed.insert(fact.about().to_owned());
      imaged.started.push(fact.about().to_owned());
      imaged.asked.insert(fact.about().to_owned(), fact);
    }
    let kept = part["kept"].take();
    if !kept.is_null() {
      self.tell(Event::Given(kept));
    }
  }
}

impl Coroutine {
  /// One event, told to the body, which takes it at its next poll. A body that ended takes nothing more.
  fn tell(&self, event: Event) {
    self.events.send(event).ok();
  }

  /// What the body said, given to the life, which says it before it polls the body again.
  fn given(&self, spoken: Spoken) -> Spoken {
    self.tell(Event::Taken);
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
  /// when it ended. What the body keeps goes back to it before it runs.
  fn stepped(&mut self, cx: &mut Context<'_>) -> Poll<Spoken> {
    if let Some(spoken) = self.spoken.pop_front() {
      return Poll::Ready(spoken);
    }
    let Some(body) = self.body.as_mut() else { return Poll::Pending };
    if let Some(kept) = self.kept.take() {
      self.events.send(Event::Kept(kept)).ok();
    }
    // What the body waits for, a pipe, a timer, a socket, the runtime of the thread wakes, or the reactor when the
    // thread runs none.
    let polled = {
      let _inside = Handle::try_current().is_err().then(|| reactor().enter());
      body.as_mut().poll(cx)
    };
    if let Poll::Ready(done) = polled {
      self.body = None;
      self.ended = Some(done.map_or_else(Step::Raised, |()| Step::Over));
    }
    while let Ok(gave) = self.gave.try_recv() {
      match gave {
        Gave::Said(spoken, question) => {
          self.imaged.said(&spoken, question);
          self.spoken.push_back(spoken);
        }
        Gave::Kept(kept) => (self.kept, self.keeps) = (Some(kept), true),
      }
    }
    self.spoken.pop_front().map_or(Poll::Pending, Poll::Ready)
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
  With::ear(first, Lasting(Box::new(make)))
}

/// How the ear that [`lasting`] makes gets its next ear: after each raise it starts again from a
/// fresh one.
struct Lasting(Box<dyn FnMut() -> Box<dyn Ear>>);

impl Hooks for Lasting {
  fn restart(&mut self, ear: &mut Box<dyn Ear>) -> bool {
    *ear = (self.0)();
    true
  }
}

/// An ear of rust from its body and what it waits for.
fn coroutine<W, F, B>(hears: Option<Object>, body: B) -> Box<dyn Ear>
where
  W: 'static,
  B: FnOnce(Co<W>) -> F,
  F: Future<Output = Result<(), Fault>> + 'static,
{
  let ((events, told), (gives, gave)) = (mpsc::channel(), mpsc::channel());
  let co = Co {
    events: told,
    gave: gives,
    unsaid: 0,
    works: SelectAll::new(),
    taken: HashMap::new(),
    hearing: None,
    heard: VecDeque::new(),
    answer: None,
    kept: None,
    given: None,
    driven: false,
    began: false,
  };
  let body = Box::pin(body(co));
  Box::new(Coroutine {
    body: Some(body),
    hears,
    events,
    gave,
    spoken: VecDeque::new(),
    accord: VecDeque::new(),
    answer: None,
    ended: None,
    given: false,
    imaged: Imaged::default(),
    kept: None,
    keeps: false,
  })
}

/// The next fact the ear hears, which is `a = yield` of python, for an ear with no work, which
/// takes an act that an earlier life took as any act.
pub async fn hear(co: &mut Co) -> Fact {
  loop {
    if let Next::Heard(fact) | Next::Resumed(fact) = co.next().await {
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

/// The runtime that wakes an ear when what it waits for is ready, a pipe, a timer, a socket, and that its work runs
/// on: the runtime of the thread that drives the life, or the reactor when that thread runs none.
pub fn runtime() -> Handle {
  Handle::try_current().unwrap_or_else(|_| reactor().clone())
}

/// A future as a task of the [`runtime`], so it goes on while the life drives no ear, and which
/// ends when what awaits it drops it, as the done of the act whose work awaits it does.
pub(crate) fn spawned<T: Send + 'static>(
  work: impl Future<Output = T> + Send + 'static,
) -> impl Future<Output = Result<T, JoinError>> {
  /// The task, which ends when its handle goes.
  struct Aborted(AbortHandle);
  impl Drop for Aborted {
    fn drop(&mut self) {
      self.0.abort();
    }
  }
  let task = runtime().spawn(work);
  let aborted = Aborted(task.abort_handle());
  async move {
    let got = task.await;
    drop(aborted);
    got
  }
}

/// The reactor of the process, for a door that drives a life with no runtime of tokio: one runtime, driven by a
/// thread of its own, while every ear runs on the thread of its life.
#[expect(
  clippy::expect_used,
  reason = "no life runs without a reactor, and the machine gives the process one or none"
)]
#[expect(
  clippy::disallowed_methods,
  reason = "the thread of the reactor blocks on the runtime that it drives, which waits for nothing"
)]
pub fn reactor() -> &'static Handle {
  static REACTOR: OnceLock<Handle> = OnceLock::new();
  REACTOR.get_or_init(|| {
    let runtime = Builder::new_current_thread()
      .enable_all()
      .build()
      .expect("the machine gives the ears a reactor");
    let handle = runtime.handle().clone();
    thread::Builder::new()
      .name("furb-reactor".to_owned())
      .spawn(move || runtime.block_on(future::pending::<()>()))
      .expect("the machine gives the reactor a thread");
    handle
  })
}
