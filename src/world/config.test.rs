use std::{fs, path::PathBuf};

use serde_json::{Value, json};

use super::{Change, Error, Layer, Layers, Setting};
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
  Setting { key: key.iter().map(|one| (*one).to_owned()).collect(), value, layer }
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
  assert_eq!(
    layers.get().unwrap(),
    vec![
      setting(&["max_thinking_tokens"], json!(2), Layer::Local),
      setting(&["providers", "a"], json!(true), Layer::User),
      setting(&["providers", "b"], json!(false), Layer::Project),
      setting(&["extensions", "x"], json!("x"), Layer::Project),
    ]
  );
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
