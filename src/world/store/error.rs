//! Why a store could not open, read or keep a record.

use std::{
  fmt, io,
  path::{Path, PathBuf},
};

use crate::value::Fault;

/// Why a store could not open, read or keep the record at a path.
#[derive(Debug, thiserror::Error)]
pub(crate) enum StoreError {
  /// A file of the record or of its lease could not be opened, read or written.
  #[error("{source}: {}", .path.display())]
  Io { path: PathBuf, source: io::Error },
  /// The database of the record refused.
  #[error("{source}: {}", .path.display())]
  Sqlite { path: PathBuf, source: rusqlite::Error },
  /// An entry of the record is no fact.
  #[error("Invalid record entry at {at} in {}.", .path.display())]
  Invalid { path: PathBuf, at: At },
  /// Another process holds the lease of the record.
  #[error("Another process owns {}.", .path.display())]
  Owned { path: PathBuf },
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
