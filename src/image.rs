//! The image of a life beside its record, `<record>.image`: the state that the replay of the record
//! gives at its tip, kept so that a later life of the same build opens with no replay. It is a
//! cache: it is derived from the record and checked against it, and a reader drops an image that
//! does not agree, so the open then goes by the record. The record holds no fact of it.
//!
//! The file holds its key, one line of JSON, then its body. The key names the tip of
//! the record that the life stood at, the shape of the body, the dump version of monty, the digest
//! of the texts of the build, since a session runs the engine and the Kernel that it holds,
//! whether the scratch directory of the record holds a file, since a life that opens from its image
//! makes no file of it again, and the digest of the body as the file holds it, since monty checks
//! no byte of a dump. A body is
//! the session that monty dumps, then the half of the host, as JSON, then the part of the host of
//! the life: what that host keeps of the life beside its engine, as the log of its events, which
//! the crate carries and does not read. The key names where each part ends. The file lands whole,
//! as each write of the World, so no reader meets a part of an image.

use std::{
  fmt::Display,
  fs::{self, File},
  io::{self, BufRead, BufReader, Read, Seek, SeekFrom},
  ops::Range,
  path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
  Engine,
  engine::Half,
  value::Fault,
  world::{self, Format, Scratch, beside, disk},
};

/// The shape of the body, which each change of [`Half`], of what an ear gives or of what a host
/// keeps raises: a build reads no image of another shape.
pub(crate) const SHAPE: u32 = 1;

/// How many of the last bytes of a record its tip names by their digest.
const TAIL: u64 = 4096;

/// What an image names, by which a reader checks it before it trusts a byte of its body.
#[derive(Serialize, Deserialize, PartialEq, Eq)]
struct Key {
  shape: u32,
  dump: u16,
  texts: String,
  /// The tip of the record: its bytes, and the digest of the last of them.
  bytes: u64,
  tail: String,
  body: String,
  /// The bytes of the session in the body, then those of the half of the host after it. The part
  /// of the host of the life is the rest.
  session: usize,
  half: usize,
  /// Whether the scratch directory of the record holds a file. A life that opens from its image
  /// makes no file of it again, so an image agrees only while the directory holds its files.
  scratch: bool,
}

impl Key {
  /// The key that this build writes at the tip of a record now, for a body of this digest and of
  /// these parts.
  fn of(record: &Path, body: String, session: usize, half: usize) -> io::Result<Self> {
    let mut file = File::open(record)?;
    let bytes = file.metadata()?.len();
    let mut last = Vec::new();
    file.seek(SeekFrom::Start(bytes.saturating_sub(TAIL)))?;
    file.read_to_end(&mut last)?;
    Ok(Self {
      shape: SHAPE,
      dump: monty::DUMP_VERSION,
      texts: world::engine(),
      bytes,
      tail: format!("{:x}", Sha256::digest(&last)),
      body,
      session,
      half,
      scratch: Scratch::of(record).ok().flatten().is_some_and(|one| one.holds()),
    })
  }

  /// Whether this key is the one that this build writes at the tip of a record now, for the body
  /// that it names.
  fn stands(&self, record: &Path) -> bool {
    Self::of(record, self.body.clone(), self.session, self.half).is_ok_and(|now| now == *self)
  }
}

/// An image that agrees with its record: its half of the host, its session and the part of the
/// host of the life in the body, and the bytes of the record at the tip that it names.
pub(crate) struct Image {
  pub(crate) half: Half,
  pub(crate) bytes: u64,
  file: Vec<u8>,
  session: Range<usize>,
  host: usize,
}

impl Image {
  /// The session of the image, as monty loads it.
  pub(crate) fn session(&self) -> &[u8] {
    &self.file[self.session.clone()]
  }

  /// The part of the host of the life, as that host gave it to [`keep`].
  pub(crate) fn host(&self) -> &[u8] {
    &self.file[self.host..]
  }
}

/// Where the image of a record stands.
fn path(record: &Path) -> PathBuf {
  beside(record, ".image")
}

/// The image of a record, when one stands beside it that this build reads and that names the tip
/// of the record as it stands now. A record that is no file of lines has none. A life that keeps
/// the record drops an image that does not agree, and a life that only reads leaves it, since the
/// life that keeps the record may write the next one meanwhile.
pub(crate) fn read(record: &Path, keeps: bool) -> Option<Image> {
  if !matches!(Format::of(record), Format::Lines) {
    return None;
  }
  let held = fs::read(path(record)).ok()?;
  let image = agreed(record, held);
  if image.is_none() && keeps {
    drop_of(record);
  }
  image
}

/// The image of a record, dropped. One that cannot go is refused again at the next open, until a
/// later one takes its place.
pub(crate) fn drop_of(record: &Path) {
  fs::remove_file(path(record)).ok();
}

/// Whether the image of a record names the tip of the record as it stands now, in a shape that
/// this build reads, by its key alone: no byte of its body is read. A host asks it before an open,
/// to know whether the life reads its record. An image that agrees here and does not load, as one
/// whose body is torn, makes the life open by its record.
pub fn agrees(record: &Path) -> bool {
  let mut key = Vec::new();
  let read =
    File::open(path(record)).and_then(|file| BufReader::new(file).read_until(b'\n', &mut key));
  read.is_ok() && serde_json::from_slice::<Key>(&key).is_ok_and(|key| key.stands(record))
}

/// The image that the bytes of a file hold, when its key is the key of its body at the tip of the
/// record now.
fn agreed(record: &Path, file: Vec<u8>) -> Option<Image> {
  let end = file.iter().position(|byte| *byte == b'\n')?;
  let key: Key = serde_json::from_slice(&file[..end]).ok()?;
  let body = file.get(end + 1..)?;
  if key.body != format!("{:x}", Sha256::digest(body)) || !key.stands(record) {
    return None;
  }
  let session = end + 1..(end + 1).checked_add(key.session)?;
  let host = session.end.checked_add(key.half)?;
  let half = serde_json::from_slice(file.get(session.end..host)?).ok()?;
  Some(Image { half, bytes: key.bytes, file, session, host })
}

/// The image of the life of an engine, written beside its record, which stands at its tip: each
/// entry that the life said is in the record. `host` is what the host of the life keeps of it
/// beside its engine, which the open of a later life gives back to that host. It says whether it
/// wrote one: a life that cannot stand again from an image writes none, as [`Engine::image`] says,
/// and neither does a life whose record lacks an entry that it said, as on a full disk.
pub fn keep(engine: &mut Engine, record: &Path, host: &impl Serialize) -> Result<bool, Fault> {
  if engine.unkept() > 0 || !matches!(Format::of(record), Format::Lines) {
    return Ok(false);
  }
  let Some((half, mut file)) = engine.image()? else { return Ok(false) };
  let unkept = |no: &dyn Display| Fault::refused(format!("the life keeps no image: {no}"));
  // The session is the most of the body, so each other part goes in after it, and the key before
  // it, with no copy of it.
  let session = file.len();
  serde_json::to_writer(&mut file, &half).map_err(|no| unkept(&no))?;
  let half = file.len() - session;
  serde_json::to_writer(&mut file, host).map_err(|no| unkept(&no))?;
  let body = format!("{:x}", Sha256::digest(&file));
  let key = Key::of(record, body, session, half).map_err(|no| unkept(&no))?;
  let mut head = serde_json::to_vec(&key).map_err(|no| unkept(&no))?;
  head.push(b'\n');
  file.splice(0..0, head);
  disk::write(&path(record), &file, disk::Access::Owner).map_err(|no| unkept(&no))?;
  Ok(true)
}
