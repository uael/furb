use serde_json::json;

use super::{Seen, record::Event};

/// An open record that the window heard whole.
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
    directory: "/a/project".to_owned(),
    logged: false,
    made: Some("e1".to_owned()),
  });
  seen.heard(facts());
  let whole = seen.life(0);
  assert_eq!(whole["whole"], json!(true));
  assert_eq!(whole["acts"].as_array().map(Vec::len), Some(2));
  assert_eq!((whole["place"].clone(), whole["made"].clone()), (json!("project"), json!("e1")));
}
