//! The images of a turn. A host attaches an image to a message as a file of a directory of images, named by the
//! digest of its bytes, and the message names it as a Markdown image, `![name](furb-image://<digest>.<extension>)`.
//! The provider hands a model each image that the message of a prompt of a user turn names, as an image of rig, and
//! reads it once while its file stays the same.

use std::{
  collections::HashMap,
  fs::{self, OpenOptions},
  io::{ErrorKind, Write},
  path::{Path, PathBuf},
  time::SystemTime,
};

use base64::{Engine, engine::general_purpose::STANDARD};
use rig_core::message::{ImageMediaType, MimeType, UserContent};
use sha2::{Digest, Sha256};

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
pub fn kind(bytes: &[u8]) -> Result<(&'static str, &'static str), String> {
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
    .ok_or_else(|| "Choose a PNG, JPEG, GIF, or WebP image.".to_owned())
}

/// An image copied into a directory of images under the digest of its bytes, with the name of the file it came from.
pub fn attach(directory: &Path, path: &Path) -> Result<Attached, String> {
  let info = fs::metadata(path).map_err(|no| no.to_string())?;
  if !info.is_file() || info.len() > LARGEST {
    return Err("An image must be a file of at most 20 MiB.".to_owned());
  }
  let bytes = fs::read(path).map_err(|no| no.to_string())?;
  let (media, extension) = kind(&bytes)?;
  let file = format!("{}.{extension}", digest(&bytes));
  fs::create_dir_all(directory).map_err(|no| no.to_string())?;
  let mut options = OpenOptions::new();
  options.write(true).create_new(true);
  #[cfg(unix)]
  std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
  match options.open(directory.join(&file)) {
    Ok(mut kept) => kept.write_all(&bytes).map_err(|no| no.to_string())?,
    Err(no) if no.kind() == ErrorKind::AlreadyExists => {}
    Err(no) => return Err(no.to_string()),
  }
  let name = path.file_name().map(|one| one.to_string_lossy().into_owned()).unwrap_or_default();
  Ok(Attached { name, uri: format!("{SCHEME}{file}"), media, size: bytes.len() as u64 })
}

/// The file of a directory of images that holds the image of a uri, and the digest its bytes have.
pub fn path(directory: &Path, uri: &str) -> Result<(PathBuf, String), String> {
  let (digest, _) = parts(uri).ok_or_else(|| "Invalid image attachment.".to_owned())?;
  let file = uri.strip_prefix(SCHEME).unwrap_or_default();
  Ok((directory.join(file), digest.to_owned()))
}

/// The bytes of the image of a uri, and its media type, once the bytes have the digest the uri names.
pub fn read(directory: &Path, uri: &str) -> Result<(&'static str, Vec<u8>), String> {
  let (path, named) = path(directory, uri)?;
  let size = fs::metadata(&path).map_err(|no| no.to_string())?.len();
  if size > LARGEST {
    return Err("The saved image is too large.".to_owned());
  }
  let bytes = fs::read(&path).map_err(|no| no.to_string())?;
  if digest(&bytes) != named {
    return Err("The saved image attachment has changed.".to_owned());
  }
  Ok((kind(&bytes)?.0, bytes))
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
    found.push(Named { text: message[open..end].to_owned(), name, uri });
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
  Sha256::digest(bytes).iter().map(|one| format!("{one:02x}")).collect()
}

/// The images of one life: the directory they stand in, and the content of each that a turn named, by its file,
/// with the size and the time of the change of the file it was read at.
pub(super) struct Images {
  directory: Option<PathBuf>,
  held: HashMap<PathBuf, ((u64, Option<SystemTime>), UserContent)>,
}

impl Images {
  pub(super) fn new(directory: Option<PathBuf>) -> Images {
    Images { directory, held: HashMap::new() }
  }

  /// The images that the prompts a user turn opens name, each once, as images of rig; and none when the life has
  /// no directory of images. A prompt tells its message in its open, and an image that anything else names is no
  /// image of the turn.
  pub(super) fn named(&mut self, python: &str) -> Result<Vec<UserContent>, String> {
    let Some(directory) = self.directory.clone() else { return Ok(Vec::new()) };
    let mut uris: Vec<String> = Vec::new();
    for paragraph in paragraphs(python) {
      let name = paragraph.strip_prefix('#').and_then(|rest| rest.split([' ', '\n']).next());
      let name = name.unwrap_or_default();
      let prompt = name
        .strip_prefix("prompt")
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|one| one.is_ascii_digit()));
      let opens = prompt
        && paragraph.lines().last().is_some_and(|last| last.starts_with(&format!("{name}: Act[")));
      if !opens {
        continue;
      }
      for one in references(&paragraph) {
        if !uris.contains(&one.uri) {
          uris.push(one.uri);
        }
      }
    }
    uris.iter().map(|uri| self.image(&directory, uri)).collect()
  }

  /// The content of the image of a uri: the one read before while its file stays the same, or read now.
  fn image(&mut self, directory: &Path, uri: &str) -> Result<UserContent, String> {
    let (path, _) = path(directory, uri)?;
    let info = fs::metadata(&path).map_err(|no| no.to_string())?;
    let stamp = (info.len(), info.modified().ok());
    if let Some((held, content)) = self.held.get(&path)
      && *held == stamp
    {
      return Ok(content.clone());
    }
    let (media, bytes) = read(directory, uri)?;
    let content = UserContent::image_base64(
      STANDARD.encode(bytes),
      ImageMediaType::from_mime_type(media),
      None,
    );
    self.held.insert(path, (stamp, content.clone()));
    Ok(content)
  }
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

#[cfg(test)]
#[path = "images.test.rs"]
mod test;
