#![expect(
  clippy::too_many_arguments,
  reason = "debt: a function of more than 4 parameters takes a struct, rule 8 of the skill rust"
)]
use std::sync::Arc;

use furb::view::{Act, Kind, Maker, Phase, State};
use serde_json::{Value, json};

use super::{Asks, PATIENCE, TAIL, Watch};
use crate::event::Event;

fn act(kind: &str, id: &str, by: &str, words: &[Value], phase: Phase) -> Event {
  Event::Act(Arc::new(Act {
    kind: Kind::from(kind),
    id: id.to_owned(),
    by: by.to_owned(),
    maker: Maker::Engine,
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
    closer: None,
    state: State::Idle,
  }))
}

/// The stand of a life whose one skill says the life word `seed()`.
fn stood() -> Event {
  Event::Fact(json!([
    "done",
    "stand1",
    "provider",
    [[], "", "", "", [["k", "f", "seed()", false]]]
  ]))
}

/// A watch that heard the facts of a whole tail.
fn aged(watch: &mut Watch) {
  for n in 0..TAIL {
    watch.heard(&Event::Fact(json!(["say", format!("x{n}"), "", []])));
  }
}

#[test]
fn a_life_writes_its_image_once_a_tail_of_its_facts_stands_in_none_and_is_not_born_again() {
  let mut watch = Watch::default();
  assert!(!watch.images(TAIL - 1), "a life on no image writes none before its log told a tail");
  assert!(watch.images(TAIL), "and writes one once its log told a tail");
  watch.imaged();
  watch.heard(&act("lasting", "other1", "rung3", &[], Phase::Started));
  aged(&mut watch);
  let (images, due) = (watch.images(0), watch.due());
  assert_eq!(
    (images, due),
    (true, false),
    "no act in flight holds it, and no count asks a rebirth"
  );
  watch.imaged();
  assert!(!watch.images(u64::MAX), "the next image waits for the next tail of its own facts");
  aged(&mut watch);
  assert!(watch.images(0), "and comes past it");
  watch.reborn();
  assert!(watch.images(TAIL), "a rebirth stands on no image");
}

#[test]
fn the_rungs_of_completed_words_hold_no_rebirth_and_the_rungs_of_life_words_go_with_their_birth() {
  let mut watch = Watch::default();
  watch.heard(&stood());
  watch.ask(true);
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
  watch.ask(true);
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

/// Three lives, each with its name, its weight and the count of its births.
fn lives(a: (u64, u64), b: (u64, u64), c: (u64, u64)) -> Vec<(String, u64, u64)> {
  [("a", a), ("b", b), ("c", c)]
    .map(|(name, (weight, births))| (name.to_owned(), weight, births))
    .to_vec()
}

#[test]
fn the_daemon_asks_one_life_at_a_time_and_the_next_once_it_is_born_again() {
  let mut asks = Asks::default();
  assert_eq!(asks.next(&lives((9, 0), (5, 0), (0, 0))), Some("a".to_owned()));
  assert_eq!(asks.next(&lives((9, 0), (5, 0), (0, 0))), None, "a is not born again yet");
  let after = asks.next(&lives((1, 1), (5, 0), (0, 0)));
  assert_eq!(after, Some("b".to_owned()), "a is born again and weighs little now");
  assert_eq!(asks.next(&lives((1, 1), (6, 0), (0, 0))), None, "b is not born again yet");
  let grown = asks.next(&lives((8, 1), (1, 1), (0, 0)));
  assert_eq!(grown, Some("a".to_owned()), "a grew again after its birth, so it is asked again");
  let mut spared = asks.spared();
  spared.sort();
  assert_eq!(spared, ["a", "b"]);
  assert_eq!(asks.next(&lives((12, 1), (6, 1), (0, 0))), Some("a".to_owned()), "a fresh share");
}

#[test]
fn a_life_that_is_not_born_again_holds_the_asks_no_more_than_its_patience() {
  let mut asks = Asks::default();
  let stuck = lives((9, 0), (5, 0), (1, 0));
  assert_eq!(asks.next(&stuck), Some("a".to_owned()));
  for _ in 0..PATIENCE {
    assert_eq!(asks.next(&stuck), None);
  }
  assert_eq!(asks.next(&stuck), Some("b".to_owned()), "past its patience, the next life");
  for _ in 0..PATIENCE {
    assert_eq!(asks.next(&stuck), None);
  }
  assert_eq!(asks.next(&stuck), Some("c".to_owned()));
  assert_eq!(asks.next(&lives((9, 0), (5, 0), (1, 0))), None, "c is not born again yet");
}

#[test]
fn a_life_that_ended_holds_no_ask() {
  let mut asks = Asks::default();
  assert_eq!(asks.next(&lives((9, 0), (5, 0), (0, 0))), Some("a".to_owned()));
  assert_eq!(asks.next(&[]), None, "no life lives");
  let back = vec![("a".to_owned(), 9, 0)];
  assert_eq!(asks.next(&back), Some("a".to_owned()), "a life that opens again is a new life");
}
