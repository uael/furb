//! The images of a turn, on files of a yard of each test.

use std::fs;

use rig_core::message::{DocumentSourceKind, ImageMediaType, UserContent};

use std::collections::HashSet;

use super::{Images, attach, kind, read, reference, references};
use crate::testing::Yard;
use std::process;

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

#[test]
fn a_turn_hands_a_model_each_image_that_a_read_gives_and_no_image_that_a_text_names() {
  let at = yard("reads");
  fs::write(at.join("a.png"), PNG).expect("the image is written");
  let kept = attach(&at, &at.join("a.png")).expect("the image is attached");
  let named = reference("a.png", &kept.uri);
  let read = format!("#read1\nread1_path = '/x/a.png'\nread1_text = '{named}'");
  let text = format!("#read2\nread2_path = '/x/a.md'\nread2_text = 'see {named}'");
  let mut images = Images::new(Some(at.to_path_buf()));
  assert_eq!(
    images.named(&read, &mut HashSet::new()).map(|seen| seen.len()).map_err(|no| no.to_string()),
    Ok(1),
    "a read of an image hands it after the text that names it"
  );
  assert_eq!(
    images.named(&text, &mut HashSet::new()).map(|seen| seen.len()).map_err(|no| no.to_string()),
    Ok(0),
    "a text that names an image hands none"
  );
}

#[test]
fn a_turn_hands_a_model_each_image_that_the_message_of_a_prompt_it_opens_names_once() {
  let at = yard("turn");
  fs::write(at.join("a.png"), PNG).expect("the image is written");
  let kept = attach(&at, &at.join("a.png")).expect("the image is attached");
  let named = reference("a.png", &kept.uri);
  let python = format!(
    "#prompt1 look at {named} and {named}\nprompt1: Act[str] = Act('prompt1')\n\n#read a.txt\n# {named}\n\n#prompt2 no open"
  );
  let mut images = Images::new(Some(at.to_path_buf()));
  let seen = images.named(&python, &mut HashSet::new()).expect("the images of the turn");
  let [one] = &seen[..] else { panic!("one image: {seen:?}") };
  let UserContent::Image(image) = &one.content else { panic!("an image: {seen:?}") };
  assert!(python[..one.end].ends_with(&named), "the image stands after the text that names it");
  assert_eq!(python[..one.end].matches(named.as_str()).count(), 1, "after the first name of it");
  let mut given = HashSet::from([kept.uri.clone()]);
  assert_eq!(
    images.named(&python, &mut given).map(|seen| seen.len()).map_err(|no| no.to_string()),
    Ok(0),
    "an image that an earlier turn of the request gave comes no more"
  );
  assert_eq!(image.media_type, Some(ImageMediaType::PNG));
  assert!(
    matches!(&image.data, DocumentSourceKind::Base64(data) if data == "iVBORw0KGgp0aGUgcmVzdA==")
  );
  assert_eq!(
    Images::new(None)
      .named(&python, &mut HashSet::new())
      .map(|seen| seen.len())
      .map_err(|no| no.to_string()),
    Ok(0),
    "a life with no directory hands none"
  );
  let file = at.join(kept.uri.strip_prefix("furb-image://").unwrap_or_default());
  fs::remove_file(&file).expect("the image goes");
  let system = fs::metadata(&file).expect_err("the image is gone");
  let gone = format!("The image {} did not load from {}: {system}", kept.uri, file.display());
  assert_eq!(
    images.named(&python, &mut HashSet::new()).map(|seen| seen.len()).map_err(|no| no.to_string()),
    Err(gone),
    "an image whose file is gone fails"
  );
}

#[test]
fn a_turn_goes_as_its_text_with_each_image_after_the_text_that_names_it() {
  let at = yard("sequence");
  fs::write(at.join("a.png"), PNG).expect("the image is written");
  fs::write(at.join("b.png"), b"\x89PNG\r\n\x1a\nother").expect("the image is written");
  let first = reference("a.png", &attach(&at, &at.join("a.png")).expect("attached").uri);
  let second = reference("b.png", &attach(&at, &at.join("b.png")).expect("attached").uri);
  let python =
    format!("#prompt1 first {first} middle {second} last\nprompt1: Act[str] = Act('prompt1')");
  let mut images = Images::new(Some(at.to_path_buf()));
  let seen = images.named(&python, &mut HashSet::new()).expect("the images of the turn");
  let parts = super::sequence(&python, seen, None);
  let kinds: Vec<&str> = parts
    .iter()
    .map(|one| match one {
      UserContent::Text(_) => "text",
      UserContent::Image(_) => "image",
      _ => "other",
    })
    .collect();
  assert_eq!(kinds, ["text", "image", "text", "image", "text"]);
  let texts: Vec<&str> = parts
    .iter()
    .filter_map(|one| match one {
      UserContent::Text(text) => Some(text.text.as_str()),
      _ => None,
    })
    .collect();
  assert_eq!(texts[0], format!("#prompt1 first {first}"));
  assert_eq!(texts[1], format!(" middle {second}"));
  assert_eq!(texts.concat(), python, "no text is lost between the parts");
}

#[test]
fn the_open_of_a_chain_stays_a_block_of_its_own_before_an_image_that_a_prompt_after_it_names() {
  let at = yard("sequence-banner");
  fs::write(at.join("a.png"), PNG).expect("the image is written");
  let named = reference("a.png", &attach(&at, &at.join("a.png")).expect("attached").uri);
  let python = format!(
    "setup\n\n#chain1\nline\n\n#prompt1 look at {named} now\nprompt1: Act[str] = Act('prompt1')"
  );
  let mut images = Images::new(Some(at.to_path_buf()));
  let seen = images.named(&python, &mut HashSet::new()).expect("the images of the turn");
  let parts = super::sequence(&python, seen, super::super::banner(&python));
  let texts: Vec<Option<&str>> = parts
    .iter()
    .map(|one| match one {
      UserContent::Text(text) => Some(text.text.as_str()),
      _ => None,
    })
    .collect();
  assert_eq!(texts.len(), 4);
  assert_eq!(texts[0], Some("setup"));
  assert!(texts[1].is_some_and(|one| one.starts_with("#chain1") && one.ends_with(&named)));
  assert_eq!(texts[2], None, "the image");
  assert_eq!(texts[3], Some(" now\nprompt1: Act[str] = Act('prompt1')"));
}
