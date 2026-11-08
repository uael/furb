#![expect(
  clippy::too_many_arguments,
  reason = "debt: a function of more than 4 parameters takes a struct, rule 8 of the skill rust"
)]
use furb::view::{Act, Kind, Phase, State};
use serde_json::{Value, json};

use super::{TAIL, Watch};
use crate::event::Event;

fn act(kind: &str, id: &str, by: &str, words: &[Value], phase: Phase) -> Event {
  Event::Act(Box::new(Act {
    kind: Kind::from(kind),
    id: id.to_owned(),
    by: by.to_owned(),
    maker: furb::view::Role::Engine,
    on: "chain1".to_owned(),
    chain: "chain1".to_owned(),
    words: words.to_vec(),
    phase,
    operator: false,
    asking: false,
    taken: Vec::new(),
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

/// A watch that heard the facts of a whole tail, so that only an act that a checkpoint does not
/// keep holds its rebirth.
fn aged(watch: &mut Watch) {
  for n in 0..TAIL {
    watch.heard(&Event::Fact(json!(["say", format!("x{n}"), "", []])));
  }
}

#[test]
fn the_rungs_of_completed_words_hold_no_rebirth_and_the_rungs_of_life_words_go_with_their_birth() {
  let mut watch = Watch::default();
  watch.heard(&stood());
  aged(&mut watch);
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
  assert!(watch.due(), "the rungs of completed words hold no rebirth");
  watch.heard(&act("rung", "rung90", "chain1", &[json!("seed()")], Phase::Started));
  watch.heard(&act("lasting", "lasting1", "rung90", &[], Phase::Started));
  watch.heard(&act("rung", "rung90", "chain1", &[json!("seed()")], Phase::Done(Value::Null)));
  watch.heard(&act("lasting", "lasting1", "rung90", &[], Phase::Started));
  assert!(watch.due(), "an act of a life word holds no rebirth, though its rung ended");
  watch.heard(&act("lasting", "other1", "rung3", &[], Phase::Started));
  assert!(!watch.due(), "an act of another word holds the rebirth");
  watch.heard(&act("lasting", "other1", "rung3", &[], Phase::Done(Value::Null)));
  assert!(watch.due(), "the rebirth is due once that act ends");
  watch.reborn();
  aged(&mut watch);
  watch.heard(&act("lasting", "again1", "rung90", &[], Phase::Started));
  assert!(
    !watch.due(),
    "a birth keeps no rung of the birth before it, so its act holds the rebirth"
  );
  watch.heard(&act("lasting", "again1", "rung90", &[], Phase::Done(Value::Null)));
  watch.heard(&act("rung", "rung91", "chain1", &[json!("seed()")], Phase::Started));
  watch.heard(&act("lasting", "life1", "rung91", &[], Phase::Started));
  assert!(watch.due(), "the life words stand until the next stand");
}
