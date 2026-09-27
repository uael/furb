//! What an extension is and where a host finds it: the word of a file, a folder loaded under its name, and the
//! configs of the user and of the project.

use std::{
  fs,
  path::{Path, PathBuf},
};

use super::{Extension, Places, configured, load, word};

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
  assert_eq!(configured(&places, &project).unwrap(), vec![], "no config turns nothing on");
  folder(&at.join("home").join("tilde"), r#"{"name": "tilde", "word": "note.py"}"#, "t = 1");
  folder(&at.join("user").join("near"), r#"{"name": "near", "word": "note.py"}"#, "n = 1");
  folder(&project.join("far"), r#"{"name": "far", "word": "note.py"}"#, "f = 1");
  config(
    &at.join("user").join("config.json"),
    r#"{"extensions": {"tilde": "~/tilde", "near": "near", "far": false}}"#,
  );
  let names = |got: Vec<Extension>| got.into_iter().map(|one| one.name).collect::<Vec<_>>();
  assert_eq!(names(configured(&places, &project).unwrap()), ["tilde", "near"]);
  config(
    &project.join(".furb").join("config.json"),
    r#"{"extensions": {"near": false, "far": "../far", "tilde": true}}"#,
  );
  assert_eq!(
    names(configured(&places, &project).unwrap()),
    ["tilde", "far"],
    "each stands where a config first names it"
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
  assert_eq!(read(r#"{"theme": "dark"}"#).unwrap(), vec![], "a config may name no extension");
  assert_eq!(refused("[]"), format!("A config is a map, whose extensions is a map: {shown}"));
  assert_eq!(
    refused(r#"{"extensions": []}"#),
    format!("A config is a map, whose extensions is a map: {shown}")
  );
  assert_eq!(
    refused(r#"{"extensions": {"note": 1}}"#),
    format!("The extension note is true, false, or a path: {shown}")
  );
  assert_eq!(
    refused(r#"{"extensions": {"note": true}}"#),
    format!("No config gives the folder of the extension note: {shown}")
  );
  assert!(refused("{").ends_with(&format!(": {shown}")));
}
