use crate::testing::on_reactor;
use serde_json::json;

use super::super::catalog;
use std::{
  io::{BufRead, BufReader, Write},
  net::TcpListener,
  thread,
};

use futures::StreamExt;
use rig_core::http_client::{
  Request,
  sse::{Event, GenericEventSource},
};

use super::{Amended, CodexHttp};

#[test]
fn subscription_events_stream_when_the_response_has_no_content_type() {
  let listener = TcpListener::bind("127.0.0.1:0").expect("a local port");
  let address = listener.local_addr().expect("address");
  let server = thread::spawn(move || {
    let (mut stream, _) = listener.accept().expect("client");
    let mut reader = BufReader::new(&mut stream);
    let mut head = String::new();
    while !head.ends_with("\r\n\r\n") {
      reader.read_line(&mut head).expect("request header");
    }
    let body = "data: close(3)\n\n";
    write!(
      stream,
      "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
      body.len()
    )
    .expect("events");
  });
  let request =
    Request::get(format!("http://{address}/responses")).body(Vec::<u8>::new()).expect("request");
  let text = on_reactor(async {
    let mut source = GenericEventSource::new(Amended::new(CodexHttp::default()), request);
    loop {
      match source.next().await.expect("an event").expect("valid subscription event") {
        Event::Open => {}
        Event::Message(message) => break message.data,
      }
    }
  });
  assert_eq!(text, "close(3)");
  server.join().expect("server");
}

#[test]
fn settings_of_codex_that_do_not_hold_fail_with_their_own_cause() {
  let zero = super::offered(None, &json!({"model_context_window": 0})).err();
  assert!(matches!(zero, Some(catalog::Error::CodexWindow)), "{zero:?}");
  let unknown = super::offered(None, &json!({"window": 1})).err();
  assert!(matches!(unknown, Some(catalog::Error::CodexSettings(_))), "{unknown:?}");
}
