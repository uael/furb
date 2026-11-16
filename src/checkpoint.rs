//! The carry of a life across builds: what each chain that lives holds, as plain data that a build
//! of any version of the major reads, so a life on another engine goes on from it, with no replay
//! of what came before it on that engine.
//!
//! The entry is `("checkpoint", "chain1", "store", data)`, which no record keeps: an open makes it
//! in memory at each place where the record crosses, as [`crossings`] says, and boots the next
//! engine on it and on the entries after it, so the record stays the one truth of the life (R148,
//! R354). Its data is a map: `schema`, the version of its own shape, [`CHECKPOINT_SCHEMA`];
//! `engine`, the engine that the carry leaves; `taken`, the highest number of each kind; `chains`,
//! each chain that lives with its parent, its label, its source, whether it is paused, its turns,
//! the definitions of each word, its plain values by name, the names that it lost, and the acts
//! that the carry cut, each word that ran on it among them, which no later life runs again;
//! `acts`, each act that a value names, the last stand and the last clock, with its value and its
//! words, or, for an open prompt, with its shape, its message, its actor and the ear that held it;
//! `jumped`, each chain that jumped, with the chain that it went on as, which the next life makes
//! again under its name, so a root that jumped stays the root;
//! and `lines`, the line of the word that made each act that a word made, which the host adds,
//! since only the sandbox of the engine that the carry leaves saw those words run.
//! `gathered` of the checkpointer that the next fact made names gathers it, so each later build
//! carries a record the same, and `seed` of the checkpointer of the next engine reads it, so the
//! checkpointer is the one home of its shape, and this file holds the entry and the places where a
//! record crosses alone.
//!
//! A life that keeps its record keeps the carry of its last crossing beside it, as
//! `<record>.carry`, so each segment before that crossing replays on its engine once, at the first
//! open that carries across it, and each later open boots on that carry. A life that crosses
//! from its own tip, as [`Engine::cross`] moves it, keeps there the carry that it gathers at that
//! tip, so no segment replays. It is a cache, as the image is: its key names each entry up to that
//! crossing and the texts of the build, a reader takes none that does not agree, and the open then
//! replays those segments again, to the same carry.

use std::{fs, path::Path, slice};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use similar::TextDiff;

use crate::{
  Engine,
  engine::{OPERATOR, ROOT, callable},
  fact::Fact,
  minify::minified,
  skill::definitions,
  value::{Fault, Object, ObjectRef, entry, field},
  wire,
  world::{self, CHECKPOINT_SCHEMA, Systems},
};

/// The kind of the fact of a carry.
pub const CHECKPOINT: &str = "checkpoint";

/// The entry of the carry of a life at its tip to the texts of the fact made `next`, which the next
/// engine boots on. Its engine is the one that the fact made of the texts that the life runs names.
/// Its `systems` holds the system prompt of each chain whose model reads an engine that the next
/// life does not run, as [`Systems`] says. Each chain keeps the system prompt that it read, and its
/// turns end with one paragraph that tells it the change, the lines of the minified engine that
/// differ.
pub(crate) fn carried(engine: &mut Engine, next: &Object) -> Result<Object, Fault> {
  let define = callable(|args: Vec<Object>| {
    let word = args.first().and_then(|one| one.as_ref().as_str().map(str::to_owned));
    Ok(Object::string(definitions(&word.unwrap_or_default())))
  });
  let floor = engine.taken().clone();
  // The gather is the engine's own work over the whole life, so no bound of a turn cuts it.
  let (_, _, by) = world::texts(slice::from_ref(next))?;
  let data = engine.unbounded(|engine| engine.gathered_for(&by, vec![define, floor]))?;
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
  // The next engine runs none of the words before the carry, so it takes the line of each act
  // that they made from here.
  let lines =
    engine.lines().iter().map(|(name, line)| (Object::string(name), Object::int(i64::from(*line))));
  fields.push((Object::string("lines"), Object::dict(lines)));
  let fact =
    [CHECKPOINT, ROOT, "store"].map(Object::string).into_iter().chain([Object::dict(fields)]);
  // The carry crosses to the next engine as plain data, in the form that a record keeps, so the
  // next engine makes each value again from its data and holds no object of the engine that
  // gathered it.
  wire::inward(&wire::record(Object::list([Object::list(fact)]).as_ref()))
}

/// What a carry says of each chain to the life born on it: the system prompt of the life that
/// it leaves, the one of the next life, the lines that differ when they differ, and the system
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

/// The system prompt of each chain that the carry of the entries that a life boots on holds, as
/// `systems` of its data says. With no carry, each chain reads
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
  /// The move of a life that opened on the texts of another engine, Kernel or checkpointer to those
  /// of this build, as a step of its own: the fact made of this build, which the ear of its store
  /// keeps, so each later open carries the life across it, as [`crossings`] says. The life ends
  /// with it, and gives the carry of its tip to this build, which [`Crossed::keep`] keeps beside
  /// the record once the record holds that fact. A life that keeps no record is refused.
  pub fn cross(mut self) -> Result<Crossed, Fault> {
    if !self.hears("store") {
      return Err(Fault::refused("a life that keeps no record does not cross"));
    }
    let made = world::running(&[]);
    let carry = carried(&mut self, &made)?;
    // A keep is no fact of the life, so it waits in the log: the feed of no entry hands it on.
    self.word("say('keep', '', __made), feed(())", vec![("__made", made)])?;
    Ok(Crossed(carry))
  }

  /// Whether the life runs the texts of another engine, Kernel or checkpointer than this build, as
  /// a life that opened on the image of another build does until it crosses.
  pub fn crosses(&self) -> bool {
    world::texts(slice::from_ref(self.running())).ok() != world::texts(&[]).ok()
  }
}

/// Where a record crosses: each fact made whose engine differs from the one of the fact made before
/// it. The entries before the first such place replay on the texts of the first fact made, and the
/// entries from each such place on the texts that its fact names.
pub(crate) fn crossings(record: &[Object]) -> Vec<usize> {
  let (mut at, mut was) = (Vec::new(), None);
  for (n, one) in record.iter().enumerate() {
    let Some(engine) = world::made(slice::from_ref(one)) else { continue };
    if was.as_ref().is_some_and(|was| *was != engine) {
      at.push(n);
    }
    was = Some(engine);
  }
  at
}

/// The fact made of this build at the tip of the record at a path, after the entries that a read of
/// it gave, under a lease of the record that the caller holds: the move of a record to the texts of
/// this build, across which each later open carries it. This is the one place where a record
/// crosses to the texts of a build.
pub(crate) fn crossed(path: &Path, lease: world::Lease, read: &world::Last) -> Result<(), Fault> {
  world::appended(path, lease, read, world::running(&[]).as_ref())
}

/// The carry of a life at the tip where it crossed, gathered from the life itself. The life stood
/// at the exact state of that tip, so the open after the crossing boots on it and replays no
/// segment before it, as an open after a replay of those segments boots on the carry that it kept.
#[derive(Debug)]
pub struct Crossed(Object);

impl Crossed {
  /// Keeps the carry beside the record at a path, as [`keep`] keeps the carry of a replay, once the
  /// fact made of the crossing ends the record. A record that holds another entry after it keeps
  /// nothing, and the next open replays.
  pub fn keep(self, path: &Path) -> Result<(), Fault> {
    let record = world::kept(path)?;
    if let Some(at) = crossings(&record).last().filter(|at| **at + 1 == record.len()) {
      keep(path, &record, *at, self.0);
    }
    Ok(())
  }
}

/// The meaning of what [`Kept`] holds, which a change of meaning raises.
const KEPT: u32 = 1;

/// The carry at a crossing of a record, as `<record>.carry` holds it, with its key: the place of
/// that crossing among the entries, the digest of each entry up to it, and the texts of the build,
/// which gathers the carry.
#[derive(Serialize, Deserialize)]
struct Kept {
  version: u32,
  engine: String,
  at: usize,
  entries: String,
  carry: Object,
}

/// The digest of entries, as JSON.
fn digest(entries: &[Object]) -> Option<String> {
  Some(format!("{:x}", Sha256::digest(serde_json::to_vec(entries).ok()?)))
}

/// The carry kept beside the record at a path, with the place of its crossing, when its key agrees
/// with the entries of the record.
pub(crate) fn kept(path: &Path, record: &[Object]) -> Option<(usize, Object)> {
  let kept: Kept = serde_json::from_slice(&fs::read(world::beside(path, ".carry")).ok()?).ok()?;
  let agrees = kept.version == KEPT
    && kept.engine == world::engine()
    && kept.at < record.len()
    && digest(&record[..=kept.at]).is_some_and(|one| one == kept.entries);
  agrees.then_some((kept.at, kept.carry))
}

/// The carry at the crossing at `at` of the entries of the record at a path, kept beside it. A
/// cache that does not land costs the next open a replay, and nothing else.
pub(crate) fn keep(path: &Path, record: &[Object], at: usize, carry: Object) {
  let Some(entries) = digest(&record[..=at]) else { return };
  let kept = Kept { version: KEPT, engine: world::engine(), at, entries, carry };
  if let Ok(bytes) = serde_json::to_vec(&kept) {
    fs::write(world::beside(path, ".carry"), bytes).ok();
  }
}

/// The highest number that each kind took in the entries that a life boots on, as a map. A later
/// life makes again no act that an ear of the outside made, as the merged of a command, so its
/// transcripts do not hold each name that the record took, and the carry keeps each of them taken.
pub(crate) fn taken(record: &[Object]) -> Object {
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

/// Where the carry of the entries that a life boots on stands, and nothing when they hold none.
pub(crate) fn last(record: &[Object]) -> Option<usize> {
  record.iter().rposition(|one| fact(one).is_some_and(|a| a.kind() == CHECKPOINT))
}

/// Whether a fact ends the opening of a life born from a carry: the stand of the root that the
/// operator asks as the life boots, or that the tail after the carry replays. Before it, the life
/// says again what its carry restates; from it on, each fact is one of its own life, a
/// fact equal to an earlier one among them. The fact is read by its kind, its speaker and the chain
/// that it stands on.
pub fn opened(kind: &str, by: &str, on: Option<&str>) -> bool {
  kind == "stand" && by == OPERATOR && on == Some(ROOT)
}

/// Whether a record holds a stand of the root that was answered.
pub fn stood(record: &[Object]) -> bool {
  record.iter().any(|one| world::stood(one).is_some())
}

/// The entries that the last engine of the entries that a life boots on made: those after the
/// carry, or all of them when they hold none.
pub fn segment(record: &[Object]) -> &[Object] {
  &record[last(record).map_or(0, |at| at + 1)..]
}
