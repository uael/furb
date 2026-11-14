use serde_json::json;

use super::{TAIL, Watch};
use crate::event::Event;

/// A watch that heard the facts of a whole tail.
fn aged(watch: &mut Watch) {
  for n in 0..TAIL {
    watch.heard(&Event::Fact(json!(["say", format!("x{n}"), "", []])));
  }
}

#[test]
fn a_life_writes_its_image_once_a_tail_of_its_facts_stands_in_none() {
  let mut watch = Watch::default();
  assert!(!watch.images(TAIL - 1), "a life on no image writes none before its log told a tail");
  assert!(watch.images(TAIL), "and writes one once its log told a tail");
  watch.imaged();
  assert!(!watch.images(u64::MAX), "the next image waits for the next tail of its own facts");
  aged(&mut watch);
  assert!(watch.images(0), "and comes past it");
  watch.reborn();
  assert!(watch.images(TAIL), "a life that told the past of its record stands on no image");
}
