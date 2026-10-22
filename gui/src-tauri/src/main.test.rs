use serde_json::json;

use super::{KEPT, Seen, kept, life::Event};

/// An open life that the window heard whole.
fn open() -> Seen {
  let mut seen = Seen { open: true, ..Seen::default() };
  let _ = seen.frame();
  seen
}

/// The facts of a small record: the root, a prompt of the operator, and its answer.
fn facts() -> Event {
  Event::Facts(vec![
    json!(["chain", "chain1", "operator", "", "root", ""]),
    json!(["prompt", "prompt1", "operator", "chain1", "str", "Do it", ""]),
    json!(["done", "prompt1", "rung1", "fine"]),
  ])
}

#[test]
fn a_frame_with_no_change_is_nothing() {
  let mut seen = open();
  assert_eq!(seen.frame(), None);
}

#[test]
fn a_frame_sends_the_acts_that_changed_once() {
  let mut seen = open();
  seen.heard(facts());
  let part = seen.frame().expect("new acts are a change");
  let ids: Vec<_> =
    part["acts"].as_array().into_iter().flatten().map(|one| one["id"].clone()).collect();
  assert_eq!(ids, vec![json!("chain1"), json!("prompt1")]);
  assert_eq!((part["whole"].clone(), part["read"].clone()), (json!(false), json!(false)));
  assert_eq!(seen.frame(), None);
}

#[test]
fn a_frame_says_once_that_the_app_read_the_whole_record() {
  let mut seen = open();
  seen.heard(facts());
  let _ = seen.frame();
  seen.heard(Event::Read);
  let part = seen.frame().expect("the end of the reading is a change");
  assert_eq!((part["read"].clone(), part["acts"].clone()), (json!(true), json!([])));
  assert_eq!(seen.frame(), None);
}

#[test]
fn a_frame_says_the_lines_of_the_app_only_when_they_changed_and_keeps_the_last() {
  let mut seen = open();
  for n in 0..60 {
    seen.heard(Event::Said(format!("line {n}")));
  }
  let part = seen.frame().expect("a line is a change");
  let said = part["said"].as_array().expect("the lines");
  assert_eq!((said.len(), said[0].clone()), (50, json!("line 10")));
  seen.heard(facts());
  assert!(seen.frame().expect("new acts are a change").get("said").is_none());
}

#[test]
fn the_record_whole_holds_every_act_and_where_it_stands() {
  let mut seen = open();
  seen.heard(Event::Opened {
    root: "chain1".to_owned(),
    directory: "/a/project".to_owned(),
    works: false,
    logged: false,
    made: Some("e1".to_owned()),
  });
  seen.heard(facts());
  let whole = seen.life(0);
  assert_eq!(whole["whole"], json!(true));
  assert_eq!(whole["acts"].as_array().map(Vec::len), Some(2));
  assert_eq!((whole["place"].clone(), whole["made"].clone()), (json!("project"), json!("e1")));
}

#[test]
fn a_frame_sends_each_piece_of_a_stream_once_where_it_starts_in_units_of_javascript() {
  let mut seen = open();
  seen.heard(Event::Writing { rung: "rung3".to_owned(), text: "h\u{e9}llo ".to_owned() });
  let first = seen.frame().expect("a piece is a change");
  assert_eq!(first["streams"], json!({"rung3": [0, "h\u{e9}llo "]}));
  assert_eq!(first["whole"], json!(false));
  seen.heard(Event::Writing { rung: "rung3".to_owned(), text: "\u{1f600}!".to_owned() });
  assert_eq!(
    seen.frame().expect("a piece is a change")["streams"],
    json!({"rung3": [6, "\u{1f600}!"]})
  );
  assert_eq!(seen.frame(), None);
}

#[test]
fn a_frame_says_once_that_a_stream_ended() {
  let mut seen = open();
  seen.heard(Event::Writing { rung: "rung3".to_owned(), text: "x".to_owned() });
  let _ = seen.frame();
  seen.heard(Event::Facts(vec![json!(["ready", "rung3", "chain1", "x"])]));
  assert_eq!(seen.frame().expect("an end is a change")["ended"], json!(["rung3"]));
  assert_eq!(seen.frame(), None);
}

#[test]
fn the_life_whole_holds_the_end_of_each_stream() {
  let mut seen = open();
  let text = "\u{1f600}".repeat(KEPT);
  seen.heard(Event::Writing { rung: "rung3".to_owned(), text: text.clone() });
  let whole = seen.life(0);
  assert_eq!(whole["whole"], json!(true));
  assert_eq!(whole["streams"]["rung3"], kept(&text));
  let [from, end] = [&kept(&text)[0], &kept(&text)[1]];
  assert_eq!(from, &json!(KEPT));
  assert_eq!(end.as_str().map(|one| one.encode_utf16().count()), Some(KEPT));
}

/// One event again, as the life told it.
fn again(event: &Event) -> Event {
  match event {
    Event::Opened { root, directory, works, logged, made } => Event::Opened {
      root: root.clone(),
      directory: directory.clone(),
      works: *works,
      logged: *logged,
      made: made.clone(),
    },
    Event::Facts(facts) => Event::Facts(facts.clone()),
    Event::Writing { rung, text } => Event::Writing { rung: rung.clone(), text: text.clone() },
    Event::Read => Event::Read,
    Event::Said(line) => Event::Said(line.clone()),
  }
}

/// The part that the app sent the window before it sent only what changed: the life whole after a count, with all
/// that the models wrote.
fn whole_part(seen: &mut Seen) -> serde_json::Value {
  let mut part = seen.life(seen.sent);
  let streams: serde_json::Map<String, serde_json::Value> = seen
    .writing
    .iter()
    .map(|(rung, held)| (rung.clone(), json!({"text": held.text, "thinking": ""})))
    .collect();
  part["streams"] = serde_json::Value::Object(streams);
  seen.sent = seen.now.count;
  part
}

#[test]
#[ignore = "a measure of the parts that the window takes, on the record that FURB_REPLAY names"]
fn replay() {
  let Ok(path) = std::env::var("FURB_REPLAY") else { return };
  let (_life, events) = crate::life::Life::open(crate::life::Source::Record(path.into()));
  let mut told = vec![];
  while let Ok(event) = events.recv_timeout(std::time::Duration::from_secs(10)) {
    told.push(event);
  }
  let (mut parts, mut wholes) = (Seen::default(), Seen::default());
  let mut facts = vec![];
  for event in told {
    match event {
      Event::Facts(more) => facts.extend(more),
      other => {
        parts.heard(again(&other));
        wholes.heard(other);
      }
    }
  }
  let _ = (parts.frame(), whole_part(&mut wholes));
  let (mut sent, mut before, mut frames) = (0, 0, 0);
  for fact in facts {
    let writing = Event::Writing {
      rung: "rung999".to_owned(),
      text: "a model writes forty characters a frame ".to_owned(),
    };
    for one in [Event::Facts(vec![fact]), writing] {
      parts.heard(again(&one));
      wholes.heard(one);
    }
    sent += parts.frame().map_or(0, |part| part.to_string().len());
    before += whole_part(&mut wholes).to_string().len();
    frames += 1;
  }
  let seconds = f64::from(frames) * 0.08;
  println!(
    "replay: {frames} frames, {seconds:.0} s at a frame each 80 ms; the whole life each frame: {} MB, {:.0} kB/s; what changed: {} MB, {:.1} kB/s",
    before / 1_000_000,
    before as f64 / seconds / 1000.0,
    sent / 1_000_000,
    sent as f64 / seconds / 1000.0
  );
}
