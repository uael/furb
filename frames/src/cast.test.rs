use super::*;

#[test]
fn a_recording_reads_back_as_it_was_written() {
  let at = Duration::from_millis;
  let cast = Cast {
    cols: 3,
    rows: 2,
    events: vec![
      Event { at: at(10), what: What::Output("a\u{1b}[1mb".to_owned()) },
      Event { at: at(250), what: What::Marker("a".to_owned()) },
      Event { at: at(300), what: What::Resize { cols: 4, rows: 2 } },
    ],
  };
  let text = cast.write();
  assert_eq!(
    text,
    "{\"version\":3,\"term\":{\"cols\":3,\"rows\":2}}\n[0.01,\"o\",\"a\\u001b[1mb\"]\n[0.24,\"m\",\"a\"]\n[0.05,\"r\",\"4x2\"]\n"
  );
  assert_eq!(Cast::read(&text).unwrap(), cast);
}

#[test]
fn a_recording_of_another_version_is_refused() {
  let refused = Cast::read("{\"version\":2,\"term\":{\"cols\":3,\"rows\":2}}\n").unwrap_err();
  assert_eq!(refused.to_string(), "the recording is not valid: its version is not 3");
}

#[test]
fn an_event_that_no_frame_shows_is_kept_as_other() {
  let cast =
    Cast::read("{\"version\":3,\"term\":{\"cols\":3,\"rows\":2}}\n[0.5,\"i\",\"q\"]\n").unwrap();
  assert_eq!(cast.events, vec![Event { at: Duration::from_millis(500), what: What::Other }]);
}
