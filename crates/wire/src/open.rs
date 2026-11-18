//! How far the open of a life stands, which a client shows as a bar of progress.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A stage of the open of a life, in the order that an open passes them. An open skips a stage that
/// it has no work for, and a stage never comes again in one open. Each stage counts its work in a
/// unit of its own, and a stage that counts nothing tells 0 of 0.
#[derive(
  Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
  /// A host tells its readers what the record told before its last crossing: the thousandths of
  /// the read done, of 1000. The two passes of the reader over the record make the first two
  /// thirds, and the fold of the facts that it tells, as far as that crossing, the last third.
  /// The crate furb never tells this stage.
  Recall,
  /// The record is read whole, for the build of its last life: the bytes passed, of the bytes of
  /// the record. It is the first
  /// stage that the crate furb tells, at once, and the life then reads its settings and the catalog
  /// of its models in it. A life that opens from its image reads no entry, and tells each byte
  /// passed once the image agrees with the record.
  Find,
  /// A life that ran another engine of the major is carried across to this build: the open
  /// replays the record in lives that inspect it, and writes the fact made of this build at its
  /// tip. It counts nothing.
  Cross,
  /// The record is read again under its lease, when it changed after the stage find: the bytes
  /// passed, of the bytes of the record.
  Read,
  /// The sandbox loads the engine, and the checkpointer makes the entries of the carry across the
  /// last crossing of the record, or the sandbox loads the session of the image of the life, which
  /// then boots on no entry. An open whose image does not load reads its record in this stage,
  /// then seeds it. It counts nothing.
  Seed,
  /// The life boots: the entries that its journal passed, of the entries that it boots on. It is
  /// the last stage that the crate furb tells, and a boot that succeeds ends with each entry
  /// counted.
  Boot,
  /// A host puts what the open told in the log that its readers read: the events put, of those
  /// events. The crate furb never tells this stage.
  Log,
}
