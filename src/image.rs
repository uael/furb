//! The image of a life beside its record, `<record>.image`: the state that the replay of the record
//! gives at its tip, kept so that a later life of the same build opens with no replay. It is a
//! cache: it is derived from the record and checked against it, and a reader drops an image that
//! does not agree, so the open then goes by the record. The record holds no fact of it.
//!
//! The file holds its key, one line of JSON, then its body. The key names the tip of
//! the record that the life stood at, the shape of the body, the dump version of monty, the digest
//! of the texts of the build, since a session runs the engine and the Kernel that it holds, and
//! the digest of the body as the file holds it, since monty checks no byte of a dump. A body is
//! the half of the host, as JSON behind its length, then the session that monty dumps. The file
//! lands whole, as each write of the World, so no reader meets a part of an image.

use std::{
  fmt::Display,
  fs::{self, File},
  io::{self, Read, Seek, SeekFrom},
  ops::Range,
  path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
  Engine,
  engine::Half,
  value::Fault,
  world::{self, Format, beside, disk},
};

/// The shape of the body, which each change of [`Half`] or of what an ear gives raises: a build
/// reads no image of another shape.
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
}

impl Key {
  /// The key of a body of this digest that this build writes at the tip of a record now.
  fn of(record: &Path, body: Sha256) -> io::Result<Self> {
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
      body: format!("{:x}", body.finalize()),
    })
  }
}

/// An image that agrees with its record: its half of the host, its session in the body, and the
/// bytes of the record at the tip that it names.
pub(crate) struct Image {
  pub(crate) half: Half,
  pub(crate) bytes: u64,
  file: Vec<u8>,
  session: Range<usize>,
}

impl Image {
  /// The session of the image, as monty loads it.
  pub(crate) fn session(&self) -> &[u8] {
    &self.file[self.session.clone()]
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

/// The image that the bytes of a file hold, when its key is the key of its body at the tip of the
/// record now.
fn agreed(record: &Path, file: Vec<u8>) -> Option<Image> {
  let end = file.iter().position(|byte| *byte == b'\n')?;
  let (key, body) = (&file[..end], &file[end + 1..]);
  let key: Key = serde_json::from_slice(key).ok()?;
  if key != Key::of(record, Sha256::new_with_prefix(body)).ok()? {
    return None;
  }
  let length = usize::try_from(u64::from_le_bytes(body.get(..8)?.try_into().ok()?)).ok()?;
  let half = serde_json::from_slice(body.get(8..length.checked_add(8)?)?).ok()?;
  let session = end + 9 + length..file.len();
  Some(Image { half, bytes: key.bytes, file, session })
}

/// The image of the life of an engine, written beside its record, which stands at its tip: each
/// entry that the life said is in the record. It says whether it wrote one: a life that cannot
/// stand again from an image writes none, as [`Engine::image`] says, and neither does a life whose
/// record lacks an entry that it said, as on a full disk.
pub fn keep(engine: &mut Engine, record: &Path) -> Result<bool, Fault> {
  if engine.unkept() > 0 || !matches!(Format::of(record), Format::Lines) {
    return Ok(false);
  }
  let Some((half, mut file)) = engine.image()? else { return Ok(false) };
  let unkept = |no: &dyn Display| Fault::refused(format!("the life keeps no image: {no}"));
  let half = serde_json::to_vec(&half).map_err(|no| unkept(&no))?;
  let length = (half.len() as u64).to_le_bytes();
  let body = Sha256::new().chain_update(length).chain_update(&half).chain_update(&file);
  let key = Key::of(record, body).map_err(|no| unkept(&no))?;
  let mut head = serde_json::to_vec(&key).map_err(|no| unkept(&no))?;
  head.push(b'\n');
  head.extend(length);
  head.extend(half);
  // The session is the most of the file, so the head goes in before it, and no copy is made.
  file.splice(0..0, head);
  disk::write(&path(record), &file, disk::Access::Owner).map_err(|no| unkept(&no))?;
  Ok(true)
}
