use std::path::PathBuf;

use super::{Found, Place, Seen, answered, indexed};
use crate::skill::Places;

#[test]
fn the_index_of_the_skills_reads_back_as_the_name_of_each_skill_and_what_it_is_for() {
  let plain = "---\nname: plain\ndescription: Plans: one step, then the next\n---\n";
  let coded = "---\nname: coded\ndescription: Writes code\nmetadata:\n  furb-word: coded.py\n  \
               furb-default: \"true\"\n---\n";
  let found = [(plain, "plain"), (coded, "coded")]
    .map(|(text, folder)| Found::of(text, folder, Place::Shipped(Vec::new())));
  let places = Places { config: PathBuf::new(), home: None };
  let seen = Seen {
    chain: "chain1".to_owned(),
    found: found.into(),
    runs: Vec::new(),
    places,
    project: PathBuf::new(),
  };
  let index = answered(&seen, "").expect("the index of the skills").content;
  let each: Vec<(&str, &str)> = indexed(&index).collect();
  assert_eq!(each, [("plain", "Plans: one step, then the next"), ("coded", "Writes code")]);
}
