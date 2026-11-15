//! The checkpoint of a life: what each chain that lives holds, as plain data that a build of any
//! version of the major reads, so a later life goes on from it, on another engine too, with no
//! replay of what came before it.
//!
//! The fact is `("checkpoint", "chain1", "store", data)`. Its data is a map: `schema`, the version
//! of its own shape, [`CHECKPOINT_SCHEMA`]; `engine`, the engine that wrote it; `taken`, the
//! highest number of each kind; `chains`, each chain that lives with its parent, its label, its
//! source, whether it is paused, its turns, the definitions of each word, its plain values by name,
//! the names that it lost, and the acts that the checkpoint cut, each word that ran on it among
//! them, which no later life runs again; and `acts`, each act that a value names, the last stand
//! and the last clock, with its value and its words, or, for an open prompt, with its shape, its
//! message, its actor and the ear that held it. `gathered` of the checkpointer of this build
//! gathers it, on the engine of an older record too when the record crosses, and `seed` of the
//! checkpointer reads it, so the checkpointer is the one home of its shape, and this file holds
//! the fact, the store and the boot alone. It holds no word that the crate crafts and no word of a
//! verb by its place: `seed` of the checkpointer of the engine that opens it makes the entries that
//! a life boots on, so any later engine of the major opens it. Each
//! reader skips each field that it does not know, so a build opens a checkpoint of each minor of
//! its schema, and refuses only one of another major.
//!
//! A life that ends with no image, as `src/image.rs` says, ends with its checkpoint, which the ear
//! of its store keeps, since a life born from a checkpoint makes acts of its own at its birth, then
//! with the fact made of the texts that it runs, so the life born on it runs them too; [`crossed`]
//! writes one on a life that ran another engine, then the fact made of this build, so the life born
//! on it runs this build: `furb checkpoint` on a record that no life holds, and the open of a life
//! that keeps its record. A later life boots on the last checkpoint of its record, whatever engine
//! wrote it, as [`booted`] says, so it replays the checkpoint and the entries after it alone, word
//! by word, whatever build of the major made them: a resume returns to the exact state at the tip.
//! A tail that a build cannot replay with no drift is a defect, and the record then opens to read
//! only. Each checkpoint is a birth: the life that writes it ends there, so the facts after it come
//! from a life born on it. An open from the first entry is born again at each checkpoint in the
//! same way, as [`reborn`] proves, and reaches the state of the open from the last one.

#![expect(
  clippy::needless_pass_by_value,
  reason = "debt: an argument taken by value and only borrowed"
)]

use std::{path::Path, slice};

use similar::TextDiff;

use crate::{
  Engine,
  engine::{OPERATOR, ROOT, callable},
  fact::Fact,
  minify::minified,
  skill::definitions,
  value::{Fault, Object, ObjectRef, entry, field},
  wire,
  world::{self, CHECKPOINT_SCHEMA, Store, Systems},
};

/// The kind of the fact of a checkpoint.
pub const CHECKPOINT: &str = "checkpoint";

/// The entry of the checkpoint of a life at its tip, as the store keeps it, before the fact made of
/// the texts that the life runs, so the life born on it runs them too, as [`written_before`] says.
pub fn written(engine: &mut Engine) -> Result<Object, Fault> {
  let next = engine.running().clone();
  written_before(engine, &next)
}

/// The entry of the checkpoint of a life at its tip, as the store keeps it, before the fact made of
/// the texts that the life born on it runs. Its engine is the one that the fact made of the texts
/// that the life runs names. Its `systems` holds the system prompt of each chain whose model reads
/// an engine that the next life does not run, as [`Systems`] says. When the next life runs another
/// engine, the checkpoint crosses: each chain keeps the system prompt that it read, and its turns
/// end with one paragraph that tells it the change, the lines of the minified engine that differ.
fn written_before(engine: &mut Engine, next: &Object) -> Result<Object, Fault> {
  let define = callable(|args: Vec<Object>| {
    let word = args.first().and_then(|one| one.as_ref().as_str().map(str::to_owned));
    Ok(Object::string(definitions(&word.unwrap_or_default())))
  });
  let floor = engine.taken().clone();
  // The gather is the engine's own work over the whole life, so no bound of a turn cuts it.
  let data = engine.unbounded(|engine| engine.checkpointed("gathered", vec![define, floor]))?;
  let mut next = Next::of(engine.running(), next)?;
  let mut fields = vec![
    (Object::string("schema"), CHECKPOINT_SCHEMA.object()),
    (
      Object::string("engine"),
      Object::string(world::made(slice::from_ref(engine.running())).unwrap_or_default()),
    ),
  ];
  for (key, one) in data.as_ref().pairs().unwrap_or_default() {
    let one = match key.as_str() {
      Some("chains") => {
        let chains = one.items().unwrap_or_default().into_iter();
        Object::list(chains.map(|chain| next.chain(engine, chain)).collect::<Result<Vec<_>, _>>()?)
      }
      _ => one.to_owned(),
    };
    fields.push((key.to_owned(), one));
  }
  if !next.systems.0.is_empty() {
    fields.push((Object::string("systems"), next.systems.object()));
  }
  let fact =
    [CHECKPOINT, ROOT, "store"].map(Object::string).into_iter().chain([Object::dict(fields)]);
  Ok(Object::list([Object::list(fact)]))
}

/// What a checkpoint says of each chain to the life born on it: the system prompt of the life that
/// writes it, the one of the next life, the lines that differ when they differ, and the system
/// prompt of each chain that does not read the one of the next life.
struct Next {
  running: String,
  after: String,
  diff: Option<String>,
  systems: Systems,
}

impl Next {
  /// From the fact made of the texts that the life runs, and of those that the next life runs.
  fn of(running: &Object, next: &Object) -> Result<Self, Fault> {
    let (running, after) = (system(running)?, system(next)?);
    let diff = (running != after).then(|| {
      let diff = TextDiff::from_lines(&running, &after);
      diff
        .unified_diff()
        .context_radius(0)
        .missing_newline_hint(false)
        .header("engine", "engine")
        .to_string()
    });
    Ok(Self { running, after, diff, systems: Systems::default() })
  }

  /// A chain of the data as the next life reads it: the engine that its model reads joins
  /// `systems` when the next life runs another one, and at a crossing its turns end with one
  /// paragraph that tells it the lines of the minified engine that differ.
  fn chain(&mut self, engine: &mut Engine, chain: ObjectRef<'_>) -> Result<Object, Fault> {
    let id = field(&chain, "id").and_then(|one| one.as_str()).unwrap_or_default().to_owned();
    let lineage = engine.lineage(&id)?;
    let read = engine.systems().of(lineage.iter().map(String::as_str));
    let read = read.unwrap_or(&self.running).to_owned();
    if read != self.after {
      self.systems.hold(read, id.clone());
    }
    let Some(diff) = &self.diff else { return Ok(chain.to_owned()) };
    let mut name = format!("{id}_diff");
    while format!("{diff}\n").contains(&format!("</s:{name}>\n")) {
      name.push('_');
    }
    let note =
      Object::list([Object::string(format!("#{id} crossed\n<s:{name}>\n{diff}</s:{name}>"))]);
    let tell = Object::tuple(["tell", &id, "store"].map(Object::string).into_iter().chain([note]));
    let pairs = chain.pairs().unwrap_or_default().into_iter().map(|(key, one)| {
      let one = match key.as_str() {
        Some("turns") => {
          let turns = one.items().unwrap_or_default().into_iter().map(|turn| turn.to_owned());
          Object::list(turns.chain([tell.clone()]))
        }
        _ => one.to_owned(),
      };
      (key.to_owned(), one)
    });
    Ok(Object::dict(pairs.collect::<Vec<_>>()))
  }
}

/// The system prompt of the life of a fact made: the engine that it holds, minified.
fn system(made: &Object) -> Result<String, Fault> {
  let (source, ..) = world::texts(slice::from_ref(made))?;
  minified(&source).ok_or_else(|| Fault::refused("this build cannot minify the engine of a life"))
}

/// The system prompt of each chain that the last checkpoint of a record holds, as `systems` of its
/// data says. A checkpoint of a build before that field holds none, and each of its chains reads
/// the engine that the life runs.
pub(crate) fn systems(record: &[Object]) -> Systems {
  let data = last(record).and_then(|at| fact(&record[at])?.word(0).map(|one| one.to_owned()));
  let held = data.as_ref().and_then(|data| field(&data.as_ref(), "systems")?.items());
  let read = |one: ObjectRef<'_>| {
    let [text, chains] = one.items()?[..] else { return None };
    let chains = chains.items()?.into_iter().filter_map(|one| one.as_str().map(str::to_owned));
    Some((text.as_str()?.to_owned(), chains.collect()))
  };
  Systems(held.unwrap_or_default().into_iter().filter_map(read).collect())
}

impl Engine {
  /// The checkpoint of the life at its tip, then the fact made of the texts that the life runs, as
  /// the last entries of its record: the ear of its store keeps them after each entry that it kept,
  /// as it keeps each fact, and the life ends with them, so a later life on the record opens from
  /// the checkpoint on the same texts. A life that is born from a checkpoint makes acts of its own
  /// at its birth, so a fact that this life said after the checkpoint would not replay there. The
  /// life ends too when it keeps no record, which it refuses.
  pub fn checkpoint(mut self) -> Result<(), Fault> {
    if !self.hears("store") {
      return Err(Fault::refused("a life that keeps no record keeps no checkpoint"));
    }
    let (entry, made) = (written(&mut self)?, self.running().clone());
    let word = "(say('keep', '', __entry), say('keep', '', __made))";
    self.word(word, vec![("__entry", entry), ("__made", made)]).map(drop)
  }
}

/// The checkpoint of the life of an engine at the tip of the record at a path, which no other life
/// holds, then the fact made of this build: the life born on that checkpoint runs the engine, the
/// Kernel and the checkpointer of this build, and each chain of it keeps the system prompt that it
/// read and is told the lines of the engine that differ. This is the one place where a record
/// crosses to the texts of a build.
pub fn crossed(path: &Path, engine: &mut Engine) -> Result<(), Fault> {
  let build = world::running(&[]);
  keep(path, written_before(engine, &build)?)?;
  keep(path, build)
}

/// Whether the checkpoint at a place of a record is a birth that a life gathers again: the life
/// that wrote it ended there, so the entry after it is the fact made of the next life, and a life
/// born on the entries before it, as the engine given gathers it, holds each field of the
/// checkpoint as the checkpoint holds it. Only this build gathers as this build wrote, so a
/// checkpoint that another engine wrote is held to the first rule alone. The refusal names the checkpoint and the first field that
/// parts, with both of its values where they part.
pub fn reborn(record: &[Object], at: usize, engine: &mut Engine) -> Result<(), Fault> {
  let place = at + 1;
  if record.get(at + 1).and_then(fact).is_some_and(|a| a.kind() != world::MADE) {
    return Err(Fault::refused(format!(
      "the checkpoint at entry {place} is no birth: its life went on after it, and only a life that ends writes one"
    )));
  }
  let Some(held) = fields(&record[at]) else { return Ok(()) };
  let engine_of = |one: &[(String, serde_json::Value)]| {
    one.iter().find(|one| one.0 == "engine").map(|one| one.1.clone())
  };
  if engine_of(&held) != Some(serde_json::Value::from(world::engine())) {
    return Ok(());
  }
  // The life that gathers makes its own stand at its birth, after the tip that the life which wrote
  // the checkpoint ended on, so it took one name more, and the tail after the checkpoint holds the
  // names that the checkpoint gave.
  for (key, value) in
    fields(&written(engine)?).unwrap_or_default().into_iter().filter(|one| one.0 != "taken")
  {
    let kept = held.iter().find(|one| one.0 == key).map(|one| &one.1);
    if kept.is_some_and(|kept| *kept != value) {
      let (value, kept) = (value.to_string(), kept.map(ToString::to_string).unwrap_or_default());
      let parts = value.chars().zip(kept.chars()).take_while(|(a, b)| a == b).count();
      let short =
        |one: &str| one.chars().skip(parts.saturating_sub(100)).take(300).collect::<String>();
      let (value, kept) = (short(&value), short(&kept));
      return Err(Fault::refused(format!(
        "the checkpoint at entry {place} drifts in {key}: a life born on the entries before it gathers {value}, the record holds {kept}"
      )));
    }
  }
  Ok(())
}

/// Each field of the data of a checkpoint, as the record keeps it.
fn fields(one: &Object) -> Option<Vec<(String, serde_json::Value)>> {
  let a = fact(one)?;
  let pairs = a.word(0)?.pairs()?;
  Some(
    pairs
      .into_iter()
      .filter_map(|(key, value)| Some((key.as_str()?.to_owned(), wire::record(value))))
      .collect(),
  )
}

/// An entry kept after the last entry of the record at a path, as the store of a life keeps one,
/// for a record that no life holds.
pub(crate) fn keep(path: &Path, one: Object) -> Result<(), Fault> {
  world::Format::of(path).open(path)?.keep(one.as_ref())
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

/// Where each checkpoint of a record stands, in order.
pub fn all(record: &[Object]) -> Vec<usize> {
  let checkpoints =
    record.iter().enumerate().filter(|(_, one)| fact(one).is_some_and(|a| a.kind() == CHECKPOINT));
  checkpoints.map(|(at, _)| at).collect()
}

/// Whether a fact ends the opening of a life born from a checkpoint: the stand of the root that the
/// operator asks as the life boots, or that the tail after the checkpoint replays. Before it, the
/// life says again what its checkpoint restates; from it on, each fact is one of its own life, a
/// fact equal to an earlier one among them. The fact is read by its kind, its speaker and the chain
/// that it stands on.
pub fn opened(kind: &str, by: &str, on: Option<&str>) -> bool {
  kind == "stand" && by == OPERATOR && on == Some(ROOT)
}

/// Whether a record holds a stand of the root that was answered.
pub fn stood(record: &[Object]) -> bool {
  record.iter().any(|one| world::stood(one).is_some())
}

/// The standing that the last answered stand gave in the last entries of a record: the done of a
/// stand after its last checkpoint, or else the stand that the checkpoint holds, when it was
/// answered.
pub(crate) fn standing(record: &[Object]) -> Option<Object> {
  segment(record).iter().rev().find_map(world::stood).or_else(|| {
    let data = fact(&record[last(record)?])?.word(0)?.to_owned();
    let acts = field(&data.as_ref(), "acts")?.items()?;
    let stand =
      |act: &&ObjectRef<'_>| field(act, "kind").is_some_and(|one| one.as_str() == Some("stand"));
    let stood = acts.iter().rev().find(stand)?;
    field(stood, "value").filter(|value| value.items().is_some()).map(|value| value.to_owned())
  })
}

/// The entries that the build of the last life on a record made: those after its last checkpoint,
/// or the whole record when it holds none.
pub fn segment(record: &[Object]) -> &[Object] {
  &record[last(record).map_or(0, |at| at + 1)..]
}

/// The record as a life boots on it: its last checkpoint, whatever engine wrote it, which boot gives
/// `seed` of the checkpointer, then the entries after it; or the record as it stands when it holds
/// no checkpoint.
pub fn booted(mut record: Vec<Object>) -> Vec<Object> {
  let Some(at) = last(&record) else { return record };
  record.split_off(at)
}
