//! The skills of a life, through the index that the World tells and that a window reads back.

use std::{fs, process};

use serde_json::json;

use super::provider::closed;
use crate::yard::Yard;
use furb::{
  Ear, Engine, Object, Text,
  view::{Fold, Held},
  world,
};

#[test]
fn the_index_of_the_skills_reads_back_as_the_name_of_each_skill_and_what_it_is_for() {
  let at = Yard::new(format!("furb-skills-index-{}", process::id()));
  let skill = |name: &str, head: &str| {
    let folder = at.join(".claude").join("skills").join(name);
    fs::create_dir_all(&folder).expect("the folder of the skill");
    fs::write(folder.join("SKILL.md"), format!("---\n{head}---\n")).expect("the skill");
  };
  skill("plain", "name: plain\ndescription: Plans: one step, then the next\n");
  skill("coded", "name: coded\ndescription: Writes code\nmetadata:\n  furb-word: coded.py\n");
  fs::write(at.join(".claude/skills/coded/coded.py"), "").expect("the word of the skill");
  let scratch = world::Scratch::at(at.join("scratch")).expect("the scratch directory");
  // The skills hear a read of their scheme before the files, which refuse any path they lack.
  let ears: [(&str, Box<dyn Ear>); 2] = [
    ("skills", world::skills::skills(at.join("config"))),
    ("files", world::files(scratch, at.join("config"), None)),
  ];
  let mut engine = Engine::boot(Vec::<Object>::new(), ears).expect("the life opens");
  let folder = serde_json::to_string(&at.display().to_string()).expect("a path");
  closed(&mut engine, &format!("cd({folder})\nclose(0)")).expect("the chain moves");
  let index = closed(&mut engine, "close(read('skills://'))").expect("the index of the skills");
  let index = Text::of(index.as_ref()).expect("a text").content;
  let facts = [
    json!(["chain", "chain1", "operator", "", "root", ""]),
    json!(["read", "read1", "rung1", "chain1", "skills://"]),
    json!(["done", "read1", "skills", {"is": "Text", "path": "skills://", "content": index}]),
  ];
  let (mut fold, mut held) = (Fold::default(), Held::default());
  for fact in facts {
    for act in fold.recorded(&fact) {
      held.heard(&act);
    }
  }
  let ours: Vec<(String, String)> = held
    .skills()
    .into_iter()
    .filter(|one| ["plain", "coded"].contains(&one.name.as_str()))
    .map(|one| (one.name, one.description))
    .collect();
  let want = [("coded", "Writes code"), ("plain", "Plans: one step, then the next")];
  assert_eq!(ours, want.map(|(name, what)| (name.to_owned(), what.to_owned())), "{index}");
}
