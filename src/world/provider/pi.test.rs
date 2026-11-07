//! The stream of pi-ai, read from chunks that a test gives, so no test asks the network.

use crate::testing::block_on;
use std::collections::VecDeque;

use bytes::Bytes;
use rig_core::streaming::RawStreamingChoice;

use super::{Heard, Pi};
use futures::stream;
use rig_core::completion::{
  CompletionRequest,
  message::{ImageMediaType, Message, UserContent},
};

#[test]
fn a_letter_that_two_chunks_of_the_stream_split_is_read_whole() {
  let events = "data: {\"type\":\"text_delta\",\"delta\":\"a\"}\r\n\r\n\
                data: {\"type\":\"text_delta\",\"delta\":\"\u{e9}\"}\r\n\r\n\
                data: {\"type\":\"done\",\"usage\":{}}\r\n\r\n";
  let cut = events.find('\u{e9}').expect("the letter") + 1;
  let chunks = [&events.as_bytes()[..cut], &events.as_bytes()[cut..]];
  let body = stream::iter(chunks.map(|one| Ok(Bytes::copy_from_slice(one))));
  let (buffer, parts) = (Vec::new(), VecDeque::new());
  let mut heard = Heard { body: Box::pin(body), buffer, parts, over: false };
  let mut said = Vec::new();
  while let Some(part) = block_on(heard.next()) {
    if let Ok(RawStreamingChoice::Message(text)) = part {
      said.push(text);
    }
  }
  assert_eq!(said, ["a", "\u{e9}"]);
}
#[test]
fn an_event_that_lacks_a_field_its_protocol_gives_fails_the_reply() {
  let events = b"data: {\"type\":\"text_delta\"}\n\n";
  let body = stream::iter([events.as_slice()].map(|one| Ok(Bytes::copy_from_slice(one))));
  let (buffer, parts) = (Vec::new(), VecDeque::new());
  let mut heard = Heard { body: Box::pin(body), buffer, parts, over: false };
  assert!(matches!(block_on(heard.next()), Some(Err(_))));
  assert!(block_on(heard.next()).is_none());
}

#[test]
fn a_message_with_an_image_in_the_middle_goes_to_pi_ai_as_text_image_text() {
  let content = vec![
    UserContent::text("before "),
    UserContent::image_base64("aGk=", Some(ImageMediaType::PNG), None),
    UserContent::text(" after"),
  ];
  let request = CompletionRequest {
    model: None,
    preamble: None,
    chat_history: vec![Message::User { content }],
    documents: Vec::new(),
    tools: Vec::new(),
    temperature: None,
    max_tokens: None,
    tool_choice: None,
    additional_params: None,
    output_schema: None,
    record_telemetry_content: false,
  };
  let pi = Pi::new("http://pi.test".to_owned(), "key".to_owned(), "m".to_owned());
  let sent = pi.context(&request);
  let parts = sent["messages"][0]["content"].as_array().expect("the parts of the message");
  let kinds: Vec<&str> = parts.iter().filter_map(|part| part["type"].as_str()).collect();
  assert_eq!(kinds, ["text", "image", "text"]);
  assert_eq!((&parts[0]["text"], &parts[1]["data"]), (&"before ".into(), &"aGk=".into()));
}
