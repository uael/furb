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
mod opening;
mod plain;
mod record;

use std::{collections::HashMap, task::Waker};

use monty_types::{ExcType, MontyUuid, NamedValues};

use self::{
  crossing::Crossing,
  hosted::{At, EAR, Handed, Hosted},
  opening::OPENING,
};
pub use self::{
  hosted::{Life, callable, handed, lending},
  plain::{Act, Plain},
  record::CRASHES,
};
pub(crate) use self::{
  opening::Opened,
  record::{left, looping},
};
use crate::{
  ENGINE, KERNEL, SHEET,
  ear::{Call, Ear},
  fact::Fact,
  gate::checked,
  sand::{Answer, Host, Nobody, Sand, id, object},
  snapshot::Keeper,
  value::{Exit, Fault, Given, Object, Text, entry},
};

/// The name of the function of the host that the gate calls with a sheet, which answers each finding by its line.
const GATE: &str = "gate";
/// The function of the host that starts an ear of the host again after it raised, by its name.
const AGAIN: &str = "again";
/// The function of the host by which the Kernel bounds the turn while a word of a rung steps, and
/// says whether it was bounded before.
const TURN: &str = "turn";

/// The kinds of act that are work an operator waits for: a prompt, a rung, a command and a wait.
const WORK: [&str; 4] = ["prompt", "rung", "bash", "wait"];

/// Whether this life took up an act that the record shows started and not done: the outside says it started in this
/// life, which it does once a wake that this life says puts the act to it, or the act is done. A started that the
/// journal says again is of an earlier life, and the view of the journal holds it.
/// An act that a snapshot holds as work that ran is taken up only once this life puts it to the outside again.
const TOOK: &str = "peek(__id, ...) is not ... or __id not in __restored and \
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

/// The record as boot is given it: a checkpoint that stands first is the data from which `seed`, a
/// function of this engine, makes the entries that the life boots on, before the entries after it.
fn seeded(
  sand: &mut Sand,
  outside: &mut Outside,
  record: Vec<Object>,
) -> Result<Vec<Object>, Fault> {
  let first = record.first().and_then(|one| entry(&one.as_ref(), 0)).and_then(Fact::of);
  let Some(data) = first
    .filter(|a| a.kind() == crate::checkpoint::CHECKPOINT)
    .and_then(|a| a.word(0).map(|one| one.to_owned()))
  else {
    return Ok(record);
  };
  let pairs = data.as_ref().pairs().unwrap_or_default();
  let schema = pairs.iter().find(|(key, _)| key.as_str() == Some("schema")).map(|one| one.1);
  crate::world::Schema::opens_checkpoint(schema)?;
  let made = sand.enter(outside, |sand, outside| {
    outside.bound(sand)?;
    let data = outside.inward(sand, &data)?;
    let got = outside.named_call(sand, "seed", vec![data], vec![])?;
    outside.outward(sand, &got)
  })??;
  let mut entries: Vec<Object> =
    made.as_ref().items().unwrap_or_default().into_iter().map(|one| one.to_owned()).collect();
  entries.extend(record.into_iter().skip(1));
  Ok(entries)
}

/// A template string, as a host says one: each expression beside its value.
fn templated(pairs: &[(&str, Object)]) -> Object {
  crate::value::paired(
    pairs.iter().map(|(expression, one)| ((*expression).to_owned(), one.clone())),
  )
}

/// The entries of a record as boot is given them, each fact alone in an entry, made in the sandbox
/// as the record of an earlier life comes in.
fn recorded(sand: &mut Sand, outside: &mut Outside, record: &[Object]) -> Result<Object, Fault> {
  let mut entries = Vec::new();
  for one in record {
    let one = outside.inward(sand, one)?;
    let fact = entry(&one.as_ref(), 0).and_then(|fact| fact.items());
    let fact = fact.ok_or_else(|| Fault::refused("a record holds each fact alone in an entry"))?;
    entries.push(Object::tuple([Object::tuple(fact.into_iter().map(|one| one.to_owned()))]));
  }
  Ok(Object::list(entries))
}

/// The side of the host in one life: its ears and its functions, which the sandbox calls, and what
/// the host holds of the sandbox, by which each value crosses. Its default is the side of the host
/// of a life of nothing, which stands in for one that is lent to code of a host.
#[derive(Default)]
struct Outside {
  hosted: Hosted,
  crossing: Crossing,
  opened: Opened,
  /// How many ids the host gave, which names the next.
  given: u128,
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
      // A gate that cannot read a word raises where the gate was called, so the ear of the gate answers that word.
      None if name == GATE => return gated(args.first()).map_or_else(Answer::Fault, Answer::Value),
      None if name == AGAIN => Ok(Object::bool(self.reborn(args.first()))),
      None if name == TURN => {
        let bounded = args.first().is_some_and(|one| one.as_ref().as_bool().unwrap_or(false));
        Ok(Object::bool(sand.bound(bounded)))
      }
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
  Ok(Object::list(found.into_iter().map(|one| {
    let line = i64::try_from(one.line).unwrap_or_default();
    Object::tuple([Object::int(line), Object::string(one.to_string())])
  })))
}

impl Outside {
  /// The side of the host, on what the opening gave: a dict of its callables by name, each of which
  /// the engine keeps.
  fn new(handed: Handed, opened: &Object) -> Result<Outside, Fault> {
    let mut held: HashMap<String, Object> = (opened.as_ref().pairs().unwrap_or_default())
      .into_iter()
      .filter_map(|(name, one)| Some((name.as_str()?.to_owned(), one.to_owned())))
      .collect();
    let kept = held.values().filter_map(|one| one.as_ref().handle()).collect();
    let opened = Opened::of(|name| {
      held.remove(name).ok_or_else(|| Fault::refused(format!("the opening gives no {name}")))
    })?;
    Ok(Outside {
      hosted: Hosted { handed, ..Hosted::default() },
      crossing: Crossing { kept, ..Crossing::default() },
      opened,
      given: 0,
    })
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
    let (site, speaks) = (self.opened.site.clone(), self.opened.speaks.clone());
    let previous = sand.call(self, &site, vec![], vec![])?;
    if let Some(value) = value.filter(|one| !one.as_ref().is_none()) {
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

  /// The side of the host of a life that a snapshot holds, on the ears that the host gives again: each ear stands
  /// under the id and at the place where its earlier ear stood, and the host holds what the engine held.
  fn restored(handed: Handed, table: &Table) -> Result<Outside, Fault> {
    let mut standing = HashMap::new();
    for (id, name, waits) in &table.ears {
      if !handed.ears.contains_key(name) {
        return Err(Fault::refused(format!(
          "the snapshot holds the ear {name}, which the host does not give"
        )));
      }
      standing.insert(*id, (name.clone(), if *waits { At::Waiting } else { At::Over }));
    }
    let given = table
      .given
      .parse()
      .map_err(|no| Fault::refused(format!("a snapshot counts the ids of the host: {no}")))?;
    let kept = table.kept.iter().copied().collect();
    Ok(Outside {
      hosted: Hosted { handed, standing, ..Hosted::default() },
      crossing: Crossing { kept, ..Crossing::default() },
      opened: table.opened.clone(),
      given,
    })
  }

  /// What the host calls while the life waits on it, answered at once: who speaks, a callable the engine made, what
  /// the gate finds against a word on a chain with no fact, or a verb of the engine, as it goes out.
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
      "against" => {
        let (args, _) = self.inwards(sand, &args, vec![])?;
        let against = self.opened.against.clone();
        let got = sand.call(self, &against, args, vec![])?;
        self.outward(sand, &got)
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

/// What a snapshot holds of the side of the host beside the session of the sandbox: the root, each ear of the host by
/// the id of its object in the sandbox, its name and whether it waits, the handles that the engine keeps, the callables
/// of the opening, how many ids the host gave, and the work of the outside that ran.
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct Table {
  pub(crate) root: String,
  pub(crate) ears: Vec<(MontyUuid, String, bool)>,
  pub(crate) kept: Vec<MontyUuid>,
  pub(crate) opened: Opened,
  pub(crate) given: String,
  /// The work of the outside that ran: each act by its name, with the name of the ear that took it.
  pub(crate) working: Vec<(String, String)>,
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
  /// Where the life keeps its snapshot, when it keeps one.
  keeper: Option<Keeper>,
  /// Whether the life opened from a snapshot.
  restored: bool,
  /// The highest number that each kind took in the record that the life opened on, as a map, which
  /// a checkpoint of the life keeps taken: the journal does not make again an act that an ear of
  /// the outside made, so the transcripts of the life do not hold each of those names.
  taken: Object,
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
  pub fn pump(&mut self, waker: &Waker) -> Result<(), Fault> {
    if !self.outside.hosted.waker.will_wake(waker) {
      self.outside.hosted.waker.clone_from(waker);
    }
    let Some(first) = self.outside.spoken() else { return Ok(()) };
    self.run(|sand, outside| outside.said(sand, first))
  }

  /// An engine on a sandbox and the side of its host, before any run: the record says what work it
  /// left and what loops, and whether the life opened from a snapshot.
  fn new(sand: Sand, outside: Outside, record: &[Object], root: String, restored: bool) -> Engine {
    Engine {
      sand,
      outside,
      watchers: HashMap::new(),
      root,
      raised: None,
      stopped: None,
      holding: left(record),
      parked: looping(record),
      keeper: None,
      restored,
      taken: crate::checkpoint::taken(record),
    }
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
    let (given, names) = Handed::of(ears)?;
    let mut sand = Sand::new();
    // The opening, the seed and the replay are the engine's own work: unbounded, but for each word
    // they run again.
    sand.bound(false);
    let mut inputs = NamedValues::new();
    inputs.push("__source", Object::string(ENGINE));
    inputs.push("__sheet_source", Object::string(SHEET));
    inputs.push("__kernel_source", Object::string(KERNEL));
    inputs.push("__gate", Object::function(GATE, None));
    inputs.push("__again", Object::function(AGAIN, None));
    inputs.push("__turn", Object::function(TURN, None));
    let opened = sand.run(OPENING, inputs, &mut Nobody("the opening"))?;
    let mut outside = Outside::new(given, &opened)?;
    let Opened { kernel, gate, lasting, parking, park, .. } = outside.opened.clone();
    let record = seeded(&mut sand, &mut outside, record.into_iter().collect())?;
    let mut engine = Engine::new(sand, outside, &record, String::new(), false);
    let names_parked = Object::list(engine.parked.iter().map(|(id, _)| Object::string(id)));
    let parks = !engine.parked.is_empty();
    let (root, raised) = engine.run(|sand, outside| {
      outside.bound(sand)?;
      outside.crossing.recorded = true;
      let entries = recorded(sand, outside, &record);
      outside.crossing.recorded = false;
      let entries = sand.call(outside, &outside.opened.recorded.clone(), vec![entries?], vec![])?;
      let mut given = vec![("kernel".to_owned(), kernel), ("gate".to_owned(), gate)];
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
      let got = outside.named_call(sand, "boot", vec![entries], given);
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
    engine.root = root;
    engine.raised = raised;
    Ok(engine)
  }

  /// An engine opened from the snapshot of a life on the same record, on the ears that the host gives again under the
  /// names that the snapshot holds: the session of the sandbox that the snapshot holds, with each ear born and told
  /// the past of the life, so no word runs again. The work of the outside that ran when the snapshot was taken waits
  /// for a wake, as the work that a record left waits in a replay.
  pub(crate) fn from_snapshot<N: Into<String>>(
    record: &[Object],
    ears: impl IntoIterator<Item = (N, Box<dyn Ear>)>,
    table: &Table,
    sand: Sand,
    past: &[Object],
  ) -> Result<Engine, Fault> {
    let (given, _) = Handed::of(ears)?;
    let outside = Outside::restored(given, table)?;
    let mut engine = Engine::new(sand, outside, record, table.root.clone(), true);
    engine.run(|sand, outside| {
      outside.bound(sand)?;
      outside.retold(sand, past)
    })?;
    let ran = table.working.iter().filter(|(id, _)| engine.holding.contains(id));
    let ran = Object::dict(ran.map(|(id, name)| (Object::string(id), Object::string(name))));
    engine.word("__restored.update(__ran)", vec![("__ran", ran)])?;
    Ok(engine)
  }

  /// What a snapshot holds of this life: the side of the host, and the session of the sandbox. Nothing when boot
  /// raised, or when the sandbox holds what a later host cannot give again: a function of the host, or an ear that a
  /// verb was given. The work of the outside that runs does not keep a snapshot from a life, since a working life
  /// always holds some: the snapshot names it, and a later life holds it as a replay holds it.
  pub(crate) fn snapshot(&self) -> Option<(Table, Vec<u8>)> {
    let outside = &self.outside;
    let foreign =
      outside.hosted.standing.values().any(|(name, at)| name.starts_with(EAR) && *at != At::Over);
    if self.raised.is_some()
      || self.stopped.is_some()
      || !outside.hosted.handed.calls.is_empty()
      || foreign
    {
      return None;
    }
    let dump = self.sand.dump().ok()?;
    let mut ears: Vec<_> = outside
      .hosted
      .standing
      .iter()
      .map(|(id, (name, at))| (*id, name.clone(), *at == At::Waiting))
      .collect();
    ears.sort_unstable_by_key(|one| one.0);
    let mut kept: Vec<_> = outside.crossing.kept.iter().copied().collect();
    kept.sort_unstable();
    let opened = outside.opened.clone();
    let mut working: Vec<_> =
      outside.hosted.working.iter().map(|(id, name)| (id.clone(), name.clone())).collect();
    working.sort_unstable();
    let given = outside.given.to_string();
    Some((Table { root: self.root.clone(), ears, kept, opened, given, working }, dump))
  }

  /// What `body` gives, run as the engine's own work, with no bound of the turn but for each word
  /// that it steps.
  pub(crate) fn unbounded<T>(&mut self, body: impl FnOnce(&mut Engine) -> T) -> T {
    let was = self.sand.bound(false);
    let got = body(self);
    self.sand.bound(was);
    got
  }

  /// Whether the life opened from the snapshot of its record, with no replay.
  pub fn restored(&self) -> bool {
    self.restored
  }

  /// The life keeps its snapshot where the keeper says, when it ends.
  pub(crate) fn keep(&mut self, keeper: Keeper) {
    self.keeper = Some(keeper);
  }

  /// The highest number that each kind took in the record that the life opened on.
  pub(crate) fn taken(&self) -> &Object {
    &self.taken
  }

  /// Whether an ear of the host hears under a name.
  pub(crate) fn hears(&self, name: &str) -> bool {
    self.outside.hosted.handed.ears.contains_key(name)
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
    let mut holding = Vec::new();
    for id in std::mem::take(&mut self.holding) {
      if !self.whether(TOOK, &id)? {
        holding.push(id);
      }
    }
    self.holding.clone_from(&holding);
    let mut pending = Vec::new();
    for id in holding {
      if self.parked.iter().any(|(one, _)| *one == id) || !self.whether(WAITS, &id)? {
        continue;
      }
      let (mut made, mut at) = (Vec::new(), id.clone());
      while let Some(act) = self.get(&at)? {
        let by = act.by().to_owned();
        if WORK.contains(&act.kind()) && self.whether(WAITS, &at)? {
          made.push((std::mem::replace(&mut at, by), act.kind().to_owned()));
        } else {
          at = by;
        }
      }
      for one in made.into_iter().rev() {
        if !pending.contains(&one) {
          pending.push(one);
        }
      }
    }
    Ok(pending)
  }

  /// The work that an earlier life left, put to the outside again with no wake from a host: a
  /// wake of each act of [`Engine::pending`] that no other act of it made, so the World asks each
  /// reply of it again, takes each wait and each prompt to the operator of it again, and closes
  /// each command of it as interrupted, and the model of its chain reads that end in a turn. A
  /// life that opens on a record and does work says it once, at its tip, since a resume returns to
  /// the state of that tip, whose work goes on. A parked act and an act that a pause holds stay as
  /// they are.
  pub fn resume(&mut self) -> Result<(), Fault> {
    let pending = self.pending()?;
    let mut tops = Vec::new();
    for (id, _) in &pending {
      let by = self.get(id)?.map(|act| act.by().to_owned()).unwrap_or_default();
      if !pending.iter().any(|(one, _)| *one == by) {
        tops.push(id.clone());
      }
    }
    for id in tops {
      self.wake(&id)?;
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
  /// when it is, after the entry in which it was. This is how a door that drives the engine itself awaits. A name of no
  /// act is refused, since no done can settle it, as `Act.__await__` refuses it.
  pub fn watch(&mut self, id: &str, then: impl FnMut(&Object) + 'static) -> Result<(), Fault> {
    if self.get(id)?.is_none() {
      return Err(Fault::refused(format!("{id} never settles")));
    }
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

impl Drop for Engine {
  /// A life that ends keeps its snapshot where its keeper says, only when it said a fact since the
  /// last one, which is known before the sandbox dumps its session: a failure leaves the snapshot
  /// before it, since a snapshot is a cache. A life writes none while an ear is behind, since the
  /// record or its log lacks a fact that the sandbox holds. No run of a life writes one, so the
  /// thread of the life writes no large file while it goes on.
  fn drop(&mut self) {
    if self.outside.behind() || !self.keeper.as_ref().is_some_and(Keeper::grew) {
      return;
    }
    if let Some((table, dump)) = self.snapshot()
      && let Some(keeper) = self.keeper.as_mut()
    {
      keeper.write(&table, &dump);
    }
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

#[cfg(test)]
#[path = "engine.test.rs"]
mod test;
