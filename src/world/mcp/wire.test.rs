//! The reader of a stream of events, and the words of a failure.

use std::time::Duration;

#[cfg(unix)]
use serde_json::Value;

use super::{Events, Failure, seconds};
#[cfg(unix)]
use super::{Heard, Peer};
#[cfg(unix)]
use crate::{ear::reactor, world::mcp::config::Transport};
#[cfg(unix)]
use futures::executor;

#[test]
fn the_reader_of_a_stream_of_events_gives_each_event_whole_as_its_bytes_come() {
  let mut events = Events::default();
  assert!(events.feed(b": a comment\r\nevent: endpoint\r\ndata: /mess").is_empty());
  assert_eq!(events.feed(b"ages\r\n\r\n"), [("endpoint".to_owned(), "/messages".to_owned())]);
  let snow = "data: {\"a\": \"\u{2744}\"}\n\n".as_bytes();
  assert!(events.feed(&snow[..12]).is_empty(), "a sign split across two chunks waits for its end");
  assert_eq!(
    events.feed(&snow[12..]),
    [("message".to_owned(), "{\"a\": \"\u{2744}\"}".to_owned())]
  );
  assert_eq!(
    events.feed(b"data: one\ndata:two\nid: 3\n\n"),
    [("message".to_owned(), "one\ntwo".to_owned())]
  );
  assert!(events.feed(b"event: empty\n\n").is_empty(), "an event with no data is no event");
  assert_eq!(
    events.feed(b"data: after\n\n"),
    [("message".to_owned(), "after".to_owned())],
    "the name goes with its event"
  );
}

#[test]
fn a_failure_says_why_in_words() {
  assert_eq!(
    Failure::Answered { code: -32601, message: "no method".to_owned() }.said(),
    "no method (error -32601)"
  );
  assert_eq!(Failure::Unauthorized(None).said(), "the server asks for a login");
  assert_eq!(Failure::Late(Duration::from_secs(60)).said(), "it took more than 60 seconds");
  assert_eq!(Failure::Broken("gone".to_owned()).said(), "gone");
  assert_eq!(seconds(Duration::from_millis(1500)), "1.5");
}

#[cfg(unix)]
#[test]
#[expect(clippy::disallowed_methods, reason = "a test waits for the reactor from its own thread")]
fn the_end_of_a_process_quotes_the_whole_of_its_stderr() {
  let transport = Transport::Stdio {
    command: "sh".to_owned(),
    args: ["-c", "echo first >&2; echo last >&2; exit 3"].map(str::to_owned).to_vec(),
    env: Vec::new(),
    cwd: None,
  };
  let heard = executor::block_on(reactor().spawn(async move {
    let (_peer, mut heard) = Peer::open(&transport, None, Value::Null).await.unwrap();
    heard.recv().await
  }))
  .unwrap();
  let Some(Heard::Closed(why)) = heard else { panic!("the wire did not close: {heard:?}") };
  assert!(why.ends_with(": first\nlast"), "{why}");
}
