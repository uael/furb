//! The engine: one life of `engine.py` in the sandbox, the ears it hears, and the verbs a host says.
//!
//! An engine is driven from one thread. A verb is one call: it runs in the sandbox, the engine does what it does,
//! every fact it says reaches its ears, and what the verb gave comes back. An act is awaited: the future drives the
//! engine until the act is done, which means each ear is polled, and what an ear says of its own accord is said
//! under its name. Nothing polls in vain: what an ear waits for wakes whoever awaits.
//!
//! The sandbox reaches the host by the gate, which reads a sheet, by each ear of the host, which the sandbox holds as
//! an object of the host that stands at a yield, and by the functions the host gave, a show or a filter. A call of
//! one of them comes to [`Outside`], and is answered on the thread of the engine, inside the call.
//!
//! What crosses, crosses as the interpreter carries it, but for what the interpreter carries no way back, which
//! crosses by a map that names what it is under `is`. A name of the engine crosses as its name, both ways, so a show
//! of the engine is the same show on both sides. A callable the engine made goes out as a number that the host calls
//! it back by, and a function of the host comes in as a function the interpreter calls back on the host. A class a
//! word defined goes out by a number too, with the class it was made with, and comes back in by it; an instance of
//! one goes out with its fields under its class and comes back in made from them, with no `__init__` run. An instance
//! of a class of the engine comes in as the name of its class and its fields, and it is made in the sandbox. A map
//! that holds the key `is` crosses as its pairs under `is` with the name `dict`, both ways, so no side reads the map
//! of a word or of a host as a mark.

use std::{
  cell::RefCell,
  collections::{HashMap, HashSet},
  future::Future,
  marker::PhantomData,
  pin::Pin,
  sync::atomic::{AtomicI64, Ordering},
  task::{Context, Poll, Waker},
};

use monty_types::{
  ExcType, MontyUuid, NamedValues,
  unstable::{self, MontyGraph, MontyNode, NodeId},
};

use crate::{
  ENGINE, KERNEL, SHEET,
  ear::{Call, Ear, Heard, Spoken, Step},
  fact::Fact,
  gate::checked,
  sand::{Answer, Host, Nobody, Sand, id, object},
  snapshot::Keeper,
  value::{Exit, Fault, IS, Object, Text, entry, inward, marked},
};

/// The views of the engine, which make nothing: the verbs that an ear calls while it hears the past at a restore,
/// which the life answers. It answers every other verb with nothing, since the life holds what it made.
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

/// For each fact of the past, the ears of the host that wait for it, among the ears `w`, each by its name and what it
/// waits for, as the bus gives a fact: an ear that waits for nothing hears it, as does an ear that waits for its kind, its act, or the
/// kind, the maker or the chain of that act, or the chain of that maker, and every ear hears a control.
const HEARING: &str = "(lambda past, w: [(lambda a: (lambda k: [n for n, x in w if x is None or k & x or a[0] in \
  ('pause', 'wake', 'cancel', 'close')])((lambda q: {a[0], *q[:4], scope(q[2])})(get(a[1]) or a[:1] * 4)))(a) \
  for a in past])(past, w)";

/// The prefix of the name a function of the host crosses in under: `call:` and its number.
const CALL: &str = "call:";

/// The prefix of the name an ear of the host is heard under when a verb is given it: `ear:` and its number.
const EAR: &str = "ear:";

/// The name of the function of the host that the gate calls with a sheet, which answers each finding by its line.
const GATE: &str = "gate";

/// The kinds of act that are work an operator waits for: a prompt, a rung, a command and a wait.
const WORK: [&str; 4] = ["prompt", "rung", "bash", "wait"];

/// Whether this life took up an act that the record shows started and not done: the outside says it started in this
/// life, which it does once a wake that this life says puts the act to it, or the act is done. A started that the
/// journal says again is of an earlier life.
const TOOK: &str = "peek(__id, ...) is not ... or ('started', __id) in [x[:2] for x in transcript(scope(__id)) \
  if x[2] != 'journal']";

/// Whether an act waits: it is not done, and no pause holds it.
const WAITS: &str = "peek(__id, ...) is ... and not paused(__id)";

/// How many lives may end while an act of the outside runs before a later life parks it. Each of those lives ended
/// while the act ran and before it was done, so the act may be what ended them.
pub const CRASHES: usize = 5;

/// The kinds of act that a life parks, which are those that can end the process that runs them again: the run of a
/// word and a reply. A command is none of them, since no later life runs it again: the ear of commands closes it as
/// interrupted at the wake.
const PARKS: [&str; 2] = ["run", "reply"];

/// The act that the operator made for a key of the client on a chain: the first question that the operator made on
/// the chain after a key fact of that key and that kind, when it is of that kind, and nothing otherwise.
const KEYED: &str = "next((b[1] for f, k, w in [(transcript(__on), __key, __kind)] \
  for i, a in enumerate(f) if a[0] == 'key' and a[3:5] == (k, w) for b in [next((x for x in f[i + 1:] if x[2] == OPERATOR and question(x)), ('',))] \
  if b[0] == w), '')";

/// The one piece of python the crate writes: it loads the engine, the sheet and the Kernel, each in a namespace of
/// its own, and gives what the host calls in the sandbox by.
///
/// It gives the park too: a generator that stands in front of an ear of the outside and holds back from it each act
/// that is parked when the act is put to it, and the parked names, which the host sets at boot and clears at a retry.
/// When a retried act is not done and no ear took it in this life, the park puts it to its ear at the next fact that
/// the ear hears, in the order of the ears, before that fact. The park waits for what its ear waits for, and for every
/// fact while it holds an act back.
///
/// The module of the engine holds the names python gives a module before the engine runs, so a word reads them in a
/// chain as it reads them in python, and as the gate reads them. No loader made the module here, so it has no spec
/// and no loader. Python binds `__debug__` among its builtins, and the sandbox binds it in its main module alone.
const OPENING: &str = r#"from string.templatelib import Interpolation, Template

__engine = {"__debug__": True, "__doc__": None, "__package__": "furb", "__spec__": None, "__loader__": None}
exec(__source, __engine)
__sheet = {}
exec(__sheet_source, __sheet)
__kernel = {}
exec(__kernel_source, __kernel)
__parked = set()


def __parking(ear):
  held, heard, wait = [], None, None
  try:
    while True:
      if heard is not None and heard[1] in __parked and heard is __engine["get"](heard[1]):
        held.append(heard)
      else:
        for a in [x for x in held if x[1] not in __parked]:
          held.remove(a)
          at = __engine["transcript"](__engine["scope"](a[1]))
          if not __engine["over"](a[1]) and ("started", a[1]) not in [x[:2] for x in at if x[2] != "journal"]:
            said = ear.send(a)
            while isinstance(said, tuple):
              said = ear.send((yield said))
            wait = said
        said = ear.send(heard)
        while isinstance(said, tuple):
          said = ear.send((yield said))
        wait = said
      heard = yield (None if held else wait)
  except StopIteration:
    return


(
  lambda: {name: x for name, x in __engine.items() if not name.startswith("_") and callable(x)},
  lambda name, /, *args, **kwargs: __engine[name](*args, **kwargs),
  __kernel["kernel"](__engine),
  __sheet["gating"](__engine, __gate),
  lambda: __engine["site"].get(),
  lambda by: __engine["site"].set(by),
  lambda token: __engine["site"].reset(token),
  lambda word, names: eval(word, __engine, names),
  lambda held: Template(*[x if isinstance(x, str) else Interpolation(*x) for x in held]),
  lambda x: None if callable(x) else repr(x),
  lambda cls: cls.__bases__[0],
  __parking,
  lambda names: __parked.update(names),
  lambda name: __parked.discard(name),
)"#;

/// The next number a callable or a class goes out under, in this process.
static NUMBERS: AtomicI64 = AtomicI64::new(1);

/// A function of the host, as the sandbox may call it back, with the life lent to it while it runs.
type Callable = Box<dyn FnMut(Vec<Object>, Life) -> (Result<Object, Fault>, Life)>;

/// The words of a call: those by position, and those by name.
type Words<'a> = (Vec<Object>, Vec<(&'a str, Object)>);

/// What a host does when an act is done, given what it came to.
type Watcher = Box<dyn FnMut(&Object)>;

/// The ears and the functions that a host handed over, by the names the sandbox calls them by.
#[derive(Default)]
struct Handed {
  ears: HashMap<String, Box<dyn Ear>>,
  calls: HashMap<String, Callable>,
}

thread_local! {
  /// What a host of this thread handed over and no life took yet. A life takes each the first time a value that names
  /// it comes into its sandbox, and holds it from then on, so each is the life's that was given it.
  static HANDED: RefCell<Handed> = RefCell::default();
}

/// How many ears and functions the hosts of this process handed over, which names the next.
static HANDS: AtomicI64 = AtomicI64::new(1);

/// An ear of the host, handed over as the value a verb is given: the object that stands in for it in the sandbox,
/// which stands at a yield when the host started the ear before it crossed.
pub fn handed(ear: Box<dyn Ear>, started: bool) -> Object {
  let name = format!("{EAR}{}", HANDS.fetch_add(1, Ordering::Relaxed));
  HANDED.with_borrow_mut(|held| held.ears.insert(name.clone(), ear));
  marked("ear", [("name", Object::string(name)), ("started", Object::bool(started))])
}

/// A function of the host, handed over as a value a verb may be given: a show, a filter. The sandbox calls it back,
/// and what it gives is what the call gave.
pub fn callable(mut call: impl FnMut(Vec<Object>) -> Result<Object, Fault> + 'static) -> Object {
  lending(move |args, life| (call(args), life))
}

/// A function of the host that calls verbs of the life, handed over as [`callable`] hands one: the life is lent to it
/// while it runs, and it gives the life back with what the call gave.
pub fn lending(
  call: impl FnMut(Vec<Object>, Life) -> (Result<Object, Fault>, Life) + 'static,
) -> Object {
  let name = format!("{CALL}{}", HANDS.fetch_add(1, Ordering::Relaxed));
  HANDED.with_borrow_mut(|held| held.calls.insert(name.clone(), Box::new(call)));
  Object::function(name, None)
}

/// The life, lent by value to code of a host while the life hears it, since every frame of the life waits until that
/// code is done: the code calls each verb of the life through it, and gives it back when it is done.
pub struct Life {
  sand: Sand,
  outside: Outside,
}

impl Life {
  /// One verb of the life, called by its name with its words, and what it gave or raised. Who speaks is the verb
  /// `spoken`, and a callable the engine made is the verb `made`, with its number and its words.
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

/// The acts that the entries of a record show started and not done, by name, in the order of the record: work that
/// the outside took in an earlier life. The record keeps the started of the outside alone, since a later life says
/// again what an act said.
pub(crate) fn left(record: &[Object]) -> Vec<String> {
  let facts: Vec<Fact> =
    record.iter().filter_map(|one| entry(&one.as_ref(), 0).and_then(Fact::of)).collect();
  let done: HashSet<&str> = facts.iter().filter(|a| a.kind() == "done").map(Fact::about).collect();
  let started = facts.iter().filter(|a| a.kind() == "started" && !done.contains(a.about()));
  started.map(|a| a.about().to_owned()).collect()
}

/// The acts that the entries of a record park, each by its name and its kind, in the order of the record: an act of
/// a kind that [`PARKS`] names, which the record shows started in [`CRASHES`] lives or more and not done. The outside
/// says an act started once in each life that takes it, and the journal keeps each such started.
fn looping(record: &[Object]) -> Vec<(String, String)> {
  let facts: Vec<Fact> =
    record.iter().filter_map(|one| entry(&one.as_ref(), 0).and_then(Fact::of)).collect();
  let done: HashSet<&str> = facts.iter().filter(|a| a.kind() == "done").map(Fact::about).collect();
  let mut lives: Vec<(&str, usize)> = Vec::new();
  for a in facts.iter().filter(|a| a.kind() == "started" && !done.contains(a.about())) {
    match lives.iter_mut().find(|(id, _)| *id == a.about()) {
      Some((_, n)) => *n += 1,
      None => lives.push((a.about(), 1)),
    }
  }
  let parks = |id: &str| {
    let kind = id.trim_end_matches(|c: char| c.is_ascii_digit());
    PARKS.contains(&kind).then(|| (id.to_owned(), kind.to_owned()))
  };
  lives.into_iter().filter(|(_, n)| *n >= CRASHES).filter_map(|(id, _)| parks(id)).collect()
}

/// A template string, as a host says one: each expression beside its value.
fn templated(pairs: &[(&str, Object)]) -> Object {
  crate::value::paired(
    pairs.iter().map(|(expression, one)| ((*expression).to_owned(), one.clone())),
  )
}

/// What the opening gave: the callables of the sandbox that the host calls it by, each held under its handle.
struct Opened {
  /// The names of the engine that are callable, read again after boot, which binds some of them.
  names: Object,
  /// A name of the engine, called where it stands when it is called.
  call: Object,
  /// Who speaks: `site`, read.
  site: Object,
  /// Who speaks from now on: `site`, set, which gives the token that resets it.
  speaks: Object,
  /// Who spoke before: `site`, reset by a token.
  spoke: Object,
  /// A word run in the names of the engine, with values bound.
  word: Object,
  /// A template string of the sandbox, made from the parts of a template string of the host.
  template: Object,
  /// Nothing for a callable, and the representation of anything else.
  shown: Object,
  /// The class a class was made with.
  base: Object,
  /// A name taken out of the parked names of the park.
  unpark: Object,
}

impl Opened {
  /// The callables of the opening, each at its place in what the opening gave, and nothing at a place that boot alone
  /// takes, as a snapshot holds them.
  fn list(&self) -> Vec<Object> {
    let mut held = vec![Object::none(); 14];
    let named = [
      (0, &self.names),
      (1, &self.call),
      (4, &self.site),
      (5, &self.speaks),
      (6, &self.spoke),
      (7, &self.word),
      (8, &self.template),
      (9, &self.shown),
      (10, &self.base),
      (13, &self.unpark),
    ];
    for (at, one) in named {
      held[at] = one.clone();
    }
    held
  }

  /// The callables of the opening, each by its place in what the opening gave.
  fn of(at: impl Fn(usize) -> Object) -> Opened {
    Opened {
      names: at(0),
      call: at(1),
      site: at(4),
      speaks: at(5),
      spoke: at(6),
      word: at(7),
      template: at(8),
      shown: at(9),
      base: at(10),
      unpark: at(13),
    }
  }
}

/// Where an ear of the host stands, as its object in the sandbox steps it.
#[derive(Clone, Copy, PartialEq)]
enum At {
  /// Not yet born: its first step is its birth.
  Unborn,
  /// At a yield, and it hears the next fact.
  Waiting,
  /// Over, or raised: it hears nothing more.
  Over,
}

/// The side of the host in one life: its ears and its functions, which the sandbox calls, and what the host holds of
/// the sandbox, by which each value crosses.
struct Outside {
  handed: Handed,
  /// The waker of whoever drives the engine, which each ear is polled with.
  waker: Waker,
  opened: Opened,
  /// The names of the engine that are callable, and the name of each by its handle.
  names: HashMap<String, Object>,
  named: HashMap<MontyUuid, String>,
  /// Each callable the engine made and each class a word defined that went out, by the number the host holds it
  /// by, and the number of each by its handle, until the host forgets it.
  made: HashMap<i64, Object>,
  numbers: HashMap<MontyUuid, i64>,
  /// The handles the engine holds for itself, which the host never releases.
  kept: HashSet<MontyUuid>,
  /// Each ear of the host, by the id of its object in the sandbox: its name and where it stands.
  standing: HashMap<MontyUuid, (String, At)>,
  /// What each ear of the host waits for since its last step, by the id of its object in the sandbox.
  waits: HashMap<MontyUuid, Object>,
  /// How many ids the host gave, which names the next.
  given: u128,
  /// Each act that an ear of the host said started and that is not done: the work of the outside that runs now.
  working: HashSet<String>,
}

impl Default for Outside {
  /// The side of the host of a life of nothing, which stands in for one that is lent to code of a host.
  fn default() -> Outside {
    Outside {
      handed: Handed::default(),
      waker: Waker::noop().clone(),
      opened: Opened::of(|_| Object::none()),
      names: HashMap::new(),
      named: HashMap::new(),
      made: HashMap::new(),
      numbers: HashMap::new(),
      kept: HashSet::new(),
      standing: HashMap::new(),
      waits: HashMap::new(),
      given: 0,
      working: HashSet::new(),
    }
  }
}

impl Host for Outside {
  /// One call of the sandbox, answered: the gate, a function of the host, or a step of an ear of the host.
  fn answer(
    &mut self,
    sand: &mut Sand,
    on: Option<MontyUuid>,
    name: &str,
    args: Vec<Object>,
  ) -> Answer {
    let got = match on {
      None if name == GATE => gated(args.first()),
      None => self.called(sand, name, &args),
      Some(on) if name == "send" => return self.sent(sand, on, args.first()),
      Some(on) => Err(Fault::refused(format!("{on} is no generator of the host that has {name}"))),
    };
    got.map_or_else(Answer::Abort, Answer::Value)
  }
}

/// What the gate found in a sheet, each finding by its line.
fn gated(sheet: Option<&Object>) -> Result<Object, Fault> {
  let sheet = sheet.and_then(|one| one.as_ref().as_str().map(str::to_owned)).unwrap_or_default();
  let found = checked(&sheet)?;
  Ok(Object::list(found.into_iter().map(|(line, why)| {
    Object::tuple([Object::int(i64::try_from(line).unwrap_or_default()), Object::string(why)])
  })))
}

impl Outside {
  /// The side of the host, on what the opening gave.
  fn new(given: Handed, opened: &Object) -> Outside {
    let got = opened.as_ref();
    let at = |i: usize| {
      entry(&got, i).map(|one| one.to_owned()).expect("the opening gives fourteen callables")
    };
    let opened = Opened::of(at);
    let kept = (0..14).filter_map(|i| at(i).as_ref().handle()).collect();
    Outside { handed: given, opened, kept, ..Outside::default() }
  }

  /// A new id of the host, for an object of the host or for an instance made again in the sandbox.
  fn fresh(&mut self) -> MontyUuid {
    self.given += 1;
    id(self.given)
  }

  /// The names of the engine that are callable, read where they stand now. Each is the engine's to hold.
  fn bound(&mut self, sand: &mut Sand) -> Result<(), Fault> {
    let names = self.opened.names.clone();
    let got = sand.call(self, &names, vec![], vec![])?;
    self.names.clear();
    self.named.clear();
    for (name, one) in got.as_ref().pairs().unwrap_or_default() {
      let (Some(name), one) = (name.as_str(), one.to_owned()) else { continue };
      if let Some(handle) = one.as_ref().handle() {
        self.kept.insert(handle);
        self.named.entry(handle).or_insert_with(|| name.to_owned());
      }
      self.names.insert(name.to_owned(), one);
    }
    Ok(())
  }

  /// A name of the engine, called with values of the sandbox where it stands now, and what it gave.
  fn named_call(
    &mut self,
    sand: &mut Sand,
    name: &str,
    mut args: Vec<Object>,
    kwargs: Vec<(&str, Object)>,
  ) -> Result<Object, Fault> {
    args.insert(0, Object::string(name));
    let call = self.opened.call.clone();
    sand.call(self, &call, args, kwargs)
  }

  /// The words of a call, values of the host, as they come in.
  fn inwards<'a>(
    &mut self,
    sand: &mut Sand,
    args: Vec<Object>,
    kwargs: Vec<(&'a str, Object)>,
  ) -> Result<Words<'a>, Fault> {
    let args = args.iter().map(|one| self.inward(sand, one)).collect::<Result<Vec<_>, _>>()?;
    let mut named = Vec::new();
    for (key, one) in kwargs {
      named.push((key, self.inward(sand, &one)?));
    }
    Ok((args, named))
  }

  /// One verb of the engine, said by its name with values of the host, and what it gave, as it goes out.
  fn verb(
    &mut self,
    sand: &mut Sand,
    name: &str,
    args: Vec<Object>,
    kwargs: Vec<(&str, Object)>,
  ) -> Result<Object, Fault> {
    let (args, kwargs) = self.inwards(sand, args, kwargs)?;
    let got = self.named_call(sand, name, args, kwargs)?;
    self.outward(sand, &got)
  }

  /// One callable the engine made, called back by the number it went out under.
  fn made(
    &mut self,
    sand: &mut Sand,
    n: i64,
    args: Vec<Object>,
    kwargs: Vec<(&str, Object)>,
  ) -> Result<Object, Fault> {
    let made = self.held(n)?;
    let (args, kwargs) = self.inwards(sand, args, kwargs)?;
    let got = sand.call(self, &made, args, kwargs)?;
    self.outward(sand, &got)
  }

  /// Who speaks in the life, and who speaks from now on when a value is given.
  fn site(&mut self, sand: &mut Sand, value: Option<&Object>) -> Result<Object, Fault> {
    let (site, speaks) = (self.opened.site.clone(), self.opened.speaks.clone());
    let previous = sand.call(self, &site, vec![], vec![])?;
    if let Some(value) = value.filter(|one| one.as_ref().type_name() != "NoneType") {
      let by = value.as_ref().as_str().map_or_else(|| value.py_repr(), str::to_owned);
      let token = sand.call(self, &speaks, vec![Object::string(by)], vec![])?;
      if let Some(handle) = token.as_ref().handle() {
        self.drop_handle(sand, &handle);
      }
    }
    Ok(previous)
  }

  /// What an act came to once it is done, as it goes out, and nothing while it lives.
  fn outcome(&mut self, sand: &mut Sand, id: &str) -> Result<Option<Object>, Fault> {
    let got =
      self.named_call(sand, "peek", vec![Object::string(id), Object::ellipsis()], vec![])?;
    if got.as_ref().type_name() == "ellipsis" {
      return Ok(None);
    }
    self.outward(sand, &got).map(Some)
  }

  /// What the first ear that stands at a yield says of its own accord, in the order the host gave the ears, which
  /// the ids the host gives keep; and nothing when no ear says anything. A poll calls nothing in the sandbox, so an
  /// ear may be polled while no life hears it.
  fn spoken(&mut self) -> Option<(String, Spoken)> {
    let mut standing: Vec<_> = self
      .standing
      .iter()
      .filter(|(_, (_, at))| matches!(at, At::Waiting))
      .map(|(id, (name, _))| (*id, name.clone()))
      .collect();
    standing.sort_unstable();
    let mut cx = Context::from_waker(&self.waker);
    standing.into_iter().find_map(|(_, name)| {
      let ear = self.handed.ears.get_mut(&name)?;
      match ear.poll(&mut cx) {
        Poll::Ready(spoken) => Some((name, spoken)),
        Poll::Pending => None,
      }
    })
  }

  /// What the ears say of their own accord, said into the life, each under the name of its ear, from what one ear
  /// said first until no ear says more: a saying, or a verb, whose value goes nowhere. The first that raises ends it.
  fn said(&mut self, sand: &mut Sand, first: (String, Spoken)) -> Result<(), Fault> {
    let (speaks, spoke) = (self.opened.speaks.clone(), self.opened.spoke.clone());
    let mut next = Some(first);
    while let Some((by, spoken)) = next {
      let token = sand.call(self, &speaks, vec![Object::string(&by)], vec![])?;
      let got = match spoken {
        Spoken::Saying(saying) => self.says(sand, &saying),
        // What the verb came to goes back to the ear, which takes it at its next poll.
        Spoken::Verb(call) => {
          let got = self.asked(sand, call);
          if let Some(ear) = self.handed.ears.get_mut(&by) {
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
  fn says(&mut self, sand: &mut Sand, saying: &Fact) -> Result<(), Fault> {
    let words = self.inward(sand, &saying.0)?;
    let words =
      words.as_ref().items().unwrap_or_default().into_iter().map(|one| one.to_owned()).collect();
    self.named_call(sand, "say", words, vec![]).map(drop)
  }

  /// A value of the sandbox that the host holds no longer, let go of by its handle, unless the engine holds it for
  /// itself or the host holds it by a number.
  fn drop_handle(&mut self, sand: &mut Sand, handle: &MontyUuid) {
    if !self.kept.contains(handle) && !self.numbers.contains_key(handle) {
      sand.release(handle);
    }
  }

  /// A callable the engine made or a class a word defined, by the number the host holds it by.
  fn held(&self, n: i64) -> Result<Object, Fault> {
    self.made.get(&n).cloned().ok_or_else(|| Fault::new("KeyError", vec![Object::int(n)]))
  }

  /// A callable the engine made or a class a word defined, forgotten by the host, so the sandbox holds it no more.
  fn forget(&mut self, sand: &mut Sand, n: i64) {
    if let Some(made) = self.made.remove(&n)
      && let Some(handle) = made.as_ref().handle()
    {
      self.numbers.remove(&handle);
      sand.release(&handle);
    }
  }

  /// The number a callable or a class goes out under, held until the host forgets it. No two lives of one process
  /// give the same number, since a door holds what crossed by its number beyond the life it crossed from.
  fn number(&mut self, handle: MontyUuid, held: &Object) -> i64 {
    if let Some(n) = self.numbers.get(&handle) {
      return *n;
    }
    let n = NUMBERS.fetch_add(1, Ordering::Relaxed);
    self.numbers.insert(handle, n);
    self.made.insert(n, held.clone());
    n
  }

  /// A function of the host, called back by the sandbox: what it is given goes out as any value goes out, and what
  /// it gives comes in as any value comes in, so a function that gives an ear gives the object that stands in for it.
  fn called(&mut self, sand: &mut Sand, name: &str, args: &[Object]) -> Result<Object, Fault> {
    let args = args.iter().map(|one| self.outward(sand, one)).collect::<Result<Vec<_>, _>>()?;
    let missing = || Fault::refused(format!("{name} is no function of the host"));
    let mut call = self.handed.calls.remove(name).ok_or_else(missing)?;
    let got = self.lent(sand, |life| call(args, life));
    self.handed.calls.insert(name.to_owned(), call);
    self.inward(sand, &got?)
  }

  /// One step of an ear of the host, which the sandbox holds as an object that stands in for a generator.
  ///
  /// At its birth the ear starts. Then every fact it is given goes to the ear, and what comes back is what the ear did
  /// with it. It said something, which the object yields, and the bus hands back the fact as it was said, which goes to
  /// the ear next. It said nothing, so the object waits for the next fact. It said a verb, which is said here by its
  /// name and its value handed back. It is over, so the object stops, and it raised, so the object raises. A generator
  /// that the host started before it crossed was born there, so its object begins where it waits.
  fn sent(&mut self, sand: &mut Sand, on: MontyUuid, a: Option<&Object>) -> Answer {
    let Some((name, at)) = self.standing.get(&on).cloned() else {
      return Answer::Abort(Fault::refused(format!("{on} is no object of the host")));
    };
    let a = a.filter(|one| one.as_ref().type_name() != "NoneType");
    let heard = match (at, a) {
      (At::Over, _) => return Answer::Fault(Fault::new("StopIteration", vec![])),
      (At::Unborn, Some(_)) => {
        return Answer::Fault(Fault::new(
          "TypeError",
          vec![Object::string("can't send non-None value to a just-started generator")],
        ));
      }
      (At::Unborn, None) => Heard::Born,
      (At::Waiting, None) => {
        return Answer::Value(self.waits.get(&on).cloned().unwrap_or_else(Object::none));
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
    let Some(mut ear) = self.handed.ears.remove(&name) else {
      return Answer::Abort(Fault::refused(format!("no ear of the host is named {name}")));
    };
    let waker = self.waker.clone();
    let mut heard = heard;
    // Each verb the ear calls is said here, under its name, and what it came to is the next thing it hears.
    let (at, answer) = loop {
      match self.lent(sand, |life| ear.lent(heard, &mut Context::from_waker(&waker), life)) {
        Step::Call(call) => heard = Heard::Answer(self.asked(sand, call)),
        Step::Say(saying) => {
          break (
            At::Waiting,
            self.inward(sand, &saying.0).map_or_else(Answer::Abort, Answer::Value),
          );
        }
        Step::Wait(wait) => {
          let wait = wait.unwrap_or_else(Object::none);
          self.waits.insert(on, wait.clone());
          break (At::Waiting, Answer::Value(wait));
        }
        Step::Over => break (At::Over, Answer::Fault(Fault::new("StopIteration", vec![]))),
        Step::Raised(fault) => break (At::Over, self.raising(sand, fault)),
      }
    };
    self.handed.ears.insert(name.clone(), ear);
    self.standing.insert(on, (name, at));
    answer
  }

  /// The work of the outside that a fact changes: an act that an ear of the host said started runs until it is done.
  fn track(&mut self, fact: &Fact) {
    match fact.kind() {
      "started" if self.standing.values().any(|(name, _)| name == fact.by()) => {
        self.working.insert(fact.about().to_owned());
      }
      "done" => {
        self.working.remove(fact.about());
      }
      _ => {}
    }
  }

  /// The side of the host of a life that a snapshot holds, on the ears that the host gives again: each ear stands
  /// under the id and at the place where its earlier ear stood, and the host holds what the engine held.
  fn restored(handed: Handed, table: &Table) -> Result<Outside, Fault> {
    let at = |i: usize| table.opened.get(i).cloned().unwrap_or_else(Object::none);
    let mut standing = HashMap::new();
    for (id, name, waits) in &table.ears {
      if !handed.ears.contains_key(name) {
        return Err(Fault::refused(format!(
          "the snapshot holds the ear {name}, which the host does not give"
        )));
      }
      standing.insert(*id, (name.clone(), if *waits { At::Waiting } else { At::Over }));
    }
    let given =
      table.given.parse().map_err(|_| Fault::refused("a snapshot counts the ids of the host"))?;
    let kept = table.kept.iter().copied().collect();
    Ok(Outside { handed, opened: Opened::of(at), kept, standing, given, ..Outside::default() })
  }

  /// Each ear that waits, in the order of its id, as the host gives it.
  fn waiting(&self) -> Vec<(MontyUuid, String)> {
    let mut ears: Vec<_> = self
      .standing
      .iter()
      .filter(|(_, (_, at))| *at == At::Waiting)
      .map(|(id, (name, _))| (*id, name.clone()))
      .collect();
    ears.sort_unstable();
    ears
  }

  /// One fact or the birth, heard by one ear of the host outside the log: each verb it calls is answered, by the life
  /// when `views` holds it or when no view is asked for, and by nothing otherwise, and each saying comes back to it as
  /// the fact that the bus would make of it, with nothing said into the life. What it waits for after is kept as a
  /// wait of the ear is kept. It gives whether the ear waits after, and refuses an ear that says something when
  /// `speaks` refuses it.
  fn told(
    &mut self,
    sand: &mut Sand,
    (on, name): &(MontyUuid, String),
    heard: Heard,
    views: Option<&[&str]>,
    speaks: bool,
  ) -> Result<bool, Fault> {
    let Some(mut ear) = self.handed.ears.remove(name) else { return Ok(false) };
    let waker = self.waker.clone();
    let mut heard = heard;
    let waits = loop {
      match self.lent(sand, |life| ear.lent(heard, &mut Context::from_waker(&waker), life)) {
        Step::Call(call) if views.is_none_or(|views| views.contains(&call.verb.as_str())) => {
          heard = Heard::Answer(self.asked(sand, call));
        }
        Step::Call(_) => heard = Heard::Answer(Ok(Object::none())),
        Step::Say(saying) if speaks => {
          let held = saying.0.as_ref().items().unwrap_or_default();
          let mut made: Vec<Object> = held.iter().take(2).map(|one| one.to_owned()).collect();
          made.push(Object::string(name));
          made.extend(held.iter().skip(2).map(|one| one.to_owned()));
          heard = Heard::Fact(Fact(Object::tuple(made)));
        }
        Step::Say(_) => {
          self.handed.ears.insert(name.clone(), ear);
          return Err(Fault::refused(format!(
            "the ear {name} speaks at its birth, so no snapshot opens it"
          )));
        }
        Step::Wait(wait) => {
          self.waits.insert(*on, wait.unwrap_or_else(Object::none));
          break true;
        }
        Step::Over | Step::Raised(_) => break false,
      }
    };
    self.handed.ears.insert(name.clone(), ear);
    if !waits {
      self.standing.insert(*on, (name.clone(), At::Over));
    }
    Ok(waits)
  }

  /// The ears that the host gives again, each born and then told the past of the life, as a replay of the record
  /// tells it: each fact of the log that is no act, but the birth of a chain, to each ear that waits for it, with the
  /// journal as the speaker of each fact that no act said, since the journal says it again. What an ear says or makes
  /// from the past reaches nothing, as the journal answers it in a replay, and the log, which holds the past, hears
  /// none of it.
  fn retold(&mut self, sand: &mut Sand, past: &[Object]) -> Result<(), Fault> {
    for one in self.waiting() {
      self.told(sand, &one, Heard::Born, None, false)?;
    }
    let mut acts: HashMap<String, bool> = HashMap::new();
    let mut told = Vec::new();
    for one in past {
      let Some(fact) = Fact::of(one.as_ref()) else { continue };
      if fact.question() && fact.kind() != "chain" {
        continue;
      }
      let by = fact.by().to_owned();
      let act = match acts.get(&by) {
        Some(act) => *act,
        None => {
          let got = self.named_call(sand, "get", vec![Object::string(&by)], vec![])?;
          let act = got.as_ref().type_name() != "NoneType";
          acts.insert(by.clone(), act);
          act
        }
      };
      let held = fact.0.as_ref().items().unwrap_or_default();
      let speaker =
        |i: usize, one: &crate::value::ObjectRef<'_>| match i == 2 && !act && !fact.question() {
          true => Object::string("journal"),
          false => one.to_owned(),
        };
      told.push(Object::tuple(
        held.iter().enumerate().map(|(i, one)| speaker(i, one)).collect::<Vec<_>>(),
      ));
    }
    let ears: Vec<_> = self.waiting().into_iter().filter(|(_, name)| name != "log").collect();
    let waits = ears.iter().map(|(on, name)| {
      Object::tuple([
        Object::string(name),
        self.waits.get(on).cloned().unwrap_or_else(Object::none),
      ])
    });
    let names = Object::dict([
      (Object::string("past"), Object::list(told.clone())),
      (Object::string("w"), Object::list(waits.collect::<Vec<_>>())),
    ]);
    let (word, names) = (self.opened.word.clone(), self.inward(sand, &names)?);
    let hearing = sand.call(self, &word, vec![Object::string(HEARING), names], vec![])?;
    let hearing = self.outward(sand, &hearing)?;
    let hearing = hearing.as_ref().items().unwrap_or_default();
    for (fact, names) in told.into_iter().zip(hearing) {
      let names = names.items().unwrap_or_default();
      let names: Vec<&str> = names.iter().filter_map(|one| one.as_str()).collect();
      for one in ears.iter().filter(|(_, name)| names.contains(&name.as_str())) {
        self.told(sand, one, Heard::Fact(Fact(fact.clone())), Some(&VIEWS), true)?;
      }
    }
    Ok(())
  }

  /// What the host does in `step`, with this life lent to it, which answers at once each verb that the host calls
  /// while it runs, since the sandbox takes a call before the call of the sandbox that the host answers.
  ///
  /// Every frame of the life waits until `step` is done, so the life goes to it by value, with a life of nothing in
  /// its place, and comes back with what `step` gave.
  fn lent<R>(&mut self, sand: &mut Sand, step: impl FnOnce(Life) -> (R, Life)) -> R {
    let life = Life { sand: std::mem::replace(sand, Sand::none()), outside: std::mem::take(self) };
    let (got, life) = step(life);
    (*sand, *self) = (life.sand, life.outside);
    got
  }

  /// What the host handed over under this name, which this life holds from now on.
  fn take(&mut self, name: &str) {
    HANDED.with_borrow_mut(|held| {
      if let Some(ear) = held.ears.remove(name) {
        self.handed.ears.insert(name.to_owned(), ear);
      }
      if let Some(call) = held.calls.remove(name) {
        self.handed.calls.insert(name.to_owned(), call);
      }
    });
  }

  /// What the host calls while the life waits on it, answered at once: who speaks, a callable the engine made, or a
  /// verb of the engine, as it goes out.
  fn asked(&mut self, sand: &mut Sand, call: Call) -> Result<Object, Fault> {
    let Call { verb, args, kwargs } = call;
    let kwargs: Vec<_> = kwargs.iter().map(|(key, one)| (key.as_str(), one.clone())).collect();
    match verb.as_str() {
      "spoken" => self.site(sand, args.first()),
      "made" => {
        let n = args.first().and_then(|one| one.as_ref().as_int());
        let n = n.ok_or_else(|| {
          Fault::refused("a callable the engine made is called back by its number")
        })?;
        let words = args.get(1).and_then(|one| one.as_ref().items()).unwrap_or_default();
        let words = words.into_iter().map(|one| one.to_owned()).collect();
        let pairs = args.get(2).and_then(|one| one.as_ref().pairs()).unwrap_or_default();
        let named =
          pairs.iter().filter_map(|(key, one)| Some((key.as_str()?, one.to_owned()))).collect();
        self.made(sand, n, words, named)
      }
      _ => self.verb(sand, &verb, args, kwargs),
    }
  }

  /// What the host raised, raised in the sandbox as the exception it is: a builtin one by its name, and one of the
  /// engine made from what it was made with.
  fn raising(&mut self, sand: &mut Sand, fault: Fault) -> Answer {
    if fault.name.parse::<ExcType>().is_ok() || !self.names.contains_key(&fault.name) {
      return Answer::Fault(fault);
    }
    let made = fault.args.iter().map(|one| self.inward(sand, one)).collect::<Result<Vec<_>, _>>();
    match made.and_then(|args| self.named_call(sand, &fault.name, args, vec![])) {
      Ok(exception) => Answer::Raise(exception),
      Err(fault) => Answer::Abort(fault),
    }
  }

  /// A value as it goes out to the host. A value with no data form that is not callable is let go of once the whole
  /// value went out, since the value may hold it more than once.
  fn outward(&mut self, sand: &mut Sand, value: &Object) -> Result<Object, Fault> {
    let (graph, root) = unstable::into_graph_parts(value.clone());
    let mut shown = HashMap::new();
    let got = self.goes(sand, &graph, root, &mut shown);
    for held in shown.into_keys() {
      self.drop_handle(sand, &held);
    }
    got
  }

  /// One node of a value as it goes out: a callable or a class of the engine as its name, a callable the engine
  /// made by the number the host holds it by, a class a word defined by that number with the class it was made with
  /// as it goes out, an instance of such a class with its fields under its class, a map that holds the key `is` as
  /// its pairs, and the entries of a container each as they go out. Anything else goes out as the interpreter
  /// carries it, but a value with no data form that is not callable, which goes out as its representation.
  fn goes(
    &mut self,
    sand: &mut Sand,
    graph: &MontyGraph,
    id: NodeId,
    shown: &mut HashMap<MontyUuid, Object>,
  ) -> Result<Object, Fault> {
    let key = |one: &NodeId| graph.value(*one).to_owned();
    Ok(match graph.node(id) {
      MontyNode::Dict(pairs)
        if pairs.iter().any(|(one, _)| graph.value(*one).as_str() == Some(IS)) =>
      {
        let mut held = Vec::new();
        for (one, value) in pairs {
          held.push(Object::tuple([key(one), self.goes(sand, graph, *value, shown)?]));
        }
        marked("dict", [("args", Object::list([Object::list(held)]))])
      }
      MontyNode::Dict(pairs) => {
        let mut held = Vec::new();
        for (one, value) in pairs {
          held.push((key(one), self.goes(sand, graph, *value, shown)?));
        }
        Object::dict(held)
      }
      MontyNode::List(held) => Object::list(self.going(sand, graph, held, shown)?),
      MontyNode::Tuple(held) => Object::tuple(self.going(sand, graph, held, shown)?),
      MontyNode::Handle { id: handle, .. } => {
        let held = graph.value(id).to_owned();
        if let Some(name) = self.named.get(handle) {
          return Ok(marked("name", [("name", Object::string(name))]));
        }
        if let Some(n) = self.numbers.get(handle) {
          return Ok(marked("made", [("id", Object::int(*n))]));
        }
        if let Some(repr) = shown.get(handle) {
          return Ok(repr.clone());
        }
        let shown_of = self.opened.shown.clone();
        let repr = sand.call(self, &shown_of, vec![held.clone()], vec![])?;
        if let Some(repr) = repr.as_ref().as_str() {
          let repr = Object::repr(repr);
          shown.insert(*handle, repr.clone());
          return Ok(repr);
        }
        marked("made", [("id", Object::int(self.number(*handle, &held)))])
      }
      MontyNode::ClassType(class) if !class.host_defined => {
        self.class(sand, graph.value(id).to_owned(), class.id, &class.name)?
      }
      MontyNode::ClassInstance { class_type, .. } => match graph.node(*class_type) {
        MontyNode::ClassType(class)
          if !class.host_defined && !self.named.contains_key(&class.id) =>
        {
          let class =
            self.class(sand, graph.value(*class_type).to_owned(), class.id, &class.name)?;
          marked("instance", [("class", class), ("value", graph.value(id).to_owned())])
        }
        _ => graph.value(id).to_owned(),
      },
      _ => graph.value(id).to_owned(),
    })
  }

  /// The entries of a container, each as it goes out.
  fn going(
    &mut self,
    sand: &mut Sand,
    graph: &MontyGraph,
    held: &[NodeId],
    shown: &mut HashMap<MontyUuid, Object>,
  ) -> Result<Vec<Object>, Fault> {
    held.iter().map(|one| self.goes(sand, graph, *one, shown)).collect()
  }

  /// A class of the sandbox as it goes out: a class of the engine as its name, and a class a word defined as the
  /// number the host holds it by, with its name and the class it was made with as it goes out.
  fn class(
    &mut self,
    sand: &mut Sand,
    class: Object,
    handle: MontyUuid,
    name: &str,
  ) -> Result<Object, Fault> {
    if let Some(name) = self.named.get(&handle) {
      return Ok(marked("name", [("name", Object::string(name))]));
    }
    let n = self.number(handle, &class);
    let base_of = self.opened.base.clone();
    let base = sand.call(self, &base_of, vec![class], vec![])?;
    let base = self.outward(sand, &base)?;
    Ok(marked("class", [("id", Object::int(n)), ("name", Object::string(name)), ("base", base)]))
  }

  /// A value of the host as the engine holds it, made in the sandbox from what crossed.
  fn inward(&mut self, sand: &mut Sand, value: &Object) -> Result<Object, Fault> {
    let (graph, root) = unstable::into_graph_parts(inward(value));
    self.comes(sand, &graph, root)
  }

  /// One node of a value as it comes in: a map that names what it is under `is` made again as that, and the entries
  /// of a container each as they come in. Anything else comes in as it is.
  fn comes(&mut self, sand: &mut Sand, graph: &MontyGraph, id: NodeId) -> Result<Object, Fault> {
    Ok(match graph.node(id) {
      MontyNode::Dict(pairs) => {
        let mark = pairs.iter().find(|(one, _)| graph.value(*one).as_str() == Some(IS));
        if let Some(mark) = mark.and_then(|(_, one)| graph.value(*one).as_str().map(str::to_owned))
          && let Some(made) = self.again(sand, graph, &mark, pairs)?
        {
          return Ok(made);
        }
        let mut held = Vec::new();
        for (key, one) in pairs {
          held.push((graph.value(*key).to_owned(), self.comes(sand, graph, *one)?));
        }
        Object::dict(held)
      }
      MontyNode::List(held) => Object::list(self.coming(sand, graph, held)?),
      MontyNode::Tuple(held) => Object::tuple(self.coming(sand, graph, held)?),
      MontyNode::Function { name, .. } => {
        self.take(name);
        graph.value(id).to_owned()
      }
      _ => graph.value(id).to_owned(),
    })
  }

  /// The entries of a container, each as it comes in.
  fn coming(
    &mut self,
    sand: &mut Sand,
    graph: &MontyGraph,
    held: &[NodeId],
  ) -> Result<Vec<Object>, Fault> {
    held.iter().map(|one| self.comes(sand, graph, *one)).collect()
  }

  /// What a map that names what it is under `is` is made again as, or nothing when it names nothing the engine
  /// makes, so it comes in as the map it is.
  fn again(
    &mut self,
    sand: &mut Sand,
    graph: &MontyGraph,
    mark: &str,
    pairs: &[(NodeId, NodeId)],
  ) -> Result<Option<Object>, Fault> {
    let at = |name: &str| {
      pairs.iter().find(|(key, _)| graph.value(*key).as_str() == Some(name)).map(|(_, one)| *one)
    };
    let text = |name: &str| at(name).and_then(|one| graph.value(one).as_str().map(str::to_owned));
    let number = |name: &str| at(name).and_then(|one| graph.value(one).as_int());
    match mark {
      // A name of the engine crosses as its name, so a show of the engine is the engine's own here.
      "name" if let Some(held) = text("name").and_then(|name| self.names.get(&name).cloned()) => {
        Ok(Some(held))
      }
      // A callable the engine made, or a class a word defined, back from the host by the number it went out under.
      "made" if let Some(n) = number("id") => self.held(n).map(Some),
      // An instance of a class a word defined, back from the host by the number of its class and its fields, made
      // here from them as the interpreter makes one, with no `__init__` run.
      "instance" if let (Some(n), Some(fields)) = (number("class"), at("fields")) => {
        let class = self.held(n)?;
        let MontyNode::Dict(fields) = graph.node(fields) else { return Ok(None) };
        let mut held = Vec::new();
        for (key, one) in fields {
          held.push((graph.value(*key).to_owned(), self.comes(sand, graph, *one)?));
        }
        Ok(Some(Object::class_instance(class, self.fresh(), held)))
      }
      "ear"
        if let (Some(name), Some(started)) =
          (text("name"), at("started").and_then(|one| graph.value(one).as_bool())) =>
      {
        self.take(&name);
        let on = self.fresh();
        self.standing.insert(on, (name, if started { At::Waiting } else { At::Unborn }));
        Ok(Some(object("Ear", on)))
      }
      "Templated" if let Some(MontyNode::List(held)) = at("parts").map(|one| graph.node(one)) => {
        let mut each = Vec::new();
        for one in held {
          let (MontyNode::List(part) | MontyNode::Tuple(part)) = graph.node(*one) else {
            each.push(graph.value(*one).to_owned());
            continue;
          };
          let [value, expression, conversion, spec] = part[..] else { return Ok(None) };
          each.push(Object::tuple([
            self.comes(sand, graph, value)?,
            graph.value(expression).to_owned(),
            graph.value(conversion).to_owned(),
            graph.value(spec).to_owned(),
          ]));
        }
        let template = self.opened.template.clone();
        sand.call(self, &template, vec![Object::list(each)], vec![]).map(Some)
      }
      _ => self.made_of(sand, graph, mark, pairs),
    }
  }

  /// An instance of a class of the engine or of a builtin class, made from its name, what it was made with, and its
  /// fields. An exception takes what it was made with by position, and by position alone: the interpreter makes a
  /// builtin one from at most one string. A dataclass takes its fields by name.
  fn made_of(
    &mut self,
    sand: &mut Sand,
    graph: &MontyGraph,
    mark: &str,
    pairs: &[(NodeId, NodeId)],
  ) -> Result<Option<Object>, Fault> {
    let builtin =
      matches!(mark, "int" | "float" | "dict" | "tuple") || mark.parse::<ExcType>().is_ok();
    if !builtin && !self.names.contains_key(mark) {
      return Ok(None);
    }
    let mut args = None;
    let mut fields = Vec::new();
    for (key, one) in pairs {
      match graph.value(*key).as_str() {
        Some(IS) => {}
        Some("args") => args = Some(self.comes(sand, graph, *one)?),
        Some(key) => fields.push((key.to_owned(), self.comes(sand, graph, *one)?)),
        None => return Ok(None),
      }
    }
    let args: Vec<Object> = args
      .as_ref()
      .and_then(|one| one.as_ref().items())
      .map(|held| held.into_iter().map(|one| one.to_owned()).collect())
      .unwrap_or_default();
    if self.names.contains_key(mark) {
      let named = fields.iter().map(|(key, one)| (key.as_str(), one.clone())).collect();
      return self.named_call(sand, mark, args, named).map(Some);
    }
    let first = args.first().map(Object::as_ref);
    Ok(match mark {
      "int" => match first.and_then(|one| one.as_str().map(str::to_owned)) {
        Some(text) => Some(match text.trim().parse::<i64>() {
          Ok(n) => Object::int(n),
          Err(_) => Object::bigint(
            text
              .trim()
              .parse()
              .map_err(|_| Fault::new("ValueError", vec![Object::string(text)]))?,
          ),
        }),
        None => first.map(|one| one.to_owned()),
      },
      "float" => match first.and_then(|one| one.as_str().map(str::to_owned)) {
        Some(text) => Some(Object::float(
          text.trim().parse().map_err(|_| Fault::new("ValueError", vec![Object::string(text)]))?,
        )),
        None => first
          .and_then(|one| one.as_float().or_else(|| one.as_int().map(|n| n as f64)))
          .map(Object::float),
      },
      "dict" => {
        let pairs = first.and_then(|one| one.items()).unwrap_or_default();
        let pairs = pairs.into_iter().map(|pair| entry(&pair, 0).zip(entry(&pair, 1)));
        let pairs = pairs.map(|pair| pair.map(|(key, one)| (key.to_owned(), one.to_owned())));
        let pairs = pairs.collect::<Option<Vec<_>>>();
        Some(Object::dict(
          pairs.ok_or_else(|| Fault::refused("a map comes in as its pairs of two"))?,
        ))
      }
      "tuple" => {
        let items = first.and_then(|one| one.items()).unwrap_or_default();
        Some(Object::tuple(items.into_iter().map(|one| one.to_owned())))
      }
      _ => mark.parse::<ExcType>().ok().map(|kind| {
        Object::exception(
          kind,
          first.map(|one| one.as_str().map_or_else(|| one.py_repr(), str::to_owned)),
        )
      }),
    })
  }
}

/// What a snapshot holds of the side of the host beside the session of the sandbox: the root, each ear of the host by
/// the id of its object in the sandbox, its name and whether it waits, the handles that the engine keeps, the callables
/// of the opening, and how many ids the host gave.
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct Table {
  pub(crate) root: String,
  pub(crate) ears: Vec<(MontyUuid, String, bool)>,
  pub(crate) kept: Vec<MontyUuid>,
  pub(crate) opened: Vec<Object>,
  pub(crate) given: String,
}

/// One life of the engine, as a host holds it.
///
/// One host holds one engine and says one thing at a time, as an operator does: its methods are the verbs of the
/// contract, and an act borrows the engine while it is awaited, since awaiting it drives the engine.
pub struct Engine {
  sand: Sand,
  outside: Outside,
  /// Who waits on an act, by its name, told once when it is done.
  watchers: HashMap<String, Vec<Watcher>>,
  root: String,
  raised: Option<Fault>,
  /// The acts that the record showed started and not done, which the journal holds from the outside and which this
  /// life has not taken up yet, in the order of the record.
  holding: Vec<String>,
  /// The acts that the record parks and that this life has not retried, by name and kind, in the order of the record.
  parked: Vec<(String, String)>,
  /// Where the life keeps its snapshot, when it keeps one.
  keeper: Option<Keeper>,
  /// Whether the life opened from a snapshot.
  restored: bool,
}

impl Engine {
  /// One entry into the sandbox, and then whoever watches an act that is done now is told, even when the entry
  /// raised.
  fn run<T>(
    &mut self,
    body: impl FnOnce(&mut Sand, &mut Outside) -> Result<T, Fault>,
  ) -> Result<T, Fault> {
    let got = match self.fed(body) {
      Ok(got) => got,
      Err(fault) => self.told().and(Err(fault)),
    };
    self.kept_now(false);
    got
  }

  /// Whoever watches an act that is done now is told.
  fn told(&mut self) -> Result<(), Fault> {
    if self.watchers.is_empty() {
      return Ok(());
    }
    self.fed(|_, _| Ok(()))?
  }

  /// One entry into the sandbox: its body, then what every watched act came to, read in the same entry, since the
  /// session holds a name for each entry until the life ends. Each watcher of an act that is done is told once. The
  /// entry fails when its run ended, and gives what its body gave otherwise.
  fn fed<T>(
    &mut self,
    body: impl FnOnce(&mut Sand, &mut Outside) -> Result<T, Fault>,
  ) -> Result<Result<T, Fault>, Fault> {
    let ids: Vec<String> = self.watchers.keys().cloned().collect();
    let (got, outcomes) = self.sand.enter(&mut self.outside, |sand, outside| {
      let got = body(sand, outside);
      let outcomes = ids.iter().map(|id| outside.outcome(sand, id)).collect::<Result<Vec<_>, _>>();
      (got, outcomes)
    })?;
    for (id, done) in ids.iter().zip(outcomes?) {
      let Some(value) = done else { continue };
      for mut watcher in self.watchers.remove(id).unwrap_or_default() {
        watcher(&value);
      }
    }
    Ok(got)
  }

  /// The engine driven: each ear is polled with the waker of whoever drives, and what the ears say of their own accord
  /// is said into it, in one entry, which it makes only when an ear says something. An awaited act drives the engine
  /// so, and a door that awaits in the loop of its own language drives it so too.
  pub(crate) fn pump(&mut self, waker: &Waker) -> Result<(), Fault> {
    if !self.outside.waker.will_wake(waker) {
      self.outside.waker = waker.clone();
    }
    let Some(first) = self.outside.spoken() else { return Ok(()) };
    self.run(|sand, outside| outside.said(sand, first))
  }

  /// An engine, opened from the record, on these ears, each under the name the engine hears it by, in the order
  /// the engine offers them a question.
  ///
  /// The record is the entries the store kept, each one fact. The engine, the sheet and the Kernel run first, each
  /// in a namespace of its own, and `boot` is given the Kernel and the gate of the crate before the ears of the host.
  pub fn boot<N: Into<String>>(
    record: impl IntoIterator<Item = Object>,
    ears: impl IntoIterator<Item = (N, Box<dyn Ear>)>,
  ) -> Result<Engine, Fault> {
    let mut given = Handed::default();
    let mut names = Vec::new();
    for (name, ear) in ears {
      let name = name.into();
      if given.ears.insert(name.clone(), ear).is_some() {
        return Err(Fault::refused(format!("{name} hears")));
      }
      names.push(name);
    }
    let mut sand = Sand::new();
    let mut inputs = NamedValues::new();
    inputs.push("__source", Object::string(ENGINE));
    inputs.push("__sheet_source", Object::string(SHEET));
    inputs.push("__kernel_source", Object::string(KERNEL));
    inputs.push("__gate", Object::function(GATE, None));
    let opened = sand.run(OPENING, inputs, &mut Nobody("the opening"))?;
    let outside = Outside::new(given, &opened);
    let got = opened.as_ref();
    let (kernel, gate) = (entry(&got, 2), entry(&got, 3));
    let (kernel, gate) = (
      kernel.expect("the opening gives the Kernel").to_owned(),
      gate.expect("and the gate").to_owned(),
    );
    let (parking, park) = (entry(&got, 11), entry(&got, 12));
    let (parking, park) = (
      parking.expect("the opening gives the park").to_owned(),
      park.expect("and its names").to_owned(),
    );
    let record: Vec<Object> = record.into_iter().collect();
    let (watchers, holding, parked) = (HashMap::new(), left(&record), looping(&record));
    let names_parked = Object::list(parked.iter().map(|(id, _)| Object::string(id)));
    let parks = !parked.is_empty();
    let mut engine = Engine {
      sand,
      outside,
      watchers,
      root: String::new(),
      raised: None,
      holding,
      parked,
      keeper: None,
      restored: false,
    };
    let (root, raised) = engine.run(|sand, outside| {
      outside.bound(sand)?;
      let mut entries = Vec::new();
      for one in &record {
        let one = outside.inward(sand, one)?;
        let fact = entry(&one.as_ref(), 0).and_then(|fact| fact.items());
        let fact =
          fact.ok_or_else(|| Fault::refused("a record holds each fact alone in an entry"))?;
        entries.push(Object::tuple([Object::tuple(fact.into_iter().map(|one| one.to_owned()))]));
      }
      let mut given = vec![("kernel".to_owned(), kernel), ("gate".to_owned(), gate)];
      for name in names {
        let on = outside.fresh();
        outside.standing.insert(on, (name.clone(), At::Unborn));
        given.push((name, object("Ear", on)));
      }
      if parks {
        sand.call(outside, &park, vec![names_parked], vec![])?;
        for (_, one) in &mut given {
          *one = sand.call(outside, &parking, vec![one.clone()], vec![])?;
        }
      }
      let given = given.iter().map(|(name, one)| (name.as_str(), one.clone())).collect();
      let got = outside.named_call(sand, "boot", vec![Object::list(entries)], given);
      // What boot raised comes out of the entry the operator went in by, and the life goes on: a drift breaks the
      // journal and keeps nothing more, so the root stands when the record held it.
      let opened = match got {
        Ok(root) => (root.as_ref().as_str().unwrap_or_default().to_owned(), None),
        Err(fault) => {
          let root = outside.named_call(sand, "get", vec![Object::string("chain1")], vec![])?;
          let root = if root.as_ref().type_name() == "NoneType" { "" } else { "chain1" };
          (root.to_owned(), Some(fault))
        }
      };
      // Boot binds the verbs of a life in place of those that stand before it.
      outside.bound(sand)?;
      Ok(opened)
    })?;
    engine.root = root;
    engine.raised = raised;
    Ok(engine)
  }

  /// An engine opened from the snapshot of a life on the same record, on the ears that the host gives again under the
  /// names that the snapshot holds: the session of the sandbox that the snapshot holds, with each ear born and told
  /// the past of the life, so no word runs again.
  pub(crate) fn from_snapshot<N: Into<String>>(
    record: &[Object],
    ears: impl IntoIterator<Item = (N, Box<dyn Ear>)>,
    table: &Table,
    sand: Sand,
    past: &[Object],
  ) -> Result<Engine, Fault> {
    let mut given = Handed::default();
    for (name, ear) in ears {
      let name = name.into();
      if given.ears.insert(name.clone(), ear).is_some() {
        return Err(Fault::refused(format!("{name} hears")));
      }
    }
    let outside = Outside::restored(given, table)?;
    let (holding, parked) = (left(record), looping(record));
    let mut engine = Engine {
      sand,
      outside,
      watchers: HashMap::new(),
      root: table.root.clone(),
      raised: None,
      holding,
      parked,
      keeper: None,
      restored: true,
    };
    engine.run(|sand, outside| {
      outside.bound(sand)?;
      outside.retold(sand, past)
    })?;
    Ok(engine)
  }

  /// What a snapshot holds of this life: the side of the host, and the session of the sandbox. Nothing when boot
  /// raised, when work of the outside runs, which a later life could not end, or when the sandbox holds what a later
  /// host cannot give again: a function of the host, or an ear that a verb was given.
  pub(crate) fn snapshot(&self) -> Option<(Table, Vec<u8>)> {
    let outside = &self.outside;
    let foreign =
      outside.standing.values().any(|(name, at)| name.starts_with(EAR) && *at != At::Over);
    if self.raised.is_some()
      || !outside.working.is_empty()
      || !outside.handed.calls.is_empty()
      || foreign
    {
      return None;
    }
    let dump = self.sand.dump().ok()?;
    let mut ears: Vec<_> = outside
      .standing
      .iter()
      .map(|(id, (name, at))| (*id, name.clone(), *at == At::Waiting))
      .collect();
    ears.sort_unstable_by_key(|one| one.0);
    let mut kept: Vec<_> = outside.kept.iter().copied().collect();
    kept.sort_unstable();
    let opened = outside.opened.list();
    Some((
      Table { root: self.root.clone(), ears, kept, opened, given: outside.given.to_string() },
      dump,
    ))
  }

  /// Whether the life opened from the snapshot of its record, with no replay.
  pub fn restored(&self) -> bool {
    self.restored
  }

  /// The life keeps its snapshot where the keeper says, now and as it goes on.
  pub(crate) fn keep(&mut self, keeper: Keeper) {
    self.keeper = Some(keeper);
    self.kept_now(true);
  }

  /// The snapshot of the life, written where its keeper says when one is due, or now, and only when the life said a
  /// fact since the last one, which is known before the sandbox dumps its session: a failure leaves the snapshot
  /// before it, since a snapshot is a cache.
  fn kept_now(&mut self, now: bool) {
    if !self.keeper.as_mut().is_some_and(|keeper| keeper.due(now) && keeper.grew()) {
      return;
    }
    if let Some((table, dump)) = self.snapshot()
      && let Some(keeper) = self.keeper.as_mut()
    {
      keeper.write(&table, &dump);
    }
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

  /// One verb of the engine by its name, said with these words, and what it gave.
  pub fn verb(
    &mut self,
    name: &str,
    args: Vec<Object>,
    kwargs: Vec<(&str, Object)>,
  ) -> Result<Object, Fault> {
    self.run(|sand, outside| outside.verb(sand, name, args, kwargs))
  }

  /// One word run in the names of the engine with these values bound, and what it gave, as it goes out, which is how
  /// a door reads what no verb reads.
  pub fn word(&mut self, word: &str, inputs: Vec<(&str, Object)>) -> Result<Object, Fault> {
    let names = Object::dict(inputs.into_iter().map(|(name, one)| (Object::string(name), one)));
    self.run(|sand, outside| {
      let (word_of, names) = (outside.opened.word.clone(), outside.inward(sand, &names)?);
      let got = sand.call(outside, &word_of, vec![Object::string(word), names], vec![])?;
      outside.outward(sand, &got)
    })
  }

  /// One callable the engine made, called back by the number it went out under, with these words, and what it gave.
  pub fn made(
    &mut self,
    n: i64,
    args: Vec<Object>,
    kwargs: Vec<(&str, Object)>,
  ) -> Result<Object, Fault> {
    self.run(|sand, outside| outside.made(sand, n, args, kwargs))
  }

  /// A callable the engine made, forgotten: the host holds its number no more, so the sandbox holds it no more.
  pub fn forget(&mut self, n: i64) -> Result<(), Fault> {
    self.outside.forget(&mut self.sand, n);
    Ok(())
  }

  /// Who speaks in the life, and who speaks from now on when a value is given: `site`, read and set where it stands.
  pub fn site(&mut self, value: Option<&str>) -> Result<String, Fault> {
    let value = value.map(Object::string);
    let got = self.run(|sand, outside| outside.site(sand, value.as_ref()))?;
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
      if self.parked.iter().any(|(one, _)| *one == id) || !self.whether(WAITS, &id)? {
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

  /// The work that a crash loop parked, each act by its name and its kind, in the order of the record: an act of the
  /// outside, a run or a reply, that the record shows started in [`CRASHES`] lives and not done, since each
  /// of those lives ended while it ran. A wake puts no parked act to the outside, and it is pending no more: it waits,
  /// not done, until a retry, or until a control ends it, so a host that wakes the work of a life as it opens does
  /// not run it again and end again.
  pub fn parked(&mut self) -> Result<Vec<(String, String)>, Fault> {
    let mut parked = Vec::new();
    for (id, kind) in self.parked.clone() {
      if self.whether("peek(__id, ...) is ...", &id)? {
        parked.push((id, kind));
      }
    }
    Ok(parked)
  }

  /// A parked act, run again: it is parked no more in this life, and a wake of it puts it to the outside, which takes
  /// it as at any wake, so the record shows one more life that started it. An act that is not parked is refused.
  pub fn retry(&mut self, id: &str) -> Result<(), Fault> {
    if !self.parked()?.iter().any(|(one, _)| one == id) {
      return Err(Fault::refused(format!("{id} is not parked")));
    }
    self.parked.retain(|(one, _)| one != id);
    let unpark = self.outside.opened.unpark.clone();
    self.run(|sand, outside| sand.call(outside, &unpark, vec![Object::string(id)], vec![]))?;
    self.verb("wake", vec![Object::string(id)], vec![]).map(|_| ())
  }

  /// The act of a verb that a key of the client makes once, across a crash of the host and a retry of the client. A
  /// key fact on the chain, `["key", chain, by, key, kind]`, goes to the record before the act, and the act of the key
  /// is the first question that the operator makes on that chain after it. When the chain holds such an act, it is
  /// given back and nothing is made. When it holds none, as when the host crashed after it kept the key and before it
  /// kept the act, the key is said again and the act is made. The chain is the one that `on` names, or the root.
  pub fn keyed(
    &mut self,
    key: &str,
    kind: &str,
    args: Vec<Object>,
    mut kwargs: Vec<(&str, Object)>,
  ) -> Result<String, Fault> {
    let named = kwargs.iter().find(|(name, _)| *name == "on");
    let on =
      named.and_then(|(_, one)| one.as_ref().as_str().map(str::to_owned)).unwrap_or_default();
    let on = if on.is_empty() { self.root.clone() } else { on };
    kwargs.retain(|(name, _)| *name != "on");
    kwargs.push(("on", Object::string(on.as_str())));
    let bound = vec![
      ("__on", Object::string(on.as_str())),
      ("__key", Object::string(key)),
      ("__kind", Object::string(kind)),
    ];
    let found = self.word(KEYED, bound)?;
    if let Some(id) = found.as_ref().as_str().filter(|one| !one.is_empty()) {
      return Ok(id.to_owned());
    }
    self.say("key", &on, vec![Object::string(key), Object::string(kind)])?;
    let got = self.verb(kind, args, kwargs)?;
    got.as_ref().as_str().map(str::to_owned).ok_or_else(|| Fault::refused("a verb gave no act"))
  }

  /// Whether a word of the names of the engine is true of an act.
  fn whether(&mut self, word: &str, id: &str) -> Result<bool, Fault> {
    let got = self.word(word, vec![("__id", Object::string(id))])?;
    Ok(got.as_ref().as_bool().unwrap_or_default())
  }

  /// What an act came to, once it is done, and nothing while it lives.
  pub fn outcome(&mut self, id: &str) -> Result<Option<Object>, Fault> {
    self.run(|sand, outside| outside.outcome(sand, id))
  }

  /// One name that the module of a chain binds, the root when no chain is given, read without calling it: the name
  /// of its type, its representation, and its value, which goes out as every value does.
  pub fn inspect(
    &mut self,
    name: &str,
    chain: Option<&str>,
  ) -> Result<(String, String, Object), Fault> {
    let chain = Object::string(chain.unwrap_or(&self.root));
    let bound = vec![("__chain", chain), ("__name", Object::string(name))];
    let got =
      self.word("(lambda x: (type(x).__name__, repr(x), x))(module(__chain)[__name])", bound)?;
    let got = got.as_ref();
    let text =
      |at| entry(&got, at).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default();
    let value = entry(&got, 2).map_or_else(Object::none, |one| one.to_owned());
    Ok((text(0), text(1), value))
  }

  /// Every name that the module of a chain binds, the root when no chain is given, in the order it bound them.
  pub fn names(&mut self, chain: Option<&str>) -> Result<Vec<String>, Fault> {
    let chain = Object::string(chain.unwrap_or(&self.root));
    Plain::plain(self.word("[str(x) for x in module(__chain)]", vec![("__chain", chain)])?)
  }

  /// What to do when an act is done, given what it came to: told at once for an act that is done already, and once
  /// when it is, after the entry in which it was. This is how a door that drives the engine itself awaits.
  pub fn watch(&mut self, id: &str, then: impl FnMut(&Object) + 'static) -> Result<(), Fault> {
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

impl Drop for Engine {
  /// A life that ends keeps its snapshot, when it keeps one.
  fn drop(&mut self) {
    self.kept_now(true);
  }
}

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

impl Plain for i64 {
  fn plain(got: Object) -> Result<Self, Fault> {
    got.as_ref().as_int().ok_or_else(|| Fault::refused(format!("{} is no int", got.py_repr())))
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

impl<T: Plain> Plain for Option<T> {
  fn plain(got: Object) -> Result<Self, Fault> {
    if got.as_ref().type_name() == "NoneType" { Ok(None) } else { T::plain(got).map(Some) }
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
/// It borrows the engine for as long as it is awaited, since awaiting it drives the engine: what the ears say of their
/// own accord is said into it until the act is done. An act that completed with an exception gives that fault. Dropping
/// it leaves the act living; the engine holds what it comes to under its name.
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
