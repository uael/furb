//! The answers of a server as the acts give them: each kind of block, an image, an error, structured content, a
//! prompt, and the cut of a long text.

use std::{fs, path::PathBuf};

use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::json;

use super::{LONGEST, called, cut, prompted, read};
use crate::{extension::mcp::fixture::PICTURE, world::images};

/// A directory of images of its own for one test, and empty.
fn gallery(name: &str) -> PathBuf {
  let at = std::env::temp_dir().join(format!("furb-mcp-content-{name}-{}", std::process::id()));
  let _ = fs::remove_dir_all(&at);
  at
}

#[test]
fn each_block_of_a_result_is_kept_as_furb_keeps_it_and_their_texts_are_joined() {
  let got = json!({"content": [
    {"type": "text", "text": "a"},
    {"type": "resource_link", "uri": "u://n", "name": "n", "description": "d", "mimeType": "text/plain"},
    {"type": "audio", "data": "AAAA", "mimeType": "audio/wav"},
    {"type": "resource", "resource": {"uri": "r://bin", "mimeType": "application/octet-stream", "blob": STANDARD.encode([0, 1, 2])}},
    {"type": "resource", "resource": {"uri": "r://text", "mimeType": "text/plain", "text": "inside"}},
    {"type": "video"},
  ]});
  assert_eq!(
    called(&got, None).unwrap(),
    json!({
      "text": "a\n[the resource n at u://n]\n[audio of type audio/wav, which furb does not play]\n[the resource r://bin holds 3 bytes of application/octet-stream, which furb shows no further]\ninside\n[a block of the kind video, which furb does not show]",
      "content": [
        {"type": "text", "text": "a"},
        {"type": "resource_link", "uri": "u://n", "name": "n", "description": "d", "mime": "text/plain"},
        {"type": "audio", "mime": "audio/wav"},
        {"type": "resource", "uri": "r://bin", "mime": "application/octet-stream", "size": 3},
        {"type": "resource", "uri": "r://text", "mime": "text/plain", "text": "inside"},
        {"type": "video"},
      ],
      "structured": null,
    })
  );
}

#[test]
fn an_image_is_kept_in_the_directory_of_the_images_and_its_text_names_it() {
  let at = gallery("image");
  let data = STANDARD.encode(PICTURE);
  let got = json!({"contents": [{"uri": "r://picture", "mimeType": "image/png", "blob": data}]});
  let kept = read(&got, Some(&at));
  let uri = kept["content"][0]["image"].as_str().unwrap().to_owned();
  assert_eq!(
    kept,
    json!({"text": images::reference("image", &uri), "content": [{"type": "image", "image": uri, "mime": "image/png"}]})
  );
  assert_eq!(images::read(&at, &uri).unwrap(), ("image/png", PICTURE.to_vec()));
  let lost =
    called(&json!({"content": [{"type": "image", "data": data, "mimeType": "image/png"}]}), None)
      .unwrap();
  assert_eq!(lost["content"][0], json!({"type": "image", "image": null, "mime": "image/png"}));
  assert_eq!(
    lost["text"],
    "[an image of type image/png: this life keeps no directory of images to show it]"
  );
  let broken = called(
    &json!({"content": [{"type": "image", "data": "%%", "mimeType": "image/png"}]}),
    Some(&at),
  );
  assert_eq!(broken.unwrap()["text"], "[an image of type image/png: its data is no base64]");
}

#[test]
fn a_result_that_is_an_error_gives_its_text_and_one_of_structured_content_alone_gives_it_as_its_text()
 {
  assert_eq!(
    called(&json!({"content": [{"type": "text", "text": "no"}], "isError": true}), None)
      .unwrap_err(),
    "no"
  );
  assert_eq!(
    called(&json!({"content": [], "isError": true}), None).unwrap_err(),
    "the tool failed and said nothing"
  );
  assert_eq!(
    called(&json!({"structuredContent": {"n": 1}}), None).unwrap(),
    json!({"text": "{\"n\":1}", "content": [], "structured": {"n": 1}})
  );
}

#[test]
fn a_prompt_gives_each_message_with_its_role_and_their_texts_joined() {
  let got = json!({"messages": [
    {"role": "user", "content": {"type": "text", "text": "Hi."}},
    {"role": "assistant", "content": {"type": "text", "text": "Hello."}},
  ]});
  assert_eq!(
    prompted(&got, None),
    json!({"text": "Hi.\n\nHello.", "messages": [
      {"role": "user", "content": {"type": "text", "text": "Hi."}},
      {"role": "assistant", "content": {"type": "text", "text": "Hello."}},
    ]})
  );
}

#[test]
fn a_text_of_at_most_the_longest_stays_whole_and_a_longer_one_is_cut_in_characters() {
  let whole = "\u{e9}".repeat(LONGEST);
  assert_eq!(cut(&whole), whole);
  let longer = format!("{whole}\u{e9}");
  assert_eq!(
    cut(&longer),
    format!("{whole}\n\n[furb cut this text at {LONGEST} of its {} characters]", LONGEST + 1)
  );
}
