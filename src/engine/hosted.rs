//! The ears that a host hands a life, the functions it hands it, and how the life steps each ear as
//! a generator of the sandbox and lends itself to the code of the host.

use std::{
  collections::{HashMap, HashSet, VecDeque},
  sync::atomic::{AtomicI64, Ordering},
  task::{Context, Poll, Waker},
  time::{Duration, Instant},
};

use monty_types::{MontyUuid, unstable};

use super::Outside;
use crate::{
  ear::{Call, Ear, Heard, Spoken, Step},
  fact::Fact,
  sand,
  sand::{Answer, Sand},
  value::{Fault, Given, Object, entry, marked},
};
use std::mem;

/// The prefix of the name a function of the host crosses in under: `call:` and its number.
pub(super) const CALL: &str = "call:";

/// The prefix of the name an ear of the host is heard under when a verb is given it: `ear:` and its
/// number.
pub(super) const EAR: &str = "ear:";

/// Who says a fact of the record again, as an ear of the outside hears it.
const JOURNAL: &str = "journal";

/// A function of the host, as the sandbox may call it back, with the life lent to it while it runs.
type Callable = Box<dyn FnMut(Vec<Object>, Life) -> (Result<Object, Fault>, Life)>;

/// The ears and the functions that a host handed over, by the names the sandbox calls them by.
#[derive(Default)]
pub(super) struct Handed {
  pub(super) ears: HashMap<String, Box<dyn Ear>>,
  pub(super) calls: HashMap<String, Callable>,
}

impl Handed {
  /// The ears that a host gives a life, each under its name, and the names in the order it gave
  /// them. A name given twice is refused.
  pub(super) fn of<N: Into<String>>(
    ears: impl IntoIterator<Item = (N, Box<dyn Ear>)>,
  ) -> Result<(Self, Vec<String>), Fault> {
    let mut given = Self::default();
    let mut names = Vec::new();
    for (name, ear) in ears {
      let name = name.into();
      if given.ears.insert(name.clone(), ear).is_some() {
        return Err(Fault::refused(format!("{name} hears")));
      }
      names.push(name);
    }
    Ok((given, names))
  }
}

/// How many ears and functions the hosts of this process handed over, which names the next.
static HANDS: AtomicI64 = AtomicI64::new(1);

/// The ears and the functions of a host that the values of a call name, which [`Engine::hand`]
/// gives the life with the call. The life takes each the first time a value that names it comes
/// into its sandbox, and holds it from then on.
///
/// [`Engine::hand`]: crate::Engine::hand
#[derive(Default)]
pub struct Hands(pub(super) Handed);

impl Hands {
  /// An ear of the host, as the value a verb is given: the object that stands in for it in the
  /// sandbox, which stands at a yield when the host started the ear before it crossed.
  pub fn ear(&mut self, ear: Box<dyn Ear>, started: bool) -> Object {
    let name = format!("{EAR}{}", HANDS.fetch_add(1, Ordering::Relaxed));
    self.0.ears.insert(name.clone(), ear);
    marked("ear", [("name", Object::string(name)), ("started", Object::bool(started))])
  }

  /// A function of the host, as a value a verb may be given: a show, a filter. The sandbox calls
  /// it back, and what it gives is what the call gave.
  pub fn callable(
    &mut self,
    mut call: impl FnMut(Vec<Object>) -> Result<Object, Fault> + 'static,
  ) -> Object {
    self.lending(move |args, life| (call(args), life))
  }

  /// A function of the host that calls verbs of the life, as [`Hands::callable`] gives one: the
  /// life is lent to it while it runs, and it gives the life back with what the call gave.
  pub fn lending(
    &mut self,
    call: impl FnMut(Vec<Object>, Life) -> (Result<Object, Fault>, Life) + 'static,
  ) -> Object {
    let name = format!("{CALL}{}", HANDS.fetch_add(1, Ordering::Relaxed));
    self.0.calls.insert(name.clone(), Box::new(call));
    Object::function(name, None)
  }

  /// These hands, with each ear and each function that `other` holds beside them.
  pub fn join(&mut self, other: Self) {
    self.0.ears.extend(other.0.ears);
    self.0.calls.extend(other.0.calls);
  }
}

/// The life, lent by value to code of a host while the life hears it, since every frame of the life
/// waits until that code is done: the code calls each verb of the life through it, and gives it
/// back when it is done.
pub struct Life {
  sand: Sand,
  outside: Outside,
}

impl Life {
  /// One verb of the life, called by its name with its words, and what it gave or raised. Who
  /// speaks is the verb `spoken`, and a callable the engine made is the verb `made`, with its
  /// number and its words.
  pub fn call(
    &mut self,
    verb: &str,
    args: Vec<Object>,
    kwargs: Vec<(&str, Object)>,
  ) -> Result<Object, Fault> {
    let kwargs = kwargs.into_iter().map(|(key, one)| (key.to_owned(), one)).collect();
    self.outside.asked(&mut self.sand, Call { verb: verb.to_owned(), args, kwargs })
  }

  /// The ears and the functions of the host that the values of the next calls name, as
  /// [`Engine::hand`] takes them.
  ///
  /// [`Engine::hand`]: crate::Engine::hand
  pub fn hand(&mut self, hands: Hands) {
    self.outside.hosted.given.join(hands);
  }
}

/// Where an ear of the host stands, as its object in the sandbox steps it.
#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub(super) enum At {
  /// Not yet born: its first step is its birth.
  Unborn,
  /// At a yield, and it hears the next fact.
  Waiting,
  /// Over, or raised: it hears nothing more.
  Over,
}

/// What an ear of the host did after the life answered each verb it called while it heard.
enum Stepped {
  /// One saying, as [`Step::Say`].
  Say(Fact),
  /// It waits, as [`Step::Wait`].
  Wait(Option<Object>),
  /// It is over.
  Over,
  /// It raised.
  Raised(Fault),
}

/// The ears that a host gave a life and where each stands, which the sandbox steps as generators.
pub(super) struct Hosted {
  pub(super) handed: Handed,
  /// What the host gave with its calls that no value that came into the sandbox named yet.
  pub(super) given: Hands,
  /// The waker of whoever drives the engine, which each ear is polled with.
  pub(super) waker: Waker,
  /// Each ear of the host, by the id of its object in the sandbox: its name and where it stands.
  pub(super) standing: HashMap<MontyUuid, (String, At)>,
  /// What each ear of the host waits for since its last step, by the id of its object in the
  /// sandbox.
  pub(super) waits: HashMap<MontyUuid, Object>,
  /// The fact that an ear of the host heard last, as [`Outside::told`] keeps it: the value that
  /// the sandbox sent, and the fact that it went out as.
  pub(super) told: Option<(Object, Fact)>,
  /// The count of the replay, while the life boots.
  pub(super) replay: Option<Replay>,
}

/// The count of a replay: of the entries that a life boots on, how many the journal passed so far.
/// The journal takes the entries in the order of the record, so an entry that it says or makes
/// again stands after each entry that it passed, and its place is the count. An ear of the host
/// hears a fact that the journal says again as said by the journal, and a question that the life
/// makes again under the name that the record holds. The first `started` or `done` of an act is
/// said again as the act is made, before the journal reaches its place, so it counts nothing.
/// The count stands still while no ear of the host hears the facts, and the end of a boot that
/// raised nothing counts each entry.
pub(super) struct Replay {
  /// The places in the record of the entries of each kind about each name, in order, each with
  /// whether the journal says or makes it again at its place.
  places: HashMap<(String, String), VecDeque<(u64, bool)>>,
  done: u64,
  of: u64,
  told: Box<dyn Fn(u64, u64)>,
}

impl Replay {
  /// The count of the replay of a record, as boot takes it, which tells that it passed no entry.
  pub(super) fn of(record: &[Object], told: Box<dyn Fn(u64, u64)>) -> Self {
    let mut places: HashMap<_, VecDeque<_>> = HashMap::new();
    let mut answered = HashSet::new();
    let facts = record.iter().map(|one| entry(&one.as_ref(), 0).and_then(Fact::of));
    for (place, fact) in (0..).zip(facts) {
      let Some(fact) = fact else { continue };
      let first =
        matches!(fact.kind(), "started" | "done") && answered.insert(fact.about().to_owned());
      let key = (fact.kind().to_owned(), fact.about().to_owned());
      places.entry(key).or_default().push_back((place, !first));
    }
    let of = record.len() as u64;
    told(0, of);
    Self { places, done: 0, of, told }
  }

  /// A fact that went out to an ear of the host.
  fn heard(&mut self, fact: &Fact) {
    if !fact.question() && fact.by() != JOURNAL {
      return;
    }
    let key = (fact.kind().to_owned(), fact.about().to_owned());
    let place = self.places.get_mut(&key).and_then(VecDeque::pop_front);
    if let Some((place, true)) = place
      && place >= self.done
    {
      self.done = place + 1;
      (self.told)(self.done, self.of);
    }
  }
}

impl Hosted {
  /// The end of the replay, which lets go of its count: the journal of a boot that raised nothing
  /// took each entry.
  pub(super) fn replayed(&mut self, whole: bool) {
    if let Some(replay) = self.replay.take()
      && whole
    {
      (replay.told)(replay.of, replay.of);
    }
  }
}

impl Default for Hosted {
  fn default() -> Self {
    Self {
      handed: Handed::default(),
      given: Hands::default(),
      waker: Waker::noop().clone(),
      standing: HashMap::new(),
      waits: HashMap::new(),
      told: None,
      replay: None,
    }
  }
}

impl Outside {
  /// Whether the ear of the host by this name, which raised, starts again: it is born again under
  /// the same object, when the ear makes a fresh one in its place.
  pub(super) fn reborn(&mut self, name: Option<&Object>) -> bool {
    let Some(name) = name.and_then(|one| one.as_ref().as_str().map(str::to_owned)) else {
      return false;
    };
    let over = self.hosted.standing.iter().find(|(_, (one, at))| *one == name && *at == At::Over);
    let Some(on) = over.map(|(on, _)| *on) else {
      return false;
    };
    if !self.hosted.handed.ears.get_mut(&name).is_some_and(|ear| ear.restart()) {
      return false;
    }
    self.hosted.waits.remove(&on);
    self.hosted.standing.insert(on, (name, At::Unborn));
    true
  }

  /// Each ear of the host has its say at the end of a run, in turn, and the first fault of one is
  /// what they give.
  pub(super) fn ran(&mut self) -> Result<(), Fault> {
    self.hosted.handed.ears.values_mut().map(|ear| ear.ran()).fold(Ok(()), Result::and)
  }

  /// What the first ear that stands at a yield says of its own accord, in the order the host gave
  /// the ears, which the ids the host gives keep; and nothing when no ear says anything. A poll
  /// calls nothing in the sandbox, so an ear may be polled while no life hears it.
  /// While an ear holds an entry that the record does not hold yet, only such an ear is polled, as
  /// `Engine::pump` says.
  pub(super) fn spoken(&mut self) -> Option<(String, Spoken)> {
    let ears = &self.hosted.handed.ears;
    let owed = ears.values().any(|ear| ear.unkept() > 0);
    let polled = |name: &String| !owed || ears.get(name).is_some_and(|ear| ear.unkept() > 0);
    let mut standing: Vec<_> = self
      .hosted
      .standing
      .iter()
      .filter(|(_, (name, at))| matches!(at, At::Waiting) && polled(name))
      .map(|(id, (name, _))| (*id, name.clone()))
      .collect();
    standing.sort_unstable();
    let mut cx = Context::from_waker(&self.hosted.waker);
    standing.into_iter().find_map(|(_, name)| {
      let ear = self.hosted.handed.ears.get_mut(&name)?;
      match ear.poll(&mut cx) {
        Poll::Ready(spoken) => Some((name, spoken)),
        Poll::Pending => None,
      }
    })
  }

  /// What the ears say of their own accord, said into the life, each under the name of its ear,
  /// from what one ear said first until no ear says more: a saying, or a verb, whose value goes
  /// nowhere. The first that raises ends it.
  pub(super) fn said(&mut self, sand: &mut Sand, first: (String, Spoken)) -> Result<(), Fault> {
    let (speaks, spoke) = (self.opened.speaks.clone(), self.opened.spoke.clone());
    let mut next = Some(first);
    while let Some((by, spoken)) = next {
      self.asking.clear();
      let token = sand.call(self, &speaks, vec![Object::string(&by)], vec![])?;
      let got = match spoken {
        Spoken::Saying(saying) => self.says(sand, &saying),
        // What the verb came to goes back to the ear, which takes it at its next poll.
        Spoken::Verb(call) => {
          let got = self.asked(sand, call);
          if let Some(ear) = self.hosted.handed.ears.get_mut(&by) {
            ear.answered(got);
          }
          Ok(())
        }
      };
      let reset = sand.call(self, &spoke, vec![token.clone()], vec![]);
      if let Some(handle) = token.as_ref().handle() {
        self.drop_handle(sand, &handle);
      }
      got?;
      reset?;
      next = self.spoken();
    }
    Ok(())
  }

  /// One saying of an ear, said into the life.
  pub(super) fn says(&mut self, sand: &mut Sand, saying: &Fact) -> Result<(), Fault> {
    let words = self.inward(sand, &saying.0)?;
    let words =
      words.as_ref().items().unwrap_or_default().into_iter().map(|one| one.to_owned()).collect();
    self.named_call(sand, "say", words, vec![]).map(drop)
  }

  /// One step of an ear of the host: what it heard goes to it, and each verb it calls is said here,
  /// under its name, and what it came to is the next thing it hears, until it does something else.
  fn stepped(&mut self, sand: &mut Sand, ear: &mut dyn Ear, mut heard: Heard) -> Stepped {
    let waker = self.hosted.waker.clone();
    loop {
      match self.lent(sand, |life| ear.lent(heard, &mut Context::from_waker(&waker), life)) {
        Step::Call(call) => heard = Heard::Answer(self.asked(sand, call)),
        Step::Say(saying) => return Stepped::Say(saying),
        Step::Wait(wait) => return Stepped::Wait(wait),
        Step::Over => return Stepped::Over,
        Step::Raised(fault) => return Stepped::Raised(fault),
      }
    }
  }

  /// One step of an ear of the host, which the sandbox holds as an object that stands in for a
  /// generator.
  ///
  /// At its birth the ear starts. Then every fact it is given goes to the ear, and what comes back
  /// is what the ear did with it. It said something, which the object yields, and the bus hands
  /// back the fact as it was said, which goes to the ear next. It said nothing, so the object waits
  /// for the next fact. It said a verb, which is said here by its name and its value handed back.
  /// It is over, so the object stops, and it raised, so the object raises. A generator that the
  /// host started before it crossed was born there, so its object begins where it waits.
  pub(super) fn sent(&mut self, sand: &mut Sand, on: MontyUuid, a: Option<Object>) -> Answer {
    let Some((name, at)) = self.hosted.standing.get(&on).cloned() else {
      return Answer::Abort(Fault::refused(format!("{on} is no object of the host")));
    };
    let a = a.filter(|one| !one.as_ref().is_none());
    let heard = match (at, a) {
      (At::Over, _) => return Answer::Fault(Fault::stop_iteration()),
      (At::Unborn, Some(_)) => {
        return Answer::Fault(Fault::type_error(
          "can't send non-None value to a just-started generator",
        ));
      }
      (At::Unborn, None) => Heard::Born,
      (At::Waiting, None) => {
        return Answer::Value(self.hosted.waits.get(&on).cloned().unwrap_or_else(Object::none));
      }
      (At::Waiting, Some(_)) if let Some((_, refusal)) = self.ending.take_if(|one| one.0 == on) => {
        return self.inward(sand, &refusal.0).map_or_else(Answer::Abort, Answer::Value);
      }
      (At::Waiting, Some(a)) => match self.told(sand, a) {
        Ok(Some(fact)) => Heard::Fact(fact),
        Ok(None) => return Answer::Abort(Fault::refused("an ear hears a fact or nothing")),
        Err(fault) => return Answer::Abort(fault),
      },
    };
    if let Heard::Fact(fact) = &heard
      && fact.question()
      && let Some(bound) = sand::past()
    {
      return self.refused(sand, on, fact, bound);
    }
    if let Heard::Fact(fact) = &heard
      && fact.question()
      && let Some(answer) = self.overran(sand, on, fact)
    {
      return answer;
    }
    let Some(mut ear) = self.hosted.handed.ears.remove(&name) else {
      return Answer::Abort(Fault::refused(format!("no ear of the host is named {name}")));
    };
    let (at, answer) = match self.stepped(sand, &mut *ear, heard) {
      Stepped::Say(saying) => {
        (At::Waiting, self.inward(sand, &saying.0).map_or_else(Answer::Abort, Answer::Value))
      }
      Stepped::Wait(wait) => {
        let wait = wait.unwrap_or_else(Object::none);
        self.hosted.waits.insert(on, wait.clone());
        (At::Waiting, Answer::Value(wait))
      }
      Stepped::Over => (At::Over, Answer::Fault(Fault::stop_iteration())),
      Stepped::Raised(fault) => (At::Over, self.raising(sand, fault)),
    };
    self.hosted.handed.ears.insert(name.clone(), ear);
    self.hosted.standing.insert(on, (name, at));
    answer
  }

  /// A fact that the sandbox sends to an ear of the host, as it goes out, or nothing for a value
  /// that is no fact. The engine sends a fact to each ear that hears it in turn, so the fact that
  /// went out last stays here beside the value that came, and an ear that is sent the same value
  /// takes a copy of that fact: a fact goes out one time for all the ears that hear it, whatever
  /// its size. Two values are the same when their arenas are, which holds for a fact that the
  /// engine sends again and costs one pass with no walk. A value that holds a handle that went
  /// out as its representation never comes the same again, since the sandbox let go of that
  /// handle, and a number that the host forgot drops the fact that named it.
  fn told(&mut self, sand: &mut Sand, a: Object) -> Result<Option<Fact>, Fault> {
    if let Some((came, fact)) = &self.hosted.told
      && unstable::graph_parts(came) == unstable::graph_parts(&a)
    {
      return Ok(Some(fact.clone()));
    }
    let fact = Fact::owned(self.outward(sand, &a)?);
    if let (Some(fact), Some(replay)) = (&fact, &mut self.hosted.replay) {
      replay.heard(fact);
    }
    self.hosted.told = fact.clone().map(|fact| (a, fact));
    Ok(fact)
  }

  /// The answer to a question that a step of a word asks past the bound of a turn, in the place of
  /// its ear. monty counts a turn again from each round trip to the host, so a word that loops on
  /// what the host answers at once never meets the bound of monty: the host refuses the question,
  /// once for all the ears that hear it, and the refusal raises in the word. A word that asks again
  /// past that refusal goes on in its loop, so its step ends there, as a second excess of the bound
  /// of monty ends it.
  fn refused(&mut self, sand: &mut Sand, on: MontyUuid, fact: &Fact, bound: Duration) -> Answer {
    let asked = (fact.about().to_owned(), fact.by().to_owned());
    match self.past.replace(asked.clone()) {
      Some(one) if one.0 == asked.0 => {
        Answer::Value(self.hosted.waits.get(&on).cloned().unwrap_or_else(Object::none))
      }
      Some(one) if one.1 == asked.1 => Answer::Abort(Fault::refused(format!(
        "{} asked {} past the refusal of the bound of a turn of {bound:?}",
        asked.1, asked.0
      ))),
      _ => {
        let why = format!("the step of the word ran past the bound of a turn of {bound:?}");
        let done = Fact::says("done", &asked.0, [Fault::refused(why).object()]);
        self.inward(sand, &done.0).map_or_else(Answer::Abort, Answer::Value)
      }
    }
  }

  /// The answer to a question of an act that asked questions of the ears of the host for longer
  /// than the bound of a turn, in live work of the ears outside any step of a word, which holds the
  /// host from its commands and from every other chain: the ear of the host that heard the
  /// question says the act done with a refusal that names it, then the question refused, so the
  /// record holds both and a later life takes them from it and from no clock. An act whose ear is
  /// `controlled` stops at its own end. Nothing when the act stands within the bound or the asker
  /// is no act.
  fn overran(&mut self, sand: &mut Sand, on: MontyUuid, fact: &Fact) -> Option<Answer> {
    if !self.stepping.is_empty() {
      return None;
    }
    let by = fact.by().to_owned();
    let bound = sand::turn();
    if self.asking.entry(by.clone()).or_insert_with(Instant::now).elapsed() < bound {
      return None;
    }
    let asker = self.named_call(sand, "get", vec![Object::string(&by)], vec![]).ok()?;
    let kind = asker.as_ref().items()?.first()?.as_str()?.to_owned();
    let why = format!(
      "{by}, an act of {kind}, asked the World for longer than the bound of a turn of {bound:?} \
       with no step of a word, so the life ends it and refuses its {}",
      fact.about()
    );
    let refusal = Fault::refused(why).object();
    let question = Fact::says("done", fact.about(), [refusal.clone()]);
    let over = self.named_call(sand, "over", vec![Object::string(&by)], vec![]).ok()?;
    let ends = if over.as_ref().as_bool() == Some(true) {
      question
    } else {
      self.ending = Some((on, question));
      Fact::says("done", &by, [refusal])
    };
    Some(self.inward(sand, &ends.0).map_or_else(Answer::Abort, Answer::Value))
  }

  /// What the host does in `step`, with this life lent to it, which answers at once each verb that
  /// the host calls while it runs, since the sandbox takes a call before the call of the sandbox
  /// that the host answers.
  ///
  /// Every frame of the life waits until `step` is done, so the life goes to it by value, with a
  /// life of nothing in its place, and comes back with what `step` gave.
  pub(super) fn lent<R>(&mut self, sand: &mut Sand, step: impl FnOnce(Life) -> (R, Life)) -> R {
    let life = Life { sand: mem::replace(sand, Sand::none()), outside: mem::take(self) };
    let (got, life) = step(life);
    (*sand, *self) = (life.sand, life.outside);
    got
  }

  /// What the host handed over under this name, which this life holds from now on.
  pub(super) fn take(&mut self, name: &str) {
    let Hosted { handed, given, .. } = &mut self.hosted;
    if let Some(ear) = given.0.ears.remove(name) {
      handed.ears.insert(name.to_owned(), ear);
    }
    if let Some(call) = given.0.calls.remove(name) {
      handed.calls.insert(name.to_owned(), call);
    }
  }
}
