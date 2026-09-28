//! What an extension is and where a host finds it: the word of a file, a folder loaded under its name, and the
//! configs of the user and of the project. Then the ear of the extensions, in lives of the real engine on the
//! record that the store keeps.

use std::{
  fs,
  path::{Path, PathBuf},
};

use super::{Extension, Places, configured, extensions, load, word};
use crate::{
  Ear, Engine, Fact, Object,
  ear::{ear, hear, say},
  verbs, world,
};

/// A directory of its own for one test, and empty.
fn place(name: &str) -> PathBuf {
  let at = std::env::temp_dir().join(format!("furb-extension-{name}-{}", std::process::id()));
  let _ = fs::remove_dir_all(&at);
  fs::create_dir_all(&at).unwrap();
  at
}

/// A folder of an extension: its manifest and the file of its word.
fn folder(root: &Path, manifest: &str, word: &str) {
  fs::create_dir_all(root).unwrap();
  fs::write(root.join("furb.json"), manifest).unwrap();
  fs::write(root.join("note.py"), word).unwrap();
}

/// A config file, with the folders above it.
fn config(file: &Path, text: &str) {
  fs::create_dir_all(file.parent().unwrap()).unwrap();
  fs::write(file, text).unwrap();
}

fn note(word: &str) -> Extension {
  Extension { name: "note".to_owned(), word: word.to_owned(), life: "note()".to_owned() }
}

#[test]
fn the_word_of_a_file_is_the_file_less_its_imports_of_furb_and_its_runs_of_empty_lines() {
  let file = "from furb.engine import (\r\n  Text,\r\n  read,\r\n)\r\nfrom furb import engine\r\nimport furb\r\n\r\n\r\ndef note(on=''):\r\n  return read('n', on=on)\r\n\r\n\r\n\r\n\r\nclass Note:\r\n\r\n  x = 1\r\n\r\n";
  assert_eq!(word(file), "def note(on=''):\n  return read('n', on=on)\n\n\nclass Note:\n\n  x = 1");
  assert_eq!(word("import json\nfrom furb.engine import read\nx = 1\n"), "import json\nx = 1");
}

#[test]
fn a_folder_loads_under_the_name_its_manifest_gives_with_the_word_of_its_file() {
  let at = place("load");
  folder(&at.join("good"), r#"{"name": "note", "word": "note.py", "life": "note()"}"#, "x = 1\n");
  assert_eq!(load("note", &at.join("good")).unwrap(), note("x = 1"));
  folder(&at.join("bare"), r#"{"name": "note", "word": "note.py"}"#, "x = 1\n");
  assert_eq!(load("note", &at.join("bare")).unwrap().life, "");
  let refused = |root: &str, name: &str| load(name, &at.join(root)).unwrap_err().message();
  let manifest = |root: &str| at.join(root).join("furb.json").display().to_string();
  let named = "The manifest must give the name ";
  assert_eq!(
    refused("good", "notes"),
    format!("{named}notes, a letter then letters, digits and -: {}", manifest("good"))
  );
  assert!(refused("good", "Note").starts_with(named));
  folder(&at.join("wordless"), r#"{"name": "note", "word": "none.py"}"#, "");
  assert!(refused("wordless", "note").starts_with("none.py: "), "{}", refused("wordless", "note"));
  folder(&at.join("broken"), "{", "");
  assert!(refused("broken", "note").ends_with(&manifest("broken")));
  folder(&at.join("nameless"), r#"{"word": "note.py"}"#, "");
  assert_eq!(
    refused("nameless", "note"),
    format!("A manifest gives a name and a word: {}", manifest("nameless"))
  );
  assert!(refused("nowhere", "note").ends_with(&manifest("nowhere")));
}

#[test]
fn the_configs_turn_on_each_extension_they_give_a_folder_the_project_after_the_user() {
  let at = place("configs");
  let places = Places { config: at.join("user"), home: Some(at.join("home")) };
  let project = at.join("project");
  let names = |got: Vec<Extension>| got.into_iter().map(|one| one.name).collect::<Vec<_>>();
  assert_eq!(
    names(configured(&places, &project).unwrap()),
    ["memory", "skills"],
    "the official ones are on"
  );
  folder(&at.join("home").join("tilde"), r#"{"name": "tilde", "word": "note.py"}"#, "t = 1");
  folder(&at.join("user").join("near"), r#"{"name": "near", "word": "note.py"}"#, "n = 1");
  folder(&project.join("far"), r#"{"name": "far", "word": "note.py"}"#, "f = 1");
  config(
    &at.join("user").join("config.json"),
    r#"{"extensions": {"tilde": "~/tilde", "near": "near", "far": false, "skills": false}}"#,
  );
  assert_eq!(names(configured(&places, &project).unwrap()), ["memory", "tilde", "near"]);
  config(
    &project.join(".furb").join("config.json"),
    r#"{"extensions": {"near": false, "far": "../far", "tilde": true, "skills": true}}"#,
  );
  assert_eq!(
    names(configured(&places, &project).unwrap()),
    ["memory", "skills", "tilde", "far"],
    "each stands where a config first names it, after the official ones"
  );
}

#[test]
fn a_config_that_is_not_of_the_form_is_refused_with_its_path() {
  let at = place("refusals");
  let places = Places { config: at.clone(), home: None };
  let file = at.join("config.json");
  let read = |text: &str| {
    config(&file, text);
    configured(&places, &at.join("project"))
  };
  let refused = |text: &str| read(text).unwrap_err().message();
  let shown = file.display();
  assert_eq!(read(r#"{"theme": "dark"}"#).unwrap().len(), 2, "a config may name no extension");
  assert_eq!(refused("[]"), format!("A config is a map, whose extensions is a map: {shown}"));
  assert_eq!(
    refused(r#"{"extensions": []}"#),
    format!("A config is a map, whose extensions is a map: {shown}")
  );
  let form = "is true, false, or the path of its folder when it is not official";
  assert_eq!(
    refused(r#"{"extensions": {"note": 1}}"#),
    format!("The extension note {form}: {shown}")
  );
  assert_eq!(
    refused(r#"{"extensions": {"skills": "."}}"#),
    format!("The extension skills {form}: {shown}")
  );
  assert_eq!(
    refused(r#"{"extensions": {"note": true}}"#),
    format!("No config gives the folder of the extension note: {shown}")
  );
  assert!(refused("{").ends_with(&format!(": {shown}")));
}

/// The word of the extension of the tests, which reads a file of the directory of the chain.
const NOTE: &str = "def note(on=\"\"):\n  return read(\"note.txt\", on=on)";

/// The ear that answers a stand with the standing of the tests: the operator alone, in their directory.
fn standing(at: PathBuf) -> Box<dyn Ear> {
  ear(move |co, _| async move {
    loop {
      let a = hear(&co).await;
      if a.kind() == "stand" && a.question() {
        let operator = Object::list([Object::string("operator"), Object::list([]), Object::int(1)]);
        let here = Object::string(at.display().to_string());
        let standing = Object::list([Object::list([operator]), here, Object::string("operator")]);
        say(&co, Fact::says("done", a.about(), [standing])).await;
      }
    }
  })
}

/// A life in a directory of the tests, on the record that the store keeps there, given these extensions.
fn lived(at: &Path, given: Vec<Extension>) -> Engine {
  let (record, store) = world::store(at.join("record.jsonl")).unwrap();
  let ears = [
    ("extensions", extensions(given)),
    ("stand", standing(at.to_owned())),
    ("files", world::files()),
    ("store", store),
  ];
  let engine = Engine::boot(record, ears).unwrap();
  assert!(engine.raised().is_none(), "{:?}", engine.raised());
  engine
}

/// Each rung that the ear of the extensions made on these chains, and not on an origin of one: its name, its chain
/// and its word.
fn played(engine: &mut Engine, chains: &[&str]) -> Vec<(String, String, String)> {
  let mut out = Vec::new();
  for chain in chains {
    let on = verbs::Transcript { on: Some((*chain).to_owned()) };
    for one in engine.transcript(on).unwrap() {
      if one.kind() == "rung" && one.by() == "extensions" && one.on() == *chain {
        let word = one.word(1).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default();
        out.push((one.about().to_owned(), (*chain).to_owned(), word));
      }
    }
  }
  out
}

fn rung(id: &str, chain: &str, word: &str) -> (String, String, String) {
  (id.to_owned(), chain.to_owned(), word.to_owned())
}

#[test]
fn a_life_enables_at_its_tip_each_extension_it_is_given_and_plays_its_word_then_its_life_word() {
  let at = place("tip");
  fs::write(at.join("note.txt"), "noted\n").unwrap();
  let mut engine = lived(&at, vec![note(NOTE)]);
  let kept = world::kept(at.join("record.jsonl")).unwrap();
  let facts =
    kept.iter().map(|one| Fact::of(crate::value::entry(&one.as_ref(), 0).unwrap()).unwrap());
  let facts = facts.map(|one| (one.kind().to_owned(), one.by().to_owned())).collect::<Vec<_>>();
  let expected =
    [("chain", "operator"), ("stand", "operator"), ("done", "stand"), ("enable", "extensions")];
  assert_eq!(facts[..4], expected.map(|(kind, by)| (kind.to_owned(), by.to_owned())));
  let enable = Fact::of(crate::value::entry(&kept[3].as_ref(), 0).unwrap()).unwrap();
  let words: Vec<_> = (0..3).map(|at| enable.word(at).and_then(|one| one.as_str())).collect();
  assert_eq!(
    (enable.about(), words),
    ("chain1", vec![Some("note"), Some(NOTE), Some("note()")]),
    "the fact is about the root, and carries the name, the word and the life word"
  );
  assert_eq!(
    facts[4],
    ("rung".to_owned(), "extensions".to_owned()),
    "the rung stands after the fact"
  );
  let full = format!("{NOTE}\n\nnote()");
  assert_eq!(played(&mut engine, &["chain1"]), [rung("rung1", "chain1", &full)]);
  let turns = engine.turns(verbs::Turns { on: Some("chain1".to_owned()) }).unwrap();
  let told = turns.last().unwrap().as_ref().items().unwrap()[1].as_str().unwrap().to_owned();
  assert!(
    told.contains(&format!("#rung1\n<s:rung1_word>\n{full}</s:rung1_word>\n\n#read note.txt\n")),
    "{told}"
  );
}

#[test]
fn a_chain_takes_each_extension_at_its_birth_and_a_later_life_plays_the_same_rungs_again() {
  let at = place("later");
  let full = format!("{NOTE}\n\nnote()");
  {
    let mut engine = lived(&at, vec![note(NOTE)]);
    engine.chain(verbs::Chain { label: Some("two".to_owned()), ..Default::default() }).unwrap();
    let from = verbs::Chain { source: Some("chain1".to_owned()), ..Default::default() };
    engine.chain(from).unwrap();
    // The chain from a source made the rung of its origin again, so it takes the life word alone.
    let expected = [
      rung("rung1", "chain1", &full),
      rung("rung2", "chain2", &full),
      rung("rung4", "chain3", "note()"),
    ];
    assert_eq!(played(&mut engine, &["chain1", "chain2", "chain3"]), expected);
  }
  let other = Extension { name: "other".to_owned(), word: "o = 1".to_owned(), life: String::new() };
  let mut engine = lived(&at, vec![note("x = 0"), other]);
  let expected = [
    rung("rung1", "chain1", &full),
    rung("rung5", "chain1", "o = 1"),
    rung("rung2", "chain2", &full),
    rung("rung6", "chain2", "o = 1"),
    rung("rung4", "chain3", "note()"),
    rung("rung7", "chain3", "o = 1"),
  ];
  assert_eq!(
    played(&mut engine, &["chain1", "chain2", "chain3"]),
    expected,
    "the record wins for a name"
  );
}
