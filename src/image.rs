//! The image of a life beside its record, `<record>.image`: the state that the replay of the record
//! gives at its tip, kept so that a later life opens with no replay. It is a cache: it is derived
//! from the record and checked against it, and a reader takes no image that does not agree, so the
//! open then goes by the record, and a life that keeps the record drops the image once it stands
//! so. The record holds no fact of it.
//!
//! A session holds the engine, the Kernel and the checkpointer that its life runs, so a life that
//! opens from its image goes on with them, whatever texts the build holds: its models read the
//! engine of the life, which the half of the host holds, and a change of the engine reaches a life
//! only when the life is handed off to a new one, by a checkpoint. So an image loads on each build
//! whose sandbox runs it, and its key names what an image depends on in a build beside those
//! texts, by one digest that the build computes, [`BUILD`]: the python that the build puts in a
//! session beside them, the Rust that makes, holds or reads the half of the host and that the
//! session calls, the view, whose acts a host keeps in its part, and the typed surface of the
//! contract, by which a host calls the engine of the life and reads what an ear answers.
//! `build.rs` names each file of it. A host names the shape of its own part in that part.
//!
//! The file holds its key, one line of JSON that spaces may end, then its body. The key names the
//! tip of the record that the life stood at, that digest, the dump version of monty, which stands
//! while monty reads it, whether the scratch directory of the record holds a file, since a life
//! that opens from its image makes no file of it again, and the digest of the body as the file
//! holds it, since monty checks no byte of a dump. A body is the session that monty dumps, then
//! the half of the host, as JSON, then the part of the host of the life: what that host keeps of
//! the life beside its engine, as the log of its events, which the crate carries and does not
//! read. The key names where each part ends. The file lands whole, as each write of the World, so
//! no reader meets a part of an image.
//!
//! A record only grows, so an image agrees with a record that holds entries after the tip that it
//! names: a life that went on from the image and wrote no later one, as a life that crashed, kept
//! them. The life that opens from the image says them again, as a boot says a record again, so a
//! crash costs its tail alone. A checkpoint or a fact that names an engine after the image is a
//! birth of its own, which no life of the image went on to: the image agrees no more.

use std::{
  fmt::Display,
  fs::{self, File},
  io::{self, BufRead, BufReader, BufWriter, IntoInnerError, Read, Seek, SeekFrom, Write},
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

/// What an image depends on in this build beside the texts of its life, as the build digests it:
/// a build reads no image of another digest.
const BUILD: &str = env!("FURB_IMAGED");

/// How many of the last bytes of a record its tip names by their digest.
const TAIL: u64 = 4096;

/// What an image names, by which a reader checks it before it trusts a byte of its body.
#[derive(Serialize, Deserialize, PartialEq, Eq)]
struct Key {
  build: String,
  dump: u16,
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
      build: BUILD.to_owned(),
      dump: monty::DUMP_VERSION,
      bytes,
      tail: format!("{:x}", Sha256::digest(&last)),
      scratch: Scratch::of(record).ok().flatten().is_some_and(|one| one.holds()),
      parts,
    })
  }

  /// Whether this key is the one that this build writes for the body that it names, at the bytes
  /// of the record that it names, but for a dump of an earlier version that monty still reads: the
  /// record holds those bytes as the life of the image left them. A record only grows, so the bytes
  /// after them are entries that a later life kept, and a life of the image goes on from it only
  /// while none of them is a birth of its own.
  fn stands(&self, record: &Path) -> bool {
    let read = (monty::MIN_SUPPORTED_DUMP_VERSION..=monty::DUMP_VERSION).contains(&self.dump);
    let ours = Self::of(record, Some(self.bytes), self.parts.clone());
    read
      && ours.is_ok_and(|now| Self { dump: self.dump, ..now } == *self)
      && world::goes_after(record, self.bytes)
  }
}

/// An image that agrees with its record: its half of the host, the file that holds its session and
/// the part of the host of the life, with where each stands in it, and the bytes of the record at
/// the tip that it names, which each entry that a later life kept stands after. The image holds no
/// byte of its file: a session and a part of the host are each as large as the life, so each is
/// read from the file when it is asked for, and the part of the host as it is parsed.
pub(crate) struct Image {
  pub(crate) half: Half,
  pub(crate) bytes: u64,
  file: File,
  session: Range<u64>,
  host: Range<u64>,
}

impl Image {
  /// The session of the image, as monty loads it.
  pub(crate) fn session(&self) -> io::Result<Vec<u8>> {
    let mut session = Vec::new();
    self.part(&self.session)?.read_to_end(&mut session)?;
    Ok(session)
  }

  /// The part of the host of the life, as that host gave it to [`keep`]. It is read whole, since a
  /// parse of JSON from a reader takes its bytes one at a time, several times slower.
  pub(crate) fn host(&self) -> io::Result<Vec<u8>> {
    let mut host = Vec::new();
    self.part(&self.host)?.read_to_end(&mut host)?;
    Ok(host)
  }

  /// The bytes of a part of the file.
  fn part(&self, part: &Range<u64>) -> io::Result<impl Read + '_> {
    let mut file = &self.file;
    file.seek(SeekFrom::Start(part.start))?;
    Ok(file.take(part.end - part.start))
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
  agreed(record, File::open(path(record)).ok()?).ok().flatten()
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

/// The image that a file holds, when its key is the key of its body at the bytes of the record that
/// it names. The digest of the body is read in one pass that keeps no byte of it.
fn agreed(record: &Path, file: File) -> io::Result<Option<Image>> {
  let mut read = BufReader::with_capacity(BUFFER, file);
  let mut key = Vec::new();
  let start = read.read_until(b'\n', &mut key)? as u64;
  let Ok(key) = serde_json::from_slice::<Key>(&key) else { return Ok(None) };
  let mut summed = Summed::new(io::sink());
  let end = start + io::copy(&mut read, &mut summed)?;
  if key.parts.body != format!("{:x}", summed.digest.finalize()) || !key.stands(record) {
    return Ok(None);
  }
  let size = |bytes: usize| u64::try_from(bytes).unwrap_or(u64::MAX);
  let session = start..start.saturating_add(size(key.parts.session));
  let host = session.end.saturating_add(size(key.parts.half))..end;
  if host.start > end {
    return Ok(None);
  }
  let mut file = read.into_inner();
  file.seek(SeekFrom::Start(session.end))?;
  let half = BufReader::new((&file).take(host.start - session.end));
  let Ok(half) = serde_json::from_reader(half) else { return Ok(None) };
  Ok(Some(Image { half, bytes: key.bytes, file, session, host }))
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
  let Some((half, session)) = engine.image()? else { return Ok(false) };
  let unkept = |no: &dyn Display| Fault::refused(format!("the life keeps no image: {no}"));
  // The key stands before the body and names its digest and its parts, which only the write of
  // the body gives. So the file takes a line as wide as the widest key first, then each part of
  // the body in turn, which no second buffer holds, then the key over that line: the spaces that
  // stay after it are a part of no value.
  let widest =
    Parts { body: format!("{:x}", Sha256::digest([])), session: usize::MAX, half: usize::MAX };
  let mut key = Key::of(record, None, widest).map_err(|no| unkept(&no))?;
  let filled = disk::filled(&path(record), disk::Access::Owner, |file| {
    let mut line = vec![b' '; serde_json::to_vec(&key)?.len()];
    line.push(b'\n');
    file.write_all(&line)?;
    let mut body = BufWriter::with_capacity(BUFFER, Summed::new(&mut *file));
    body.write_all(&session)?;
    serde_json::to_writer(&mut body, &half)?;
    body.flush()?;
    let half = body.get_ref().bytes - session.len();
    serde_json::to_writer(&mut body, host)?;
    let summed = body.into_inner().map_err(IntoInnerError::into_error)?;
    let body = format!("{:x}", summed.digest.finalize());
    key.parts = Parts { body, session: session.len(), half };
    file.seek(SeekFrom::Start(0))?;
    file.write_all(&serde_json::to_vec(&key)?)
  });
  filled.map_err(|no| unkept(&no))?;
  Ok(true)
}

/// How many bytes of a body wait for the file and for the digest at a time.
const BUFFER: usize = 1 << 20;

/// A writer that passes each byte on, with the count and the digest of the bytes that passed.
struct Summed<W> {
  to: W,
  digest: Sha256,
  bytes: usize,
}

impl<W> Summed<W> {
  fn new(to: W) -> Self {
    Self { to, digest: Sha256::new(), bytes: 0 }
  }
}

impl<W: Write> Write for Summed<W> {
  fn write(&mut self, part: &[u8]) -> io::Result<usize> {
    let passed = self.to.write(part)?;
    self.digest.update(&part[..passed]);
    self.bytes += passed;
    Ok(passed)
  }

  fn flush(&mut self) -> io::Result<()> {
    self.to.flush()
  }
}
