//! The schemas of what a life keeps, each with a version of its own, apart from the engine that the
//! fact made names, on whose texts a segment replays.
//!
//! - [`RECORD_SCHEMA`] is the schema of the record: the line of an entry, the marks by which the
//!   record holds a value that JSON holds only in part, as a tuple or a whole float, and the layout
//!   of a fact, `(kind, name, by, on, *words)`. The fact made names it.
//! - [`CHECKPOINT_SCHEMA`] is the schema of the data of a checkpoint, which its field `schema`
//!   names.
//!
//! A schema is `major.minor`. Within a major a schema only adds: a minor adds a kind of fact or a
//! field. Each reader skips each kind of fact and each field that it does not know, so it opens a
//! schema of each minor of its major, a later one too, and a build opens a record that a later
//! build of the major made. A reader refuses a schema of a later major, and the refusal names
//! both. A life refuses a record from before the stable cut, an earlier major than the one of
//! [`RECORD_SCHEMA`], and a reader of a record reads it.

use std::fmt;

use crate::value::{Fault, Object, ObjectRef};

/// A version of a schema: its major, which a change that a reader cannot skip moves, and its minor,
/// which a change that only adds moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Schema {
  /// The major, which only a change that an earlier reader cannot skip moves.
  pub major: i64,
  /// The minor, which a change that only adds moves.
  pub minor: i64,
}

/// The schema of the record that this build writes and reads. Its major 3 is the stable cut, where
/// the replay stands in the engine and a record keeps no checkpoint: from it each record replays
/// with no drift on each later build of the major. A record of an earlier major, or of a build
/// before the schemas, comes from a build before the cut, which the promise does not hold: no life
/// opens it, as [`Schema::opens`] says, and a reader of a record reads it.
pub const RECORD_SCHEMA: Schema = Schema { major: 3, minor: 0 };

/// The schema of the data of the checkpoint that this build writes and reads. The minor 1 holds
/// `systems`, the system prompt of each chain that reads an engine that the next life does not run;
/// a reader of the minor 0 skips it, and gives each chain the engine that the life runs.
pub const CHECKPOINT_SCHEMA: Schema = Schema { major: 2, minor: 1 };

/// Why a build does not read what a schema names.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum SchemaError {
  /// The schema is of another major than the one that this build reads.
  #[error(
    "the {of} holds the schema {theirs}, of another major than the schema {ours} that this build \
     writes: open it with a build that writes the major {major}, or start a new life",
    major = theirs.major
  )]
  Major {
    /// What holds the schema: the record or the checkpoint.
    of: &'static str,
    /// The schema that it holds.
    theirs: Schema,
    /// The schema that this build writes.
    ours: Schema,
  },
  /// What holds the schema comes from a build before the stable cut, an earlier major than the
  /// one that this build writes, which the promise does not hold.
  #[error(
    "the {of} comes from a build before the stable cut of the schema of the record, which the \
     promise does not hold: no life opens it, and a window reads its facts"
  )]
  BeforeCut {
    /// What holds the schema: the record or the checkpoint.
    of: &'static str,
  },
  /// The schema names no version of the form `major.minor`.
  #[error("the {of} names its schema as {held}, which is no version of the form major.minor")]
  Unread {
    /// What holds the schema: the record or the checkpoint.
    of: &'static str,
    /// The value that it names, as python shows it.
    held: String,
  },
}

impl From<SchemaError> for Fault {
  fn from(no: SchemaError) -> Self {
    Self::refused(no.to_string())
  }
}

impl fmt::Display for Schema {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}.{}", self.major, self.minor)
  }
}

impl Schema {
  /// The schema as a fact or a checkpoint names it: `major.minor`.
  #[must_use]
  pub fn object(self) -> Object {
    Object::string(self.to_string())
  }

  /// The schema that a value names: `major.minor`.
  #[must_use]
  pub fn read(held: ObjectRef<'_>) -> Option<Self> {
    let (major, minor) = held.as_str()?.split_once('.')?;
    Some(Self { major: major.parse().ok()?, minor: minor.parse().ok()? })
  }

  /// Whether a life of this build, which writes this schema, opens what holds the schema that a
  /// value names: a schema of each minor of the same major. A value of an earlier major, or one
  /// that names no schema, as a build before the schemas wrote, comes from before the stable cut.
  pub fn opens(self, of: &'static str, held: Option<ObjectRef<'_>>) -> Result<(), SchemaError> {
    let held =
      held.filter(|one| one.type_name() != "NoneType").ok_or(SchemaError::BeforeCut { of })?;
    let theirs =
      Self::read(held).ok_or_else(|| SchemaError::Unread { of, held: held.py_repr() })?;
    if theirs.major < self.major {
      return Err(SchemaError::BeforeCut { of });
    }
    if theirs.major != self.major {
      return Err(SchemaError::Major { of, theirs, ours: self });
    }
    Ok(())
  }
}

#[cfg(test)]
#[path = "schema.test.rs"]
mod test;
