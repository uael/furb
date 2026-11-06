//! The images of a turn. A host attaches an image to a message as a file of a directory of images, named by the
//! digest of its bytes, and the message names it as a Markdown image, `![name](furb-image://<digest>.<extension>)`.
//! The provider hands a model each image that the message of a prompt of a user turn names, and each image that a read
//! of a user turn gives, as an image of rig, and reads it once while its file stays the same.

use std::{
  collections::{HashMap, HashSet},
  fs, io, mem,
  path::{Path, PathBuf},
  time::SystemTime,
};

use base64::{Engine, engine::general_purpose::STANDARD};
use rig_core::message::{ImageMediaType, MimeType, UserContent};
use sha2::{Digest, Sha256};

use crate::fact;
use crate::value;
use crate::world::disk::{self, Access};

/// The most bytes an image holds: 20 MiB.
const LARGEST: u64 = 20 * 1024 * 1024;

/// The scheme of the uri of an image.
const SCHEME: &str = "furb-image://";

/// The types of an image, each by its media type, its extension, and the first bytes of its file.
const KINDS: [(&str, &str, &[u8]); 4] = [
  ("image/png", "png", b"\x89PNG\r\n\x1a\n"),
  ("image/jpeg", "jpg", b"\xff\xd8\xff"),
  ("image/gif", "gif", b"GIF8"),
  ("image/webp", "webp", b"RIFF"),
];

/// Why an image did not attach, keep or read.
#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("Choose a PNG, JPEG, GIF, or WebP image.")]
  NotImage,
  #[error("An image must be a file of at most 20 MiB.")]
  TooLarge,
  #[error("The saved image is too large.")]
  SavedTooLarge,
  #[error("The saved image attachment has changed.")]
  Changed,
  #[error("Invalid image attachment.")]
  Invalid,
  #[error("The image {uri} did not load from {}: {source}", path.display())]
  Unloaded { uri: String, path: PathBuf, source: io::Error },
  #[error("{0}")]
  Io(#[from] io::Error),
}

/// An image a host attached: the name of its file, its uri, its media type, and its size in bytes.
pub struct Attached {
  pub name: String,
  pub uri: String,
  pub media: &'static str,
  pub size: u64,
}

/// An image a message names: its text in the message, its name, and its uri.
#[cfg_attr(feature = "typescript", napi_derive::napi(object, object_from_js = false))]
#[cfg_attr(feature = "python", derive(pyo3::IntoPyObject))]
pub struct Named {
  pub text: String,
  pub name: String,
  pub uri: String,
}

/// The type of an image by its first bytes: its media type and its extension, for a PNG, a JPEG, a GIF or a WebP.
pub fn kind(bytes: &[u8]) -> Result<(&'static str, &'static str), Error> {
  let gif = bytes.get(4..6).is_some_and(|version| version == b"7a" || version == b"9a");
  let webp = bytes.get(8..12) == Some(b"WEBP");
  KINDS
    .iter()
    .find(|(_, extension, head)| {
      bytes.starts_with(head)
        && match *extension {
          "gif" => gif,
          "webp" => webp,
          _ => true,
        }
    })
    .map(|(media, extension, _)| (*media, *extension))
    .ok_or(Error::NotImage)
}

/// An image copied into a directory of images under the digest of its bytes, with the name of the file it came from.
pub fn attach(directory: &Path, path: &Path) -> Result<Attached, Error> {
  let info = fs::metadata(path)?;
  if !info.is_file() || info.len() > LARGEST {
    return Err(Error::TooLarge);
  }
  let bytes = fs::read(path)?;
  let (uri, media) = keep(directory, &bytes)?;
  let name = path.file_name().map(|one| one.to_string_lossy().into_owned()).unwrap_or_default();
  Ok(Attached { name, uri, media, size: bytes.len() as u64 })
}

/// An image kept in a directory of images under the digest of its bytes: its uri and its media type.
pub fn keep(directory: &Path, bytes: &[u8]) -> Result<(String, &'static str), Error> {
  if bytes.len() as u64 > LARGEST {
    return Err(Error::TooLarge);
  }
  let (media, extension) = kind(bytes)?;
  let file = format!("{}.{extension}", digest(bytes));
  let at = directory.join(&file);
  // An image is named by the digest of its bytes, and lands whole, so a file that stands under that
  // name is that image.
  if !at.exists() {
    disk::write(&at, bytes, Access::Owner).map_err(|no| no.source)?;
  }
  Ok((format!("{SCHEME}{file}"), media))
}

/// The file of a directory of images that holds the image of a uri, and the digest its bytes have.
pub struct Located {
  pub path: PathBuf,
  pub digest: String,
}

/// Where the image of a uri stands in a directory of images.
pub fn path(directory: &Path, uri: &str) -> Result<Located, Error> {
  let (digest, _) = parts(uri).ok_or(Error::Invalid)?;
  let file = uri.strip_prefix(SCHEME).unwrap_or_default();
  Ok(Located { path: directory.join(file), digest: digest.to_owned() })
}

/// The bytes of the image of a uri, and its media type, once the bytes have the digest the uri names.
pub fn read(directory: &Path, uri: &str) -> Result<(&'static str, Vec<u8>), Error> {
  let Located { path, digest: named } = path(directory, uri)?;
  let size = fs::metadata(&path).map_err(|no| unloaded(uri, &path, no))?.len();
  if size > LARGEST {
    return Err(Error::SavedTooLarge);
  }
  let bytes = fs::read(&path).map_err(|no| unloaded(uri, &path, no))?;
  if digest(&bytes) != named {
    return Err(Error::Changed);
  }
  Ok((kind(&bytes)?.0, bytes))
}

/// Why the image of a uri did not load: the file it was read from, and what the system said.
fn unloaded(uri: &str, path: &Path, source: io::Error) -> Error {
  Error::Unloaded { uri: uri.to_owned(), path: path.to_owned(), source }
}

/// How a message names an image: `![name](uri)`, with each bracket and line break of the name as `_`.
pub fn reference(name: &str, uri: &str) -> String {
  let name: String = name
    .chars()
    .map(|one| if matches!(one, '[' | ']' | '\r' | '\n') { '_' } else { one })
    .collect();
  format!("![{name}]({uri})")
}

/// Each image a message names, in order.
pub fn references(message: &str) -> Vec<Named> {
  placed(message).into_iter().map(|(_, one)| one).collect()
}

/// Each image a message names, in order, with the place in the message where its text starts.
fn placed(message: &str) -> Vec<(usize, Named)> {
  let mut found = Vec::new();
  let mut from = 0;
  while let Some(at) = message[from..].find("![") {
    let open = from + at;
    from = open + 2;
    let rest = &message[from..];
    let Some(close) = rest.find(']') else { break };
    let Some((uri, _)) = rest[close + 1..].strip_prefix('(').and_then(|tail| tail.split_once(')'))
    else {
      continue;
    };
    if parts(uri).is_none() {
      continue;
    }
    let end = from + close + 1 + uri.len() + 2;
    let (name, uri) = (rest[..close].to_owned(), uri.to_owned());
    found.push((open, Named { text: message[open..end].to_owned(), name, uri }));
    from = end;
  }
  found
}

/// The digest and the extension a uri names, when it is the uri of an image.
fn parts(uri: &str) -> Option<(&str, &str)> {
  let (digest, extension) = uri.strip_prefix(SCHEME)?.split_once('.')?;
  let hex =
    digest.len() == 64 && digest.bytes().all(|one| matches!(one, b'0'..=b'9' | b'a'..=b'f'));
  (hex && KINDS.iter().any(|(_, known, _)| *known == extension)).then_some((digest, extension))
}

/// The digest of bytes, as lowercase hex.
fn digest(bytes: &[u8]) -> String {
  value::hex(&Sha256::digest(bytes))
}

/// The images of one life: the directory they stand in, and the content of each that a turn named, by its file,
/// with the size and the time of the change of the file it was read at.
pub(super) struct Images {
  directory: Option<PathBuf>,
  held: HashMap<PathBuf, Held>,
  /// The uri of each image whose file was lost, which a turn named, and those that were not told
  /// yet.
  lost: HashSet<String>,
  fresh: Vec<String>,
}

/// The content of an image, with the size and the time of the change of the file it was read at.
struct Held {
  len: u64,
  modified: Option<SystemTime>,
  content: UserContent,
}

impl Images {
  pub(super) fn new(directory: Option<PathBuf>) -> Self {
    Self { directory, held: HashMap::new(), lost: HashSet::new(), fresh: Vec::new() }
  }

  /// The uri of each image that a turn names and whose file is lost, which came since the last
  /// call: each image comes once in the life, so the life tells it once.
  pub(super) fn lost(&mut self) -> Vec<String> {
    mem::take(&mut self.fresh)
  }

  /// The images that the prompts a user turn opens name, and the images that its reads give, each
  /// with the place of the python of the turn where the text that names it ends, so [`sequence`]
  /// hands them to a model after that text, in the order of the turn; and none when the life has no
  /// directory of images. A prompt tells its message in its open, and a read of an image tells its
  /// text as that image alone, so an image that anything else names, as a text file that a read
  /// shows, is no image of the turn. An image comes once in a request: an image that an earlier
  /// turn of it gave, as the turns of the origin of a fork, is not given again, since the model
  /// reads it there under its name. An image whose file is lost is given to no model, and the text
  /// of the turn still names it, so the turn goes with the rest of it; [`Images::lost`] tells it.
  pub(super) fn named(
    &mut self,
    python: &str,
    given: &mut HashSet<String>,
  ) -> Result<Vec<Seen>, Error> {
    let Some(directory) = self.directory.clone() else { return Ok(Vec::new()) };
    let mut uris: Vec<(usize, Named)> = Vec::new();
    let mut start = 0;
    for paragraph in paragraphs(python) {
      let at = start;
      start += paragraph.len() + 2;
      let name = paragraph.strip_prefix('#').and_then(|rest| rest.split([' ', '\n']).next());
      let name = name.unwrap_or_default();
      let opens = fact::named(name, "prompt")
        && paragraph.lines().last().is_some_and(|last| last.starts_with(&format!("{name}: Act[")));
      let reads = fact::named(name, "read");
      if !opens && !reads {
        continue;
      }
      for (place, one) in placed(&paragraph) {
        if (opens || alone(&paragraph, name, &one)) && given.insert(one.uri.clone()) {
          uris.push((at + place + one.text.len(), one));
        }
      }
    }
    let mut seen = Vec::new();
    for (end, one) in uris {
      match self.image(&directory, &one.uri) {
        Ok(content) => seen.push(Seen { end, content }),
        Err(Error::Unloaded { source, .. }) if source.kind() == io::ErrorKind::NotFound => {
          if self.lost.insert(one.uri.clone()) {
            self.fresh.push(one.uri);
          }
        }
        Err(no) => return Err(no),
      }
    }
    Ok(seen)
  }

  /// The content of the image of a uri: the one read before while its file stays the same, or read now.
  fn image(&mut self, directory: &Path, uri: &str) -> Result<UserContent, Error> {
    let path = path(directory, uri)?.path;
    let info = fs::metadata(&path).map_err(|no| unloaded(uri, &path, no))?;
    let (len, modified) = (info.len(), info.modified().ok());
    if let Some(held) = self.held.get(&path)
      && held.len == len
      && held.modified == modified
    {
      return Ok(held.content.clone());
    }
    let (media, bytes) = read(directory, uri)?;
    let content = UserContent::image_base64(
      STANDARD.encode(bytes),
      ImageMediaType::from_mime_type(media),
      None,
    );
    self.held.insert(path, Held { len, modified, content: content.clone() });
    Ok(content)
  }
}

/// Whether the paragraph of a read tells the text of the read as this image alone, as a read of an image gives it.
fn alone(paragraph: &str, name: &str, one: &Named) -> bool {
  let told = format!("{name}_text = ");
  paragraph.lines().any(|line| {
    line.strip_prefix(&told).is_some_and(|value| {
      value.len() == one.text.len() + 2 && value.get(1..value.len() - 1) == Some(one.text.as_str())
    })
  })
}

/// The paragraphs of the python of a user turn, in order: a blank line that a header follows ends one, since a word
/// its caller wrote may hold a blank line of its own.
fn paragraphs(python: &str) -> Vec<String> {
  let mut paragraphs: Vec<String> = Vec::new();
  for piece in python.split("\n\n") {
    let headed = piece
      .strip_prefix('#')
      .and_then(|rest| rest.chars().next())
      .is_some_and(|one| !one.is_whitespace());
    match paragraphs.last_mut() {
      Some(last) if !headed => {
        last.push_str("\n\n");
        last.push_str(piece);
      }
      _ => paragraphs.push(piece.to_owned()),
    }
  }
  paragraphs
}

/// An image that a model is handed with a user turn: the place in the python of the turn where the
/// text that names it ends, and its content.
#[derive(Debug)]
pub(super) struct Seen {
  pub(super) end: usize,
  pub(super) content: UserContent,
}

/// The parts that a model is handed for a user turn, in the order of its text: the text up to the
/// end of the text that names an image, the image, and the text after it, so each image stands
/// where the turn names it, as the APIs of Anthropic and of `OpenAI` take a message. `banner` is
/// where the open of a chain starts, as [`super::banner`] finds it, which two line breaks stand
/// before: the text from there is a block of its own, so the setup before it ends a block that a
/// provider can keep in its cache.
pub(super) fn sequence(
  python: &str,
  seen: Vec<Seen>,
  mut banner: Option<usize>,
) -> Vec<UserContent> {
  let mut parts = Vec::new();
  let mut from = 0;
  let text = |parts: &mut Vec<UserContent>, from: usize, to: usize| {
    if let Some(one) = python.get(from..to).filter(|one| !one.is_empty()) {
      parts.push(UserContent::text(one));
    }
  };
  for one in seen {
    if let Some(at) = banner.filter(|at| one.end > at.saturating_sub(2)) {
      text(&mut parts, from, at.saturating_sub(2));
      (from, banner) = (at, None);
    }
    text(&mut parts, from, one.end);
    parts.push(one.content);
    from = one.end;
  }
  if let Some(at) = banner {
    text(&mut parts, from, at.saturating_sub(2));
    from = at;
  }
  text(&mut parts, from, python.len());
  parts
}

#[cfg(test)]
#[path = "images.test.rs"]
mod test;
