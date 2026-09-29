//! The opening of a life: what it enables, what it keeps, and the ears it hears, on the record that the store keeps.

use std::{
  fs,
  path::{Path, PathBuf},
};

use super::Opening;
use crate::{
  Ear, Engine, Fact, Object, Text,
  ear::{ear, hear, say},
  extension::enabled,
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

/// A life in a directory of the tests, on its record and on the configs of that directory, whose provider offers
/// the operator alone.
fn opening(at: &Path, keeps: bool) -> Opening {
  let place = |name: &str| Some(at.join(name).display().to_string());
  Opening {
    directory: at.display().to_string(),
    record: place("record.jsonl"),
    keeps: Some(keeps),
    config: place("config"),
    roster: Some(Vec::new()),
    ..Opening::default()
  }
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
  let (mut first, _) = opening(&at, true).boot::<String>([]).unwrap();
  assert_eq!(names(&mut first), ["memory", "skills"]);
  drop(first);
  fs::create_dir_all(at.join("config")).unwrap();
  fs::write(at.join("config").join("config.json"), r#"{"extensions": {"skills": false}}"#).unwrap();
  let (mut later, record) = opening(&at, true).boot::<String>([]).unwrap();
  assert_eq!(names(&mut later), ["memory", "skills"], "the record wins over the configs");
  drop(later);
  let kept = world::kept(at.join("record.jsonl")).unwrap();
  assert!(!kinds(&kept[record.len()..]).contains(&"enable".to_owned()), "it enabled nothing new");
  let fresh = Some(at.join("fresh.jsonl").display().to_string());
  let (mut fresh, _) = Opening { record: fresh, ..opening(&at, true) }.boot::<String>([]).unwrap();
  assert_eq!(
    names(&mut fresh),
    ["memory"],
    "a life on a new record enables what the configs turn on"
  );
}

#[test]
fn a_life_that_inspects_its_record_keeps_nothing_enables_nothing_new_and_its_ears_that_do_work_are_silent()
 {
  let at = place("inspects");
  fs::write(at.join("a.txt"), "one\n").unwrap();
  drop(Opening { extensions: Some(false), ..opening(&at, true) }.boot::<String>([]).unwrap());
  let before = fs::read_to_string(at.join("record.jsonl")).unwrap();
  let (mut engine, _) =
    Opening { inspecting: Some(true), ..opening(&at, true) }.boot::<String>([]).unwrap();
  let read = verbs::Read { on: Some("chain1".to_owned()), ..Default::default() };
  assert_eq!(engine.read("a.txt", read).unwrap_err().message(), "nothing takes read");
  assert_eq!(names(&mut engine), Vec::<String>::new(), "it enables nothing the record does not");
  drop(engine);
  assert_eq!(fs::read_to_string(at.join("record.jsonl")).unwrap(), before);
}

#[test]
fn the_ears_of_the_crate_come_after_the_ears_of_the_host_each_under_the_name_every_host_hears_it_by()
 {
  let at = place("ears");
  let (_, ears) = opening(&at, true).parts().unwrap();
  let names: Vec<&str> = ears.iter().map(|(name, _)| name.as_str()).collect();
  assert_eq!(
    names,
    ["provider", "extensions", "memory", "skills", "files", "bash", "time", "store"]
  );
  drop(ears);
  let (mut engine, _) = opening(&at, true).boot([("mine", mine())]).unwrap();
  let read = verbs::Read { on: Some(engine.root().to_owned()), ..Default::default() };
  assert_eq!(engine.read("mine://a", read).unwrap(), Text::new("mine://a", "mine\n"));
}

/// An ear of the host that takes a read of a path of its own, which the files refuse.
fn mine() -> Box<dyn Ear> {
  ear(|mut co| async move {
    loop {
      let a = hear(&mut co).await;
      let path = a.word(1).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default();
      if a.question() && a.kind() == "read" && path.starts_with("mine://") {
        say(&mut co, Fact::says("done", a.about(), [Text::new(path, "mine\n").object()])).await;
      }
    }
  })
}
