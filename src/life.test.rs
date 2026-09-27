//! The opening of a life: what it enables, what it keeps, and the ears it hears, on the record that the store keeps.

use std::{
  fs,
  path::{Path, PathBuf},
};

use super::Opening;
use crate::{
  Engine, Fact, Object,
  extension::{Places, enabled},
  value::entry,
  verbs, world,
};

/// A directory of its own for one test, and empty.
fn place(name: &str) -> PathBuf {
  let at = std::env::temp_dir().join(format!("furb-life-{name}-{}", std::process::id()));
  let _ = fs::remove_dir_all(&at);
  fs::create_dir_all(&at).unwrap();
  at
}

/// A life in a directory of the tests, on the configs of that directory, with the provider of no model.
fn opened(at: &Path, keeps: bool) -> (Engine, Vec<Object>) {
  let places = Places { config: at.join("config"), home: None };
  let opening = Opening::new().places(places).record(at.join("record.jsonl"), keeps);
  let provider = world::Provider::new(at.display().to_string(), vec![]).ear();
  opening.configured(at).unwrap().boot([("provider", provider)]).unwrap()
}

/// The names of the extensions that a life runs.
fn names(engine: &mut Engine) -> Vec<String> {
  let root = engine.transcript(verbs::Transcript { on: Some("chain1".to_owned()) }).unwrap();
  enabled(&root).into_iter().map(|one| one.name).collect()
}

/// The kind of each fact of a record.
fn kinds(record: &[Object]) -> Vec<String> {
  let fact = |one: &Object| entry(&one.as_ref(), 0).and_then(Fact::of).unwrap().kind().to_owned();
  record.iter().map(fact).collect()
}

#[test]
fn a_life_enables_what_the_configs_turn_on_and_a_later_life_runs_what_its_record_enables() {
  let at = place("configs");
  let (mut first, _) = opened(&at, true);
  assert_eq!(names(&mut first), ["memory", "skills"]);
  drop(first);
  fs::create_dir_all(at.join("config")).unwrap();
  fs::write(at.join("config").join("config.json"), r#"{"extensions": {"skills": false}}"#).unwrap();
  let (mut later, record) = opened(&at, true);
  assert_eq!(names(&mut later), ["memory", "skills"], "the record wins over the configs");
  drop(later);
  let kept = world::kept(at.join("record.jsonl")).unwrap();
  assert!(!kinds(&kept[record.len()..]).contains(&"enable".to_owned()), "it enabled nothing new");
}

#[test]
fn a_life_that_inspects_its_record_keeps_nothing_and_its_ears_that_do_work_are_silent() {
  let at = place("inspects");
  fs::write(at.join("a.txt"), "one\n").unwrap();
  drop(opened(&at, true));
  let before = fs::read_to_string(at.join("record.jsonl")).unwrap();
  let places = Places { config: at.join("config"), home: None };
  let opening = Opening::new().places(places).record(at.join("record.jsonl"), true).inspecting();
  let provider = world::Provider::new(at.display().to_string(), vec![]).ear();
  let (mut engine, _) = opening.boot([("provider", provider)]).unwrap();
  let read = verbs::Read { on: Some("chain1".to_owned()), ..Default::default() };
  assert_eq!(engine.read("a.txt", read).unwrap_err().message(), "nothing takes read");
  assert_eq!(names(&mut engine), ["memory", "skills"], "it runs what its record enables");
  drop(engine);
  assert_eq!(fs::read_to_string(at.join("record.jsonl")).unwrap(), before);
}

#[test]
fn the_ears_of_the_crate_come_after_the_ears_of_the_host_each_under_the_name_every_host_hears_it_by()
 {
  let at = place("ears");
  let (_, ears) = Opening::new().record(at.join("record.jsonl"), true).parts().unwrap();
  let names: Vec<&str> = ears.iter().map(|(name, _)| name.as_str()).collect();
  assert_eq!(names, ["extensions", "memory", "skills", "files", "bash", "time", "store"]);
}
