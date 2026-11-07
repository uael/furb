//! The ears that a host hands a life, the functions it hands it, and how the life steps each ear as
//! a generator of the sandbox and lends itself to the code of the host.

#![expect(
  clippy::too_many_arguments,
  reason = "debt: a function of more than 4 parameters takes a struct, rule 8 of the skill rust"
)]
use std::{
  cell::RefCell,
  collections::HashMap,
  sync::atomic::{AtomicI64, Ordering},
  task::{Context, Poll, Waker},
  time::Duration,
};

use monty_types::MontyUuid;

use super::Outside;
use crate::{
  ear::{Call, Ear, Heard, Spoken, Step},
  fact::Fact,
  sand::{Answer, Sand},
  value::{Fault, Given, Object, marked},
};

/// The prefix of the name a function of the host crosses in under: `call:` and its number.
pub(super) const CALL: &str = "call:";

/// The prefix of the name an ear of the host is heard under when a verb is given it: `ear:` and its
/// number.
pub(super) const EAR: &str = "ear:";

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
  ) -> Result<(Handed, Vec<String>), Fault> {
    let mut given = Handed::default();
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

thread_local! {
  /// What a host of this thread handed over and no life took yet. A life takes each the first time
  /// a value that names it comes into its sandbox, and holds it from then on, so each is the life's
  /// that was given it.
  static HANDED: RefCell<Handed> = RefCell::default();
}

/// How many ears and functions the hosts of this process handed over, which names the next.
static HANDS: AtomicI64 = AtomicI64::new(1);

/// An ear of the host, handed over as the value a verb is given: the object that stands in for it
/// in the sandbox, which stands at a yield when the host started the ear before it crossed.
pub fn handed(ear: Box<dyn Ear>, started: bool) -> Object {
  let name = format!("{EAR}{}", HANDS.fetch_add(1, Ordering::Relaxed));
  HANDED.with_borrow_mut(|held| held.ears.insert(name.clone(), ear));
  marked("ear", [("name", Object::string(name)), ("started", Object::bool(started))])
}

/// A function of the host, handed over as a value a verb may be given: a show, a filter. The
/// sandbox calls it back, and what it gives is what the call gave.
pub fn callable(mut call: impl FnMut(Vec<Object>) -> Result<Object, Fault> + 'static) -> Object {
  lending(move |args, life| (call(args), life))
}

/// A function of the host that calls verbs of the life, handed over as [`callable`] hands one: the
/// life is lent to it while it runs, and it gives the life back with what the call gave.
pub fn lending(
  call: impl FnMut(Vec<Object>, Life) -> (Result<Object, Fault>, Life) + 'static,
) -> Object {
  let name = format!("{CALL}{}", HANDS.fetch_add(1, Ordering::Relaxed));
  HANDED.with_borrow_mut(|held| held.calls.insert(name.clone(), Box::new(call)));
  Object::function(name, None)
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
}

/// Where an ear of the host stands, as its object in the sandbox steps it.
#[derive(Clone, Copy, PartialEq)]
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
  /// The waker of whoever drives the engine, which each ear is polled with.
  pub(super) waker: Waker,
  /// Each ear of the host, by the id of its object in the sandbox: its name and where it stands.
  pub(super) standing: HashMap<MontyUuid, (String, At)>,
  /// What each ear of the host waits for since its last step, by the id of its object in the
  /// sandbox.
  pub(super) waits: HashMap<MontyUuid, Object>,
}

impl Default for Hosted {
  fn default() -> Hosted {
    Hosted {
      handed: Handed::default(),
      waker: Waker::noop().clone(),
      standing: HashMap::new(),
      waits: HashMap::new(),
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
  pub(super) fn spoken(&mut self) -> Option<(String, Spoken)> {
    let mut standing: Vec<_> = self
      .hosted
      .standing
      .iter()
      .filter(|(_, (_, at))| matches!(at, At::Waiting))
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
  pub(super) fn sent(&mut self, sand: &mut Sand, on: MontyUuid, a: Option<&Object>) -> Answer {
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
      (At::Waiting, Some(a)) => match self.outward(sand, a).map(|a| Fact::of(a.as_ref())) {
        Ok(Some(fact)) => Heard::Fact(fact),
        Ok(None) => return Answer::Abort(Fault::refused("an ear hears a fact or nothing")),
        Err(fault) => return Answer::Abort(fault),
      },
    };
    if let Heard::Fact(fact) = &heard
      && fact.question()
      && let Some(bound) = crate::sand::past()
    {
      return self.refused(sand, on, fact, bound);
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

  /// What the host does in `step`, with this life lent to it, which answers at once each verb that
  /// the host calls while it runs, since the sandbox takes a call before the call of the sandbox
  /// that the host answers.
  ///
  /// Every frame of the life waits until `step` is done, so the life goes to it by value, with a
  /// life of nothing in its place, and comes back with what `step` gave.
  pub(super) fn lent<R>(&mut self, sand: &mut Sand, step: impl FnOnce(Life) -> (R, Life)) -> R {
    let life = Life { sand: std::mem::replace(sand, Sand::none()), outside: std::mem::take(self) };
    let (got, life) = step(life);
    (*sand, *self) = (life.sand, life.outside);
    got
  }

  /// What the host handed over under this name, which this life holds from now on.
  pub(super) fn take(&mut self, name: &str) {
    HANDED.with_borrow_mut(|held| {
      if let Some(ear) = held.ears.remove(name) {
        self.hosted.handed.ears.insert(name.to_owned(), ear);
      }
      if let Some(call) = held.calls.remove(name) {
        self.hosted.handed.calls.insert(name.to_owned(), call);
      }
    });
  }
}
