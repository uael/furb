//! The store of a record as a database of SQLite: one row for each entry, in the table `entries`.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};
use serde_json::value::RawValue;

use crate::{
  value::{Fault, Object, ObjectRef},
  wire::record,
};

use super::{Lease, Store, entry_of, failed, leased, now};

/// The table of the entries: the number of each row, in the order that the store kept them, the entry as JSON, and
/// the time at which the store kept it, in milliseconds since the start of 1970, which the life never reads.
const TABLE: &str = "CREATE TABLE IF NOT EXISTS entries \
  (n INTEGER PRIMARY KEY AUTOINCREMENT, entry TEXT NOT NULL, at INTEGER NOT NULL)";

/// The record at a path as a database of SQLite, under its lease, as for a file of lines.
///
/// The database writes ahead in a log and syncs each entry in full, so an entry that the store kept outlives a
/// crash, and a crash while it keeps one leaves the record as it stood before.
pub struct Sqlite {
  path: PathBuf,
  db: Connection,
  _lease: Lease,
}

impl Sqlite {
  /// The database of SQLite at a path, under its lease, or the refusal when another process holds it.
  pub fn open(path: impl AsRef<Path>) -> Result<Self, Fault> {
    let path = path.as_ref().to_path_buf();
    let lease = leased(&path)?;
    let db = Connection::open(&path).map_err(|no| failed(&path, no))?;
    db.execute_batch(&format!("PRAGMA journal_mode = WAL; PRAGMA synchronous = FULL; {TABLE};"))
      .map_err(|no| failed(&path, no))?;
    Ok(Self { path, db, _lease: lease })
  }
}

impl Store for Sqlite {
  fn entries(&mut self) -> Result<Vec<Object>, Fault> {
    rows(&self.db, &self.path)
  }

  fn keep(&mut self, entry: ObjectRef<'_>) -> Result<(), Fault> {
    let at = i64::try_from(now()).unwrap_or(i64::MAX);
    self
      .db
      .execute("INSERT INTO entries (entry, at) VALUES (?1, ?2)", (record(entry).to_string(), at))
      .map(drop)
      .map_err(|no| failed(&self.path, no))
  }
}

/// What the database at a path holds, read with no lease and changed in nothing: nothing when no database or no
/// table of entries stands there.
pub(super) fn kept(path: &Path) -> Result<Vec<Object>, Fault> {
  if !path.exists() {
    return Ok(Vec::new());
  }
  let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
    .map_err(|no| failed(path, no))?;
  let tables = db
    .query_row(
      "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'entries'",
      [],
      |row| row.get::<_, i64>(0),
    )
    .map_err(|no| failed(path, no))?;
  match tables {
    0 => Ok(Vec::new()),
    _ => rows(&db, path),
  }
}

/// The entries of the database, in the order of their rows, each the fact it kept in a list of one, as boot takes
/// them.
fn rows(db: &Connection, path: &Path) -> Result<Vec<Object>, Fault> {
  let mut asked =
    db.prepare("SELECT n, entry FROM entries ORDER BY n").map_err(|no| failed(path, no))?;
  let mut rows = asked.query([]).map_err(|no| failed(path, no))?;
  let mut entries = Vec::new();
  while let Some(row) = rows.next().map_err(|no| failed(path, no))? {
    let n = row.get::<_, i64>(0).map_err(|no| failed(path, no))?;
    let invalid =
      || Fault::refused(format!("Invalid record entry at row {n} in {}.", path.display()));
    let text = row.get::<_, String>(1).map_err(|_| invalid())?;
    let held = serde_json::from_str::<Vec<&RawValue>>(&text).map_err(|_| invalid())?;
    let [fact] = held.as_slice() else { return Err(invalid()) };
    entries.push(entry_of(fact).ok_or_else(invalid)?);
  }
  Ok(entries)
}
