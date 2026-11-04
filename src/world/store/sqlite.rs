//! The store of a record as a database of SQLite: one row for each entry, in the table `entries`.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};
use serde_json::value::RawValue;

use crate::{
  value::{Fault, Object, ObjectRef},
  wire::record,
};

use super::{
  Lease, Store, entry_of,
  error::{At, StoreError},
  leased, now,
};

/// The table of the entries: the number of each row, in the order that the store kept them, the entry as JSON, and
/// the time at which the store kept it, in milliseconds since the start of 1970, which the life never reads.
const TABLE: &str = "CREATE TABLE IF NOT EXISTS entries \
  (n INTEGER PRIMARY KEY AUTOINCREMENT, entry TEXT NOT NULL, at INTEGER NOT NULL)";

/// The record at a path as a database of SQLite, under its lease, as for a file of lines.
///
/// The database writes ahead in a log and syncs each entry in full, so an entry that the store kept outlives a
/// crash, and a crash while it keeps one leaves the record as it stood before. So a sync has nothing more to do.
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
    let db = Connection::open(&path).map_err(StoreError::sqlite(&path))?;
    db.execute_batch(&format!("PRAGMA journal_mode = WAL; PRAGMA synchronous = FULL; {TABLE};"))
      .map_err(StoreError::sqlite(&path))?;
    Ok(Self { path, db, _lease: lease })
  }
}

impl Store for Sqlite {
  fn entries(&mut self) -> Result<Vec<Object>, Fault> {
    let mut entries = Vec::new();
    rows(&self.db, &self.path, &mut |one| entries.push(one))?;
    Ok(entries)
  }

  fn keep(&mut self, entry: ObjectRef<'_>) -> Result<(), Fault> {
    let at = i64::try_from(now()).unwrap_or(i64::MAX);
    self
      .db
      .execute("INSERT INTO entries (entry, at) VALUES (?1, ?2)", (record(entry).to_string(), at))
      .map(drop)
      .map_err(StoreError::sqlite(&self.path))?;
    Ok(())
  }

  fn sync(&mut self) -> Result<(), Fault> {
    Ok(())
  }

  fn room(&self) -> Option<(&Path, u64)> {
    super::room(&self.path)
  }
}

/// Each entry of the database at a path, in the order of its rows, read with no lease and changed in nothing, one row
/// at a time: none when no database or no table of entries stands there.
pub(super) fn each(path: &Path, take: &mut dyn FnMut(Object)) -> Result<(), StoreError> {
  if !path.exists() {
    return Ok(());
  }
  let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
    .map_err(StoreError::sqlite(path))?;
  let tables = db
    .query_row(
      "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'entries'",
      [],
      |row| row.get::<_, i64>(0),
    )
    .map_err(StoreError::sqlite(path))?;
  match tables {
    0 => Ok(()),
    _ => rows(&db, path, take),
  }
}

/// Each entry of the database, in the order of its rows, each the fact it kept in a list of one, as boot takes them.
fn rows(db: &Connection, path: &Path, take: &mut dyn FnMut(Object)) -> Result<(), StoreError> {
  let mut asked =
    db.prepare("SELECT n, entry FROM entries ORDER BY n").map_err(StoreError::sqlite(path))?;
  let mut rows = asked.query([]).map_err(StoreError::sqlite(path))?;
  while let Some(row) = rows.next().map_err(StoreError::sqlite(path))? {
    let n = row.get::<_, i64>(0).map_err(StoreError::sqlite(path))?;
    let invalid = || StoreError::Invalid { path: path.to_path_buf(), at: At::Row(n) };
    let Ok(text) = row.get::<_, String>(1) else { return Err(invalid()) };
    let Ok(held) = serde_json::from_str::<Vec<&RawValue>>(&text) else { return Err(invalid()) };
    let [fact] = held.as_slice() else { return Err(invalid()) };
    take(entry_of(fact).ok_or_else(invalid)?);
  }
  Ok(())
}
