//! The native search of a model of the network: a search of an actor asks the API of its provider
//! at the address that the config of the project names, and answers with the text of the response
//! and the sources that it cites. Each test runs again as a child of itself, whose credentials
//! offer each provider, and whose model stands in the snapshot with its native search, so no test
//! falls back to the free search on the network. The free search asks an address that no config
//! names, so its answers stand beside the module, in `src/world/provider/search.test.rs`.

#![expect(clippy::expect_used, reason = "a helper of a test fails the test where it fails")]

use std::{fs, path::Path, time::Duration};

use serde_json::{Value, json};

use super::{
  catalog::{body, opened, served},
  offered::{cached, listing, run, say},
  provider::closed,
};
use crate::{
  child::{child, here},
  drive::park,
};
use furb::{
  Text,
  life::Opening,
  world::{Catalog, Provider},
};

/// A response of a provider of the network with this body of JSON.
fn ok(body: &Value) -> String {
  let body = body.to_string();
  format!(
    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: \
     close\r\n\r\n{body}",
    body.len()
  )
}

/// What a search for `rust` comes to for each actor in turn, whose provider answers with the body
/// beside it, on a port of its own that the local config of the project names; and the request
/// that each provider read. An answer is its text, and a search that raised is why.
fn searched(test: &str, rows: &[(&str, Value)], vars: &[&str]) -> (Vec<String>, Vec<Value>) {
  let served: Vec<_> = rows.iter().map(|(_, answer)| served(vec![ok(answer)])).collect();
  let ports: Vec<u16> = served.iter().map(|(port, _)| *port).collect();
  let actors: Vec<&str> = rows.iter().map(|(actor, _)| *actor).collect();
  let at = cached(&test.replace("::", "-"), &listing());
  fs::write(at.join("asks.json"), json!({"ports": ports, "actors": actors}).to_string())
    .expect("the ports");
  fs::create_dir_all(at.join(".furb")).expect("the folder of the project");
  let at = run(test, at, vars);
  let said = fs::read_to_string(at.join("said")).expect("the child said");
  let said: Vec<String> = serde_json::from_str(&said).expect("the child said JSON");
  // The child ended, so a provider that was asked has answered, and one that was not never will.
  for ((_, asked), actor) in served.iter().zip(&actors) {
    let answered = (0..500).any(|_| {
      asked.is_finished() || {
        park(Duration::from_millis(10));
        false
      }
    });
    assert!(answered, "{actor} was not asked at the address of its config: {said:?}");
  }
  let asked = served.into_iter().map(|(_, asked)| asked.join().expect("the provider read"));
  (said, asked.map(|one| body(&one[0])).collect())
}

/// A search for `rust` of each actor that the test gave, each in a life of its own on the yard of
/// the child, whose local config names the next port as the address of the provider of the actor.
fn searches(at: &Path) {
  let given: Value =
    serde_json::from_str(&fs::read_to_string(at.join("asks.json")).expect("the actors"))
      .expect("the actors are JSON");
  let actors = given["actors"].as_array().into_iter().flatten().filter_map(Value::as_str);
  let (catalog, mut said) = (Catalog::load(None), Vec::new());
  for (actor, port) in actors.zip(given["ports"].as_array().into_iter().flatten()) {
    let provider = actor.split(':').next().unwrap_or_default();
    let url = json!({"url": format!("http://127.0.0.1:{port}/v1")});
    let config = json!({"providers": {provider: url}}).to_string();
    fs::write(at.join(".furb/config.local.json"), config).expect("the local config");
    let model = catalog.find(actor).unwrap_or_else(|| panic!("{actor} stands in the catalog"));
    let native = Provider::new(String::new(), vec![model]).search()(actor);
    assert!(native.is_some(), "{actor} has a native search");
    let opening = Opening { actor: Some(actor.to_owned()), ..opened(at) };
    let mut engine = opening.boot::<String>([]).expect("the life opens").0;
    let word = format!("close(await act('search', '', None, 'rust', '{actor}'))");
    said.push(match closed(&mut engine, &word) {
      Ok(got) => Text::of(got.as_ref()).expect("a search answers a text").content,
      Err(why) => why,
    });
  }
  say(at, &json!(said));
}

const ANTHROPIC: &str = "anthropic:claude-haiku-4-5";

#[test]
fn the_messages_api_answers_with_its_texts_and_the_sources_they_cite_or_else_the_results_of_its_searches()
 {
  if let Some(at) = child() {
    return searches(&at);
  }
  let result = json!({"type": "web_search_tool_result", "content": [{"type": "web_search_result", "url": "https://r", "title": "R"}]});
  let cite = |url: &str, title: &str| json!({"type": "web_search_result_location", "url": url, "title": title});
  let cited = json!({"content": [
    {"type": "server_tool_use", "name": "web_search"},
    result,
    {"type": "text", "text": "Rust is ", "citations": [cite("https://c", "C"), cite("https://b", "")]},
    {"type": "text", "text": "fast.\n", "citations": [cite("https://c", "Again"), cite("", "x")]},
  ]});
  let uncited = json!({"content": [result, {"type": "text", "text": "Rust is fast."}]});
  let rows = [(ANTHROPIC, cited), (ANTHROPIC, uncited), (ANTHROPIC, json!({"content": [result]}))];
  let (said, asked) = searched(here!(), &rows, &["ANTHROPIC_API_KEY"]);
  assert_eq!(said[0], "Rust is fast.\n\nSources:\n- [C](https://c)\n- [https://b](https://b)");
  assert_eq!(said[1], "Rust is fast.\n\nSources:\n- [R](https://r)");
  assert!(said[2].contains("the search answered with no text"), "{}", said[2]);
  let tools = json!([{"type": "web_search_20250305", "name": "web_search", "max_uses": 5}]);
  assert_eq!((&asked[0]["model"], &asked[0]["tools"]), (&json!("claude-haiku-4-5"), &tools));
  assert_eq!(asked[0]["messages"], json!([{"role": "user", "content": "rust"}]));
}

#[test]
fn a_search_of_the_messages_api_that_failed_or_a_block_it_does_not_read_leaves_the_text_alone() {
  if let Some(at) = child() {
    return searches(&at);
  }
  let failed = json!({"type": "web_search_tool_result", "content": {"type": "web_search_tool_result_error", "error_code": "too_many_requests"}});
  let thought = json!({"type": "server_tool_use", "id": "s", "name": "web_search", "input": {}});
  let untitled = json!({"type": "text", "text": "cited", "citations": [{"type": "web_search_result_location", "url": "https://u", "title": null}]});
  let rows = [
    (ANTHROPIC, json!({"content": [thought, failed, {"type": "text", "text": "an answer"}]})),
    (ANTHROPIC, json!({"content": [untitled]})),
    (ANTHROPIC, json!({"content": "no list"})),
  ];
  let (said, _) = searched(here!(), &rows, &["ANTHROPIC_API_KEY"]);
  assert_eq!(said[0], "an answer");
  assert_eq!(said[1], "cited\n\nSources:\n- [https://u](https://u)", "a null title is no title");
  assert!(said[2].contains("the search answered with no text"), "{}", said[2]);
}

#[test]
fn the_responses_api_answers_with_the_text_of_its_messages_and_the_sources_it_cites() {
  if let Some(at) = child() {
    return searches(&at);
  }
  let got = json!({"output": [
    {"type": "web_search_call", "status": "completed"},
    {"type": "message", "content": [{"type": "output_text", "text": "Rust is fast.", "annotations": [
      {"type": "url_citation", "url": "https://c", "title": "C"},
      {"type": "file_citation", "file_id": "f"},
    ]}]},
  ]});
  let rows = [("openai:gpt-5", got), ("openai:gpt-5", json!({"output": []}))];
  let (said, asked) = searched(here!(), &rows, &["OPENAI_API_KEY"]);
  assert_eq!(said[0], "Rust is fast.\n\nSources:\n- [C](https://c)");
  assert!(said[1].contains("the search answered with no text"), "{}", said[1]);
  assert_eq!(asked[0]["tools"], json!([{"type": "web_search"}]));
}

#[test]
fn deepseek_answers_with_each_result_its_title_its_address_and_its_summary() {
  if let Some(at) = child() {
    return searches(&at);
  }
  let got = json!({"results": [
    {"title": "One", "url": "https://one", "summary": " First. "},
    {"title": "", "url": "https://two", "snippet": "Second."},
    {"title": "None", "url": ""},
  ]});
  let rows =
    [("deepseek:deepseek-flash", got), ("deepseek:deepseek-flash", json!({"results": []}))];
  let (said, asked) = searched(here!(), &rows, &["DEEPSEEK_API_KEY"]);
  assert_eq!(
    said[0],
    "1. [One](https://one)\n   First.\n2. [https://two](https://two)\n   Second."
  );
  assert_eq!(said[1], "No result.");
  assert_eq!(asked[0], json!({"query": "rust", "count": 10, "need_content": false}));
}

#[test]
fn gemini_answers_with_the_text_of_its_first_candidate_and_the_pages_that_grounded_it() {
  if let Some(at) = child() {
    return searches(&at);
  }
  let got = json!({"candidates": [{
    "content": {"parts": [{"text": "Rust "}, {"text": "is fast."}]},
    "groundingMetadata": {"groundingChunks": [{"web": {"uri": "https://g", "title": "g.dev"}}]},
  }]});
  let rows = [("google:gemini-flash", got), ("google:gemini-flash", json!({"candidates": []}))];
  let (said, asked) = searched(here!(), &rows, &["GEMINI_API_KEY"]);
  assert_eq!(said[0], "Rust is fast.\n\nSources:\n- [g.dev](https://g)");
  assert!(said[1].contains("the search answered with no text"), "{}", said[1]);
  assert_eq!(asked[0]["tools"], json!([{"google_search": {}}]));
}
