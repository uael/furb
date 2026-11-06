use furb::view::{Act, Kind, Phase, State};
use serde_json::{Value, json};

use super::Watch;
use crate::event::Event;

fn act(kind: &str, id: &str, by: &str, words: &[Value], phase: Phase) -> Event {
  Event::Act(Box::new(Act {
    kind: Kind::from(kind),
    id: id.to_owned(),
    by: by.to_owned(),
    on: "chain1".to_owned(),
    chain: "chain1".to_owned(),
    words: words.to_vec(),
    phase,
    operator: false,
    asking: false,
    pause: None,
    run: None,
    word: None,
    began: 0,
    ended: None,
    spend: None,
    state: State::Idle,
  }))
}

/// The stand of a life whose one extension says the life word `seed()`.
fn stood() -> Event {
  Event::Fact(json!([
    "done",
    "stand1",
    "provider",
    [[], "", "", "", [["k", "f", "seed()", false]]]
  ]))
}

#[test]
fn the_rungs_of_completed_words_are_not_kept_and_the_rungs_of_life_words_go_with_their_birth() {
  let mut watch = Watch::default();
  watch.heard(&stood());
  for n in 0..50 {
    let id = format!("rung{n}");
    watch.heard(&act("rung", &id, "chain1", &[json!(format!("x = {n}"))], Phase::Started));
    watch.heard(&act(
      "rung",
      &id,
      "chain1",
      &[json!(format!("x = {n}"))],
      Phase::Done(Value::Null),
    ));
  }
  watch.heard(&act("rung", "rung90", "chain1", &[json!("seed()")], Phase::Started));
  watch.heard(&act("lasting", "lasting1", "rung90", &[], Phase::Started));
  watch.heard(&act("rung", "rung90", "chain1", &[json!("seed()")], Phase::Done(Value::Null)));
  watch.heard(&act("lasting", "lasting1", "rung90", &[], Phase::Started));
  assert!(watch.held.is_empty(), "an act of a life word holds no rebirth, though its rung ended");
  assert_eq!(watch.seeds.len(), 1, "only the rung of a life word is kept: {:?}", watch.seeds);
  watch.heard(&act("lasting", "other1", "rung3", &[], Phase::Started));
  assert_eq!(watch.held.len(), 1, "an act of another word holds the rebirth");
  watch.reborn();
  assert!(watch.seeds.is_empty(), "a birth keeps no rung of the birth before it");
  assert_eq!(watch.lives.len(), 1, "the life words stand until the next stand");
}
