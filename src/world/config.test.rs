use std::{fs, path::PathBuf};

use serde_json::{Value, json};

use super::{Change, Error, Kind, Layer, Layers, Property, Setting};
use crate::world::catalog::blocking;

/// A project and a config directory of a user, each empty, in a folder of the test alone.
fn place(name: &str) -> (PathBuf, Layers) {
  let at = std::env::temp_dir().join(format!("furb-config-{name}-{}", std::process::id()));
  drop(fs::remove_dir_all(&at));
  fs::create_dir_all(at.join("project/.furb")).unwrap();
  fs::create_dir_all(at.join("user")).unwrap();
  let layers = Layers::new(at.join("project"), Some(at.join("user")));
  (at, layers)
}

fn custom(model: &str) -> Value {
  json!({"protocol": "chat", "url": "http://127.0.0.1:9/v1", "key": "k", "models": {model: {"window": 8192}}})
}

fn change(key: &[&str], value: Option<Value>, layer: Layer) -> Change {
  Change { key: key.iter().map(|one| (*one).to_owned()).collect(), value, layer }
}

fn setting(key: &[&str], value: Value, layer: Layer) -> Setting {
  let key = key.iter().map(|one| (*one).to_owned()).collect();
  Setting { key, value, layer: Some(layer), property: None, extension: None }
}

/// The key, the value and the layer of each setting, which the tests of the order read.
fn won(layers: &Layers) -> Vec<(String, Value, Option<Layer>)> {
  let settings = layers.get().unwrap().into_iter();
  settings.map(|one| (one.key.join("."), one.value, one.layer)).collect()
}

/// An extension in a folder of the project, which the project config turns on, and whose manifest
/// names the file of its keys.
fn extension(at: &std::path::Path, name: &str, keys: &Value) {
  let root = at.join("project/.furb").join(name);
  fs::create_dir_all(&root).unwrap();
  let manifest = format!(
    "---\nname: {name}\ndescription: A test.\nmetadata:\n  furb-word: {name}.py\n  furb-config: keys.json\n---\n"
  );
  fs::write(root.join("SKILL.md"), manifest).unwrap();
  fs::write(root.join(format!("{name}.py")), "").unwrap();
  fs::write(root.join("keys.json"), keys.to_string()).unwrap();
  let config = at.join("project/.furb/config.json");
  fs::write(config, json!({"extensions": {name: name}}).to_string()).unwrap();
}

/// A change kept as a life keeps it, which a test of no runtime waits for.
fn set(layers: &Layers, change: Change) -> Result<(), Error> {
  blocking(layers.set(change))
}

fn text(file: &PathBuf) -> Value {
  serde_json::from_str(&fs::read_to_string(file).unwrap()).unwrap()
}

#[test]
fn the_configs_give_each_key_with_the_value_and_the_layer_that_win_for_it() {
  let (at, layers) = place("get");
  fs::write(
    at.join("user/config.json"),
    json!({"max_thinking_tokens": 1, "providers": {"a": true, "b": true}}).to_string(),
  )
  .unwrap();
  fs::write(
    at.join("project/.furb/config.json"),
    json!({"providers": {"b": false}, "extensions": {"x": "x"}}).to_string(),
  )
  .unwrap();
  fs::write(
    at.join("project/.furb/config.local.json"),
    json!({"max_thinking_tokens": 2}).to_string(),
  )
  .unwrap();
  let got = layers.get().unwrap();
  assert!(got.contains(&setting(&["providers", "a"], json!(true), Layer::User)), "{got:?}");
  assert!(got.contains(&setting(&["providers", "b"], json!(false), Layer::Project)), "{got:?}");
  assert!(got.contains(&setting(&["extensions", "x"], json!("x"), Layer::Project)), "{got:?}");
  let thinking = got.iter().find(|one| one.key == ["max_thinking_tokens"]).unwrap();
  assert_eq!((&thinking.value, thinking.layer), (&json!(2), Some(Layer::Local)));
  fs::remove_dir_all(at).unwrap();
}

#[test]
fn the_keys_of_furb_come_first_with_their_defaults_and_each_object_with_its_names_after_it() {
  let (at, layers) = place("schema");
  fs::write(
    at.join("project/.furb/config.json"),
    json!({"other": 1, "providers": {"b": false}, "gate": false}).to_string(),
  )
  .unwrap();
  assert_eq!(
    won(&layers),
    vec![
      ("gate".to_owned(), json!(false), Some(Layer::Project)),
      ("max_thinking_tokens".to_owned(), json!(32768), None),
      ("automatic".to_owned(), json!([]), None),
      ("classes".to_owned(), json!({}), None),
      ("providers".to_owned(), json!({"b": false}), None),
      ("providers.b".to_owned(), json!(false), Some(Layer::Project)),
      ("extensions".to_owned(), json!({}), None),
      ("other".to_owned(), json!(1), Some(Layer::Project)),
    ]
  );
  let gate = layers.get().unwrap().into_iter().next().unwrap();
  assert_eq!(gate.property.map(|one| one.kind), Some(Kind::Boolean));
  fs::remove_dir_all(at).unwrap();
}

#[test]
fn an_extension_adds_the_keys_of_its_schema_under_its_name() {
  let (at, layers) = place("extension");
  let keys = json!({"properties": {
    "folder": {"type": "string", "default": "notes", "description": "Where notes go."},
    "mode": {"type": "string", "enum": ["a", "b"]},
  }});
  extension(&at, "notes", &keys);
  set(&layers, change(&["notes", "mode"], Some(json!("b")), Layer::Local)).unwrap();
  let got = layers.get().unwrap();
  let folder = got.iter().find(|one| one.key == ["notes", "folder"]).unwrap();
  assert_eq!(
    (&folder.value, folder.layer, folder.extension.as_deref()),
    (&json!("notes"), None, Some("notes"))
  );
  let property = Property {
    kind: Kind::String,
    default: json!("notes"),
    description: "Where notes go.".to_owned(),
    values: vec![],
  };
  assert_eq!(folder.property.as_ref(), Some(&property));
  let mode = got.iter().find(|one| one.key == ["notes", "mode"]).unwrap();
  assert_eq!((&mode.value, mode.layer), (&json!("b"), Some(Layer::Local)));
  fs::remove_dir_all(at).unwrap();
}

#[test]
fn a_value_that_the_property_of_its_key_does_not_take_is_refused_and_writes_nothing() {
  let (at, layers) = place("kind");
  extension(&at, "notes", &json!({"properties": {"mode": {"type": "string", "enum": ["a", "b"]}}}));
  let before = fs::read_to_string(at.join("project/.furb/config.json")).unwrap();
  let refused = set(&layers, change(&["gate"], Some(json!("no")), Layer::Project));
  assert!(matches!(&refused, Err(Error::Value { key, .. }) if key == "gate"), "{refused:?}");
  let refused = set(&layers, change(&["notes", "mode"], Some(json!("c")), Layer::Project));
  assert_eq!(
    refused.map_err(|no| no.to_string()),
    Err(r#"The key notes.mode takes one of "a", "b", and "c" is not"#.to_owned())
  );
  assert_eq!(fs::read_to_string(at.join("project/.furb/config.json")).unwrap(), before);
  set(&layers, change(&["gate"], None, Layer::Project)).unwrap();
  fs::remove_dir_all(at).unwrap();
}

#[test]
fn an_extension_that_adds_keys_under_a_name_of_furb_is_refused() {
  let (at, layers) = place("clash");
  extension(&at, "providers", &json!({"properties": {}}));
  assert!(matches!(layers.get(), Err(Error::Clash(name)) if name == "providers"));
  fs::remove_dir_all(at).unwrap();
}

#[test]
fn a_change_lands_in_its_layer_and_a_flag_on_a_provider_keeps_its_settings() {
  let (at, layers) = place("set");
  let project = at.join("project/.furb/config.json");
  fs::write(&project, json!({"max_thinking_tokens": 0}).to_string()).unwrap();
  set(&layers, change(&["providers", "strata"], Some(custom("qwen")), Layer::Project)).unwrap();
  set(&layers, change(&["providers", "strata"], Some(json!(false)), Layer::Project)).unwrap();
  let mut off = custom("qwen");
  off["enabled"] = json!(false);
  assert_eq!(text(&project), json!({"max_thinking_tokens": 0, "providers": {"strata": off}}));
  assert!(fs::read_to_string(&project).unwrap().ends_with("}\n"), "the config ends its last line");
  set(&layers, change(&["providers", "other"], Some(custom("m")), Layer::Local)).unwrap();
  assert_eq!(
    text(&at.join("project/.furb/config.local.json")),
    json!({"providers": {"other": custom("m")}})
  );
  assert!(!at.join("user/config.json").exists(), "no change reaches another layer");
  fs::remove_dir_all(at).unwrap();
}

#[test]
fn a_change_with_no_value_takes_its_key_out_of_its_layer() {
  let (at, layers) = place("out");
  let local = at.join("project/.furb/config.local.json");
  fs::write(
    &local,
    json!({"providers": {"strata": custom("qwen")}, "max_thinking_tokens": 0}).to_string(),
  )
  .unwrap();
  set(&layers, change(&["providers", "strata"], None, Layer::Local)).unwrap();
  set(&layers, change(&["max_thinking_tokens"], None, Layer::Local)).unwrap();
  assert_eq!(text(&local), json!({"providers": {}}));
  fs::remove_dir_all(at).unwrap();
}

#[test]
fn a_change_that_the_catalog_refuses_puts_its_config_back_as_it_was() {
  let (at, layers) = place("back");
  let refused = set(&layers, change(&["providers", "nothing"], Some(json!(true)), Layer::Project));
  assert!(matches!(refused, Err(Error::Refused(_))), "{refused:?}");
  assert!(
    !at.join("project/.furb/config.json").exists(),
    "a config that was not there is not there"
  );
  let local = at.join("project/.furb/config.local.json");
  let before = "{\"max_thinking_tokens\":   0}\n";
  fs::write(&local, before).unwrap();
  let refused =
    set(&layers, change(&["providers", "bad name"], Some(custom("qwen")), Layer::Local));
  assert!(matches!(refused, Err(Error::Refused(_))), "{refused:?}");
  assert_eq!(fs::read_to_string(&local).unwrap(), before, "the config stands as its text was");
  fs::remove_dir_all(at).unwrap();
}

#[test]
fn a_key_of_no_name_or_of_three_names_is_refused_and_writes_nothing() {
  let (at, layers) = place("key");
  for key in [&[][..], &["a", "b", "c"][..]] {
    let refused = set(&layers, change(key, Some(json!(1)), Layer::Project));
    assert!(matches!(refused, Err(Error::Key(_))), "{refused:?}");
  }
  assert!(!at.join("project/.furb/config.json").exists());
  fs::remove_dir_all(at).unwrap();
}
