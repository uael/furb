//! The images of the input and of his messages: the image file of a path that he pastes, as a
//! terminal pastes a file that he drags on it, and the mark that stands for an image in a text.

use std::{fs, path::Path};

use furb::world::images;
use furb_client::PICTURE;
use opentui::style::Style;

use crate::line::{Line, run};

/// The most bytes of a file that a paste reads as an image, as a life keeps one.
const LARGEST: u64 = 20 << 20;

/// The bytes of the image file whose path a paste is, as a terminal pastes a file dragged on it:
/// one path, quoted or with its spaces escaped, or a `file://` url, from the folder of the window
/// when it is relative. None for any other text, which the input takes as it is.
pub(crate) fn file(text: &str, folder: &Path) -> Option<Vec<u8>> {
  let text = text.trim();
  if text.is_empty() || text.contains('\n') {
    return None;
  }
  let unquoted = ['\'', '"'].iter().find_map(|one| text.strip_prefix(*one)?.strip_suffix(*one));
  let path = unquoted.unwrap_or(text).replace("\\ ", " ");
  let path = path.strip_prefix("file://").unwrap_or(&path);
  let path = folder.join(path);
  let info = fs::metadata(&path).ok().filter(|one| one.is_file() && one.len() <= LARGEST)?;
  let bytes = fs::read(&path).ok().filter(|one| one.len() as u64 == info.len())?;
  images::kind(&bytes).is_ok().then_some(bytes)
}

/// A message with each image that it names as its mark, the sign and the name of the image.
pub(crate) fn shown(text: &str) -> String {
  let named = images::references(text);
  named.into_iter().fold(text.to_owned(), |text, one| {
    text.replacen(&one.text, &format!("{PICTURE}{}", one.name), 1)
  })
}

/// A line of text in a style, with each mark of an image in the style of a mark: from its sign to
/// the next space.
pub(crate) fn marked(line: &str, plain: Style, mark: Style) -> Line {
  let mut out = Vec::new();
  let mut rest = line;
  while let Some(at) = rest.find(PICTURE) {
    let after = &rest[at + PICTURE.len()..];
    let end = at + PICTURE.len() + after.find(' ').unwrap_or(after.len());
    if at > 0 {
      out.push(run(&rest[..at], plain));
    }
    out.push(run(&rest[at..end], mark));
    rest = &rest[end..];
  }
  if !rest.is_empty() || out.is_empty() {
    out.push(run(rest, plain));
  }
  out
}

#[cfg(test)]
#[path = "picture.test.rs"]
mod test;
