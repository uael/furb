//! The images of a turn, on files of a yard of each test.

use std::{fs, path::PathBuf};

use rig_core::message::{DocumentSourceKind, ImageMediaType, UserContent};

use super::{Images, attach, kind, read, reference, references};

/// The first bytes of a PNG, which is all the type of an image is read by.
const PNG: &[u8] = b"\x89PNG\r\n\x1a\nthe rest";

/// A yard of one test, empty.
fn yard(name: &str) -> PathBuf {
  let at = std::env::temp_dir().join(format!("furb-images-{name}"));
  let _ = fs::remove_dir_all(&at);
  fs::create_dir_all(&at).expect("a yard of the test");
  at
}

#[test]
fn the_type_of_an_image_is_read_off_its_first_bytes() {
  assert_eq!(kind(PNG), Ok(("image/png", "png")));
  assert_eq!(kind(b"\xff\xd8\xff\xe0"), Ok(("image/jpeg", "jpg")));
  assert_eq!(kind(b"GIF89a..."), Ok(("image/gif", "gif")));
  assert_eq!(kind(b"RIFF\0\0\0\0WEBPVP8 "), Ok(("image/webp", "webp")));
  assert_eq!(kind(b"RIFF\0\0\0\0WAVE"), Err("Choose a PNG, JPEG, GIF, or WebP image.".to_owned()));
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
fn a_markdown_names_an_image_as_a_markdown_image_and_names_nothing_else_so() {
  let uri = format!("furb-image://{}.png", "a".repeat(64));
  let named = reference("my [draft]\n", &uri);
  assert_eq!(named, format!("![my _draft__]({uri})"));
  let markdown =
    format!("see {named} and ![x](https://elsewhere/a.png) and ![y](furb-image://short.png)");
  let found = references(&markdown);
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
  assert_eq!(read(&at, &kept.uri), Ok(("image/png", PNG.to_vec())));
  let file = at.join(kept.uri.strip_prefix("furb-image://").unwrap_or_default());
  fs::write(&file, b"\x89PNG\r\n\x1a\nchanged").expect("the kept image changes");
  assert_eq!(read(&at, &kept.uri), Err("The saved image attachment has changed.".to_owned()));
}

#[test]
fn a_turn_hands_a_model_each_image_that_the_markdown_of_a_thread_it_opens_names_once() {
  let at = yard("turn");
  fs::write(at.join("a.png"), PNG).expect("the image is written");
  let kept = attach(&at, &at.join("a.png")).expect("the image is attached");
  let named = reference("a.png", &kept.uri);
  let python = format!(
    "#thread1 look at {named} and {named}\nthread1: Act[str] = Act('thread1')\n\n#read a.txt\n# {named}\n\n#thread2 no open"
  );
  let mut images = Images::new(Some(at.clone()));
  let seen = images.named(&python).expect("the images of the turn");
  let [UserContent::Image(image)] = &seen[..] else { panic!("one image: {seen:?}") };
  assert_eq!(image.media_type, Some(ImageMediaType::PNG));
  assert!(
    matches!(&image.data, DocumentSourceKind::Base64(data) if data == "iVBORw0KGgp0aGUgcmVzdA==")
  );
  assert_eq!(
    Images::new(None).named(&python).map(|seen| seen.len()),
    Ok(0),
    "a life with no directory hands none"
  );
  fs::remove_file(at.join(kept.uri.strip_prefix("furb-image://").unwrap_or_default()))
    .expect("the image goes");
  assert!(images.named(&python).is_err(), "an image whose file is gone is read again, and fails");
}
