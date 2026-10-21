//! The snapshot of a life: a cache of its state beside its record, `<record>.snapshot`, which the life derives from
//! its record and checks against it, so a later life on the same record opens with no replay. The record stays the
//! one truth, and a chain stays a function of its record: a snapshot that does not agree is dropped, and the life
//! replays.
//!
//! A snapshot is one line of JSON, its head, then the session of the sandbox as monty dumps it. The head holds the
//! version of its form, the key of the build that wrote it, how many entries of the record it covers and a digest of
//! the bytes of the record, the length and a digest of the log of the life that it covers, and what the engine holds of the side of the
//! host. The key of the build is a digest of the engine, the sheet and the Kernel, of the version of the dumps of
//! monty and of the crate, and of the names of the ears in the order of boot, since a replay on another build or on
//! other ears could give another life.

use std::{
  fs,
  path::{Path, PathBuf},
  time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{ENGINE, KERNEL, SHEET, engine::Table, sand::Sand, value::Object, wire, world};

/// The version of the form of a snapshot.
const VERSION: u64 = 1;

/// How long a life waits after it wrote its snapshot before it writes it again, but when it ends.
const INTERVAL: Duration = Duration::from_secs(30);

/// The head of a snapshot, as a life reads it.
#[derive(Deserialize)]
struct Head {
  version: u64,
  build: String,
  entries: usize,
  record: String,
  log: u64,
  logs: String,
  table: Table,
}

/// The head of a snapshot, as a life writes it.
#[derive(Serialize)]
struct Written<'a> {
  version: u64,
  build: &'a str,
  entries: usize,
  record: String,
  log: u64,
  logs: String,
  table: &'a Table,
}

/// The snapshot of the record at a path.
pub(crate) fn path(record: &Path) -> PathBuf {
  PathBuf::from(format!("{}.snapshot", record.display()))
}

/// The key of this build, on the ears of a life in the order of boot.
pub(crate) fn build(names: &[String]) -> String {
  let mut digest = Sha256::new();
  for one in [ENGINE, SHEET, KERNEL, env!("CARGO_PKG_VERSION")] {
    digest.update(one.as_bytes());
    digest.update([0]);
  }
  digest.update(monty::DUMP_VERSION.to_le_bytes());
  for one in names {
    digest.update([0]);
    digest.update(one.as_bytes());
  }
  hex(&digest.finalize())
}

/// Bytes as hexadecimal text.
fn hex(bytes: &[u8]) -> String {
  bytes.iter().map(|one| format!("{one:02x}")).collect()
}

/// The record at a path, read whole as the store wrote it: how many entries it holds, and a digest of them. A file of
/// lines holds an entry on each line that holds something, and the digest is of its bytes; a database is read for its
/// entries, and the digest is of each as the record keeps it.
fn record(path: &Path) -> Option<(usize, String)> {
  if world::database(path) {
    let entries = world::kept(path).ok()?;
    let mut digest = Sha256::new();
    for one in &entries {
      digest.update(wire::record(one.as_ref()).to_string().as_bytes());
      digest.update(b"\n");
    }
    return Some((entries.len(), hex(&digest.finalize())));
  }
  let bytes = fs::read(path).ok()?;
  let entries =
    bytes.split(|one| *one == b'\n').filter(|line| !line.trim_ascii().is_empty()).count();
  Some((entries, hex(&Sha256::digest(&bytes))))
}

/// The log of the record at a path, read whole: its length and the digest of its bytes.
fn log(record: &Path) -> Option<(u64, String)> {
  let bytes = fs::read(format!("{}.log", record.display())).ok()?;
  Some((u64::try_from(bytes.len()).ok()?, hex(&Sha256::digest(&bytes))))
}

/// Where a life keeps its snapshot, and when it wrote it last.
pub(crate) struct Keeper {
  record: PathBuf,
  build: String,
  last: Option<Instant>,
  /// The length of the log that the last snapshot covers, so a life that said nothing since writes none.
  covered: Option<u64>,
}

impl Keeper {
  /// The keeper of the snapshot of the record at a path, for a life of a build.
  pub(crate) fn new(record: PathBuf, build: String) -> Keeper {
    Keeper { record, build, last: None, covered: None }
  }

  /// Whether a snapshot is due: now, or when the interval passed since the last one, which starts again.
  pub(crate) fn due(&mut self, now: bool) -> bool {
    let due = now || self.last.is_none_or(|last| last.elapsed() >= INTERVAL);
    if due {
      self.last = Some(Instant::now());
    }
    due
  }

  /// The snapshot written, on the record and the log as they stand now. A snapshot is written whole under another name, which then takes its name, so a crash leaves the old snapshot or
  /// the new one. A failure writes nothing, since a snapshot is a cache.
  pub(crate) fn write(&mut self, table: &Table, dump: &[u8]) {
    let _ = self.written(table, dump);
  }

  fn written(&mut self, table: &Table, dump: &[u8]) -> Option<()> {
    let (log, logs) = log(&self.record)?;
    if self.covered == Some(log) {
      return Some(());
    }
    let (entries, record) = record(&self.record)?;
    let head = Written { version: VERSION, build: &self.build, entries, record, log, logs, table };
    let mut bytes = serde_json::to_vec(&head).ok()?;
    bytes.push(b'\n');
    bytes.extend_from_slice(dump);
    let (at, aside) =
      (path(&self.record), PathBuf::from(format!("{}.tmp", path(&self.record).display())));
    fs::write(&aside, &bytes).ok()?;
    fs::rename(&aside, &at).ok()?;
    self.covered = Some(log);
    Some(())
  }
}

/// What a life opens from a snapshot on: the side of the host, the session of the sandbox, and the past of the life,
/// which is the facts of its log, and how many entries of the record the snapshot covers.
pub(crate) struct Restore {
  pub(crate) table: Table,
  pub(crate) sand: Sand,
  pub(crate) past: Vec<Object>,
  pub(crate) entries: usize,
}

/// The snapshot of the record at a path, when it agrees with the record, its log and this build: it is of this
/// version and of this key, the record holds exactly the entries that it covers, and the log is exactly the one that
/// it covers. Nothing otherwise, and a life on it replays.
pub(crate) fn restore(record: &Path, build: &str) -> Option<Restore> {
  let bytes = fs::read(path(record)).ok()?;
  let at = memchr::memchr(b'\n', &bytes)?;
  let head: Head = serde_json::from_slice(&bytes[..at]).ok()?;
  if head.version != VERSION || head.build != build {
    return None;
  }
  if self::record(record)? != (head.entries, head.record) || log(record)? != (head.log, head.logs) {
    return None;
  }
  let past = world::logged(record).ok()??;
  let sand = Sand::loaded(&bytes[at + 1..]).ok()?;
  Some(Restore { table: head.table, sand, past, entries: head.entries })
}

/// The snapshot of the record at a path dropped, so the next life on the record replays.
pub(crate) fn dropped(record: &Path) {
  let _ = fs::remove_file(path(record));
}
