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
//! build of the major made. A reader refuses only a schema of another major, and the refusal names
//! both. A record or a checkpoint of a build before the schemas names none, and reads as its major
//! 1.

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

/// The schema of the record that this build writes and reads.
pub const RECORD_SCHEMA: Schema = Schema { major: 1, minor: 0 };

/// The schema of the data of the checkpoint that this build writes and reads. The data of the major
/// 1 also holds `again`, each word that ran on a chain, which `seed` names as cut, so a build reads
/// it too, as [`Schema::opens_checkpoint`] says.
pub const CHECKPOINT_SCHEMA: Schema = Schema { major: 2, minor: 0 };

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

  /// The schema that a value names: `major.minor`, or an int, the major alone, as the checkpoint of
  /// a build before the minors names it.
  #[must_use]
  pub fn read(held: ObjectRef<'_>) -> Option<Self> {
    if let Some(major) = held.as_int() {
      return Some(Self { major, minor: 0 });
    }
    let (major, minor) = held.as_str()?.split_once('.')?;
    Some(Self { major: major.parse().ok()?, minor: minor.parse().ok()? })
  }

  /// Whether this build opens the data of a checkpoint that names a schema: one of each minor of
  /// the major of [`CHECKPOINT_SCHEMA`], or of the major 1, whose `again` `seed` names as cut.
  pub fn opens_checkpoint(held: Option<ObjectRef<'_>>) -> Result<(), SchemaError> {
    let older = held.and_then(Self::read).is_some_and(|one| one.major == 1);
    if older { Ok(()) } else { CHECKPOINT_SCHEMA.opens("checkpoint", held) }
  }

  /// Whether this build, which writes this schema, opens what holds the schema that a value names:
  /// a schema of each minor of the same major. A value that names no schema, as one of a build
  /// before the schemas, opens too.
  pub fn opens(self, of: &'static str, held: Option<ObjectRef<'_>>) -> Result<(), SchemaError> {
    let Some(held) = held.filter(|one| one.type_name() != "NoneType") else { return Ok(()) };
    let theirs =
      Self::read(held).ok_or_else(|| SchemaError::Unread { of, held: held.py_repr() })?;
    if theirs.major != self.major {
      return Err(SchemaError::Major { of, theirs, ours: self });
    }
    Ok(())
  }
}

#[cfg(test)]
#[path = "schema.test.rs"]
mod test;
