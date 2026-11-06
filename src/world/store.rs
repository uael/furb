//! The store: the record of a life, which one process at a time owns by a lease. A record whose name ends in
//! `.sqlite`, `.sqlite3` or `.db` is a database of SQLite, and any other record is a file of lines, one fact per line.

mod error;
mod lines;
pub(super) mod schema;
mod sqlite;

use std::{
  cell::RefCell,
  collections::VecDeque,
  ffi::OsStr,
  fs::{File, OpenOptions, TryLockError, create_dir_all},
  path::Path,
  rc::Rc,
  time::{Duration, SystemTime, UNIX_EPOCH},
};

use futures::stream;

use serde_json::value::RawValue;
use sha2::{Digest, Sha256};

use monty_types::{
  NamedValues,
  unstable::{MontyGraph, MontyNode},
};

use self::{error::StoreError, lines::Head};
use super::{beside, free};
use crate::{
  ENGINE, KERNEL,
  ear::{Co, Ear, Hooks, Next, With, hearing, say},
  engine::ROOT,
  fact::Fact,
  sand::{Nobody, Sand},
  value::{Fault, Object, ObjectRef, entry},
  wire::{built, whole},
};

use crate::checkpoint;
use crate::fact;
pub use lines::Lines;
pub use schema::{CHECKPOINT_SCHEMA, RECORD_SCHEMA, Schema, SchemaError};
pub use sqlite::Sqlite;
use std::io;
use tokio::time;

/// The place that keeps the record of a life, which a host can write for a place of its own and give to `keeping`.
///
/// A store owns its record from its opening to its end, by a lease, so no other store opens on the record while it
/// lives. A life reads the entries once, as it opens, before it keeps anything. Each store passes the suite in
/// `store.test.rs`.
pub trait Store {
  /// Each entry that the record holds, in the order that the store kept them, each the fact it kept in a list of
  /// one, as boot takes them.
  fn entries(&mut self) -> Result<Vec<Object>, Fault>;

  /// Keep one entry after each entry that the store kept before it. When keep returns, the entry outlives a crash of
  /// the process. When keep fails, the record holds nothing of the entry, so a later keep of the same entry holds it
  /// once.
  fn keep(&mut self, entry: ObjectRef<'_>) -> Result<(), Fault>;

  /// Make each entry that the store kept durable. When sync returns, each entry kept before it outlives a loss of
  /// power. The life syncs its store once at the end of each run of the sandbox, so a loss of power loses at most what
  /// the last run kept.
  fn sync(&mut self) -> Result<(), Fault>;

  /// The folder of the record and the bytes that its disk still takes, which the life reads as it
  /// keeps, so it can tell the operator before a keep fails. A store that does not say gives
  /// nothing, and the life tells nothing before a keep fails.
  fn room(&self) -> Option<(&Path, u64)> {
    None
  }
}

/// The room of the record at a path: the folder that holds it and the bytes that its disk still
/// takes, nothing when the disk does not say.
fn room(path: &Path) -> Option<(&Path, u64)> {
  let folder =
    path.parent().filter(|parent| !parent.as_os_str().is_empty()).unwrap_or(Path::new("."));
  Some((folder, free(folder)?))
}

/// The record at a path, read under its lease, and the ear that keeps on it what the journal says to keep: a
/// database of SQLite when the name of the path ends in `.sqlite`, `.sqlite3` or `.db`, and else a file of lines.
pub fn store(path: impl AsRef<Path>) -> Result<(Vec<Object>, Box<dyn Ear>), Fault> {
  let path = path.as_ref();
  keeping(Format::of(path).open(path)?)
}

/// What the store kept at a path, read with no lease and changed in nothing on the disk, as a later life would open
/// on it.
pub fn kept(path: impl AsRef<Path>) -> Result<Vec<Object>, Fault> {
  let path = path.as_ref();
  Ok(Format::of(path).kept(path)?.into_iter().map(streamed).collect())
}

/// Each entry that the store kept at a path, in order, read with no lease and changed in nothing, one at a time, so
/// a reader holds no more of a large record than it keeps of each entry.
pub(crate) fn each(path: impl AsRef<Path>, take: &mut dyn FnMut(Object)) -> Result<(), Fault> {
  let path = path.as_ref();
  Ok(Format::of(path).each(path, take)?)
}

/// What the heads of the lines of a record tell, which one pass over the record reads: the entries of its facts made
/// and of its checkpoints, in order, the place of each checkpoint among the entries, counted from 0, and the standing
/// that its last answered stand gave.
#[derive(Default)]
pub struct Marks {
  pub heads: Vec<Object>,
  pub checkpoints: Vec<usize>,
  pub standing: Option<Object>,
}

impl Marks {
  fn hear(&mut self, place: usize, entry: Object) {
    if let Some(standing) = stood(&entry) {
      self.standing = Some(standing);
      return;
    }
    let kind = entry.as_ref().items().and_then(|held| Fact::of(*held.first()?));
    match kind.as_ref().map(Fact::kind) {
      Some(checkpoint::CHECKPOINT) => {
        self.checkpoints.push(place);
        self.heads.push(entry);
      }
      Some(MADE) => self.heads.push(entry),
      _ => {}
    }
  }
}

/// The marks of the record at a path, read with no lease and changed in nothing. A file of lines is read from the
/// head of each line in one pass, and parses only the lines of a fact made, of a checkpoint and of the done of a stand.
pub fn marks(path: impl AsRef<Path>) -> Result<Marks, Fault> {
  let path = path.as_ref();
  let format = Format::of(path);
  let mut marks = Marks::default();
  if format == Format::Lines {
    let heads = [Head::Kind(MADE), Head::Kind(checkpoint::CHECKPOINT), Head::Done("stand")];
    lines::heads(path, &heads, &mut |place, entry| marks.hear(place, entry))?;
  } else {
    for (place, entry) in format.kept(path)?.into_iter().enumerate() {
      marks.hear(place, entry);
    }
  }
  Ok(marks)
}

/// The standing that an entry gives, when it is the done of a stand of the root: the value of that done, when it is a
/// list. This is the one place that finds the answer of a stand.
pub fn stood(entry: &Object) -> Option<Object> {
  let fact = Fact::of(*entry.as_ref().items()?.first()?)?;
  let value = fact.word(0)?;
  let stood =
    fact.kind() == "done" && fact::named(fact.about(), "stand") && value.items().is_some();
  stood.then(|| value.to_owned())
}

/// The kind of the fact by which a record names the build that made it: `("made", "chain1",
/// "store", engine, version, schema, release, source, kernel)`, where engine is what [`engine`]
/// gives, version is null, which a build before the texts gave its version of the replay, schema
/// is [`RECORD_SCHEMA`], release is the release of furb that
/// made it, as `0.4.2`, and source and kernel are the texts of its engine and of its Kernel, which
/// a later life runs to replay the segment that the fact begins, as [`texts`] says. A record of a
/// build before the texts holds none, one of a build before the versions holds no version, and one
/// of a build before the schemas holds no schema and no release. No contract holds it, so this is
/// the one home of its shape.
pub const MADE: &str = "made";

/// The engine of this build, which a record names: a digest of the engine and of the Kernel, which make each act of
/// a replay. Another engine can make other acts from the same record, so a record replays only on the engine that
/// made it.
pub fn engine() -> String {
  digest(ENGINE, KERNEL)
}

/// The digest of the texts of an engine and of its Kernel, which a fact made names.
fn digest(source: &str, kernel: &str) -> String {
  let mut hash = Sha256::new();
  for one in [source, kernel] {
    hash.update(one.as_bytes());
    hash.update([0]);
  }
  format!("{:x}", hash.finalize())
}

/// The engine that a record names, and nothing when it names none, as a record of a build before the fact made.
pub fn made(record: &[Object]) -> Option<String> {
  record.iter().find_map(|entry| {
    let fact = Fact::of(*entry.as_ref().items()?.first()?)?;
    (fact.kind() == MADE).then(|| fact.word(0)?.as_str().map(str::to_owned)).flatten()
  })
}

/// The texts of the engine and of the Kernel that a segment of a record runs on: those that its
/// last fact made holds, or this build's own when it holds none. Texts that are not the engine that
/// their fact names refuse the record.
pub fn texts(record: &[Object]) -> Result<(String, String), Fault> {
  let held = record.iter().rev().find_map(|entry| {
    let fact = Fact::of(*entry.as_ref().items()?.first()?)?;
    let word = |at| fact.word(at).and_then(|one| one.as_str().map(str::to_owned));
    (fact.kind() == MADE).then(|| (word(0), word(4), word(5)))
  });
  let Some((Some(named), Some(source), Some(kernel))) = held else {
    return Ok((ENGINE.to_owned(), KERNEL.to_owned()));
  };
  if digest(&source, &kernel) != named {
    return Err(Fault::refused(format!(
      "the record names the engine {named}, and the texts that it holds are another engine"
    )));
  }
  Ok((adapted(&named, source)?, kernel))
}

/// The adapters of the engines of this major, by the digest of an engine, as `src/furb/adapters.py`
/// says.
const ADAPTERS: &str = include_str!("../furb/adapters.py");

/// The text of the engine that a digest names, with each change of its adapter made, or as it is
/// when no adapter names it. A change whose text the engine does not hold is refused, since the
/// adapter is then of another engine.
fn adapted(named: &str, mut source: String) -> Result<String, Fault> {
  let mut inputs = NamedValues::new();
  inputs.push("__named", Object::string(named));
  let word = format!("{ADAPTERS}\nADAPTERS.get(__named, [])");
  let changes = Sand::new().run(&word, inputs, &mut Nobody("the adapters"))?;
  for change in changes.as_ref().items().unwrap_or_default() {
    let text = |at| entry(&change, at).and_then(|one| one.as_str().map(str::to_owned));
    let (Some(old), Some(new)) = (text(0), text(1)) else { continue };
    if !source.contains(&old) {
      return Err(Fault::refused(format!("the adapter of the engine {named} finds no {old:?}")));
    }
    source = source.replacen(&old, &new, 1);
  }
  Ok(source)
}

/// The release of furb that a record names, as `0.4.2`, and nothing when it names none, as a record
/// of a build before the release was in the fact made.
pub fn release(record: &[Object]) -> Option<String> {
  record.iter().find_map(|entry| {
    let fact = Fact::of(*entry.as_ref().items()?.first()?)?;
    (fact.kind() == MADE).then(|| fact.word(3)?.as_str().map(str::to_owned)).flatten()
  })
}

/// Whether this build reads a record from its facts made: each names the schema of the record that
/// its build wrote, and a record that names a schema of another major is refused, with both
/// schemas.
pub fn readable(record: &[Object]) -> Result<(), SchemaError> {
  for entry in record {
    let Some(fact) = entry.as_ref().items().and_then(|held| Fact::of(*held.first()?)) else {
      continue;
    };
    if fact.kind() == MADE {
      RECORD_SCHEMA.opens("record", fact.word(2))?;
    }
  }
  Ok(())
}

/// The kind of the fact by which a provider says a part of the word of a rung that its model finished.
const WRITING: &str = "writing";

/// An entry as this build reads it. A build before the writing was a fact kept each writing as a question,
/// `("writing", writingN, by, chain, rung, part, before)`, which the provider made as its model wrote. A later life
/// asks no model again, so no ear makes that question again, and the journal only keeps its name: the Kernel runs no
/// part where the life ran it, and the replay drifts. The entry of such a question is read as the writing that a
/// provider says now, `("writing", rung, by, part, before)`, which the journal says again where the life said it.
fn streamed(entry: Object) -> Object {
  let fact = entry.as_ref().items().and_then(|held| Fact::of(*held.first()?));
  let Some(fact) = fact.filter(|one| one.kind() == WRITING && one.question()) else {
    return entry;
  };
  let (Some(rung), Some(part), Some(before)) = (fact.word(1), fact.word(2), fact.word(3)) else {
    return entry;
  };
  let said = [
    Object::string(WRITING),
    rung.to_owned(),
    Object::string(fact.by()),
    part.to_owned(),
    before.to_owned(),
  ];
  Object::list([Object::list(said)])
}

/// The fact made of this build: its engine, no version, the schema of its record,
/// its release, and the texts of its engine and of its Kernel.
fn named() -> Fact {
  let build = [
    Object::string(engine()),
    Object::none(),
    RECORD_SCHEMA.object(),
    Object::string(env!("CARGO_PKG_VERSION")),
    Object::string(ENGINE),
    Object::string(KERNEL),
  ];
  Fact::says(MADE, ROOT, build)
}

/// How long the ear of the record waits before it tries again to write the entries that it could not write.
const AGAIN: Duration = Duration::from_secs(5);

/// The free bytes of the disk of the record under which the ear tells the operator that the disk is
/// almost full. It tells again only once the disk held twice as much, so a disk that stands near
/// the bound gives one notice.
const LOW: u64 = 1 << 30;

/// What the ear of the record told the operator last, so it tells each change once.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Told {
  /// The record takes each entry, and its disk has room, or the operator was told nothing yet.
  Room,
  /// The disk of the record holds less than [`LOW`] free.
  Low,
  /// The record takes no entry.
  Stopped,
}

/// The record that a store holds, and the ear that keeps in the store what the journal says to keep.
///
/// A new record names the engine and the version of this build: the ear says them as it hears the root at its birth,
/// so a later life says them again at the same place. A record of a build before the fact made names no engine, and
/// stays so, since this engine did not make it.
///
/// The ear tells the operator once when the disk of the record holds less than [`LOW`] free, with
/// the folder and the space left, before a keep fails. An entry that the store cannot keep, as on a
/// full disk, waits in memory with each entry after it, and the ear tries again in order at each
/// keep, at the end of each run and every few seconds, until they land: no entry is lost or kept
/// twice, and the life goes on. The ear tells the operator once when the record stops growing, with
/// the fault, and once when it grows again. A message that did not go is said again at the next
/// pass.
pub fn keeping(store: impl Store + 'static) -> Result<(Vec<Object>, Box<dyn Ear>), Fault> {
  let mut store = Held { store, waiting: VecDeque::new(), fault: None };
  let entries: Vec<Object> = store.store.entries()?.into_iter().map(streamed).collect();
  let store = Rc::new(RefCell::new(store));
  let held = Rc::clone(&store);
  // A record whose last checkpoint has no fact made after it is new to the build that goes on from it.
  let new = entries.is_empty()
    || checkpoint::last(&entries).is_some_and(|at| made(&entries[at + 1..]).is_none());
  let kept = hearing(&["keep", "chain"], move |mut co: Co<()>| async move {
    // What the ear told the operator last, and whether a try waits for its time.
    let (mut said, mut again) = (Told::Room, false);
    loop {
      match co.next().await {
        Next::Heard(a) | Next::Resumed(a) => match a.kind() {
          "keep" => {
            let Some(entry) = a.word(0) else { continue };
            held.borrow_mut().waiting.push_back(entry.to_owned());
          }
          "chain" if new && a.about() == ROOT => {
            say(&mut co, named());
            continue;
          }
          _ => continue,
        },
        Next::Worked(()) => again = false,
      }
      let fault = held.borrow_mut().flushed();
      match fault {
        Some(fault) => {
          if !again {
            again = true;
            co.work(stream::once(time::sleep(AGAIN)));
          }
          if said != Told::Stopped {
            // A message that cannot go leaves what the ear said, so the next pass says it again.
            if told(&mut co, stopped(&fault)).await.is_ok() {
              said = Told::Stopped;
            }
          }
        }
        None if said == Told::Stopped => {
          let message =
            "The record of this life is written again: it holds each fact that waited, in order.";
          if told(&mut co, message.into()).await.is_ok() {
            said = Told::Room;
          }
        }
        None => {
          let low = held.borrow().low(&mut said);
          if let Some(message) = low
            && told(&mut co, message).await.is_ok()
          {
            said = Told::Low;
          }
        }
      }
    }
  });
  Ok((entries, With::ear(kept, store)))
}

/// The message that the record stopped growing, with the fault of the store.
fn stopped(fault: &Fault) -> String {
  format!(
    "The record of this life cannot be written: {}. The life goes on, and holds in memory each fact \
     that the record does not hold yet. It tries again until the record takes them, in order, and tells \
     you then. Give the disk room.",
    fault.message()
  )
}

/// A message to the operator, from the ear of the record, or why it did not go.
async fn told(co: &mut Co<()>, message: String) -> Result<(), Fault> {
  let to = [("to", Object::string("operator")), ("on", Object::string(ROOT))];
  co.call("prompt", vec![Object::none(), Object::string(message)], to.into()).await.map(drop)
}

/// A store, and the entries that it could not keep yet, in order, with the last fault of the store.
struct Held<S> {
  store: S,
  waiting: VecDeque<Object>,
  /// The fault of the last keep that failed, or of the last sync that failed, until a sync succeeds with no entry
  /// waiting.
  fault: Option<Fault>,
}

impl<S: Store> Held<S> {
  /// The notice that the disk of the record is almost full, when the operator knows nothing of it
  /// yet. A disk that holds twice [`LOW`] again ends what the operator knows, so a later fall under
  /// the bound tells again.
  fn low(&self, said: &mut Told) -> Option<String> {
    let (folder, free) = self.store.room()?;
    if free >= 2 * LOW {
      *said = Told::Room;
    }
    (free < LOW && *said == Told::Room).then(|| {
      format!(
        "The disk of the record of this life is almost full: {} has {} MB left. When it is full, the \
         life holds each fact in memory until the record takes them again: give the disk room.",
        folder.display(),
        free / 1_000_000
      )
    })
  }

  /// Each entry that waits kept in order, and the fault that stops the record, if one does.
  fn flushed(&mut self) -> Option<Fault> {
    while let Some(entry) = self.waiting.front() {
      if let Err(fault) = self.store.keep(entry.as_ref()) {
        self.fault = Some(fault);
        break;
      }
      self.waiting.pop_front();
    }
    self.fault.clone()
  }

  /// Each entry that waits kept, then each entry kept made durable, which ends the fault when it succeeds.
  fn synced(&mut self) {
    if self.flushed().is_some() && !self.waiting.is_empty() {
      return;
    }
    self.fault = self.store.sync().err();
  }
}

/// The ear that keeps the record syncs its store at the end of each run.
impl<S: Store> Hooks for Rc<RefCell<Held<S>>> {
  /// A fault of the store ends no run: the ear holds it, tells the operator and tries again.
  fn ran(&mut self, _: &mut dyn Ear) -> Result<(), Fault> {
    self.borrow_mut().synced();
    Ok(())
  }

  /// Each entry that waits for the store, which no keep took yet.
  fn unkept(&self, _: &dyn Ear) -> usize {
    self.borrow().waiting.len()
  }
}

/// A store that a box holds is a store, so a record opens in the store that its format gives.
impl<S: Store + ?Sized> Store for Box<S> {
  fn entries(&mut self) -> Result<Vec<Object>, Fault> {
    (**self).entries()
  }

  fn keep(&mut self, entry: ObjectRef<'_>) -> Result<(), Fault> {
    (**self).keep(entry)
  }

  fn sync(&mut self) -> Result<(), Fault> {
    (**self).sync()
  }

  fn room(&self) -> Option<(&Path, u64)> {
    (**self).room()
  }
}

/// How the record at a path is kept, which its name says: [`Format::of`] chooses it once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Format {
  /// A file of lines, one fact per line.
  Lines,
  /// A database of SQLite, one row for each entry.
  Sqlite,
}

impl Format {
  /// The format of the record at a path: a database when its name ends in `.sqlite`, `.sqlite3`
  /// or `.db`.
  pub(crate) fn of(path: &Path) -> Self {
    let named = path.extension().and_then(OsStr::to_str).unwrap_or_default();
    if ["sqlite", "sqlite3", "db"].iter().any(|one| named.eq_ignore_ascii_case(one)) {
      Self::Sqlite
    } else {
      Self::Lines
    }
  }

  /// The store of the record at a path, under its lease.
  pub(crate) fn open(self, path: &Path) -> Result<Box<dyn Store>, Fault> {
    Ok(match self {
      Self::Sqlite => Box::new(Sqlite::open(path)?),
      Self::Lines => Box::new(Lines::open(path)?),
    })
  }

  /// Each entry of the record at a path, in order, read with no lease and changed in nothing.
  fn each(self, path: &Path, take: &mut dyn FnMut(Object)) -> Result<(), StoreError> {
    match self {
      Self::Sqlite => sqlite::each(path, take),
      Self::Lines => lines::each(path, take),
    }
  }

  /// What the record at a path holds, read with no lease and changed in nothing.
  fn kept(self, path: &Path) -> Result<Vec<Object>, StoreError> {
    let mut entries = Vec::new();
    self.each(path, &mut |one| entries.push(one))?;
    Ok(entries)
  }
}

/// The entry of one fact, from the JSON that the record holds of the fact, or nothing when it is no fact: a fact is
/// a list of three words at least, whose first word is its kind.
fn entry_of(fact: &RawValue) -> Option<Object> {
  let mut graph = MontyGraph::new();
  let fact = built(&mut graph, fact, 1).ok()?;
  let root = graph.push(MontyNode::List(vec![fact]));
  let entry = whole(graph, root).ok()?;
  let fact = entry.as_ref().items().and_then(|held| held.first().and_then(ObjectRef::items));
  let holds = fact.is_some_and(|fact| fact.len() >= 3 && fact[0].as_str().is_some());
  holds.then_some(entry)
}

/// The time now, in milliseconds since the start of 1970.
fn now() -> u64 {
  let since = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
  u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
}

/// The lease of a record: the lock on the file beside it, which ends when the lease goes. A process that this one
/// starts holds a copy of the file until its program runs, so the lease ends the lock itself, and not with the last
/// copy of the file.
struct Lease(File);

impl Drop for Lease {
  #[expect(
    clippy::let_underscore_must_use,
    reason = "a drop cannot return the fault of an unlock, and the kernel ends the lock with the process"
  )]
  fn drop(&mut self) {
    let _: io::Result<()> = self.0.unlock();
  }
}

/// The lease of the record at a path, or the refusal when another process holds it: the lock on the file beside
/// the record, `<record>.lock`, which every store of the crate takes, and which the kernel ends when the process ends.
///
/// The lease is the lock on the file at the path, and nothing checks which file that is. A rename of the file would
/// leave a lock that no later process meets, and on macOS a lookup of a renamed path can go on giving the moved file
/// to every process for seconds, so no holder moves or removes the lock file.
fn leased(path: &Path) -> Result<Lease, StoreError> {
  let lock = beside(path, ".lock");
  if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
    create_dir_all(parent).map_err(StoreError::io(&lock))?;
  }
  let file = OpenOptions::new()
    .read(true)
    .write(true)
    .create(true)
    .truncate(false)
    .open(&lock)
    .map_err(StoreError::io(&lock))?;
  match file.try_lock() {
    Ok(()) => Ok(Lease(file)),
    Err(TryLockError::WouldBlock) => Err(StoreError::Owned { path: path.to_path_buf() }),
    Err(TryLockError::Error(no)) => Err(StoreError::io(&lock)(no)),
  }
}

#[cfg(test)]
#[path = "store.test.rs"]
mod tests;
