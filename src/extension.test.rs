//! What an extension is and where a host finds it: the word of a file, a folder loaded under its name, and the
//! configs of the user and of the project. Then the ear of the extensions, in lives of the real engine on the
//! record that the store keeps.

use std::{
  fs,
  path::{Path, PathBuf},
};

use super::{
  Extension, Part, Places, configured, definitions, enabled, extensions, load, parts, reloads, word,
};
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
  Extension {
    name: "note".to_owned(),
    word: word.to_owned(),
    life: "note()".to_owned(),
    root: false,
    file: String::new(),
  }
}

/// The extension that a folder loads, with the file of its word in that folder.
fn filed(one: Extension, root: &Path) -> Extension {
  Extension {
    file: crate::world::files::tilded(&root.join("note.py"), std::env::home_dir().as_deref()),
    ..one
  }
}

#[test]
fn the_word_of_a_file_is_the_file_less_its_imports_of_furb_and_its_runs_of_empty_lines() {
  let file = "from furb.engine import (\r\n  Text,\r\n  read,\r\n)\r\nfrom furb import engine\r\nimport furb\r\n\r\n\r\ndef note(on=''):\r\n  return read('n', on=on)\r\n\r\n\r\n\r\n\r\nclass Note:\r\n\r\n  x = 1\r\n\r\n";
  assert_eq!(word(file), "def note(on=''):\n  return read('n', on=on)\n\n\nclass Note:\n\n  x = 1");
  assert_eq!(word("import json\nfrom furb.engine import read\nx = 1\n"), "import json\nx = 1");
  let others = "import furbish\nfrom furbish import x\nimport furb.engine\nfrom furb import (\n  read,\n)\ny = x";
  assert_eq!(
    word(others),
    "import furbish\nfrom furbish import x\ny = x",
    "a module of another name stays"
  );
}

#[test]
fn a_folder_loads_under_the_name_its_manifest_gives_with_the_word_of_its_file() {
  let at = place("load");
  folder(&at.join("good"), r#"{"name": "note", "word": "note.py", "life": "note()"}"#, "x = 1\n");
  assert_eq!(load("note", &at.join("good")).unwrap(), filed(note("x = 1"), &at.join("good")));
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
    ["catchup", "memory", "skills", "compaction", "ouroboros", "web", "mcp"],
    "the official ones are on"
  );
  folder(&at.join("home").join("tilde"), r#"{"name": "tilde", "word": "note.py"}"#, "t = 1");
  folder(&at.join("user").join("near"), r#"{"name": "near", "word": "note.py"}"#, "n = 1");
  folder(&project.join("far"), r#"{"name": "far", "word": "note.py"}"#, "f = 1");
  config(
    &at.join("user").join("config.json"),
    r#"{"extensions": {"tilde": "~/tilde", "near": "near", "far": false, "skills": false}}"#,
  );
  assert_eq!(
    names(configured(&places, &project).unwrap()),
    ["catchup", "memory", "compaction", "ouroboros", "web", "mcp", "tilde", "near"]
  );
  config(
    &project.join(".furb").join("config.json"),
    r#"{"extensions": {"near": false, "far": "../far", "tilde": true, "skills": true}}"#,
  );
  assert_eq!(
    names(configured(&places, &project).unwrap()),
    ["catchup", "memory", "skills", "compaction", "ouroboros", "web", "mcp", "tilde", "far"],
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
  assert_eq!(read(r#"{"theme": "dark"}"#).unwrap().len(), 7, "a config may name no extension");
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

#[test]
fn a_manifest_names_the_part_of_an_extension_for_a_tui_as_text() {
  let at = place("tui");
  folder(&at.join("good"), r#"{"name": "note", "word": "note.py", "tui": "tui.ts"}"#, "x = 1\n");
  assert_eq!(
    load("note", &at.join("good")).unwrap(),
    filed(Extension { life: String::new(), ..note("x = 1") }, &at.join("good"))
  );
  folder(&at.join("bad"), r#"{"name": "note", "word": "note.py", "tui": 1}"#, "x = 1\n");
  let manifest = at.join("bad").join("furb.json").display().to_string();
  assert_eq!(
    load("note", &at.join("bad")).unwrap_err().message(),
    format!("The tui of the manifest is no text: {manifest}")
  );
}

#[test]
fn the_parts_for_a_tui_are_those_of_the_extensions_that_the_configs_turn_on_and_that_name_one() {
  let at = place("parts");
  let places = Places { config: at.join("user"), home: None };
  let project = at.join("project");
  let compaction = Part {
    name: "compaction".to_owned(),
    file: "extensions/compaction/tui.ts".to_owned(),
    text: Some(include_str!("../extensions/compaction/tui.ts").to_owned()),
  };
  let mcp = Part {
    name: "mcp".to_owned(),
    file: "extensions/mcp/tui.ts".to_owned(),
    text: Some(include_str!("../extensions/mcp/tui.ts").to_owned()),
  };
  let alone = parts(&places, &project).unwrap();
  assert_eq!(
    alone,
    [compaction.clone(), mcp.clone()],
    "the crate carries the part of an official extension"
  );
  let todo = project.join(".furb").join("extensions").join("todo");
  folder(&todo, r#"{"name": "todo", "word": "note.py", "tui": "tui.ts"}"#, "t = 1");
  folder(&project.join("plain"), r#"{"name": "plain", "word": "note.py"}"#, "p = 1");
  folder(&project.join("off"), r#"{"name": "off", "word": "note.py", "tui": "off.ts"}"#, "o = 1");
  config(
    &project.join(".furb").join("config.json"),
    r#"{"extensions": {"todo": "extensions/todo", "plain": "../plain", "off": false}}"#,
  );
  let got = parts(&places, &project).unwrap();
  let todo =
    Part { name: "todo".to_owned(), file: todo.join("tui.ts").display().to_string(), text: None };
  assert_eq!(got, [compaction, mcp, todo]);
  config(
    &project.join(".furb").join("config.json"),
    r#"{"extensions": {"compaction": false, "mcp": false}}"#,
  );
  assert_eq!(
    parts(&places, &project).unwrap(),
    [],
    "a config turns off the part of an official extension"
  );
}

/// The word of the extension of the tests, which reads a file of the directory of the chain.
const NOTE: &str = "def note(on=\"\"):\n  return read(\"note.txt\", on=on)";

/// The ear that answers a stand with the standing of the tests: the operator alone, in their directory.
fn standing(at: PathBuf) -> Box<dyn Ear> {
  ear(move |mut co| async move {
    loop {
      let a = hear(&mut co).await;
      if a.kind() == "stand" && a.question() {
        let operator = Object::list([Object::string("operator"), Object::list([]), Object::int(1)]);
        let here = Object::string(at.display().to_string());
        let standing = Object::list([Object::list([operator]), here, Object::string("operator")]);
        say(&mut co, Fact::says("done", a.about(), [standing]));
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
    ("files", world::files(world::Scratch::own(), None)),
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
    let on = verbs::Transcript { on: Some((*chain).to_owned()), of: None };
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
fn a_chain_that_a_word_makes_takes_each_extension_before_what_the_word_says_on_it() {
  let at = place("word");
  fs::write(at.join("note.txt"), "noted\n").unwrap();
  let word = "prompt(None, 'task', on=chain('w'))\nprompt(None, 'more', on=chain('s', 'chain1'))";
  let shown = |engine: &mut Engine, chain: &str| {
    let on = verbs::Transcript { on: Some(chain.to_owned()), of: None };
    let facts = engine.transcript(on).unwrap();
    facts
      .iter()
      .map(|one| (one.kind().to_owned(), one.about().to_owned(), one.by().to_owned()))
      .collect::<Vec<_>>()
  };
  let first = {
    let mut engine = lived(&at, vec![note(NOTE)]);
    let word = Some(word.to_owned());
    engine.rung(verbs::Rung { word, on: Some("chain1".to_owned()), ..Default::default() }).unwrap();
    ["chain2", "chain3"].map(|chain| shown(&mut engine, chain))
  };
  for facts in &first {
    let at =
      |kind: &str, by: &str| facts.iter().position(|one| one.0 == kind && one.2.starts_with(by));
    assert!(at("rung", "extensions").unwrap() < at("prompt", "rung").unwrap(), "{facts:?}");
  }
  let mut engine = lived(&at, vec![note(NOTE)]);
  let again = ["chain2", "chain3"].map(|chain| shown(&mut engine, chain));
  let places = |all: &[Vec<(String, String, String)>; 2]| {
    all.clone().map(|facts| facts.into_iter().map(|one| (one.0, one.1)).collect::<Vec<_>>())
  };
  let lived = places(&first);
  let mut again = places(&again);
  for (one, was) in again.iter_mut().zip(&lived) {
    one.truncate(was.len());
  }
  assert_eq!(
    again, lived,
    "a later life plays the same rungs at the same place, then catches up at its tip"
  );
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
  let expected = [
    ("chain", "operator"),
    ("made", "store"),
    ("stand", "operator"),
    ("done", "stand"),
    ("enable", "extensions"),
  ];
  assert_eq!(facts[..5], expected.map(|(kind, by)| (kind.to_owned(), by.to_owned())));
  let enable = Fact::of(crate::value::entry(&kept[4].as_ref(), 0).unwrap()).unwrap();
  let words: Vec<_> = (0..3).map(|at| enable.word(at).and_then(|one| one.as_str())).collect();
  assert_eq!(
    (enable.about(), words),
    ("chain1", vec![Some("note"), Some(NOTE), Some("note()")]),
    "the fact is about the root, and carries the name, the word and the life word"
  );
  assert_eq!(
    facts[5],
    ("rung".to_owned(), "extensions".to_owned()),
    "the rung stands after the fact"
  );
  let full = format!("{NOTE}\n\nnote()");
  assert_eq!(played(&mut engine, &["chain1"]), [rung("rung1", "chain1", &full)]);
  let turns = engine.turns(verbs::Turns { on: Some("chain1".to_owned()) }).unwrap();
  let told = turns.last().unwrap().as_ref().items().unwrap()[1].as_str().unwrap().to_owned();
  assert!(
    told.contains(&format!(
      "#rung1 by extensions\n<s:rung1_word>\n{full}</s:rung1_word>\n\n#read1\nread1_path = 'note.txt'\n"
    )),
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
      rung("rung5", "chain3", "note()"),
    ];
    assert_eq!(played(&mut engine, &["chain1", "chain2", "chain3"]), expected);
  }
  let other = Extension {
    name: "other".to_owned(),
    word: "o = 1".to_owned(),
    life: String::new(),
    root: false,
    file: String::new(),
  };
  let mut engine = lived(&at, vec![note(NOTE), other]);
  let expected = [
    rung("rung1", "chain1", &full),
    rung("rung6", "chain1", "o = 1"),
    rung("rung2", "chain2", &full),
    rung("rung7", "chain2", "o = 1"),
    rung("rung5", "chain3", "note()"),
    rung("rung8", "chain3", "o = 1"),
  ];
  assert_eq!(
    played(&mut engine, &["chain1", "chain2", "chain3"]),
    expected,
    "a later life plays the same rungs again, and a new extension after them"
  );
}

#[test]
fn the_definitions_of_a_word_are_its_imports_its_functions_its_classes_its_types_and_its_names_in_capitals()
 {
  let word = "import json\nfrom re import sub\n\nseen = []\nLIMIT = 2\nA = B = 3\nTOP: int = 4\nLATER: int\nmixed_Case = 5\n\n\
              @staticmethod\ndef note(on=\"\"):\n  return read(\"note.txt\", on=on)\n\nclass Note:\n  x = 1\n\ntype Pair = \
              tuple[int, int]\n\nnote()\nif LIMIT:\n  OTHER = 1";
  assert_eq!(
    definitions(word),
    "import json\n\nfrom re import sub\n\nLIMIT = 2\n\nA = B = 3\n\nTOP: int = 4\n\n@staticmethod\ndef note(on=\"\"):\n  \
     return read(\"note.txt\", on=on)\n\nclass Note:\n  x = 1\n\ntype Pair = tuple[int, int]"
  );
  assert_eq!(definitions("def broken(:"), "def broken(:", "a word that does not parse stays whole");
}

#[test]
fn a_manifest_says_whether_its_extension_plays_on_every_chain_or_on_the_root_alone() {
  let at = place("plays");
  let manifest = |plays: &str| {
    format!(r#"{{"name": "note", "word": "note.py", "life": "note()", "plays": {plays}}}"#)
  };
  folder(&at.join("every"), &manifest(r#""every""#), "x = 1\n");
  assert_eq!(load("note", &at.join("every")).unwrap(), filed(note("x = 1"), &at.join("every")));
  folder(&at.join("root"), &manifest(r#""root""#), "x = 1\n");
  assert_eq!(
    load("note", &at.join("root")).unwrap(),
    filed(Extension { root: true, ..note("x = 1") }, &at.join("root"))
  );
  let refused = |root: &str| load("note", &at.join(root)).unwrap_err().message();
  let file = |root: &str| at.join(root).join("furb.json").display().to_string();
  folder(&at.join("some"), &manifest(r#""some""#), "x = 1\n");
  assert_eq!(
    refused("some"),
    format!(
      r#"The manifest plays "some", and an extension plays "every" or "root": {}"#,
      file("some")
    )
  );
  folder(&at.join("one"), &manifest("1"), "x = 1\n");
  assert_eq!(refused("one"), format!("The plays of the manifest is no text: {}", file("one")));
}

#[test]
fn an_extension_of_the_root_alone_plays_on_the_root_and_on_a_chain_whose_source_had_it() {
  let at = place("rooted");
  let first = format!("{NOTE}\n\nseen = ['first']");
  let full = format!("{first}\n\nnote()");
  let rooted = |word: &str| Extension { root: true, ..note(word) };
  let chains = ["chain1", "chain2", "chain3"];
  {
    let mut engine = lived(&at, vec![rooted(&first)]);
    engine.chain(verbs::Chain { label: Some("two".to_owned()), ..Default::default() }).unwrap();
    let from = verbs::Chain { source: Some("chain1".to_owned()), ..Default::default() };
    engine.chain(from).unwrap();
    let expected = [rung("rung1", "chain1", &full), rung("rung4", "chain3", "note()")];
    assert_eq!(played(&mut engine, &chains), expected, "a chain with no source takes nothing");
  }
  let kept = world::kept(at.join("record.jsonl")).unwrap();
  let mut facts =
    kept.iter().map(|one| Fact::of(crate::value::entry(&one.as_ref(), 0).unwrap()).unwrap());
  let enable = facts.rfind(|one| one.kind() == "enable").unwrap();
  assert_eq!(
    enable.word(3).and_then(|one| one.as_str()),
    Some("root"),
    "the fact carries root last"
  );
  let newer = "def note(on=\"\"):\n  return read(\"other.txt\", on=on)";
  let mut engine = lived(&at, vec![rooted(newer)]);
  let expected = [
    ("chain1".to_owned(), full.clone()),
    ("chain1".to_owned(), newer.to_owned()),
    ("chain3".to_owned(), "note()".to_owned()),
    ("chain3".to_owned(), newer.to_owned()),
  ];
  assert_eq!(
    words(&mut engine, &chains),
    expected,
    "a reload plays on each chain that has it alone"
  );
  let root =
    engine.transcript(verbs::Transcript { on: Some("chain1".to_owned()), of: None }).unwrap();
  assert_eq!(enabled(&root), [rooted(newer)]);
}

/// A life in a directory of the tests whose standing names a doctrine, so the ear of the extensions sets up its root,
/// given these extensions, each with the file of its word in that directory.
fn set_up(at: &Path, given: Vec<Extension>) -> Engine {
  set_up_in(at, given, at)
}

/// A life as `set_up` opens it, whose standing names another directory.
fn set_up_in(at: &Path, given: Vec<Extension>, directory: &Path) -> Engine {
  fs::write(at.join("DOCTRINE.md"), "Work.\n").unwrap();
  let doctrine = at.join("DOCTRINE.md").display().to_string();
  let here = directory.to_owned();
  let stand = ear(move |mut co| async move {
    loop {
      let a = hear(&mut co).await;
      if a.kind() == "stand" && a.question() {
        let operator = Object::list([Object::string("operator"), Object::list([]), Object::int(1)]);
        let place = Object::string(here.display().to_string());
        let standing = Object::list([
          Object::list([operator]),
          place,
          Object::string("operator"),
          Object::string(&doctrine),
        ]);
        say(&mut co, Fact::says("done", a.about(), [standing]));
      }
    }
  });
  let given = given.into_iter().map(|one| {
    let file = at.join(format!("{}.py", one.name));
    fs::write(&file, &one.word).unwrap();
    Extension { file: file.display().to_string(), ..one }
  });
  let (record, store) = world::store(at.join("record.jsonl")).unwrap();
  let ears = [
    ("extensions", extensions(given.collect())),
    ("stand", stand),
    ("files", world::files(world::Scratch::own(), None)),
    ("store", store),
  ];
  let engine = Engine::boot(record, ears).unwrap();
  assert!(engine.raised().is_none(), "{:?}", engine.raised());
  engine
}

#[test]
fn the_setup_of_the_root_ends_with_the_extensions_of_the_root_alone_and_a_chain_with_no_source_holds_none_of_them()
 {
  let at = place("setup");
  let shared = Extension { name: "shared".to_owned(), life: "y = 2".to_owned(), ..note("x = 1") };
  let rooted =
    Extension { name: "rooted".to_owned(), life: String::new(), root: true, ..note("z = 3") };
  let mut engine = set_up(&at, vec![shared, rooted]);
  let rooted = at.join("rooted.py").display().to_string();
  let texts = |engine: &mut Engine, chain: &str| -> Vec<String> {
    let turns = engine.turns(verbs::Turns { on: Some(chain.to_owned()) }).unwrap();
    turns.iter().map(|one| one.as_ref().items().unwrap()[1].as_str().unwrap().to_owned()).collect()
  };
  let root = texts(&mut engine, "chain1");
  let life = root.iter().position(|one| one == "y = 2").unwrap();
  let read = format!("read({})", Object::string(&rooted).py_repr());
  assert_eq!(root[life + 2], read, "{root:?}");
  assert!(root[life + 3].starts_with("#read3\n"), "{root:?}");
  let stood = format!("stand()\ncd({})", Object::string(at.display().to_string()).py_repr());
  assert_eq!(root[life + 4], stood, "the root tells its standing after every extension: {root:?}");
  let last = &root[life + 5];
  assert!(
    last.starts_with("#stand2\nstand2_roster = ") && last.contains("\n\n#cd1\ncd1_path = "),
    "{last}"
  );
  assert!(last.contains("#chain1\nchain1_label = 'root'"), "{last}");
  assert!(last.ends_with("chain1: Act[Never] = Act('chain1')"), "{last}");
  engine.chain(verbs::Chain { label: Some("two".to_owned()), ..Default::default() }).unwrap();
  let two = texts(&mut engine, "chain2");
  assert_eq!(two[..=life + 1], root[..=life + 1], "a chain with no source holds the shared setup");
  assert_eq!(
    two[life + 2],
    stood,
    "the ear tells a chain with no source its standing at its birth: {two:?}"
  );
  assert!(two[life + 3].starts_with("#stand3\nstand3_roster = "), "{two:?}");
  assert!(two.iter().all(|one| !one.contains(&read)), "{two:?}");
  assert!(two.last().unwrap().contains("#chain2\nchain2_label = 'two'"), "{two:?}");
}

#[test]
fn a_later_life_on_a_new_standing_plays_a_stand_on_each_chain_and_a_cd_where_the_chain_ran_none_of_its_own()
 {
  let at = place("restand");
  let (sub, other) = (at.join("sub"), at.join("other"));
  fs::create_dir_all(&sub).unwrap();
  fs::create_dir_all(&other).unwrap();
  let last = |engine: &mut Engine, chain: &str| -> String {
    let turns = engine.turns(verbs::Turns { on: Some(chain.to_owned()) }).unwrap();
    let mut said =
      turns.iter().filter(|one| one.as_ref().items().unwrap()[0].as_str() == Some("assistant"));
    said.next_back().unwrap().as_ref().items().unwrap()[1].as_str().unwrap().to_owned()
  };
  let mut engine = set_up(&at, vec![]);
  for label in ["two", "three"] {
    engine.chain(verbs::Chain { label: Some(label.to_owned()), ..Default::default() }).unwrap();
  }
  let moved = verbs::Rung {
    word: Some("cd('sub')".to_owned()),
    on: Some("chain3".to_owned()),
    ..Default::default()
  };
  engine.rung(moved).unwrap();
  let stood = format!("stand()\ncd({})", Object::string(at.display().to_string()).py_repr());
  assert_eq!(
    last(&mut engine, "chain2"),
    stood,
    "a chain with no source holds the standing of the setup"
  );
  drop(engine);
  let mut engine = set_up_in(&at, vec![], &other);
  let stood = format!("stand()\ncd({})", Object::string(other.display().to_string()).py_repr());
  assert_eq!(last(&mut engine, "chain1"), stood, "the cd of the setup is no cd of the root");
  assert_eq!(last(&mut engine, "chain2"), stood);
  assert_eq!(
    last(&mut engine, "chain3"),
    "stand()",
    "a chain that ran a cd of its own stays where it stands"
  );
  let cwd = |engine: &mut Engine, chain: &str| {
    engine.cwd(verbs::Cwd { on: Some(chain.to_owned()) }).unwrap()
  };
  assert!(cwd(&mut engine, "chain2").ends_with("/other"), "the cd moved the chain");
  assert!(cwd(&mut engine, "chain3").ends_with("/sub"), "its own cd holds");
  drop(engine);
  let mut engine = set_up_in(&at, vec![], &other);
  assert_eq!(
    last(&mut engine, "chain2"),
    stood,
    "a later life on the same standing plays nothing more"
  );
}

/// The chain and the word of each rung that the ear of the extensions made on these chains, in order.
fn words(engine: &mut Engine, chains: &[&str]) -> Vec<(String, String)> {
  played(engine, chains).into_iter().map(|(_, chain, word)| (chain, word)).collect()
}

/// The words of each fact enable that the record at a place keeps.
fn enables(at: &Path) -> Vec<String> {
  let kept = world::kept(at.join("record.jsonl")).unwrap();
  let facts =
    kept.iter().map(|one| Fact::of(crate::value::entry(&one.as_ref(), 0).unwrap()).unwrap());
  let enabled = facts
    .filter(|one| one.kind() == "enable")
    .map(|one| one.word(1).unwrap().as_str().unwrap().to_owned());
  enabled.collect()
}

#[test]
fn a_later_life_given_a_new_word_reloads_it_and_each_chain_plays_its_definitions_alone_so_its_state_stays()
 {
  let at = place("reload");
  fs::write(at.join("note.txt"), "noted\n").unwrap();
  let first = format!("{NOTE}\n\nseen = ['first']");
  let newer = "seen = []\n\ndef note(on=\"\"):\n  return read(\"other.txt\", on=on)\n\nLIMIT = 2";
  let defined = "seen = globals().setdefault(\"seen\", [])\n\ndef note(on=\"\"):\n  return read(\"other.txt\", on=on)\n\nLIMIT = 2";
  let full = format!("{first}\n\nnote()");
  {
    let mut engine = lived(&at, vec![note(&first)]);
    engine.chain(verbs::Chain { label: Some("two".to_owned()), ..Default::default() }).unwrap();
  }
  let expected = [
    ("chain1".to_owned(), full.clone()),
    ("chain1".to_owned(), defined.to_owned()),
    ("chain2".to_owned(), full.clone()),
    ("chain2".to_owned(), defined.to_owned()),
  ];
  {
    let mut engine = lived(&at, vec![note(newer)]);
    assert_eq!(words(&mut engine, &["chain1", "chain2"]), expected);
    let word = Some("write(Text('seen.txt', repr((seen, LIMIT))))".to_owned());
    engine.rung(verbs::Rung { word, on: Some("chain1".to_owned()), ..Default::default() }).unwrap();
    assert_eq!(
      fs::read_to_string(at.join("seen.txt")).unwrap(),
      "(['first'], 2)",
      "a name of state keeps its value"
    );
  }
  assert_eq!(enables(&at), [first.clone(), newer.to_owned()]);
  let mut engine = lived(&at, vec![note(newer)]);
  assert_eq!(
    words(&mut engine, &["chain1", "chain2"]),
    expected,
    "a later life plays the same rungs again"
  );
  assert_eq!(enables(&at), [first, newer.to_owned()], "and enables nothing more");
  let root =
    engine.transcript(verbs::Transcript { on: Some("chain1".to_owned()), of: None }).unwrap();
  assert_eq!(enabled(&root), [note(newer)], "the life runs the word that it enabled last");
}

#[test]
fn the_reloads_of_a_word_are_its_definitions_and_each_name_of_state_that_it_assigns_by_setdefault()
{
  let word = "import json\n\nseen = []\nLIMIT = 2\nnames: dict = globals().setdefault(\"names\", {})\nmixed_Case = 5\nleft, right = 1, 2\nLATER: int\n\ndef note():\n  return 1\n\nnote()\nif LIMIT:\n  other = 1";
  assert_eq!(
    reloads(word),
    "import json\n\nseen = globals().setdefault(\"seen\", [])\n\nLIMIT = 2\n\nnames: dict = globals().setdefault(\"names\", {})\n\nmixed_Case = globals().setdefault(\"mixed_Case\", 5)\n\ndef note():\n  return 1"
  );
  assert_eq!(reloads("def broken(:"), "def broken(:", "a word that does not parse stays whole");
}

#[test]
fn a_reload_plays_a_new_function_and_a_new_name_of_state_and_keeps_the_state_that_a_chain_holds() {
  let at = place("fresh");
  fs::write(at.join("note.txt"), "noted\n").unwrap();
  let first = format!("{NOTE}\n\nseen = ['first']");
  let newer = "seen = []\nfresh: dict = {'made': 1}\nTRAIN = 3\n\ndef trained():\n  return TRAIN";
  let defined = "seen = globals().setdefault(\"seen\", [])\n\nfresh: dict = globals().setdefault(\"fresh\", {'made': 1})\n\nTRAIN = 3\n\ndef trained():\n  return TRAIN";
  let full = format!("{first}\n\nnote()");
  {
    let mut engine = lived(&at, vec![note(&first)]);
    engine.chain(verbs::Chain { label: Some("two".to_owned()), ..Default::default() }).unwrap();
  }
  let expected = [
    ("chain1".to_owned(), full.clone()),
    ("chain1".to_owned(), defined.to_owned()),
    ("chain2".to_owned(), full.clone()),
    ("chain2".to_owned(), defined.to_owned()),
  ];
  let mut engine = lived(&at, vec![note(newer)]);
  assert_eq!(words(&mut engine, &["chain1", "chain2"]), expected);
  for chain in ["chain1", "chain2"] {
    let word = Some("write(Text('seen.txt', repr((seen, fresh, trained()))))".to_owned());
    engine.rung(verbs::Rung { word, on: Some(chain.to_owned()), ..Default::default() }).unwrap();
    assert_eq!(
      fs::read_to_string(at.join("seen.txt")).unwrap(),
      "(['first'], {'made': 1}, 3)",
      "a new function and a new name of state play, and the state that the chain holds stays"
    );
  }
}

#[test]
fn a_write_of_the_life_in_the_folder_of_an_extension_that_a_config_turns_on_reloads_it() {
  let at = place("rewrite");
  fs::write(at.join("note.txt"), "noted\n").unwrap();
  let root = at.join(".furb").join("extensions").join("note");
  folder(&root, r#"{"name": "note", "word": "note.py", "life": "note()"}"#, NOTE);
  config(&at.join(".furb").join("config.json"), r#"{"extensions": {"note": "extensions/note"}}"#);
  let mut engine = lived(&at, vec![load("note", &root).unwrap()]);
  let newer = "def note(on=\"\"):\n  return read(\"other.txt\", on=on)\n";
  let file = root.join("note.py").display().to_string();
  engine
    .write(&crate::value::Text::new(file, newer), verbs::Write { on: Some("chain1".to_owned()) })
    .unwrap();
  let full = format!("{NOTE}\n\nnote()");
  let expected = [("chain1".to_owned(), full), ("chain1".to_owned(), newer.trim_end().to_owned())];
  assert_eq!(words(&mut engine, &["chain1"]), expected);
  assert_eq!(enables(&at), [NOTE.to_owned(), newer.trim_end().to_owned()]);
  fs::write(at.join("elsewhere.py"), "").unwrap();
  let other = at.join("elsewhere.py").display().to_string();
  engine
    .write(
      &crate::value::Text::new(other, "X = 1\n"),
      verbs::Write { on: Some("chain1".to_owned()) },
    )
    .unwrap();
  assert_eq!(enables(&at).len(), 2, "a write outside the folder of an extension reloads nothing");
}
