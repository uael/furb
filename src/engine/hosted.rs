//! The ears that a host hands a life, the functions it hands it, and how the life steps each ear as
//! a generator of the sandbox and lends itself to the code of the host.

use std::{
  cell::RefCell,
  collections::HashMap,
  sync::atomic::{AtomicI64, Ordering},
  task::{Context, Poll, Waker},
};

use monty_types::MontyUuid;

use super::Outside;
use crate::{
  ear::{Call, Ear, Heard, Spoken, Step},
  fact::Fact,
  sand::{Answer, Sand},
  value::{Fault, Given, Object, ObjectRef, marked},
};

/// The views of the engine, which make nothing: the verbs that an ear calls while it hears the past
/// at a restore, which the life answers. It answers every other verb with nothing, since the life
/// holds what it made.
const VIEWS: [&str; 18] = [
  "get",
  "peek",
  "transcript",
  "question",
  "scope",
  "under",
  "over",
  "tells",
  "paused",
  "covers",
  "asked",
  "answer",
  "standing",
  "cwd",
  "module",
  "program",
  "turns",
  "filled",
];

/// For each fact of the past, the ears of the host that wait for it, among the ears `w`, each by
/// its name and what it waits for, as the bus of the engine gives a fact, whose `ears` this rule is
/// word for word: an ear that waits for nothing hears it, as does an ear that waits for its kind,
/// its act, or the kind, the maker, the chain or the thread of that act, or the chain of that maker
/// or of that chain or thread, and every ear hears a control.
pub(crate) const HEARING: &str = "(lambda past, w: [(lambda a: (lambda k: [n for n, x in w if x is None or k & x or a[0] in \
  ('pause', 'wake', 'cancel', 'close')])((lambda q: {a[0], *q[:4], *map(scope, q[2:4])})(get(a[1]) or \
  a[:1] * 4)))(a) for a in past])(past, w)";

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
  /// Each act that an ear of the host said started and that is not done, by its name, with the name
  /// of that ear: the work of the outside that runs now.
  pub(super) working: HashMap<String, String>,
}

impl Default for Hosted {
  fn default() -> Hosted {
    Hosted {
      handed: Handed::default(),
      waker: Waker::noop().clone(),
      standing: HashMap::new(),
      waits: HashMap::new(),
      working: HashMap::new(),
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

  /// Whether an ear of the host holds a fact that it writes beside the record and could not write
  /// yet.
  pub(super) fn behind(&self) -> bool {
    self.hosted.handed.ears.values().any(|ear| ear.behind())
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
  /// The life answers a verb when `views` holds it or when no view is asked for, and answers it
  /// with nothing otherwise.
  fn stepped(
    &mut self,
    sand: &mut Sand,
    ear: &mut dyn Ear,
    mut heard: Heard,
    views: Option<&[&str]>,
  ) -> Stepped {
    let waker = self.hosted.waker.clone();
    loop {
      match self.lent(sand, |life| ear.lent(heard, &mut Context::from_waker(&waker), life)) {
        Step::Call(call) if views.is_none_or(|views| views.contains(&call.verb.as_str())) => {
          heard = Heard::Answer(self.asked(sand, call));
        }
        Step::Call(_) => heard = Heard::Answer(Ok(Object::none())),
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
        Ok(Some(fact)) => {
          self.track(&fact);
          Heard::Fact(fact)
        }
        Ok(None) => return Answer::Abort(Fault::refused("an ear hears a fact or nothing")),
        Err(fault) => return Answer::Abort(fault),
      },
    };
    let Some(mut ear) = self.hosted.handed.ears.remove(&name) else {
      return Answer::Abort(Fault::refused(format!("no ear of the host is named {name}")));
    };
    let (at, answer) = match self.stepped(sand, &mut *ear, heard, None) {
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

  /// The work of the outside that a fact changes: an act that an ear of the host said started runs
  /// until it is done.
  pub(super) fn track(&mut self, fact: &Fact) {
    match fact.kind() {
      "started" if self.hosted.standing.values().any(|(name, _)| name == fact.by()) => {
        self.hosted.working.insert(fact.about().to_owned(), fact.by().to_owned());
      }
      "done" => {
        self.hosted.working.remove(fact.about());
      }
      _ => {}
    }
  }

  /// Each ear that waits, in the order of its id, as the host gives it.
  pub(super) fn waiting(&self) -> Vec<(MontyUuid, String)> {
    let mut ears: Vec<_> = self
      .hosted
      .standing
      .iter()
      .filter(|(_, (_, at))| *at == At::Waiting)
      .map(|(id, (name, _))| (*id, name.clone()))
      .collect();
    ears.sort_unstable();
    ears
  }

  /// One fact or the birth, heard by one ear of the host outside the log: each verb it calls is
  /// answered, by the life when `views` holds it or when no view is asked for, and by nothing
  /// otherwise, and each saying comes back to it as the fact that the bus would make of it, with
  /// nothing said into the life. What it waits for after is kept as a wait of the ear is kept. It
  /// gives whether the ear waits after, and refuses an ear that says something when `speaks`
  /// refuses it.
  fn aside(
    &mut self,
    sand: &mut Sand,
    (on, name): &(MontyUuid, String),
    heard: Heard,
    views: Option<&[&str]>,
    speaks: bool,
  ) -> Result<bool, Fault> {
    let Some(mut ear) = self.hosted.handed.ears.remove(name) else { return Ok(false) };
    let mut heard = heard;
    let waits = loop {
      match self.stepped(sand, &mut *ear, heard, views) {
        Stepped::Say(saying) if speaks => {
          let held = saying.0.as_ref().items().unwrap_or_default();
          let mut made: Vec<Object> = held.iter().take(2).map(ObjectRef::to_owned).collect();
          made.push(Object::string(name));
          made.extend(held.iter().skip(2).map(ObjectRef::to_owned));
          heard = Heard::Fact(Fact(Object::tuple(made)));
        }
        Stepped::Say(_) => {
          self.hosted.handed.ears.insert(name.clone(), ear);
          return Err(Fault::refused(format!(
            "the ear {name} speaks at its birth, so no snapshot opens it"
          )));
        }
        Stepped::Wait(wait) => {
          self.hosted.waits.insert(*on, wait.unwrap_or_else(Object::none));
          break true;
        }
        Stepped::Over | Stepped::Raised(_) => break false,
      }
    };
    self.hosted.handed.ears.insert(name.clone(), ear);
    if !waits {
      self.hosted.standing.insert(*on, (name.clone(), At::Over));
    }
    Ok(waits)
  }

  /// The facts of the past that ears hear again, as a replay of the record tells them: each fact of
  /// the log that is no act, but the birth of a chain, with the journal as the speaker of each fact
  /// that no act said.
  fn journaled(&mut self, sand: &mut Sand, past: &[Object]) -> Result<Vec<Object>, Fault> {
    let mut acts: HashMap<String, bool> = HashMap::new();
    let mut told = Vec::new();
    for one in past {
      let Some(fact) = Fact::of(one.as_ref()) else { continue };
      if fact.question() && fact.kind() != "chain" {
        continue;
      }
      let by = fact.by().to_owned();
      let act = if let Some(act) = acts.get(&by) {
        *act
      } else {
        let got = self.named_call(sand, "get", vec![Object::string(&by)], vec![])?;
        let act = !got.as_ref().is_none();
        acts.insert(by.clone(), act);
        act
      };
      let journal = !act && !fact.question();
      let held = fact.0.as_ref().items().unwrap_or_default();
      let speaker = |(i, one): (usize, &ObjectRef<'_>)| {
        if i == 2 && journal { Object::string("journal") } else { one.to_owned() }
      };
      told.push(Object::tuple(held.iter().enumerate().map(speaker).collect::<Vec<_>>()));
    }
    Ok(told)
  }

  /// The ears that the host gives again, each born and then told the past of the life, as a replay
  /// of the record tells it: each fact of the log that is no act, but the birth of a chain, to each
  /// ear that waits for it, with the journal as the speaker of each fact that no act said, since
  /// the journal says it again. What an ear says or makes from the past reaches nothing, as the
  /// journal answers it in a replay, and the log, which holds the past, hears none of it.
  pub(super) fn retold(&mut self, sand: &mut Sand, past: &[Object]) -> Result<(), Fault> {
    for one in self.waiting() {
      self.aside(sand, &one, Heard::Born, None, false)?;
    }
    let told = self.journaled(sand, past)?;
    let ears: Vec<_> = self.waiting().into_iter().filter(|(_, name)| name != "log").collect();
    let waits = ears.iter().map(|(on, name)| {
      Object::tuple([
        Object::string(name),
        self.hosted.waits.get(on).cloned().unwrap_or_else(Object::none),
      ])
    });
    // The rule reads the kind and the act of a fact alone, so the rest of the fact stays out of the
    // sandbox: it can be large, and it can hold a callable that only the life that told it held.
    let heads = told.iter().map(|one| {
      let held = one.as_ref().items().unwrap_or_default();
      Object::tuple(held.iter().take(2).map(ObjectRef::to_owned).collect::<Vec<_>>())
    });
    let names = Object::dict([
      (Object::string("past"), Object::list(heads.collect::<Vec<_>>())),
      (Object::string("w"), Object::list(waits.collect::<Vec<_>>())),
    ]);
    let (word, names) = (self.opened.word.clone(), self.inward(sand, &names)?);
    let hearing = sand.call(self, &word, vec![Object::string(HEARING), names], vec![])?;
    let hearing = self.outward(sand, &hearing)?;
    let hearing = hearing.as_ref().items().unwrap_or_default();
    for (fact, names) in told.into_iter().zip(hearing) {
      let names = names.items().unwrap_or_default();
      let names: Vec<&str> = names.iter().filter_map(ObjectRef::as_str).collect();
      for one in ears.iter().filter(|(_, name)| names.contains(&name.as_str())) {
        self.aside(sand, one, Heard::Fact(Fact(fact.clone())), Some(&VIEWS), true)?;
      }
    }
    Ok(())
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
