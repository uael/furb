//! The images of the input and of his messages: the image file of a path that he pastes, as a
//! terminal pastes a file that he drags on it, and the mark that stands for an image in a text.

use std::{fs, ops::Range, path::Path};

use furb::world::images;
use furb_client::{PICTURE, Picture, mentioned};
use opentui::style::Style;

use crate::line::{Line, run};

/// The most bytes of a file that a paste reads as an image, as a life keeps one.
const LARGEST: u64 = 20 << 20;

/// The most rows that the box of an image in the feed takes.
const HIGH: u64 = 12;

/// The pixels of an image that one row of its box shows when it is smaller than the box.
const ROW: u64 = 16;

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

/// A line of text in a style, with each mark of an image, from its sign to the next space, and
/// each file that it mentions in the style of a mark, as the input shows them.
pub(crate) fn marked(line: &str, plain: Style, mark: Style) -> Line {
  let images = line.match_indices(PICTURE).map(|(at, _)| {
    let after = line.get(at + PICTURE.len()..).unwrap_or_default();
    at..at + PICTURE.len() + after.find(' ').unwrap_or(after.len())
  });
  let mut spans: Vec<Range<usize>> =
    images.chain(mentioned(line).into_iter().map(|(span, _)| span)).collect();
  spans.sort_by_key(|span| span.start);
  let mut out = Vec::new();
  let mut from = 0;
  for span in spans {
    // A mark inside another one, as an @ in the name of an image, is part of it.
    if span.start < from {
      continue;
    }
    if span.start > from {
      out.push(run(line.get(from..span.start).unwrap_or_default().to_owned(), plain));
    }
    out.push(run(line.get(span.clone()).unwrap_or_default().to_owned(), mark));
    from = span.end;
  }
  if from < line.len() || out.is_empty() {
    out.push(run(line.get(from..).unwrap_or_default().to_owned(), plain));
  }
  out
}

/// The columns and the rows of the box of an image in a room of columns: as high as `HIGH` rows,
/// or as the image at `ROW` pixels a row when it is smaller, and as wide as its shape, with a cell
/// twice as high as it is wide, inside the room. An image that its machine has not given yet keeps
/// the most rows, so the feed moves little once it comes, and one that does not draw takes none.
pub(crate) fn cells(held: Option<&Picture>, room: usize) -> (u16, u16) {
  let room = u64::try_from(room).unwrap_or(u64::MAX).max(1);
  let (wide, high) = match held {
    Some(Picture::Held(one)) => {
      let (width, height) = (u64::from(one.width.max(1)), u64::from(one.height.max(1)));
      let high = height.div_ceil(ROW).clamp(1, HIGH);
      let wide = (high * 2 * width).div_ceil(height);
      if wide <= room { (wide, high) } else { (room, (room * height / (2 * width)).max(1)) }
    }
    Some(Picture::Refused(_)) => (room, 0),
    _ => (room, HIGH),
  };
  let cells = |count: u64| u16::try_from(count).unwrap_or(u16::MAX);
  (cells(wide), cells(high))
}
