//! The events of the claude command line, read from the lines that a test gives.

use serde_json::json;

use super::{Control, Event, Streamed};

#[test]
fn a_delta_that_lacks_the_text_it_gives_is_refused() {
  let line = json!({"type": "stream_event", "event": {
    "type": "content_block_delta", "index": 0, "delta": {"type": "text_delta"}}});
  assert!(Event::read(line).is_err());
}

#[test]
fn an_event_of_a_kind_that_is_not_read_is_ignored() {
  let system = Event::read(json!({"type": "system", "subtype": "init"}));
  assert!(matches!(system, Ok(Event::Other)));
  let typeless = Event::read(json!({"session_id": "a"}));
  assert!(matches!(typeless, Ok(Event::Other)));
  let ping = Event::read(json!({"type": "stream_event", "event": {"type": "ping"}}));
  assert!(matches!(ping, Ok(Event::Stream { event: Streamed::Other })));
}

#[test]
fn a_control_response_reads_as_the_id_of_its_request_and_its_answer_or_its_failure() {
  let read = |subtype: &str| {
    Event::read(json!({"type": "control_response", "response": {
      "subtype": subtype, "request_id": "furb-1", "response": {"maxTokens": 9}, "error": "no"}}))
  };
  let Ok(Event::ControlResponse { response: Control::Success { request_id, response } }) =
    read("success")
  else {
    panic!("a success reads as one");
  };
  assert_eq!((request_id.as_str(), response), ("furb-1", json!({"maxTokens": 9})));
  let failed = read("error");
  assert!(matches!(
    failed,
    Ok(Event::ControlResponse { response: Control::Error { request_id } }) if request_id == "furb-1"
  ));
}
