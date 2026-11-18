//! The claude command line as a model of rig, while its program stays missing past the wait of a
//! process, against a command line that answers from a script, which `fake-claude.sh` says. That
//! wait is a minute, and no door shortens it, so these tests shorten it with `Claude::waiting`.
//! The rest of the command line stands in the suite of the crate.

#![expect(clippy::expect_used, reason = "a test blocks on its runtime and fails by a panic")]

use std::{fs, os::unix::fs::PermissionsExt, time::Duration};

use rig_core::{
  completion,
  completion::{CompletionError, CompletionModel, CompletionRequest},
  message::{Message, Text, UserContent},
};
use serde_json::{Value, json};

use super::Completion;
use crate::testing::{Yard as Base, on_reactor, pause};

#[path = "../../../../tests/common/claude.rs"]
mod fake;

// The other tests of the provider beside their modules run the same fake.
pub(crate) use fake::{FAKE, Yard, after};

/// A message of the user.
fn user(text: &str) -> Message {
  Message::User { content: vec![UserContent::Text(Text::new(text))] }
}

/// One whole turn of a model.
fn asked(
  model: &Completion,
  messages: &[Message],
  settings: Value,
) -> Result<completion::CompletionResponse, CompletionError> {
  let request = CompletionRequest {
    model: None,
    preamble: Some("ENGINE ONLY".to_owned()),
    chat_history: messages.to_vec(),
    documents: Vec::new(),
    tools: Vec::new(),
    temperature: None,
    max_tokens: None,
    tool_choice: None,
    additional_params: Some(settings),
    output_schema: None,
    record_telemetry_content: false,
  };
  on_reactor(model.completion(request))
}

#[test]
fn a_turn_whose_program_stays_away_past_its_wait_names_the_program_that_it_did_not_find() {
  let yard = Yard::new("update");
  let settings = json!({"session": "chain"});
  let model = yard.claude(10_000).waiting(Duration::ZERO).completion_model("sonnet");
  yard.away();
  let no = asked(&model, &[user("first")], settings).unwrap_err();
  let bin = yard.at.join("claude");
  let said = format!("{} did not start: No such file or directory (os error 2)", bin.display());
  assert!(matches!(super::error(&no), Some(super::Error::Missing(why)) if *why == said), "{no}");
}

#[test]
fn a_message_that_the_command_line_did_not_hear_goes_whole_at_the_next_turn() {
  // The turn after the one that did not go gives the message grown again, or the same message
  // whole.
  let cases = [
    ("unheard", "second FAIL\n\nthird\n\nfourth", "third\n\nfourth"),
    ("unsent", "second FAIL\n\nthird", "third"),
  ];
  for (name, last, sent) in cases {
    let yard = Yard::new(name);
    let model = yard.claude(10_000).waiting(Duration::ZERO).completion_model("sonnet");
    let settings = json!({"session": "chain"});
    let one = asked(&model, &[user("first")], settings.clone()).unwrap();
    let answer = Message::Assistant { id: None, content: one.choice };
    let turn =
      |last: &str| asked(&model, &[user("first"), answer.clone(), user(last)], settings.clone());
    turn("second FAIL").unwrap_err();
    let bin = yard.at.join("claude");
    fs::remove_file(&bin).unwrap();
    turn("second FAIL\n\nthird").unwrap_err();
    fs::write(&bin, FAKE).unwrap();
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).unwrap();
    turn(last).unwrap();
    let pids = yard.pids();
    assert_eq!(pids.len(), 2, "{name}");
    let first = after(&yard.args(&pids[0]), "--session-id");
    assert_eq!(after(&yard.args(&pids[1]), "--resume"), first, "the same conversation goes on");
    let line: Value = serde_json::from_str(yard.heard().last().unwrap()).unwrap();
    assert_eq!(line["message"]["content"], json!([{"type": "text", "text": sent}]), "{name}");
  }
}
