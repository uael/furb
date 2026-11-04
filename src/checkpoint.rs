//! The checkpoint of a life: what each chain that lives holds, as plain data that a build of any
//! version of the major reads, so a later life goes on from it, on another engine too, with no
//! replay of what came before it.
//!
//! The fact is `("checkpoint", "chain1", "store", data)`. Its data is a map: `schema`, the version
//! of its own shape, [`SCHEMA`]; `version` and `engine`, the build that wrote it; `taken`, the
//! highest number of each kind; `chains`, each chain that lives with its parent, its label, its
//! source, whether it is paused, its turns, the text of its definitions, its plain values by name,
//! the names that it lost, and the acts that the checkpoint cut, each word that ran on it among
//! them, which no later life runs again; and
//! `acts`, each act that a value names, the last stand and the last clock, with its value, or, for
//! an open prompt, with its shape, its message, its actor and the ear that held it. `gathered` of
//! the engine gathers it and `seed` of the engine reads it, so the engine is the one home of its
//! shape, and this file holds the fact, the store and the boot alone. It holds no word that the
//! crate crafts and no word of a verb by its place: `seed` of the engine that opens it makes the
//! entries that a life boots on, so any later engine of the major opens it.
//!
//! A life ends with its checkpoint, which the ear of its store keeps, since a life born from a
//! checkpoint makes acts of its own at its birth; [`keep`] writes one on a record that no life
//! holds. A later life boots on the last checkpoint of its record, whatever engine wrote it, as
//! [`booted`] says, so it replays the checkpoint and the entries after it alone.
//!
//! The checkpoint of the build before the schema holds `entries` in the place of the data: facts
//! that the journal of any engine replays, and `("reserve", kind, "store", n)`, which stands for
//! each name of that kind up to n that no entry carries. A later life boots on them as they stand,
//! so a life that a build of that form handed over goes on.

#![expect(clippy::match_bool, reason = "debt: a match on a bool")]
#![expect(
  clippy::needless_pass_by_value,
  reason = "debt: an argument taken by value and only borrowed"
)]
#![expect(
  clippy::redundant_closure_for_method_calls,
  reason = "debt: a closure where a method path serves"
)]

use std::{
  collections::{HashMap, HashSet},
  path::Path,
};

use crate::{
  Engine,
  engine::{ROOT, callable},
  extension::definitions,
  fact::Fact,
  value::{Fault, Object, ObjectRef, entry},
  world::{self, Lines, Sqlite, Store, VERSION},
};

/// The kind of the fact of a checkpoint.
pub const CHECKPOINT: &str = "checkpoint";

/// The version of the shape of the data of a checkpoint, apart from the version of the record. The
/// data of schema 1 also holds `again`, each word that ran on a chain, which `seed` names as cut.
pub const SCHEMA: i64 = 2;

/// The kind of an entry of a checkpoint of the form before [`SCHEMA`] that stands for the names of
/// a kind that no entry carries.
const RESERVE: &str = "reserve";

/// The kind of an entry of a checkpoint of the form before [`SCHEMA`] that holds the turns of a
/// chain.
const CARRIED: &str = "carried";

/// The entry of the checkpoint of a life at its tip, as the store keeps it.
pub fn written(engine: &mut Engine) -> Result<Object, Fault> {
  let define = callable(|args: Vec<Object>| {
    let word = args.first().and_then(|one| one.as_ref().as_str().map(str::to_owned));
    Ok(Object::string(definitions(&word.unwrap_or_default())))
  });
  let floor = engine.taken().clone();
  let data = engine.gathered(define, floor)?;
  let mut fields = vec![
    (Object::string("schema"), Object::int(SCHEMA)),
    (Object::string("version"), Object::int(VERSION)),
    (Object::string("engine"), Object::string(world::engine())),
  ];
  let pairs = data.as_ref().pairs().unwrap_or_default();
  fields.extend(pairs.into_iter().map(|(key, one)| (key.to_owned(), one.to_owned())));
  let fact =
    [CHECKPOINT, ROOT, "store"].map(Object::string).into_iter().chain([Object::dict(fields)]);
  Ok(Object::list([Object::list(fact)]))
}

impl Engine {
  /// The checkpoint of the life at its tip, as the last entry of its record: the ear of its store
  /// keeps it after each entry that it kept, as it keeps each fact, and the life ends with it, so a
  /// later life on the record opens from it. A life that is born from a checkpoint makes acts of
  /// its own at its birth, so a fact that this life said after the checkpoint would not replay
  /// there. The life ends too when it keeps no record, which it refuses.
  pub fn checkpoint(mut self) -> Result<(), Fault> {
    if !self.hears("store") {
      return Err(Fault::refused("a life that keeps no record keeps no checkpoint"));
    }
    let entry = written(&mut self)?;
    self.word("say('keep', '', __entry)", vec![("__entry", entry)]).map(drop)
  }
}

/// An entry kept after the last entry of the record at a path, as the store of a life keeps one,
/// for a record that no life holds.
pub fn keep(path: &Path, one: Object) -> Result<(), Fault> {
  match world::database(path) {
    true => Sqlite::open(path)?.keep(one.as_ref()),
    false => Lines::open(path)?.keep(one.as_ref()),
  }
}

/// An entry of a checkpoint as the record reads it: a fact, which the record holds as the mark of a
/// tuple.
fn unmarked(one: ObjectRef<'_>) -> Option<Object> {
  let Some(pairs) = one.pairs() else { return Some(one.to_owned()) };
  let field =
    |name: &str| pairs.iter().find(|(key, _)| key.as_str() == Some(name)).map(|(_, value)| *value);
  match field("is").and_then(|is| is.as_str()) {
    Some("tuple") => field("args")?.items()?.first().map(|args| args.to_owned()),
    _ => Some(one.to_owned()),
  }
}

/// The highest number that each kind took in a record, its checkpoints among them, as a map. A
/// later life makes again no act that an ear of the outside made, as the merged of a command, so
/// its transcripts do not hold each name that the record took, and the checkpoint keeps each of
/// them taken.
pub(crate) fn taken(record: &[Object]) -> Object {
  let opened;
  let record = match last(record) {
    Some(_) => {
      opened = booted(record.to_vec());
      opened.as_slice()
    }
    None => record,
  };
  let mut most: Vec<(String, i64)> = Vec::new();
  for a in record.iter().filter_map(fact).filter(Fact::question) {
    let Some(n) = a.about().strip_prefix(a.kind()).and_then(|one| one.parse::<i64>().ok()) else {
      continue;
    };
    match most.iter_mut().find(|one| one.0 == a.kind()) {
      Some(one) => one.1 = one.1.max(n),
      None => most.push((a.kind().to_owned(), n)),
    }
  }
  Object::dict(most.into_iter().map(|(kind, n)| (Object::string(kind), Object::int(n))))
}

/// The fact of an entry.
fn fact(one: &Object) -> Option<Fact> {
  entry(&one.as_ref(), 0).and_then(Fact::of)
}

/// Where the last checkpoint of a record stands, and nothing when it holds none.
pub fn last(record: &[Object]) -> Option<usize> {
  record.iter().rposition(|one| fact(one).is_some_and(|a| a.kind() == CHECKPOINT))
}

/// The entries that the build of the last life on a record made: those after its last checkpoint,
/// or the whole record when it holds none.
pub fn segment(record: &[Object]) -> &[Object] {
  &record[last(record).map_or(0, |at| at + 1)..]
}

/// The record as a life boots on it: the entries of its last checkpoint, whatever engine wrote it,
/// each reserve as the questions of the root that it stands for, then the entries after it; or the
/// record as it stands when it holds no checkpoint.
pub fn booted(record: Vec<Object>) -> Vec<Object> {
  let Some(at) = last(&record) else { return record };
  let mut out = carried(&record[at]);
  out.extend(record.into_iter().skip(at + 1));
  out
}

/// The entries that a checkpoint carries: the fact itself when it holds data, which boot gives
/// `seed` of the engine, or, for the form before [`SCHEMA`], its entries, each reserve as the
/// questions of the root that it stands for, and the turns of each chain just after that chain.
fn carried(checkpoint: &Object) -> Vec<Object> {
  let Some(a) = fact(checkpoint) else { return Vec::new() };
  let pairs = a.word(0).and_then(|one| one.pairs()).unwrap_or_default();
  let field =
    |name: &str| pairs.iter().find(|(key, _)| key.as_str() == Some(name)).map(|one| one.1);
  let Some(held) = field("entries").and_then(|one| one.items()) else {
    return vec![checkpoint.clone()];
  };
  let held: Vec<Object> = held.into_iter().filter_map(unmarked).collect();
  let facts: Vec<Fact> = held.iter().filter_map(|one| Fact::of(one.as_ref())).collect();
  let carried: HashSet<String> = facts
    .iter()
    .filter(|a| a.kind() != RESERVE && a.question())
    .map(|a| a.about().to_owned())
    .collect();
  let (mut out, mut reserved) = (Vec::new(), HashMap::<String, i64>::new());
  // A later life of that form wrote the turns of a chain after the stand of the root, and the
  // root seeds itself at its first stand unless it holds its turns, so it ran its seed again and
  // drifted: the turns of each chain come at once after the chain.
  for a in facts.iter().filter(|a| a.kind() != CARRIED) {
    if a.kind() != RESERVE {
      out.push(Object::list([a.0.clone()]));
      let turns = facts
        .iter()
        .filter(|b| a.kind() == "chain" && b.kind() == CARRIED && b.about() == a.about());
      out.extend(turns.map(|b| Object::list([b.0.clone()])));
      continue;
    }
    let (kind, n) = (a.about(), a.word(0).and_then(|one| one.as_int()).unwrap_or_default());
    let from = reserved.get(kind).copied().unwrap_or_default();
    for name in (from + 1..=n).map(|i| format!("{kind}{i}")).filter(|name| !carried.contains(name))
    {
      out.push(Object::list([Object::list([kind, &name, ROOT, ""].map(Object::string))]));
    }
    reserved.insert(kind.to_owned(), from.max(n));
  }
  out
}
