//! The answers of a server as the acts give them: the blocks of a result, each as furb keeps it, and their texts
//! joined. An image goes to the directory of the images of the life, as the images of a turn do, and its text names
//! it, so a model that reads the text sees it. A text past [`LONGEST`] characters is cut, and says so.

use std::path::Path;

use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};

use crate::world::images;

/// The most characters of a text that an answer holds.
pub const LONGEST: usize = 100_000;

/// What a call of a tool gives: its text, its blocks and its structured content; or the text of a result that is an
/// error.
pub fn called(got: &Value, gallery: Option<&Path>) -> Result<Value, String> {
  let (mut text, content) = blocks(listed(&got["content"]), gallery);
  let structured = got.get("structuredContent").cloned().unwrap_or(Value::Null);
  if got["isError"] == true {
    return Err(if text.is_empty() { "the tool failed and said nothing".to_owned() } else { text });
  }
  if content.is_empty() && !structured.is_null() {
    text = cut(&structured.to_string());
  }
  Ok(json!({"text": text, "content": content, "structured": structured}))
}

/// What a read of a resource gives: its text and its blocks.
pub fn read(got: &Value, gallery: Option<&Path>) -> Value {
  let (mut texts, mut content) = (Vec::new(), Vec::new());
  for one in listed(&got["contents"]) {
    let (text, block) = embedded(one, gallery);
    texts.push(text);
    content.push(block);
  }
  json!({"text": cut(&texts.join("\n")), "content": content})
}

/// What a prompt of a server gives: its messages, each with its role and its block, and their texts joined.
pub fn prompted(got: &Value, gallery: Option<&Path>) -> Value {
  let (mut texts, mut messages) = (Vec::new(), Vec::new());
  for one in listed(&got["messages"]) {
    let (text, mut content) = blocks(std::slice::from_ref(&one["content"]), gallery);
    texts.push(text);
    let role = one["role"].as_str().unwrap_or("user");
    messages.push(json!({"role": role, "content": content.pop().unwrap_or(Value::Null)}));
  }
  json!({"text": cut(&texts.join("\n\n")), "messages": messages})
}

/// The items of a list of JSON, or none.
fn listed(list: &Value) -> &[Value] {
  list.as_array().map(Vec::as_slice).unwrap_or_default()
}

/// The text of blocks, joined and then cut, and each block as furb keeps it, its own text cut.
fn blocks(list: &[Value], gallery: Option<&Path>) -> (String, Vec<Value>) {
  let (mut texts, mut content) = (Vec::new(), Vec::new());
  for block in list {
    let kind = block["type"].as_str().unwrap_or_default();
    let text = |key: &str| block[key].as_str().unwrap_or_default().to_owned();
    let (said, kept) = match kind {
      "text" => {
        let said = text("text");
        let kept = json!({"type": "text", "text": cut(&said)});
        (said, kept)
      }
      "image" => image(&text("data"), &text("mimeType"), gallery),
      "audio" => {
        let mime = text("mimeType");
        (
          format!("[audio of type {mime}, which furb does not play]"),
          json!({"type": "audio", "mime": mime}),
        )
      }
      "resource_link" => {
        let (uri, name) = (text("uri"), text("name"));
        let kept = json!({"type": "resource_link", "uri": uri, "name": name, "description": text("description"), "mime": text("mimeType")});
        (format!("[the resource {name} at {uri}]"), kept)
      }
      "resource" => embedded(&block["resource"], gallery),
      other => {
        (format!("[a block of the kind {other}, which furb does not show]"), json!({"type": other}))
      }
    };
    texts.push(said);
    content.push(kept);
  }
  (cut(&texts.join("\n")), content)
}

/// The text of a resource that a server gives whole, and the block of it, its text cut: its text, or the image it
/// holds, or what it holds when it is other bytes.
fn embedded(resource: &Value, gallery: Option<&Path>) -> (String, Value) {
  let (uri, mime) = (
    resource["uri"].as_str().unwrap_or_default(),
    resource["mimeType"].as_str().unwrap_or_default(),
  );
  if let Some(text) = resource["text"].as_str() {
    return (
      text.to_owned(),
      json!({"type": "resource", "uri": uri, "mime": mime, "text": cut(text)}),
    );
  }
  let blob = resource["blob"].as_str().unwrap_or_default();
  let bytes = STANDARD.decode(blob).unwrap_or_default();
  if images::kind(&bytes).is_ok() {
    return image(blob, mime, gallery);
  }
  let said = format!(
    "[the resource {uri} holds {} bytes of {mime}, which furb shows no further]",
    bytes.len()
  );
  (said, json!({"type": "resource", "uri": uri, "mime": mime, "size": bytes.len()}))
}

/// The text of an image of a server, which names the image as the directory of the images keeps it, and its block.
fn image(data: &str, mime: &str, gallery: Option<&Path>) -> (String, Value) {
  let lost = |why: String| {
    (
      format!("[an image of type {mime}: {why}]"),
      json!({"type": "image", "image": null, "mime": mime}),
    )
  };
  let Ok(bytes) = STANDARD.decode(data.trim()) else {
    return lost("its data is no base64".to_owned());
  };
  let Some(gallery) = gallery else {
    return lost("this life keeps no directory of images to show it".to_owned());
  };
  match images::keep(gallery, &bytes) {
    Ok((uri, media)) => {
      (images::reference("image", &uri), json!({"type": "image", "image": uri, "mime": media}))
    }
    Err(why) => lost(why),
  }
}

/// A text cut at [`LONGEST`] characters, which then says so.
pub fn cut(text: &str) -> String {
  let size = text.chars().count();
  if size <= LONGEST {
    return text.to_owned();
  }
  let kept: String = text.chars().take(LONGEST).collect();
  format!("{kept}\n\n[furb cut this text at {LONGEST} of its {size} characters]")
}

#[cfg(test)]
#[path = "content.test.rs"]
mod test;
