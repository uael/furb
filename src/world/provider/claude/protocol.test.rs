//! The events of the claude command line, read from the lines that a test gives.

use serde_json::json;

use super::{Event, Streamed};

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
