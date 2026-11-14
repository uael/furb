//! The image of a life beside its record, `<record>.image`: the state that the replay of the record
//! gives at its tip, kept so that a later life of the same build opens with no replay. It is a
//! cache: it is derived from the record and checked against it, and a reader takes no image that
//! does not agree, so the open then goes by the record, and a life that keeps the record drops the
//! image once it stands so. The record holds no fact of it.
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
//!
//! A record only grows, so an image agrees with a record that holds entries after the tip that it
//! names: a life that went on from the image and wrote no later one, as a life that crashed, kept
//! them. The life that opens from the image says them again, as a boot says a record again, so a
//! crash costs its tail alone. A checkpoint or a fact that names an engine after the image is a
//! birth of its own, which no life of the image went on to: the image agrees no more.

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
  /// The tip of the record that the life stood at: its bytes, and the digest of the last of them.
  bytes: u64,
  tail: String,
  /// Whether the scratch directory of the record holds a file. A life that opens from its image
  /// makes no file of it again, so an image agrees only while the directory holds its files.
  scratch: bool,
  #[serde(flatten)]
  parts: Parts,
}

/// What a key names of its body: its digest, the bytes of the session in it, then those of the
/// half of the host after it. The part of the host of the life is the rest.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
struct Parts {
  body: String,
  session: usize,
  half: usize,
}

impl Key {
  /// The key that this build writes for a body of these parts, at the first `bytes` bytes of a
  /// record, or at its tip now.
  fn of(record: &Path, bytes: Option<u64>, parts: Parts) -> io::Result<Self> {
    let mut file = File::open(record)?;
    let held = file.metadata()?.len();
    let bytes = bytes.unwrap_or(held).min(held);
    let mut last = vec![0; usize::try_from(bytes.min(TAIL)).unwrap_or_default()];
    file.seek(SeekFrom::Start(bytes - last.len() as u64))?;
    file.read_exact(&mut last)?;
    Ok(Self {
      shape: SHAPE,
      dump: monty::DUMP_VERSION,
      texts: world::engine(),
      bytes,
      tail: format!("{:x}", Sha256::digest(&last)),
      scratch: Scratch::of(record).ok().flatten().is_some_and(|one| one.holds()),
      parts,
    })
  }

  /// Whether this key is the one that this build writes for the body that it names, at the bytes
  /// of the record that it names: the record holds them as the life of the image left them. A
  /// record only grows, so the bytes after them are entries that a later life kept, and a life of
  /// the image goes on from it only while none of them is a birth of its own.
  fn stands(&self, record: &Path) -> bool {
    Self::of(record, Some(self.bytes), self.parts.clone()).is_ok_and(|now| now == *self)
      && world::goes_after(record, self.bytes)
  }
}

/// An image that agrees with its record: its half of the host, its session and the part of the
/// host of the life in the body, and the bytes of the record at the tip that it names, which each
/// entry that a later life kept stands after.
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

/// The image of a record, when one stands beside it that this build reads and that names the
/// record as it stands now, as far as the tip of the life that wrote it. A record that is no file
/// of lines has none. The read changes no file: an image that does not agree stays until a life
/// that keeps the record stands on it by the record, as [`drop_of`] says.
pub(crate) fn read(record: &Path) -> Option<Image> {
  if !matches!(Format::of(record), Format::Lines) {
    return None;
  }
  agreed(record, fs::read(path(record)).ok()?)
}

/// The image of a record, dropped, once a life that keeps the record stands on it by the record:
/// that life goes on from no image, so none is the image of what it keeps. A life that only reads
/// the record, and one whose open fails, as on a record that drifts, leaves each file beside the
/// record as it was. One that cannot go is refused again at the next open, until a later one takes
/// its place.
pub(crate) fn drop_of(record: &Path) {
  fs::remove_file(path(record)).ok();
}

/// Whether the image of a record names the record as it stands now, as far as the tip of the life
/// that wrote it, in a shape that this build reads, by its key alone: no byte of its body is read.
/// A host asks it before an open, to know whether the life reads its record. An image that agrees
/// here and does not load, as one whose body is torn, or one that a checkpoint of the record
/// passed, makes the life open by its record.
pub fn agrees(record: &Path) -> bool {
  let mut key = Vec::new();
  let read =
    File::open(path(record)).and_then(|file| BufReader::new(file).read_until(b'\n', &mut key));
  read.is_ok() && serde_json::from_slice::<Key>(&key).is_ok_and(|key| key.stands(record))
}

/// The image that the bytes of a file hold, when its key is the key of its body at the bytes of
/// the record that it names.
fn agreed(record: &Path, file: Vec<u8>) -> Option<Image> {
  let end = file.iter().position(|byte| *byte == b'\n')?;
  let key: Key = serde_json::from_slice(&file[..end]).ok()?;
  let body = file.get(end + 1..)?;
  if key.parts.body != format!("{:x}", Sha256::digest(body)) || !key.stands(record) {
    return None;
  }
  let session = end + 1..(end + 1).checked_add(key.parts.session)?;
  let host = session.end.checked_add(key.parts.half)?;
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
  let key = Key::of(record, None, Parts { body, session, half }).map_err(|no| unkept(&no))?;
  let mut head = serde_json::to_vec(&key).map_err(|no| unkept(&no))?;
  head.push(b'\n');
  file.splice(0..0, head);
  disk::write(&path(record), &file, disk::Access::Owner).map_err(|no| unkept(&no))?;
  Ok(true)
}
