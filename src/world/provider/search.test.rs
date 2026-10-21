//! The native searches: what each API answers comes to an answer with its sources, and the claude command line
//! searches with its tool WebSearch alone.

#[cfg(unix)]
use std::{fs, path::Path, sync::Arc};

use serde_json::json;

#[cfg(unix)]
use super::claude;
use super::{anthropic, deepseek, gemini, responses, sourced};
#[cfg(unix)]
use crate::ear::reactor;

fn pair(title: &str, url: &str) -> (String, String) {
  (title.to_owned(), url.to_owned())
}

#[test]
fn an_answer_ends_with_its_sources_each_once_under_its_title_or_its_address() {
  let sources =
    [pair("One", "https://a"), pair("", "https://b"), pair("Again", "https://a"), pair("x", "")];
  assert_eq!(
    sourced(" Text.\n", &sources),
    "Text.\n\nSources:\n- [One](https://a)\n- [https://b](https://b)"
  );
  assert_eq!(sourced("Text.", &[]), "Text.", "an answer with no source is its text");
}

#[test]
fn the_messages_api_answers_with_its_texts_and_the_sources_they_cite_or_else_the_results_of_its_searches()
 {
  let result = json!({"type": "web_search_tool_result", "content": [{"type": "web_search_result", "url": "https://r", "title": "R"}]});
  let cited = json!({"content": [
    {"type": "server_tool_use", "name": "web_search"},
    result,
    {"type": "text", "text": "Rust is ", "citations": [{"type": "web_search_result_location", "url": "https://c", "title": "C"}]},
    {"type": "text", "text": "fast."},
  ]});
  assert_eq!(anthropic(&cited).unwrap(), "Rust is fast.\n\nSources:\n- [C](https://c)");
  let uncited = json!({"content": [result, {"type": "text", "text": "Rust is fast."}]});
  assert_eq!(anthropic(&uncited).unwrap(), "Rust is fast.\n\nSources:\n- [R](https://r)");
  assert_eq!(
    anthropic(&json!({"content": [result]})),
    None,
    "a response with no text answers nothing"
  );
}

#[test]
fn the_responses_api_answers_with_the_text_of_its_messages_and_the_sources_it_cites() {
  let got = json!({"output": [
    {"type": "web_search_call", "status": "completed"},
    {"type": "message", "content": [{"type": "output_text", "text": "Rust is fast.", "annotations": [
      {"type": "url_citation", "url": "https://c", "title": "C"},
      {"type": "file_citation", "file_id": "f"},
    ]}]},
  ]});
  assert_eq!(responses(&got).unwrap(), "Rust is fast.\n\nSources:\n- [C](https://c)");
  assert_eq!(responses(&json!({"output": []})), None);
}

#[test]
fn gemini_answers_with_the_text_of_its_first_candidate_and_the_pages_that_grounded_it() {
  let got = json!({"candidates": [{
    "content": {"parts": [{"text": "Rust "}, {"text": "is fast."}]},
    "groundingMetadata": {"groundingChunks": [{"web": {"uri": "https://g", "title": "g.dev"}}]},
  }]});
  assert_eq!(gemini(&got).unwrap(), "Rust is fast.\n\nSources:\n- [g.dev](https://g)");
  assert_eq!(gemini(&json!({"candidates": []})), None);
}

#[test]
fn deepseek_answers_with_each_result_its_title_its_address_and_its_summary() {
  let got = json!({"results": [
    {"title": "One", "url": "https://one", "summary": " First. "},
    {"title": "", "url": "https://two", "snippet": "Second."},
    {"title": "None", "url": ""},
  ]});
  assert_eq!(
    deepseek(&got).unwrap(),
    "1. [One](https://one)\n   First.\n2. [https://two](https://two)\n   Second."
  );
  assert_eq!(deepseek(&json!({"results": []})).unwrap(), "No result.");
}

/// A command line that writes its arguments and what its stdin holds, or that fails, in a folder of its own.
#[cfg(unix)]
fn program(name: &str, body: &str) -> Arc<Path> {
  let at = std::env::temp_dir().join(format!("furb-search-{name}-{}", std::process::id()));
  let _ = fs::remove_dir_all(&at);
  fs::create_dir_all(&at).unwrap();
  let bin = at.join("claude");
  fs::write(&bin, format!("#!/bin/sh\n{body}\n")).unwrap();
  use std::os::unix::fs::PermissionsExt;
  fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).unwrap();
  bin.into()
}

#[cfg(unix)]
#[test]
fn the_claude_command_line_searches_on_its_model_with_its_tool_web_search_alone_and_the_query_on_its_stdin()
 {
  let bin = program("echo", r#"echo "$@"; cat"#);
  let got = reactor().block_on(claude(bin, "haiku".to_owned())("what is rust".to_owned())).unwrap();
  let (args, query) = got.split_once('\n').unwrap();
  assert!(
    args.starts_with("-p --model haiku --output-format text --system-prompt Search the web"),
    "{args}"
  );
  assert!(args.ends_with("--allowedTools WebSearch --tools WebSearch"), "{args}");
  assert_eq!(query, "what is rust");
  let bin = program("fails", "echo out; echo 'no credit' >&2; exit 3");
  let no = reactor().block_on(claude(bin, "haiku".to_owned())("q".to_owned())).unwrap_err();
  assert_eq!(no, "claude exit status: 3: no credit");
}
