//! Why a store could not open, read or keep a record.

use std::{
  fmt, io,
  path::{Path, PathBuf},
};

use crate::value::Fault;

/// Why a store could not open, read or keep the record at a path.
#[derive(Debug, thiserror::Error)]
pub(crate) enum StoreError {
  /// A file of the record or of its lease could not be opened, read or written. The text of the
  /// system comes last, since it differs on each system.
  #[error("Could not use {}: {source}", .path.display())]
  Io { path: PathBuf, source: io::Error },
  /// The database of the record refused.
  #[error("Could not use {}: {source}", .path.display())]
  Sqlite { path: PathBuf, source: rusqlite::Error },
  /// A line or a row of the record is no entry.
  #[error("Invalid record entry at {at} in {}: {why}.", .path.display())]
  Invalid { path: PathBuf, at: At, why: Unread },
  /// Another process holds the lease of the record.
  #[error("Another process owns {}.", .path.display())]
  Owned { path: PathBuf },
}

/// Why a line or a row of a record is no entry: an entry that a read skipped would make its replay
/// drift, so a line that is no entry is an error, which names why.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum Unread {
  /// The line or the row is no JSON list.
  #[error("it is no list")]
  List,
  /// The entry holds no word, or a fact and one word more, or a line three words, which no schema
  /// gives: an entry holds its fact alone, or its fact, its place, whether the life rested and each
  /// word that a minor adds after them, and a line its time after them.
  #[error("it holds {0} words, which no entry holds")]
  Words(usize),
  /// A word after the fact is not of its kind: the place is a count or null, whether the life
  /// rested is true or false, and the time is a count.
  #[error("a word after its fact is not of its kind")]
  Word,
  /// The fact is no list of three words or more whose first is its kind.
  #[error("its fact is no list of three words whose first is a kind")]
  Fact,
}

/// Where an entry stands in its record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum At {
  /// The byte of a file of lines where the line starts.
  Byte(usize),
  /// The number of a row of a database.
  Row(i64),
}

impl fmt::Display for At {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Byte(at) => write!(f, "byte {at}"),
      Self::Row(at) => write!(f, "row {at}"),
    }
  }
}

impl StoreError {
  /// The fault of a file at a path, as `map_err` takes it.
  pub(super) fn io(path: &Path) -> impl FnOnce(io::Error) -> Self {
    move |source| Self::Io { path: path.to_path_buf(), source }
  }

  /// The fault of the database at a path, as `map_err` takes it.
  pub(super) fn sqlite(path: &Path) -> impl FnOnce(rusqlite::Error) -> Self {
    move |source| Self::Sqlite { path: path.to_path_buf(), source }
  }
}

impl From<StoreError> for Fault {
  fn from(no: StoreError) -> Self {
    Self::refused(no.to_string())
  }
}
