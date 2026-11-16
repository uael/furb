//! The store: the record of a life, which one process at a time owns by a lease. A record whose name ends in
//! `.sqlite`, `.sqlite3` or `.db` is a database of SQLite, and any other record is a file of lines, one fact per line.

mod error;
mod lines;
pub(super) mod schema;
mod sqlite;

use std::{
  collections::VecDeque,
  ffi::OsStr,
  fs::{self, File, OpenOptions, TryLockError, create_dir_all},
  io::{BufRead, BufReader, Read, Seek, SeekFrom},
  path::Path,
  slice,
  sync::Arc,
  time::{Duration, SystemTime, UNIX_EPOCH},
};

use futures::stream;

use serde::{Deserialize, Serialize};
use serde_json::{Value, value::RawValue};
use sha2::{Digest, Sha256};

use monty_types::unstable::{MontyGraph, MontyNode};

use self::error::StoreError;
use super::{beside, free};
use crate::{
  CHECKPOINTER, ENGINE, KERNEL,
  ear::{Co, Ear, Kept, Next, hearing, say},
  engine::ROOT,
  fact::Fact,
  value::{Fault, Object, ObjectRef},
  wire::{built, whole},
};

use crate::checkpoint;
use crate::fact;
pub use lines::Lines;
use lines::Tail;
pub(crate) use lines::spoken;
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
  /// Each entry that the record holds, in the order that the store kept them, each the fact it
  /// kept, its place and whether the life rested, in a list, as boot takes them.
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

  /// The bytes of a last line that a stop tore as it was written, which the entries passed over and
  /// the next keep cuts, so the life tells the operator once. A store that tears no line gives
  /// nothing.
  fn cut(&self) -> Option<u64> {
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

/// The entries of a record that a life opens on: each entry that it holds.
#[derive(Default)]
pub(crate) struct Last {
  pub entries: Vec<Object>,
  /// The byte of the file of lines where the read ended, and how the last line ends there.
  end: u64,
  tail: Tail,
}

/// Keeps one entry after the entries of the record at a path that a read gave, under a lease of it
/// that the caller holds: a last line that a stop tore goes first, as the first keep of a life cuts
/// it, so the entry follows the last line written whole.
pub(crate) fn appended(
  path: &Path,
  lease: Lease,
  read: &Last,
  entry: ObjectRef<'_>,
) -> Result<(), Fault> {
  match Format::of(path) {
    Format::Sqlite => Sqlite::under(path, lease)?.keep(entry),
    Format::Lines => {
      let mut store = Lines::under(path, lease)?;
      let ended = Last { entries: Vec::new(), end: read.end, tail: read.tail };
      store.last(Some(ended), &mut |_| {})?;
      store.keep(entry)
    }
  }
}

/// The last entries of the record at a path, under its lease, and the ear that [`store`] gives. A
/// file of lines that still ends where a read with no lease ended holds what that read gave, so
/// the store takes those entries and reads no line. Else it reads them, with the bytes that the
/// read passed told after each line. A database reads each time and tells none.
pub(crate) fn stored(
  path: &Path,
  lease: Lease,
  read: Option<Last>,
  reached: &mut dyn FnMut(u64),
) -> Result<(Last, Box<dyn Ear>), Fault> {
  match Format::of(path) {
    Format::Sqlite => {
      let (entries, ear) = keeping(Sqlite::under(path, lease)?)?;
      Ok((Last { entries, ..Last::default() }, ear))
    }
    Format::Lines => {
      let mut store = Lines::under(path, lease)?;
      let Last { entries, end, tail } = store.last(read, reached)?;
      let (entries, ear) = kept_by(store, Some(entries));
      Ok((Last { entries, end, tail }, ear))
    }
  }
}

/// The entries of the record at a path that stand after the first `bytes` bytes of its file of
/// lines, where the image of a life stands, and, for a life that keeps the record, the ear that
/// [`store`] gives, under the lease of the record. The life of the image kept each entry before
/// those bytes, so the record names its engine already, and a life that crashed after it went on
/// from the image kept each entry after them: the life that opens from the image says them again,
/// as a boot says a record again. Nothing when no life goes on from the image: the record is
/// shorter than those bytes, or an entry after them is a checkpoint or names an engine, since each
/// fact after such an entry comes from a life that was born on it.
pub(crate) fn stored_after(
  path: &Path,
  bytes: u64,
  lease: Option<Lease>,
) -> Result<Option<After>, Fault> {
  let mut store = lease.map(|lease| Lines::under(path, lease)).transpose()?;
  if fs::metadata(path).is_ok_and(|held| held.len() < bytes) {
    return Ok(None);
  }
  let (entries, tail) = lines::after(path, bytes)?;
  let fact = |entry: &Object| entry.as_ref().items().and_then(|held| Fact::of(*held.first()?));
  if entries.iter().any(|entry| fact(entry).is_some_and(|one| born(one.kind()))) {
    return Ok(None);
  }
  if let Some(store) = &mut store {
    store.ends(tail);
  }
  Ok(Some(After { entries, store: store.map(|store| kept_by(store, None).1) }))
}

/// Whether a fact of this kind is a birth that the facts after it come from: a checkpoint, or the
/// fact that names an engine.
fn born(kind: &str) -> bool {
  matches!(kind, checkpoint::CHECKPOINT | MADE)
}

/// How many of the last bytes of a record its tip names by their digest.
const TAIL: u64 = 4096;

/// The tip of a record: its length, and the sha256 of its last bytes. A record only grows, so each
/// entry that it takes changes both, and a cache of a record names the tip that it was made from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tip {
  pub bytes: u64,
  pub tail: String,
}

impl Tip {
  /// The tip of the file of a record at its first `bytes` bytes, or at its end now.
  pub fn of(record: &Path, bytes: Option<u64>) -> io::Result<Self> {
    let mut file = File::open(record)?;
    let held = file.metadata()?.len();
    let bytes = bytes.unwrap_or(held).min(held);
    let mut last = Vec::new();
    file.seek(SeekFrom::Start(bytes.saturating_sub(TAIL)))?;
    file.take(bytes.min(TAIL)).read_to_end(&mut last)?;
    Ok(Self { bytes, tail: format!("{:x}", Sha256::digest(&last)) })
  }
}

/// Whether a life of the image that stands at the first `bytes` bytes of the file of lines at a
/// path can go on from it, as [`stored_after`] says, read from the start of each line after those
/// bytes alone, with no lease and no parse: a host asks it before an open.
pub(crate) fn goes_after(path: &Path, bytes: u64) -> bool {
  let from = |mut file: File| file.seek(SeekFrom::Start(bytes)).map(|_| file);
  let Ok(file) = File::open(path).and_then(from) else { return false };
  let (mut lines, mut head) = (BufReader::with_capacity(1 << 16, file), Vec::new());
  loop {
    head.clear();
    match lines.by_ref().take(256).read_until(b'\n', &mut head) {
      Ok(0) => return true,
      Ok(_) if spoken(&head).is_some_and(|[kind, ..]| born(kind)) => return false,
      Ok(_) if head.ends_with(b"\n") => {}
      Ok(_) if lines.skip_until(b'\n').is_ok() => {}
      _ => return false,
    }
  }
}

/// What [`stored_after`] gives: the entries after an image, and the ear of the store of a life
/// that keeps the record.
#[derive(Default)]
pub(crate) struct After {
  pub entries: Vec<Object>,
  pub store: Option<Box<dyn Ear>>,
}

/// The last entries of the record at a path, read with no lease and changed in nothing, as a later
/// life would open on them, with the bytes of a file of lines that the read passed told after each
/// line.
pub(crate) fn last(path: &Path, reached: &mut dyn FnMut(u64)) -> Result<Last, Fault> {
  let format = Format::of(path);
  let last = match format {
    Format::Sqlite => Last { entries: format.kept(path)?, ..Last::default() },
    Format::Lines => lines::last(path, reached)?,
  };
  Ok(Last { entries: last.entries.into_iter().map(streamed).collect(), ..last })
}

/// What the store kept at a path, read with no lease and changed in nothing on the disk, as a later life would open
/// on it.
pub fn kept(path: impl AsRef<Path>) -> Result<Vec<Object>, Fault> {
  let path = path.as_ref();
  Ok(Format::of(path).kept(path)?.into_iter().map(streamed).collect())
}

/// Each entry that the store kept at a path, in order, read with no lease and changed in nothing,
/// one at a time, so a reader holds no more of a large record than it keeps of each entry. A
/// checkpoint of a file of lines comes as the text of the JSON of its fact, as the record holds it,
/// with no parse: a checkpoint holds most of the bytes of a large record, and a reader that only
/// passes it on makes no value of it. A database gives each entry as a value. Each entry comes with
/// the bytes of the file of lines that the read passed, which a database does not count.
pub(crate) fn each_but_checkpoints(
  path: impl AsRef<Path>,
  take: &mut dyn FnMut(Taken, u64),
) -> Result<(), Fault> {
  let path = path.as_ref();
  match Format::of(path) {
    Format::Lines => {
      let head = format!("[[\"{}\",", checkpoint::CHECKPOINT);
      lines::each_but(path, head.as_bytes(), take)?;
    }
    Format::Sqlite => sqlite::each(path, &mut |entry| take(Taken::Entry(entry), 0))?,
  }
  Ok(())
}

/// An entry of a record as a read gives it: the value of the entry, or the text of the JSON of its
/// fact, as the record holds it.
pub(crate) enum Taken {
  Entry(Object),
  Fact(String),
}

impl Taken {
  /// The value of the entry, which a fact given as its text is parsed into, as a read of the
  /// record parses it.
  pub(crate) fn entry(self) -> Option<Object> {
    match self {
      Self::Entry(entry) => Some(entry),
      Self::Fact(text) => {
        serde_json::from_str::<&RawValue>(&text).ok().and_then(|fact| entry_of(&[fact]))
      }
    }
  }
}

/// The place of each fact made of the record at a path where it crosses to another engine, among
/// its entries, counted from 0, in order, as [`checkpoint::crossings`] says, read with no lease and
/// changed in nothing. A file of lines is read from the start of each line, and only its facts made
/// are parsed.
pub fn crossings(path: impl AsRef<Path>) -> Result<Vec<usize>, Fault> {
  let path = path.as_ref();
  let (mut places, mut made) = (Vec::new(), Vec::new());
  match Format::of(path) {
    Format::Lines => {
      let (head, mut place) = (format!("[[\"{MADE}\","), 0);
      let mut wanted = |line: &[u8]| {
        let wanted = line.starts_with(head.as_bytes());
        places.extend(wanted.then_some(place));
        place += 1;
        wanted
      };
      lines::heads(path, u64::MAX, &mut wanted, &mut |entry| made.push(streamed(entry)))?;
    }
    Format::Sqlite => {
      made = Format::Sqlite.kept(path)?;
      places = (0..made.len()).collect();
    }
  }
  Ok(checkpoint::crossings(&made).into_iter().filter_map(|at| places.get(at).copied()).collect())
}

/// The last fact made of the record at a path before a byte of its file of lines, which the read
/// finds from that byte toward the start of the file. Nothing before byte 0.
fn made_before(path: &Path, to: u64) -> Result<Option<Object>, Fault> {
  match to {
    0 => Ok(None),
    _ => Ok(lines::before(path, format!("[[\"{MADE}\",").as_bytes(), to)?),
  }
}

/// The standing that an entry gives, when it is the done of a stand of the root: the value of that
/// done, when it is a list. This is the one place that finds the answer of a stand.
pub fn stood(entry: &Object) -> Option<Object> {
  let fact = Fact::of(*entry.as_ref().items()?.first()?)?;
  let value = fact.word(0)?;
  let stood =
    fact.kind() == "done" && fact::named(fact.about(), "stand") && value.items().is_some();
  stood.then(|| value.to_owned())
}

/// The kind of the fact by which a record names the build that made it: `("made", "chain1",
/// "store", engine, version, schema, release, source, kernel, checkpointer)`, where engine is what
/// [`engine`] gives, version is null, which a build before the texts gave its version of the
/// replay, schema is [`RECORD_SCHEMA`], release is the release of furb that made it, as `0.4.2`,
/// and source and kernel are the texts of its engine and of its Kernel, which a later life runs to
/// replay the segment that the fact begins, as [`texts`] says. A record of a build before the texts
/// holds none, one of a build before the versions holds no version, and one of a build before the
/// schemas holds no schema and no release. No contract holds it, so this is the one home of its
/// shape.
pub const MADE: &str = "made";

/// The name that a life hears the ear of its store by: a life that hears it keeps its record.
pub(crate) const STORE: &str = "store";

/// The engine of this build, which a record names: a digest of the engine, of the Kernel and of the
/// checkpointer, which make each act of a replay and each birth. Another engine can make other acts
/// from the same record, so a record replays only on the engine that made it.
pub fn engine() -> String {
  digest(&[ENGINE, KERNEL, CHECKPOINTER])
}

/// The digest of the texts of an engine, of its Kernel and of its checkpointer, which a fact made
/// names.
fn digest(texts: &[&str]) -> String {
  let mut hash = Sha256::new();
  for one in texts {
    hash.update(one.as_bytes());
    hash.update([0]);
  }
  format!("{:x}", hash.finalize())
}

/// The engine that a record names, and nothing when it names none, as a new record.
pub fn made(record: &[Object]) -> Option<String> {
  record.iter().find_map(|entry| {
    let fact = Fact::of(*entry.as_ref().items()?.first()?)?;
    (fact.kind() == MADE).then(|| fact.word(0)?.as_str().map(str::to_owned)).flatten()
  })
}

/// The texts of the engine, of the Kernel and of the checkpointer that a segment of a record runs
/// on: those that its last fact made holds, or this build's own when it holds none, as a new
/// record. Texts that are not the engine that their fact names refuse the record.
pub fn texts(record: &[Object]) -> Result<(String, String, String), Fault> {
  let Some([Some(named), Some(source), Some(kernel), Some(checkpointer)]) = held(record) else {
    return Ok((ENGINE.to_owned(), KERNEL.to_owned(), CHECKPOINTER.to_owned()));
  };
  if digest(&[&source, &kernel, &checkpointer]) != named {
    return Err(Fault::refused(format!(
      "the record names the engine {named}, and the texts that it holds are another engine"
    )));
  }
  Ok((source, kernel, checkpointer))
}

/// The entry of the fact made of the texts that a segment of a record runs on, as [`texts`] gives
/// them: its last fact made when that fact holds the texts of its engine, or the fact made of this
/// build. A crossing writes it at the tip of the record, so the life born across that crossing
/// runs the same texts.
pub fn running(record: &[Object]) -> Object {
  let made = |entry: &&Object| {
    entry
      .as_ref()
      .items()
      .and_then(|held| Fact::of(*held.first()?))
      .is_some_and(|one| one.kind() == MADE)
  };
  match record.iter().rev().find(made) {
    Some(last) if matches!(held(slice::from_ref(last)), Some([Some(_), Some(_), Some(_), _])) => {
      last.clone()
    }
    _ => {
      let said = [MADE, ROOT, "store"].map(Object::string);
      Object::list([Object::list(said.into_iter().chain(build()))])
    }
  }
}

/// The digest, the engine, the Kernel and the checkpointer that the last fact made of a segment
/// names and holds.
fn held(record: &[Object]) -> Option<[Option<String>; 4]> {
  record.iter().rev().find_map(|entry| {
    let fact = Fact::of(*entry.as_ref().items()?.first()?)?;
    let word = |at| fact.word(at).and_then(|one| one.as_str().map(str::to_owned));
    (fact.kind() == MADE).then(|| [word(0), word(4), word(5), word(6)])
  })
}

/// The release of furb that a record names, as `0.4.2`, and nothing when it names none, as a record
/// of a build before the release was in the fact made.
pub fn release(record: &[Object]) -> Option<String> {
  record.iter().find_map(|entry| {
    let fact = Fact::of(*entry.as_ref().items()?.first()?)?;
    (fact.kind() == MADE).then(|| fact.word(3)?.as_str().map(str::to_owned)).flatten()
  })
}

/// Why no life opens the record at a path, read from its last fact made alone, as a record from
/// before the stable cut, and nothing when a life may open it: a host reads it before an open, to
/// show the facts of such a record with no life.
pub fn unopened(path: &Path) -> Option<SchemaError> {
  let made = made_before(path, fs::metadata(path).ok()?.len()).ok()??;
  readable(slice::from_ref(&made)).err()
}

/// Whether a life opens a record from its facts made: each names the schema of the record that its
/// build wrote, and a record that names a schema of another major, or one from before the stable
/// cut, is refused, as [`Schema::opens`] says.
pub fn readable(record: &[Object]) -> Result<(), SchemaError> {
  let fact = |entry: &Object| entry.as_ref().items().and_then(|held| Fact::of(*held.first()?));
  let made = record.iter().filter_map(fact).filter(|one| one.kind() == MADE);
  made.into_iter().try_for_each(|fact| RECORD_SCHEMA.opens("record", fact.word(2)))
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

/// The fact made of this build, as the store says it.
fn named() -> Fact {
  Fact::says(MADE, ROOT, build())
}

/// The words of the fact made of this build: its engine, no version, the schema of its record, its
/// release, and the texts of its engine, of its Kernel and of its checkpointer.
fn build() -> [Object; 7] {
  [
    Object::string(engine()),
    Object::none(),
    RECORD_SCHEMA.object(),
    Object::string(env!("CARGO_PKG_VERSION")),
    Object::string(ENGINE),
    Object::string(KERNEL),
    Object::string(CHECKPOINTER),
  ]
}

/// How long the ear of the record waits before it tries again to write the entries that it could not write.
/// A try that fails costs one write that the system refuses.
const AGAIN: Duration = Duration::from_secs(1);

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
/// A new record names the engine and the version of this build: the ear says them as it hears the
/// root at its birth, so a later life says them again at the same place.
///
/// The ear tells the operator once of the bytes of a torn last line that the open passed over.
/// It tells the operator once when the disk of the record holds less than [`LOW`] free, with
/// the folder and the space left, before a keep fails. An entry that the store cannot keep, as on a
/// full disk, waits in memory with each entry after it, and the ear tries again in order at each
/// keep, at the end of each run and every few seconds, until they land: no entry is lost or kept
/// twice, and the life goes on, while no other ear does work until they land, as `Engine::pump`
/// says. The ear tells the operator once when the record stops growing, with
/// the fault, and once when it grows again. A message that did not go is said again at the next
/// pass.
pub fn keeping(mut store: impl Store + 'static) -> Result<(Vec<Object>, Box<dyn Ear>), Fault> {
  let entries = store.entries()?;
  Ok(kept_by(store, Some(entries)))
}

/// The record and the ear of [`keeping`], for a store whose entries were read. A store that read
/// none, of a record that names its engine, gives no record.
fn kept_by<S: Store + 'static>(store: S, read: Option<Vec<Object>>) -> (Vec<Object>, Box<dyn Ear>) {
  let held = Held { store, waiting: VecDeque::new(), fault: None };
  let new = read.as_deref().is_some_and(unnamed);
  let entries: Vec<Object> = read.into_iter().flatten().map(streamed).collect();
  let kept = hearing(&["keep", "chain"], move |mut co: Co<()>| async move {
    // What the ear told the operator last, and whether a try waits for its time.
    let (mut said, mut again, mut cut) = (Told::Room, false, held.store.cut());
    co.keep(held);
    loop {
      match co.next().await {
        Next::Heard(a) | Next::Resumed(a) => match a.kind() {
          "keep" => {
            let Some(entry) = a.word(0) else { continue };
            co.kept::<Held<S>>()?.waiting.push_back(entry.to_owned());
          }
          "chain" if new && a.about() == ROOT => {
            say(&mut co, named());
            continue;
          }
          _ => continue,
        },
        Next::Worked(()) => again = false,
      }
      if let Some(bytes) = cut
        && told(&mut co, torn(bytes)).await.is_ok()
      {
        cut = None;
      }
      let fault = co.kept::<Held<S>>()?.flushed();
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
          let low = co.kept::<Held<S>>()?.low(&mut said);
          if let Some(message) = low
            && told(&mut co, message).await.is_ok()
          {
            said = Told::Low;
          }
        }
      }
    }
  });
  (entries, kept)
}

/// Whether a record names no engine for the build that goes on from it: no fact made stands after
/// its last checkpoint, or in it when it holds none.
fn unnamed(entries: &[Object]) -> bool {
  made(checkpoint::segment(entries)).is_none()
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

/// The message that the open of the record passed over a last line that a stop tore as it was
/// written.
fn torn(bytes: u64) -> String {
  format!(
    "The last line of the record of this life was torn as it was written, so the life opened \
     without it: it cut {bytes} bytes, a part of one fact that the record never held whole."
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

/// The ear that keeps the record keeps its store, which the image of a life holds nothing of, and
/// syncs it at the end of each run.
impl<S: Store + 'static> Kept for Held<S> {
  fn image(&self) -> Option<Value> {
    Some(Value::Null)
  }

  /// A fault of the store ends no run: the ear holds it, tells the operator and tries again.
  fn ran(&mut self) -> Result<(), Fault> {
    self.synced();
    Ok(())
  }

  /// Each entry that waits for the store, which no keep took yet.
  fn unkept(&self) -> usize {
    self.waiting.len()
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

  fn cut(&self) -> Option<u64> {
    (**self).cut()
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
    self.under(path, leased(path)?)
  }

  /// The store of the record at a path, under a lease of it that the caller holds.
  pub(crate) fn under(self, path: &Path, lease: Lease) -> Result<Box<dyn Store>, Fault> {
    Ok(match self {
      Self::Sqlite => Box::new(Sqlite::under(path, lease)?),
      Self::Lines => Box::new(Lines::under(path, lease)?),
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

/// The entry of one fact, from the JSON that the record holds of the fact, of its place and of
/// whether the life rested as it said the fact, or nothing when it is no entry: a fact is a list of
/// three words at least, whose first word is its kind, and its place, when the entry has one, is
/// the count of the facts that the life said before it. The end that a stop gave a run has no
/// place, which is null.
fn entry_of(held: &[&RawValue]) -> Option<Object> {
  let placed = match held {
    [_] => true,
    [_, place, rest] => {
      serde_json::from_str::<Option<u64>>(place.get()).is_ok()
        && serde_json::from_str::<bool>(rest.get()).is_ok()
    }
    _ => false,
  };
  if !placed {
    return None;
  }
  let mut graph = MontyGraph::new();
  let nodes =
    held.iter().map(|one| built(&mut graph, one, 1)).collect::<Result<Vec<_>, _>>().ok()?;
  let root = graph.push(MontyNode::List(nodes));
  let entry = whole(graph, root).ok()?;
  let fact = entry.as_ref().items().and_then(|held| held.first().and_then(ObjectRef::items));
  let holds = fact
    .is_some_and(|fact| fact.len() >= 3 && fact.first().is_some_and(|one| one.as_str().is_some()));
  holds.then_some(entry)
}

/// The time now, in milliseconds since the start of 1970.
fn now() -> u64 {
  let since = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
  u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
}

/// The lease of a record: the lock on the file beside it, which ends when the last clone of the
/// lease goes. A host takes it when a life of the record is born, before the life reads its record,
/// and the store of the life holds a clone of it, so no other process takes the record between the
/// two. A process that this one starts holds a copy of the file until its program runs, so the
/// lease ends the lock itself, and not with the last copy of the file.
#[derive(Clone)]
pub struct Lease {
  _locked: Arc<Locked>,
}

/// The lock of a lease, which ends when it goes.
struct Locked(File);

impl Lease {
  /// The lease of the record at a path, or the refusal when another process holds it.
  pub fn of(path: impl AsRef<Path>) -> Result<Self, Fault> {
    Ok(leased(path.as_ref())?)
  }
}

impl Drop for Locked {
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
    Ok(()) => Ok(Lease { _locked: Arc::new(Locked(file)) }),
    Err(TryLockError::WouldBlock) => Err(StoreError::Owned { path: path.to_path_buf() }),
    Err(TryLockError::Error(no)) => Err(StoreError::io(&lock)(no)),
  }
}
