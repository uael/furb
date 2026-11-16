//! The images that a host attaches and that a message names, through `furb::world::images`, on files
//! of a yard of each test. The images of a turn run through a life in `provider.rs`.

use std::{fs, process};

use crate::yard::Yard;
use furb::world::images::{attach, kind, read, reference, references};

/// The first bytes of a PNG, which is all the type of an image is read by.
const PNG: &[u8] = b"\x89PNG\r\n\x1a\nthe rest";

/// A yard of one test, empty.
fn yard(name: &str) -> Yard {
  Yard::new(format!("furb-images-{name}-{}", process::id()))
}

#[test]
fn the_type_of_an_image_is_read_off_its_first_bytes() {
  assert_eq!(kind(PNG).expect("a known image"), ("image/png", "png"));
  assert_eq!(kind(b"\xff\xd8\xff\xe0").expect("a known image"), ("image/jpeg", "jpg"));
  assert_eq!(kind(b"GIF89a...").expect("a known image"), ("image/gif", "gif"));
  assert_eq!(kind(b"RIFF\0\0\0\0WEBPVP8 ").expect("a known image"), ("image/webp", "webp"));
  assert_eq!(
    kind(b"RIFF\0\0\0\0WAVE").unwrap_err().to_string(),
    "Choose a PNG, JPEG, GIF, or WebP image."
  );
}

#[test]
fn an_attached_image_is_kept_once_under_the_digest_of_its_bytes() {
  let at = yard("attach");
  let (source, images) = (at.join("design.png"), at.join("images"));
  fs::write(&source, PNG).expect("the image is written");
  let first = attach(&images, &source).expect("the image is attached");
  let again = attach(&images, &source).expect("the same image is attached again");
  assert_eq!(
    (first.name.as_str(), first.media, first.size),
    ("design.png", "image/png", PNG.len() as u64)
  );
  assert_eq!(first.uri, again.uri);
  let file = first.uri.strip_prefix("furb-image://").expect("the scheme of an image");
  assert_eq!(fs::read(images.join(file)).expect("the kept image"), PNG);
  fs::write(at.join("notes.txt"), "text").expect("a text is written");
  assert!(attach(&images, &at.join("notes.txt")).is_err(), "a text is no image");
}

#[test]
fn a_message_names_an_image_as_markdown_and_names_nothing_else_so() {
  let uri = format!("furb-image://{}.png", "a".repeat(64));
  let named = reference("my [draft]\n", &uri);
  assert_eq!(named, format!("![my _draft__]({uri})"));
  let message =
    format!("see {named} and ![x](https://elsewhere/a.png) and ![y](furb-image://short.png)");
  let found = references(&message);
  assert_eq!(found.len(), 1);
  assert_eq!(
    (found[0].name.as_str(), found[0].uri.as_str(), found[0].text.as_str()),
    ("my _draft__", uri.as_str(), named.as_str())
  );
}

#[test]
fn an_image_is_read_only_while_its_bytes_have_the_digest_it_is_named_by() {
  let at = yard("read");
  fs::write(at.join("a.png"), PNG).expect("the image is written");
  let kept = attach(&at, &at.join("a.png")).expect("the image is attached");
  assert_eq!(read(&at, &kept.uri).expect("the image reads"), ("image/png", PNG.to_vec()));
  let file = at.join(kept.uri.strip_prefix("furb-image://").unwrap_or_default());
  fs::write(&file, b"\x89PNG\r\n\x1a\nchanged").expect("the kept image changes");
  assert_eq!(
    read(&at, &kept.uri).unwrap_err().to_string(),
    "The saved image attachment has changed."
  );
}
