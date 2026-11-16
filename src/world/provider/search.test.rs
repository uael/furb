//! The native searches whose address no config names, so no door reaches them: the generation of
//! Gemini grounded in Google Search, and the free search. The suite proves the other APIs and the
//! claude command line through a search of a life.

use serde_json::json;

use super::{found, gemini};

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
fn the_free_search_answers_each_result_under_its_title_and_its_address_with_its_excerpts_cut() {
  let long = "x".repeat(2000);
  let listed = json!({"results": [
    {"url": "https://one", "title": "One", "excerpts": ["First.", "Second."]},
    {"url": "https://two", "title": "", "excerpts": [long]},
  ]});
  let reply = json!({"jsonrpc": "2.0", "id": 1, "result": {"content": [{"type": "text", "text": listed.to_string()}]}});
  let got = found(&reply.to_string()).unwrap();
  let cut = format!("## [https://two](https://two)\n\n{}…", "x".repeat(1500));
  assert_eq!(got, format!("## [One](https://one)\n\nFirst.\n\nSecond.\n\n{cut}"));
  let streamed = format!("event: message\ndata: {reply}\n\n");
  assert_eq!(
    found(&streamed).unwrap(),
    got,
    "a stream of events carries the same answer in its data"
  );
  let none = json!({"result": {"content": [{"type": "text", "text": "{\"results\": []}"}]}});
  assert_eq!(found(&none.to_string()).unwrap(), "No result.");
}

#[test]
fn the_free_search_refuses_with_the_error_of_its_server_and_gives_a_text_that_is_no_list_as_it_is()
{
  let error =
    json!({"jsonrpc": "2.0", "id": 1, "error": {"code": -32000, "message": "Too many requests"}});
  assert_eq!(found(&error.to_string()).unwrap_err().to_string(), "Too many requests");
  let failed =
    json!({"result": {"isError": true, "content": [{"type": "text", "text": "No query."}]}});
  assert_eq!(found(&failed.to_string()).unwrap_err().to_string(), "No query.");
  let plain = json!({"result": {"content": [{"type": "text", "text": "Plain."}]}});
  assert_eq!(found(&plain.to_string()).unwrap(), "Plain.");
  assert!(found("<html>").unwrap_err().to_string().starts_with("the free search answered no JSON"));
}
