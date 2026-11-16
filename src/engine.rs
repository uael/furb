//! The engine: one life of `engine.py` in the sandbox, the ears it hears, and the verbs a host says.
//!
//! An engine is driven from one thread. A verb is one call: it runs in the sandbox, the engine does what it does,
//! every fact it says reaches its ears, and what the verb gave comes back. An act is awaited: the future drives the
//! engine until the act is done, which means each ear is polled, and what an ear says of its own accord is said
//! under its name. Nothing polls in vain: what an ear waits for wakes whoever awaits.
//!
//! The sandbox reaches the host by each ear of the host, which the sandbox holds as
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
#![expect(
  clippy::needless_pass_by_value,
  reason = "debt: a method that the build makes takes its words by value and only borrows them"
)]
#![expect(
  clippy::unreadable_literal,
  reason = "debt: a constant that the build makes is a long literal with no separator"
)]

mod crossing;
mod hosted;
mod image;
mod opening;
mod order;
mod plain;
mod record;

use std::{
  collections::{HashMap, HashSet},
  sync::{Arc, LazyLock},
  task::Waker,
  time::Instant,
};

use monty_types::{ExcType, MontyUuid, NamedValues};

use self::{
  crossing::Crossing,
  hosted::{At, Handed, Hosted, Replay},
  opening::OPENING,
};
pub use self::{
  hosted::{Hands, Life},
  plain::{Act, Plain},
  record::CRASHES,
};
pub(crate) use self::{
  image::Half,
  opening::Opened,
  record::{left, looping, settled},
};
use crate::{
  RECORD, checkpoint,
  checkpoint::CHECKPOINT,
  ear::{Call, Ear, Heard},
  fact,
  fact::Fact,
  sand,
  sand::{Answer, Host, Nobody, Sand, id, object},
  value,
  value::{Exit, Fault, Given, Object, ObjectRef, Text, entry},
  world,
  world::{CHECKPOINT_SCHEMA, Systems},
};
use std::{fmt, iter, mem};

/// The function of the host that starts an ear of the host again after it raised, by its name.
const AGAIN: &str = "again";
/// The function of the host by which the Kernel bounds the turn while a word of a rung steps, and
/// says whether it was bounded before. It names the job that steps as the step begins, so a stop
/// of the sandbox in the step of a run ends that run.
const TURN: &str = "turn";

/// The kinds of act that are work an operator waits for: a prompt, a rung, a command and a wait.
const WORK: [&str; 4] = ["prompt", "rung", "bash", "wait"];

/// Whether this life took up an act that the record shows started and not done: the outside says it started in this
/// life, which it does once a wake that this life says puts the act to it, or the act is done. A started that the
/// journal says again is of an earlier life, and the view of the journal holds it.
const TOOK: &str = "peek(__id, ...) is not ... or \
  [x[1] for x in transcript(scope(__id), 'started')].count(__id) > \
  [x[1] for x in transcript(scope(__id), 'journal') if x[0] == 'started'].count(__id)";

/// Whether an act waits: it is not done, and no pause holds it.
const WAITS: &str = "peek(__id, ...) is ... and not paused(__id)";

/// The act that the operator made for a key of the client on a chain: the first question that the operator made on
/// the chain after a key fact of that key and that kind, when it is of that kind, and nothing otherwise.
const KEYED: &str = "next((b[1] for f, k, w in [(transcript(__on), __key, __kind)] \
  for i, a in enumerate(f) if a[0] == 'key' and a[3:5] == (k, w) for b in [next((x for x in f[i + 1:] if x[2] == OPERATOR and question(x)), ('',))] \
  if b[0] == w), '')";

/// The words of a call: those by position, and those by name.
type Words<'a> = (Vec<Object>, Vec<(&'a str, Object)>);

/// What a host does when an act is done, given what it came to.
type Watcher = Box<dyn FnMut(&Object)>;

/// The record as the host reads it for boot, and how many of its first entries stand for the
/// entries of a seed. A checkpoint that stands first is the data from which `seed` of the
/// checkpointer of this engine makes the entries that the life boots on, before the entries after
/// it. A checkpoint can hold each turn of a long life, so its data crosses into the sandbox one
/// time, and the entries of the seed stay there, where boot takes them. In the place of each, the
/// host holds an entry of the kind and the name of its fact alone, which is all that it reads of
/// them.
fn seeded(
  sand: &mut Sand,
  outside: &mut Outside,
  mut record: Vec<Object>,
) -> Result<(Vec<Object>, usize), Fault> {
  let first = record.first().and_then(|one| entry(&one.as_ref(), 0)).and_then(Fact::of);
  let Some(data) =
    first.filter(|a| a.kind() == CHECKPOINT).and_then(|a| a.word(0).map(|one| one.to_owned()))
  else {
    return Ok((record, 0));
  };
  let pairs = data.as_ref().pairs().unwrap_or_default();
  let field =
    |name: &str| pairs.iter().find(|(key, _)| key.as_str() == Some(name)).map(|one| one.1);
  CHECKPOINT_SCHEMA.opens("checkpoint", field("schema"))?;
  let lines = field("lines").and_then(|one| one.pairs()).unwrap_or_default().into_iter();
  sand.lined(
    lines
      .filter_map(|(name, line)| {
        Some((name.as_str()?.to_owned(), u32::try_from(line.as_int()?).ok()?))
      })
      .collect(),
  );
  let heads = sand.enter(outside, |sand, outside| {
    outside.bound(sand)?;
    let data = outside.inward(sand, &data)?;
    sand.call(outside, &Arc::clone(&outside.opened).seed, vec![data], vec![])
  })??;
  // The sandbox gives a part that many heads share as one part, and a copy of a head walks the
  // arena down to its lowest part, so each head is made here from its two texts.
  let head = |one: ObjectRef<'_>| {
    let texts = one.items().unwrap_or_default().into_iter().filter_map(|text| text.as_str());
    Object::list([Object::tuple(texts.map(Object::string))])
  };
  let heads: Vec<Object> =
    heads.as_ref().items().unwrap_or_default().into_iter().map(head).collect();
  let born = heads.len();
  record.splice(..1, heads);
  Ok((record, born))
}

/// A template string, as a host says one: each expression beside its value.
fn templated(pairs: &[(&str, Object)]) -> Object {
  value::paired(pairs.iter().map(|(expression, one)| ((*expression).to_owned(), one.clone())))
}

/// The entries of a record that the host holds, each its fact and the place of the fact when it
/// has one, made in the sandbox as the record of an earlier life comes in.
fn recorded(sand: &mut Sand, outside: &mut Outside, record: &[Object]) -> Result<Object, Fault> {
  let mut entries = Vec::new();
  for one in record {
    let one = outside.inward(sand, one)?;
    let held = one.as_ref().items().unwrap_or_default();
    let fact = held.first().and_then(ObjectRef::items);
    let fact = fact.ok_or_else(|| Fault::refused("a record holds a fact first in each entry"))?;
    let fact = Object::tuple(fact.into_iter().map(|one| one.to_owned()));
    let rest = held.iter().skip(1).map(ObjectRef::to_owned);
    entries.push(Object::tuple(iter::once(fact).chain(rest)));
  }
  Ok(Object::list(entries))
}

/// The side of the host in one life: its ears and its functions, which the sandbox calls, and what
/// the host holds of the sandbox, by which each value crosses. Its default is the side of the host
/// of a life of nothing, which stands in for one that is lent to code of a host at each step of an
/// ear, so it allocates nothing.
struct Outside {
  hosted: Hosted,
  crossing: Crossing,
  /// The callables of the opening, which a life of nothing shares with each other one, and which a
  /// call of one of them borrows with no copy of its value.
  opened: Arc<Opened>,
  /// How many ids the host gave, which names the next.
  given: u128,
  /// The jobs whose word the Kernel steps, the innermost last: a run, or the part of a writing.
  stepping: Vec<String>,
  /// The question that the host refused in this step of a word, past the bound of a turn, and its
  /// maker.
  past: Option<(String, String)>,
  /// When each act first asked a question that an ear of the host heard, in the live work of the
  /// ears outside any step of a word: since the host went in, an ear said a thing of its own
  /// accord, or a step of a word began or ended.
  asking: HashMap<String, Instant>,
  /// The refusal of the question of an act that the host ended past the bound of a turn, which the
  /// ear of the host says next, in place of what the ear hears then, as [`Outside::overran`] says.
  ending: Option<(MontyUuid, Fact)>,
  /// The questions of the record that the life opened on, as [`settled`] gives them: while boot
  /// replays the record, no bound of a turn holds a step of the word of such a run.
  settled: HashSet<String>,
}

/// The callables of a life of nothing, made once: each of its values is a graph of its own, which a
/// life of nothing at each step of an ear would allocate again.
static NOTHING: LazyLock<Arc<Opened>> = LazyLock::new(|| Arc::new(Opened::default()));

impl Default for Outside {
  fn default() -> Self {
    Self {
      hosted: Hosted::default(),
      crossing: Crossing::default(),
      opened: Arc::clone(&NOTHING),
      given: 0,
      stepping: Vec::new(),
      past: None,
      asking: HashMap::new(),
      ending: None,
      settled: HashSet::new(),
    }
  }
}

impl Host for Outside {
  /// One call of the sandbox, answered: a function of the host, or a step of an ear of the host.
  fn answer(
    &mut self,
    sand: &mut Sand,
    on: Option<MontyUuid>,
    name: &str,
    args: Vec<Object>,
  ) -> Answer {
    let got = match on {
      None if name == AGAIN => Ok(Object::bool(self.reborn(args.first()))),
      None if name == TURN => {
        let bounded = args.first().is_some_and(|one| one.as_ref().as_bool().unwrap_or(false));
        match args.get(1).and_then(|one| one.as_ref().as_str().map(str::to_owned)) {
          Some(job) => self.stepping.push(job),
          None => drop(self.stepping.pop()),
        }
        let bounded =
          bounded && !self.stepping.last().is_some_and(|job| self.settled.contains(job));
        // Each step of a word starts a stretch of the bound, and a round trip to the host
        // starts none.
        sand::step(bounded && !self.stepping.is_empty());
        self.past = None;
        self.asking.clear();
        Ok(Object::bool(sand.bound(bounded)))
      }
      None => self.called(sand, name, &args),
      Some(on) if name == "send" => return self.sent(sand, on, args.into_iter().next()),
      Some(on) => Err(Fault::refused(format!("{on} is no generator of the host that has {name}"))),
    };
    got.map_or_else(Answer::Abort, Answer::Value)
  }
}

/// The inputs of the opening of a sandbox: the texts of an engine, of the rule of the record, of a
/// Kernel and of a checkpointer, and the functions of the host that the opening binds.
fn inputs(source: &str, kernel: &str, checkpointer: &str) -> NamedValues {
  let mut inputs = NamedValues::new();
  inputs.push("__source", Object::string(source));
  inputs.push("__record_source", Object::string(RECORD));
  inputs.push("__kernel_source", Object::string(kernel));
  inputs.push("__checkpointer_source", Object::string(checkpointer));
  inputs.push("__again", Object::function(AGAIN, None));
  inputs.push("__turn", Object::function(TURN, None));
  inputs
}

impl Outside {
  /// The side of the host, on what the opening gave: a dict of its callables by name, each of which
  /// the engine keeps.
  fn new(handed: Handed, opened: &Object) -> Result<Self, Fault> {
    let mut held: HashMap<String, Object> = (opened.as_ref().pairs().unwrap_or_default())
      .into_iter()
      .filter_map(|(name, one)| Some((name.as_str()?.to_owned(), one.to_owned())))
      .collect();
    let kept = held.values().filter_map(|one| one.as_ref().handle()).collect();
    let opened = Opened::of(|name| {
      held.remove(name).ok_or_else(|| Fault::refused(format!("the opening gives no {name}")))
    })?;
    Ok(Self {
      hosted: Hosted { handed, ..Hosted::default() },
      crossing: Crossing { kept, ..Crossing::default() },
      opened: Arc::new(opened),
      given: 0,
      stepping: Vec::new(),
      past: None,
      asking: HashMap::new(),
      ending: None,
      settled: HashSet::new(),
    })
  }

  /// The end of the run whose word a stop of the sandbox cut, kept as the refusal of an act that a
  /// life ended before the act did, with no place, since the keep of the run itself may not have
  /// reached the store: a later life says it once that run stands and each entry before it was
  /// said, and the Kernel steps no run that is over, so it never runs the word into the same stop,
  /// and the prompt of its rung asks its model again. The sandbox hears nothing after the stop, so
  /// each ear of the host hears the keep from here.
  fn cut(&mut self) {
    let Some(run) = self.stepping.pop().filter(|job| fact::named(job, "run")) else {
      return;
    };
    let why = world::interruption(&run);
    let done = [
      Object::string("done"),
      Object::string(&run),
      Object::string(""),
      Fault::refused(why).object(),
    ];
    let entry = [Object::tuple(done), Object::none(), Object::bool(false)];
    let keep = Object::tuple([
      Object::string("keep"),
      Object::string(""),
      Object::string("journal"),
      Object::tuple(entry),
    ]);
    let Some(fact) = Fact::of(keep.as_ref()) else { return };
    let mut cx = std::task::Context::from_waker(&self.hosted.waker);
    for ear in self.hosted.handed.ears.values_mut() {
      drop(ear.resume(Heard::Fact(fact.clone()), &mut cx));
      // A store that refuses the entry holds it, and Engine::unkept counts it for the host.
      drop(ear.ran());
    }
  }

  /// A new id of the host, for an object of the host or for an instance made again in the sandbox.
  fn fresh(&mut self) -> MontyUuid {
    self.given += 1;
    id(self.given)
  }

  /// The names of the engine that are callable, read where they stand now. Each is the engine's to hold.
  fn bound(&mut self, sand: &mut Sand) -> Result<(), Fault> {
    let opened = Arc::clone(&self.opened);
    let got = sand.call(self, &opened.names, vec![], vec![])?;
    self.crossing.names.clear();
    self.crossing.named.clear();
    for (name, one) in got.as_ref().pairs().unwrap_or_default() {
      let (Some(name), one) = (name.as_str(), one.to_owned()) else { continue };
      if let Some(handle) = one.as_ref().handle() {
        self.crossing.kept.insert(handle);
        self.crossing.named.entry(handle).or_insert_with(|| name.to_owned());
      }
      self.crossing.names.insert(name.to_owned(), one);
    }
    Ok(())
  }

  /// A name of the engine, called with values of the sandbox where it stands now, and what it gave.
  #[expect(
    clippy::too_many_arguments,
    reason = "the side of the host and the sandbox are lent apart, since each call of the sandbox lends the host again, and a call is its callee, its values and its keyword values"
  )]
  fn named_call(
    &mut self,
    sand: &mut Sand,
    name: &str,
    mut args: Vec<Object>,
    kwargs: Vec<(&str, Object)>,
  ) -> Result<Object, Fault> {
    args.insert(0, Object::string(name));
    let opened = Arc::clone(&self.opened);
    sand.call(self, &opened.call, args, kwargs)
  }

  /// A function of the checkpointer, `seed` or `gathered`, called by its name on the names of the
  /// engine with values of the host, and what it gave, as it goes out: the checkpointer of the
  /// record, or the one whose text `by` holds.
  #[expect(
    clippy::too_many_arguments,
    reason = "the side of the host and the sandbox are lent apart, since each call of the sandbox lends the host again, and a function of the checkpointer is its name, its values and the text that it runs on"
  )]
  fn checkpointed(
    &mut self,
    sand: &mut Sand,
    name: &str,
    args: &[Object],
    by: Option<&str>,
  ) -> Result<Object, Fault> {
    let (mut args, _) = self.inwards(sand, args, vec![])?;
    args.insert(0, Object::string(name));
    let opened = Arc::clone(&self.opened);
    let by = by.map(|text| ("by", Object::string(text))).into_iter().collect();
    let got = sand.call(self, &opened.checkpoint, args, by)?;
    self.outward(sand, &got)
  }

  /// The words of a call, values of the host, as they come in.
  fn inwards<'a>(
    &mut self,
    sand: &mut Sand,
    args: &[Object],
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
  #[expect(
    clippy::too_many_arguments,
    reason = "the side of the host and the sandbox are lent apart, since each call of the sandbox lends the host again, and a call is its callee, its values and its keyword values"
  )]
  fn verb(
    &mut self,
    sand: &mut Sand,
    name: &str,
    args: Vec<Object>,
    kwargs: Vec<(&str, Object)>,
  ) -> Result<Object, Fault> {
    let (args, kwargs) = self.inwards(sand, &args, kwargs)?;
    let got = self.named_call(sand, name, args, kwargs)?;
    self.outward(sand, &got)
  }

  /// One callable the engine made, called back by the number it went out under.
  #[expect(
    clippy::too_many_arguments,
    reason = "the side of the host and the sandbox are lent apart, since each call of the sandbox lends the host again, and a call is its callee, its values and its keyword values"
  )]
  fn called_back(
    &mut self,
    sand: &mut Sand,
    n: i64,
    args: Vec<Object>,
    kwargs: Vec<(&str, Object)>,
  ) -> Result<Object, Fault> {
    let made = self.held(n)?;
    let (args, kwargs) = self.inwards(sand, &args, kwargs)?;
    let got = sand.call(self, &made, args, kwargs)?;
    self.outward(sand, &got)
  }

  /// Who speaks in the life, and who speaks from now on when a value is given.
  fn site(&mut self, sand: &mut Sand, value: Option<&Object>) -> Result<Object, Fault> {
    let opened = Arc::clone(&self.opened);
    let previous = sand.call(self, &opened.site, vec![], vec![])?;
    if let Some(value) = value.filter(|one| !one.as_ref().is_none()) {
      let by = value.as_ref().as_str().map_or_else(|| value.py_repr(), str::to_owned);
      let token = sand.call(self, &opened.speaks, vec![Object::string(by)], vec![])?;
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

  /// A function of the host, called back by the sandbox: what it is given goes out as any value goes out, and what
  /// it gives comes in as any value comes in, so a function that gives an ear gives the object that stands in for it.
  fn called(&mut self, sand: &mut Sand, name: &str, args: &[Object]) -> Result<Object, Fault> {
    let args = args.iter().map(|one| self.outward(sand, one)).collect::<Result<Vec<_>, _>>()?;
    let missing = || Fault::refused(format!("{name} is no function of the host"));
    let mut call = self.hosted.handed.calls.remove(name).ok_or_else(missing)?;
    let got = self.lent(sand, |life| call(args, life));
    self.hosted.handed.calls.insert(name.to_owned(), call);
    self.inward(sand, &got?)
  }

  /// What the host calls while the life waits on it, answered at once: who speaks, a callable the
  /// engine made, or a verb of the engine, as it goes out.
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
        self.called_back(sand, n, words, named)
      }
      _ => self.verb(sand, &verb, args, kwargs),
    }
  }

  /// What the host raised, raised in the sandbox as the exception it is: a builtin one by its name, and one of the
  /// engine made from what it was made with.
  fn raising(&mut self, sand: &mut Sand, fault: Fault) -> Answer {
    if fault.name.parse::<ExcType>().is_ok() || !self.crossing.names.contains_key(&fault.name) {
      return Answer::Fault(fault);
    }
    let made = fault.args.iter().map(|one| self.inward(sand, one)).collect::<Result<Vec<_>, _>>();
    match made.and_then(|args| self.named_call(sand, &fault.name, args, vec![])) {
      Ok(exception) => Answer::Raise(exception),
      Err(fault) => Answer::Abort(fault),
    }
  }
}

/// The state of a record while no life holds it, as [`Engine::kept`] gives it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kept {
  /// It holds work that the open of the next life on it resumes at once.
  Runs,
  /// It holds no such work.
  Saved,
}

impl Kept {
  /// The name of the state, as a list of records shows it.
  pub fn name(self) -> &'static str {
    match self {
      Self::Runs => "runs",
      Self::Saved => "saved",
    }
  }
}

/// How deep the representation of a value by its fields goes: a part below shows as `…`.
const FIELDS: usize = 3;

/// A value as a debugger shows it, from the value as it went out of the sandbox, so no code of the
/// life runs: an instance of a class of the life by its fields, as `P(x=1)`, a list, a tuple, a set
/// or a dict by its parts, each the same way, and any other value as the host writes it.
fn fields(value: ObjectRef<'_>, depth: usize) -> String {
  if depth >= FIELDS {
    return "…".to_owned();
  }
  let pairs = value.pairs().unwrap_or_default();
  let at = |name: &str| pairs.iter().find(|one| one.0.as_str() == Some(name)).map(|one| one.1);
  if value.type_name() == "dict" && at("is").and_then(|one| one.as_str()) == Some("instance") {
    let inner = at("value").and_then(|one| Some((one.pairs()?, one.type_name().to_owned())));
    let Some((attributes, class)) = inner else { return value.py_repr() };
    let shown = attributes.iter().map(|(name, one)| {
      format!("{}={}", name.as_str().unwrap_or_default(), fields(*one, depth + 1))
    });
    return format!("{class}({})", shown.collect::<Vec<_>>().join(", "));
  }
  // A map that holds the key `is` goes out as its pairs, under the mark `dict`.
  if value.type_name() == "dict" && at("is").and_then(|one| one.as_str()) == Some("dict") {
    let held = at("args").and_then(|one| one.items()?.first()?.items()).unwrap_or_default();
    let pairs = held.iter().filter_map(|pair| {
      let items = pair.items()?;
      let [key, one] = items[..] else { return None };
      Some(format!("{}: {}", fields(key, depth + 1), fields(one, depth + 1)))
    });
    return format!("{{{}}}", pairs.collect::<Vec<_>>().join(", "));
  }
  let shown = |items: Vec<ObjectRef<'_>>| -> Vec<String> {
    items.into_iter().map(|one| fields(one, depth + 1)).collect()
  };
  match (value.type_name(), value.items()) {
    ("list", Some(items)) => return format!("[{}]", shown(items).join(", ")),
    ("tuple", Some(items)) => return format!("({})", shown(items).join(", ")),
    ("set" | "frozenset", Some(items)) => return format!("{{{}}}", shown(items).join(", ")),
    _ => {}
  }
  if value.type_name() == "dict" {
    let pairs = pairs
      .iter()
      .map(|(key, one)| format!("{}: {}", fields(*key, depth + 1), fields(*one, depth + 1)));
    return format!("{{{}}}", pairs.collect::<Vec<_>>().join(", "));
  }
  value.py_repr()
}

/// The read of a name of a module, which gives the name of the type of its value, its
/// representation when no code of the life makes it, and the value. A value of the builtins or of
/// the engine, or a list, a tuple, a set or a dict of them at any depth, has a representation that
/// no model wrote. The walk stops 64 levels down, so a container that holds itself takes the
/// representation of its fields.
const INSPECT: &str = "(lambda x, plain: (type(x).__name__, repr(x) if plain(x, plain, 0) else '', x))(\
module(__chain)[__name], lambda one, plain, depth: depth < 64 and (type(one) in (str, int, float, bool, bytes, \
type(None), Act, Text) or type(one) in (list, tuple, set, frozenset) and all(plain(x, plain, depth + 1) for x in one) \
or type(one) is dict and all(plain(k, plain, depth + 1) and plain(v, plain, depth + 1) for k, v in one.items())))";

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
  /// The fault that cut a run of the sandbox, when one did. monty ends the run at a fault
  /// that no code catches, as a second excess of the turn bound, with no `finally` of the
  /// frames it cuts: the bus of the engine stays busy with the ear that it stepped, and hears
  /// no fact after it. So each later entry is refused with this fault.
  stopped: Option<Fault>,
  /// The acts that the record showed started and not done, which the journal holds from the outside and which this
  /// life has not taken up yet, in the order of the record.
  holding: Vec<String>,
  /// The acts that the record parks and that this life has not retried, by name and kind, in the order of the record.
  parked: Vec<(String, String)>,
  /// The highest number that each kind took in the record that the life opened on, as a map, which
  /// the carry of the life keeps taken: the journal does not make again an act that an ear of
  /// the outside made, so the transcripts of the life do not hold each of those names.
  taken: Object,
  /// The entry of the fact made of the texts that the life runs, as [`world::running`] gives it,
  /// which a crossing writes at the tip of the record. It is the build only for a life that was
  /// born on it.
  made: Object,
  /// The system prompt of each chain that read first an engine that the life does not run.
  systems: Systems,
}

impl Engine {
  /// One entry into the sandbox, and then whoever watches an act that is done now is told.
  fn run<T>(
    &mut self,
    body: impl FnOnce(&mut Sand, &mut Outside) -> Result<T, Fault>,
  ) -> Result<T, Fault> {
    let got = self.fed(body)?;
    let ran = self.outside.ran();
    got.and_then(|got| ran.map(|()| got))
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
  /// entry fails when its run ended, which stops the life, and gives what its body gave otherwise.
  fn fed<T>(
    &mut self,
    body: impl FnOnce(&mut Sand, &mut Outside) -> Result<T, Fault>,
  ) -> Result<Result<T, Fault>, Fault> {
    if let Some(cut) = &self.stopped {
      return Err(Fault::refused(format!(
        "The life stopped at {}: {}. It cut the step of an ear in the sandbox, so the life hears no fact after it. \
         Open the life again on its record.",
        cut.name,
        cut.message()
      )));
    }
    self.outside.asking.clear();
    let ids: Vec<String> = self.watchers.keys().cloned().collect();
    let entered = self.sand.enter(&mut self.outside, |sand, outside| {
      let got = body(sand, outside);
      let outcomes = ids.iter().map(|id| outside.outcome(sand, id)).collect::<Result<Vec<_>, _>>();
      (got, outcomes)
    });
    let (got, outcomes) = match entered {
      Ok(both) => both,
      Err(cut) => {
        // No act that a watcher waits on comes to anything after the cut, so each watcher goes, unanswered.
        self.watchers.clear();
        sand::step(false);
        self.outside.cut();
        self.stopped = Some(cut.clone());
        return Err(cut);
      }
    };
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
  ///
  /// While the record lacks an entry, as on a full disk, only the ear that keeps the record is
  /// polled, so no work of an act starts before the record holds the act, and a later life never
  /// runs it a second time.
  pub fn pump(&mut self, waker: &Waker) -> Result<(), Fault> {
    if !self.outside.hosted.waker.will_wake(waker) {
      self.outside.hosted.waker.clone_from(waker);
    }
    let Some(first) = self.outside.spoken() else { return Ok(()) };
    self.run(|sand, outside| outside.said(sand, first))
  }

  /// An engine on a sandbox and the side of its host, before any run: the record says what work it
  /// left and what loops.
  fn new(sand: Sand, outside: Outside, record: &[Object], made: Object) -> Self {
    let systems = checkpoint::systems(record);
    Self {
      sand,
      outside,
      watchers: HashMap::new(),
      root: String::new(),
      raised: None,
      stopped: None,
      holding: left(record),
      parked: looping(record),
      taken: checkpoint::taken(record),
      made,
      systems,
    }
  }

  /// An engine, opened from the record, on these ears, each under the name the engine hears it by, in the order
  /// the engine offers them a question.
  ///
  /// The record is the entries the store kept, each one fact. The engine and the Kernel run first,
  /// each in a namespace of its own, and `boot` is given the Kernel of the crate before the ears of
  /// the host.
  pub fn boot<N: Into<String>>(
    record: impl IntoIterator<Item = Object>,
    ears: impl IntoIterator<Item = (N, Box<dyn Ear>)>,
  ) -> Result<Self, Fault> {
    Self::booting(record, ears, Box::new(|_, _| {}))
  }

  /// An engine, as [`Engine::boot`] opens it, with the replay told: how many of the entries that
  /// the life boots on the journal passed, and how many it boots on, from the moment that the seed
  /// gave them, and each of them once boot ended with no fault.
  pub(crate) fn booting<N: Into<String>>(
    record: impl IntoIterator<Item = Object>,
    ears: impl IntoIterator<Item = (N, Box<dyn Ear>)>,
    replayed: Box<dyn Fn(u64, u64)>,
  ) -> Result<Self, Fault> {
    let (given, names) = Handed::of(ears)?;
    let record: Vec<Object> = record.into_iter().collect();
    // A record replays on the engine that the fact made of its last segment holds, in the order of
    // this build, and carries on the checkpointer of this build in place of one that drops a chain.
    let (source, kernel, checkpointer) = world::texts(checkpoint::segment(&record))?;
    let (source, checkpointer) = (order::ordered(source)?, order::carrier(&checkpointer));
    let mut sand = Sand::new(source != crate::ENGINE);
    // The opening, the seed and the replay are the engine's own work: unbounded, but for each word
    // they run again.
    sand.bound(false);
    let opened =
      sand.run(OPENING, inputs(&source, &kernel, checkpointer), &mut Nobody("the opening"))?;
    sand.watch();
    let mut outside = Outside::new(given, &opened)?;
    let Opened { kernel, lasting, parking, park, .. } = Opened::clone(&outside.opened);
    let (record, born) = seeded(&mut sand, &mut outside, record)?;
    outside.settled = settled(&record);
    outside.hosted.replay = Some(Replay::of(&record, replayed));
    let made = world::running(checkpoint::segment(&record));
    let mut engine = Self::new(sand, outside, &record, made);
    let names_parked = Object::list(engine.parked.iter().map(|(id, _)| Object::string(id)));
    let parks = !engine.parked.is_empty();
    let (root, raised) = engine.run(|sand, outside| {
      outside.bound(sand)?;
      outside.crossing.recorded = true;
      let entries = recorded(sand, outside, record.get(born..).unwrap_or_default());
      outside.crossing.recorded = false;
      let mut given = vec![("kernel".to_owned(), kernel)];
      for name in names {
        let on = outside.fresh();
        outside.hosted.standing.insert(on, (name.clone(), At::Unborn));
        given.push((name, object("Ear", on)));
      }
      // Each ear that boot is given starts again after it raises, or tells the operator why it cannot.
      for (name, one) in &mut given {
        *one =
          sand.call(outside, &lasting, vec![Object::string(name.clone()), one.clone()], vec![])?;
      }
      if parks {
        sand.call(outside, &park, vec![names_parked], vec![])?;
        for (_, one) in &mut given {
          *one = sand.call(outside, &parking, vec![one.clone()], vec![])?;
        }
      }
      let given = given.iter().map(|(name, one)| (name.as_str(), one.clone())).collect();
      let got = sand.call(outside, &Arc::clone(&outside.opened).boot, vec![entries?], given);
      // What boot raised comes out of the entry the operator went in by, and the life goes on: a drift breaks the
      // journal and keeps nothing more, so the root stands when the record held it.
      let opened = match got {
        Ok(root) => (root.as_ref().as_str().unwrap_or_default().to_owned(), None),
        Err(fault) => {
          let root = outside.named_call(sand, "get", vec![Object::string("chain1")], vec![])?;
          let root = if root.as_ref().is_none() { "" } else { "chain1" };
          (root.to_owned(), Some(fault))
        }
      };
      // Boot binds the verbs of a life in place of those that stand before it.
      outside.bound(sand)?;
      Ok(opened)
    })?;
    engine.sand.bound(true);
    // The replay ends with boot: a run of the record that goes on after it is live, and the bound
    // holds it.
    engine.outside.settled.clear();
    engine.outside.hosted.replayed(raised.is_none());
    engine.root = root;
    engine.raised = raised;
    Ok(engine)
  }

  /// Gives the journal of the life the entries that follow its tip, as the record of a life that
  /// went on from this state holds them, and the journal says them again at once, as boot says a
  /// record again: each word runs again with no bound of a turn, each act takes the answer that
  /// the entries hold, and no ear of the World is asked for one.
  pub fn feed(&mut self, tail: &[Object]) -> Result<(), Fault> {
    self.outside.settled = settled(tail);
    let fed = self.unbounded(|engine| {
      engine.run(|sand, outside| {
        outside.crossing.recorded = true;
        let entries = recorded(sand, outside, tail);
        outside.crossing.recorded = false;
        sand.call(outside, &Arc::clone(&outside.opened).fed, vec![entries?], vec![]).map(drop)
      })
    });
    self.outside.settled.clear();
    fed
  }

  /// The names of the acts that the journal of the life holds, each that an ear of the outside
  /// took and that is not done, as `held` of the checkpointer gives them.
  pub fn held(&mut self) -> Result<Object, Fault> {
    self.run(|sand, outside| {
      let got = sand.call(outside, &Arc::clone(&outside.opened).held, vec![], vec![])?;
      outside.outward(sand, &got)
    })
  }

  /// What `body` gives, run as the engine's own work, with no bound of the turn but for each word
  /// that it steps.
  pub(crate) fn unbounded<T>(&mut self, body: impl FnOnce(&mut Self) -> T) -> T {
    let was = self.sand.bound(false);
    let got = body(self);
    self.sand.bound(was);
    got
  }

  /// The highest number that each kind took in the record that the life opened on.
  pub(crate) fn taken(&self) -> &Object {
    &self.taken
  }

  /// The entry of the fact made of the texts that the life runs.
  pub(crate) fn running(&self) -> &Object {
    &self.made
  }

  /// The system prompt of each chain that read first an engine that the life does not run.
  pub(crate) fn systems(&self) -> &Systems {
    &self.systems
  }

  /// A chain and each source of it in turn, by their names, as the life holds them.
  pub(crate) fn lineage(&mut self, chain: &str) -> Result<Vec<String>, Fault> {
    let mut lineage = vec![chain.to_owned()];
    loop {
      let last = lineage.last().map(|one| Object::string(one.as_str()));
      let act = self.verb("get", last.into_iter().collect(), vec![])?;
      let source = act.as_ref().items().and_then(|words| words.get(5)?.as_str().map(str::to_owned));
      match source.filter(|one| !one.is_empty()) {
        Some(source) => lineage.push(source),
        None => return Ok(lineage),
      }
    }
  }

  /// Whether an ear of the host hears under a name.
  pub(crate) fn hears(&self, name: &str) -> bool {
    self.outside.hosted.handed.ears.contains_key(name)
  }

  /// How many entries the ears of the host hold that the record did not take yet, as on a full
  /// disk: the ear that keeps the record writes them in order once its store takes them again, and
  /// an end of the life before that loses them.
  pub fn unkept(&self) -> usize {
    self.outside.hosted.handed.ears.values().map(|ear| ear.unkept()).sum()
  }

  /// The ears and the functions of the host that the values of the next calls name, which the
  /// life takes as each value comes into its sandbox.
  pub fn hand(&mut self, hands: Hands) {
    self.outside.hosted.given.join(hands);
  }

  /// An ear of the host, as the value a verb is given, which [`Hands::ear`] says.
  pub(crate) fn handed(&mut self, ear: Box<dyn Ear>, started: bool) -> Object {
    let mut hands = Hands::default();
    let handed = hands.ear(ear, started);
    self.hand(hands);
    handed
  }

  /// The root chain of the life, which is the first act of any record.
  pub fn root(&self) -> &str {
    &self.root
  }

  /// The line of the word that made each act while a word stood on the stack, counted from 1, in
  /// the order the acts were made, as [`Lines`](crate::view::Lines) says.
  pub fn lines(&self) -> &[(String, u32)] {
    self.sand.lines()
  }

  /// What boot raised, if it raised: a drift, which breaks the journal while the life goes on with nothing kept,
  /// or a refusal of the ears it was given.
  pub fn raised(&self) -> Option<&Fault> {
    self.raised.as_ref()
  }

  /// The fault that cut a run of the sandbox, after which the life refuses each entry, or nothing while the life goes
  /// on. A host tells it, since the life tells it to no chain.
  pub fn stopped(&self) -> Option<&Fault> {
    self.stopped.as_ref()
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

  /// A function of the checkpointer of the engine, `seed` or `gathered`, by its name, said with
  /// these words, and what it gave.
  pub fn checkpointed(&mut self, name: &str, args: Vec<Object>) -> Result<Object, Fault> {
    self.run(|sand, outside| outside.checkpointed(sand, name, &args, None))
  }

  /// The carry of the life to the texts of a fact made, as `gathered` of the checkpointer that the
  /// fact names gives it, with these words.
  pub(crate) fn gathered_for(&mut self, made: &str, args: Vec<Object>) -> Result<Object, Fault> {
    let made = order::carrier(made);
    self.run(|sand, outside| outside.checkpointed(sand, "gathered", &args, Some(made)))
  }

  /// One word run in the names of the engine with these values bound, and what it gave, as it goes out, which is how
  /// a door reads what no verb reads.
  pub fn word(&mut self, word: &str, inputs: Vec<(&str, Object)>) -> Result<Object, Fault> {
    let names = Object::dict(inputs.into_iter().map(|(name, one)| (Object::string(name), one)));
    self.run(|sand, outside| {
      let (opened, names) = (Arc::clone(&outside.opened), outside.inward(sand, &names)?);
      let got = sand.call(outside, &opened.word, vec![Object::string(word), names], vec![])?;
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
    self.run(|sand, outside| outside.called_back(sand, n, args, kwargs))
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
    let holding = self.holding()?;
    self.above(holding)
  }

  /// Each act that the outside started in an earlier life and that no ear of this life took yet,
  /// once each: the record shows an act that each of several lives started at each start.
  fn holding(&mut self) -> Result<Vec<String>, Fault> {
    let mut holding = Vec::new();
    for id in mem::take(&mut self.holding) {
      if !holding.contains(&id) && !self.whether(TOOK, &id)? {
        holding.push(id);
      }
    }
    self.holding.clone_from(&holding);
    Ok(holding)
  }

  /// Each act of a list that waits and that no pause holds, and each act above it that made it,
  /// directly or not, when that act is a prompt, a rung, a command or a wait that is not done and
  /// that no pause holds, in order.
  fn above(&mut self, holding: Vec<String>) -> Result<Vec<(String, String)>, Fault> {
    let mut pending = Vec::new();
    for id in holding {
      if self.parked.iter().any(|(one, _)| *one == id) || !self.whether(WAITS, &id)? {
        continue;
      }
      let (mut made, mut at) = (Vec::new(), id.clone());
      while let Some(act) = self.get(&at)? {
        let by = act.by().to_owned();
        if WORK.contains(&act.kind()) && self.whether(WAITS, &at)? {
          made.push((mem::replace(&mut at, by), act.kind().to_owned()));
        } else {
          at = by;
        }
      }
      // An act that no work above it waits for is the work that the earlier life left.
      if made.is_empty() {
        made.extend(self.get(&id)?.map(|act| (id.clone(), act.kind().to_owned())));
      }
      for one in made.into_iter().rev() {
        if !pending.contains(&one) {
          pending.push(one);
        }
      }
    }
    Ok(pending)
  }

  /// The state of the record as a list of records shows it while no life holds it: [`Kept::Runs`]
  /// when it holds the work of [`Engine::pending`], which the open of the next life on it resumes
  /// at once, so that work runs and no pause holds it, and [`Kept::Saved`] when it holds none.
  pub fn kept(&mut self) -> Result<Kept, Fault> {
    Ok(if self.pending()?.is_empty() { Kept::Saved } else { Kept::Runs })
  }

  /// The name of [`Engine::kept`], as a door gives it: `runs` or `saved`.
  pub fn saved(&mut self) -> Result<&'static str, Fault> {
    self.kept().map(Kept::name)
  }

  /// The work that an earlier life left, put to the outside again with no wake from a host: a
  /// wake of each act that the outside started then, so the World asks each reply of it again,
  /// takes each wait, stand and prompt to the operator of it again, and closes each other act of it
  /// that it takes as interrupted, a command among them, and the word that awaits that act reads
  /// that end. A life that opens on a record and does work says it once, at its tip, since a
  /// resume returns to the state of that tip, whose work goes on. A parked act and an act that a
  /// pause holds stay as they are.
  pub fn resume(&mut self) -> Result<(), Fault> {
    let holding = self.holding()?;
    self.woken(holding)
  }

  /// The acts that the ears of the life of an image held in flight, put to their fresh ears as
  /// [`Engine::resume`] puts the work of an earlier life: the journal holds each act that an ear of
  /// the outside took and that is not done.
  pub(crate) fn flight(&mut self) -> Result<(), Fault> {
    let held = self.held()?;
    let ids = held.as_ref().items().unwrap_or_default();
    self.woken(ids.iter().filter_map(|one| one.as_str().map(str::to_owned)).collect())
  }

  /// A wake of each act of a list that waits and that no pause holds, which puts it to the outside
  /// again. The wake names that act alone and no act above it: a wake of the prompt that a word
  /// runs for would ask its chain again, while the word that awaits the act still runs.
  fn woken(&mut self, holding: Vec<String>) -> Result<(), Fault> {
    for id in holding {
      if !self.parked.iter().any(|(one, _)| *one == id) && self.whether(WAITS, &id)? {
        self.wake(&id)?;
      }
    }
    Ok(())
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
    let opened = Arc::clone(&self.outside.opened);
    self
      .run(|sand, outside| sand.call(outside, &opened.unpark, vec![Object::string(id)], vec![]))?;
    self.verb("wake", vec![Object::string(id)], vec![]).map(|_| ())
  }

  /// The act of a verb that a key of the client makes once, across a crash of the host and a retry of the client. A
  /// key fact on the chain, `["key", chain, by, key, kind]`, goes to the record before the act, and the act of the key
  /// is the first question that the operator makes on that chain after it. When the chain holds such an act, it is
  /// given back and nothing is made. When it holds none, as when the host crashed after it kept the key and before it
  /// kept the act, the key is said again and the act is made. The chain is the one that `on` names, or the root.
  #[expect(
    clippy::too_many_arguments,
    reason = "a verb said once by its key: the key, the kind of its act, and its values and keyword values as the verb takes them"
  )]
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

  /// One name that the module of a chain binds, the root when no chain is given, read without
  /// calling it: the name of its type, its representation, and its value, which goes out as every
  /// value does. The read runs no code of the life, so it makes no act and no fact: the
  /// representation is the one of a value of the builtins or of the engine, or of a container of
  /// such values, and any other value shows as a debugger shows it, an instance by its fields,
  /// since its own representation is code that a model may have written.
  pub fn inspect(
    &mut self,
    name: &str,
    chain: Option<&str>,
  ) -> Result<(String, String, Object), Fault> {
    let chain = Object::string(chain.unwrap_or(&self.root));
    let bound = vec![("__chain", chain), ("__name", Object::string(name))];
    let got = self.word(INSPECT, bound)?;
    let got = got.as_ref();
    let text =
      |at| entry(&got, at).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default();
    let value = entry(&got, 2).map_or_else(Object::none, |one| one.to_owned());
    let shown =
      Some(text(1)).filter(|one| !one.is_empty()).unwrap_or_else(|| fields(value.as_ref(), 0));
    Ok((text(0), shown, value))
  }

  /// Every name that the module of a chain binds, the root when no chain is given, in the order it bound them.
  pub fn names(&mut self, chain: Option<&str>) -> Result<Vec<String>, Fault> {
    let chain = Object::string(chain.unwrap_or(&self.root));
    Plain::plain(self.word("[str(x) for x in module(__chain)]", vec![("__chain", chain)])?)
  }

  /// What to do when an act is done, given what it came to: told at once for an act that is done
  /// already, and once when it is, after the entry in which it was. This is how a door that drives
  /// the engine itself awaits. A chain and a name of no act are refused, since no done can settle
  /// them, as `Act.__await__` refuses them.
  pub fn watch(&mut self, id: &str, then: impl FnMut(&Object) + 'static) -> Result<(), Fault> {
    self.run(|sand, outside| {
      sand
        .call(outside, &Arc::clone(&outside.opened).settles, vec![Object::string(id)], vec![])
        .map(drop)
    })?;
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
    Ok(Act::of(self, id))
  }
}

// The verbs of the contract, one method each, and its constants that are a number or a text, which the build makes
// from the contract.
include!(concat!(env!("OUT_DIR"), "/methods.rs"));
include!(concat!(env!("OUT_DIR"), "/constants.rs"));

impl fmt::Debug for Engine {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("Engine").field("root", &self.root).finish_non_exhaustive()
  }
}

/// The struct of each verb that takes words with a default: each is none until a host gives it, and none leaves
/// the default of the engine.
pub mod verbs {
  include!(concat!(env!("OUT_DIR"), "/verbs.rs"));
}

#[cfg(test)]
#[path = "engine.test.rs"]
mod test;
