//! The one writer of the configs of a life, in their layers: `config.json` in the config directory
//! of the user, then `.furb/config.json` of the project, which git keeps, then
//! `.furb/config.local.json` beside it, which git ignores. Each reader reads them in this order,
//! as [`crate::extension::configs`] gives them, and a later layer wins for a key. A change that
//! the catalog cannot read stays on no disk: the writer puts its file back as it was.
//!
//! A schema names the keys, as the settings of VS Code do: furb names the keys that it reads, and
//! each extension that the configs turn on adds the keys of the file that its manifest names under
//! `furb-config`, each under the name of the extension. The writer refuses a value that the
//! property of its key does not take, and keeps each key that no schema names as it is.

use std::{
  fs, io,
  path::{Path, PathBuf},
};

use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use super::{
  Catalog, catalog,
  disk::{self, Access},
  provider::{KEY_AUTOMATIC, KEY_CLASSES, KEY_THINKING, THINKING, catalog::switched},
};
use crate::{
  extension::{self, Places, configs},
  gate,
  value::Fault,
};

/// A layer of the configs, from the one that wins least to the one that wins most.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Layer {
  User,
  #[default]
  Project,
  Local,
}

const LAYERS: [Layer; 3] = [Layer::User, Layer::Project, Layer::Local];

/// A key of the configs, the value that wins for it, and the layer that gives that value. A key is
/// a name at the top of a config, or a name in an object at the top, as `["providers", "codex"]`,
/// since each reader merges these objects name by name. The reader of the providers merges the
/// fields of a provider too, so a value of a provider can hold fewer fields than it reads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Setting {
  pub key: Vec<String>,
  pub value: Value,
  /// The layer that gives the value, or none when no one layer gives it: the default of its
  /// property, or an object of furb whose names come from the settings that follow it.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub layer: Option<Layer>,
  /// The property that a schema gives the key, or none when no schema names it.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub property: Option<Property>,
  /// The extension whose schema names the key, or none for a key of furb or of no schema.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub extension: Option<String>,
}

/// A key that a schema names, in the form of a setting of VS Code: the kind of its value, the
/// value that holds when no config gives one, what it does, and under `enum` the values that it
/// takes when it takes only some.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Property {
  #[serde(rename = "type")]
  pub kind: Kind,
  #[serde(default)]
  pub default: Value,
  #[serde(default)]
  pub description: String,
  #[serde(default, rename = "enum", skip_serializing_if = "Vec::is_empty")]
  pub values: Vec<Value>,
}

/// The kind of the value of a key, as JSON Schema names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
  Boolean,
  Integer,
  Number,
  String,
  Array,
  Object,
}

impl Property {
  fn takes(&self, value: &Value) -> bool {
    let kind = match self.kind {
      Kind::Boolean => value.is_boolean(),
      Kind::Integer => value.is_i64() || value.is_u64(),
      Kind::Number => value.is_number(),
      Kind::String => value.is_string(),
      Kind::Array => value.is_array(),
      Kind::Object => value.is_object(),
    };
    kind && (self.values.is_empty() || self.values.contains(value))
  }

  /// What the property takes, as a person reads it.
  fn wanted(&self) -> String {
    if !self.values.is_empty() {
      let values: Vec<String> = self.values.iter().map(Value::to_string).collect();
      return format!("one of {}", values.join(", "));
    }
    let kind = match self.kind {
      Kind::Boolean => "a boolean",
      Kind::Integer => "an integer",
      Kind::Number => "a number",
      Kind::String => "a string",
      Kind::Array => "an array",
      Kind::Object => "an object",
    };
    kind.to_owned()
  }
}

/// The keys of the file that the manifest of an extension names under `furb-config`.
#[derive(Deserialize)]
struct Keys {
  properties: IndexMap<String, Property>,
}

/// A key that a schema names, with its property and the extension that names it.
struct Named {
  key: Vec<String>,
  property: Property,
  extension: Option<String>,
}

/// The objects of furb that each reader merges name by name, which the settings give with each of
/// their names after them.
const PROVIDERS: &str = "providers";
const EXTENSIONS: &str = "extensions";

/// The keys that furb reads.
/// Whether a name is a key of furb at the top of the configs, which no extension takes as its name,
/// since the configs hold the keys of an extension under its name.
pub(crate) fn furb_key(name: &str) -> bool {
  own().iter().any(|one| one.key[0] == name)
}

fn own() -> Vec<Named> {
  let one = |name: &str, kind, default, description: &str| Named {
    key: vec![name.to_owned()],
    property: Property { kind, default, description: description.to_owned(), values: vec![] },
    extension: None,
  };
  vec![
    one(
      gate::KEY,
      Kind::Boolean,
      json!(gate::ON),
      "Read each word of a model before it runs, and refuse a word in which the gate finds an \
       error. With the gate off, a word runs one statement at a time and stops at the first that \
       raises. A life reads it when it opens.",
    ),
    one(
      KEY_THINKING,
      Kind::Integer,
      json!(THINKING),
      "The most tokens of thought of a model in one turn, 0 turning the thought off.",
    ),
    one(KEY_AUTOMATIC, Kind::Array, json!([]), "The models of automatic work, the first first."),
    one(KEY_CLASSES, Kind::Object, json!({}), "The class, cheap or dear, of each model it names."),
    one(PROVIDERS, Kind::Object, json!({}), "The providers of models, each by its name."),
    one(
      EXTENSIONS,
      Kind::Object,
      json!({}),
      "The extensions that a life runs, each by its name: true, false, or the path of its folder.",
    ),
  ]
}

/// A key of a layer set to a value, or taken out when the value is none. A true or a false on a
/// provider whose value is an object is [`switched`], as each reader of the providers takes it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Change {
  pub key: Vec<String>,
  #[serde(default)]
  pub value: Option<Value>,
  #[serde(default)]
  pub layer: Layer,
}

/// Why the configs were not read, or a change of them was not kept.
#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("Could not read {}: {source}", path.display())]
  Read { path: PathBuf, source: io::Error },
  #[error("Invalid config at {}: {source}", path.display())]
  Parse { path: PathBuf, source: serde_json::Error },
  #[error("The config at {} must be an object", path.display())]
  NotObject { path: PathBuf },
  #[error("A key of a config has one or two names, and {0:?} has {n}", n = .0.len())]
  Key(Vec<String>),
  #[error("The {name} of the config at {} is no object, so it holds no key", path.display())]
  Within { path: PathBuf, name: String },
  #[error(transparent)]
  Write(#[from] disk::Error),
  #[error("The change was not kept: {0}")]
  Refused(#[source] catalog::Error),
  #[error("The change was not kept: {refused}, and its config was not put back: {source}")]
  Restore { refused: catalog::Error, source: disk::Error },
  #[error("The keys of the extensions were not read: {0}")]
  Extensions(#[source] Fault),
  #[error("The keys of the extension {name} are no schema of keys: {source}")]
  Schema { name: String, source: serde_json::Error },
  #[error("The key {key} takes {wanted}, and {value} is not")]
  Value { key: String, wanted: String, value: Value },
}

/// The three configs of a project, under the config directory of a user.
#[derive(Debug, Clone)]
pub struct Layers {
  project: PathBuf,
  user: PathBuf,
}

impl Layers {
  /// The configs of a project, under the config directory of the user of this process when none
  /// is given.
  pub fn new(project: impl Into<PathBuf>, user: Option<PathBuf>) -> Self {
    Self { project: project.into(), user: user.unwrap_or_else(|| Places::here().config) }
  }

  fn file(&self, layer: Layer) -> PathBuf {
    let [user, project, local] = configs(&self.user, &self.project);
    match layer {
      Layer::User => user,
      Layer::Project => project,
      Layer::Local => local,
    }
  }

  /// Each key of the configs with the value that wins for it: each key of the schema, in its
  /// order, with its default when no layer gives it, then each key that no schema names, in the
  /// order in which the layers first give the keys. An object of furb comes with the object of
  /// the values that win for its names, then with a setting for each of its names. A config that
  /// is not on the disk gives no key.
  pub fn get(&self) -> Result<Vec<Setting>, Error> {
    let mut won = self.won()?;
    let mut out = Vec::new();
    for Named { key, property, extension } in self.schema()? {
      let (value, layer, within) = if property.kind == Kind::Object && extension.is_none() {
        let (within, rest) =
          won.into_iter().partition(|(one, _)| one.len() == 2 && one[0] == key[0]);
        won = rest;
        let within: Vec<Setting> = settings(within);
        let names = within.iter().map(|one| (one.key[1].clone(), one.value.clone()));
        let value =
          if within.is_empty() { property.default.clone() } else { Value::Object(names.collect()) };
        (value, None, within)
      } else {
        match won.shift_remove(&key) {
          Some((value, layer)) => (value, Some(layer), vec![]),
          None => (property.default.clone(), None, vec![]),
        }
      };
      out.push(Setting { key, value, layer, property: Some(property), extension });
      out.extend(within);
    }
    out.extend(settings(won));
    Ok(out)
  }

  /// Each key that the layers give with the value and the layer that win for it, in the order in
  /// which the layers first give the keys.
  fn won(&self) -> Result<IndexMap<Vec<String>, (Value, Layer)>, Error> {
    let mut won = IndexMap::new();
    for layer in LAYERS {
      for (name, value) in read(&self.file(layer))?.0 {
        if let Value::Object(within) = value {
          for (one, value) in within {
            won.insert(vec![name.clone(), one], (value, layer));
          }
        } else {
          won.insert(vec![name], (value, layer));
        }
      }
    }
    Ok(won)
  }

  /// Each key that a schema names: the keys of furb, then the keys that each extension that the
  /// configs turn on adds under its name, in the order a life runs them. A file of keys that is no
  /// schema names no key, so the configs stay readable and a config can turn its extension off,
  /// and the read of `settings://` and the name of that extension says why. An extension that
  /// takes the name of a key of furb plays not, as [`extension::load`] says, and names no key.
  fn schema(&self) -> Result<Vec<Named>, Error> {
    let places = Places { config: self.user.clone(), home: Places::here().home };
    let mut named = own();
    for (name, text) in extension::contributed(&places, &self.project).map_err(Error::Extensions)? {
      if furb_key(&name) {
        continue;
      }
      let Ok(keys) = serde_json::from_str::<Keys>(&text) else { continue };
      named.extend(keys.properties.into_iter().map(|(one, property)| Named {
        key: vec![name.clone(), one],
        property,
        extension: Some(name.clone()),
      }));
    }
    Ok(named)
  }

  /// The value of each key of the configs as one object, as a word reads it: each key of furb,
  /// and each extension by its name with the object of its keys, each with the value that wins
  /// for it, or the default of its property when no layer gives it.
  pub fn values(&self) -> Result<Map<String, Value>, Error> {
    let mut out = Map::new();
    for Setting { key, value, .. } in self.get()? {
      match key.as_slice() {
        [one] => {
          out.insert(one.clone(), value);
        }
        [one, two] => {
          let within = out.entry(one.clone()).or_insert_with(|| Value::Object(Map::new()));
          if let Value::Object(within) = within {
            within.insert(two.clone(), value);
          }
        }
        _ => {}
      }
    }
    Ok(out)
  }

  /// A change written to its layer, then the catalog of this machine configured on the layers, as
  /// a stand of a life configures it. A value that the property of its key does not take writes
  /// nothing. A change that the catalog refuses puts the file back as it was, and gives why.
  pub async fn set(&self, change: Change) -> Result<(), Error> {
    if let Some(value) = &change.value
      && let Some(one) = self.schema()?.into_iter().find(|one| one.key == change.key)
      && !one.property.takes(value)
    {
      let (key, wanted) = (change.key.join("."), one.property.wanted());
      return Err(Error::Value { key, wanted, value: value.clone() });
    }
    let path = self.file(change.layer);
    let (mut config, before) = read(&path)?;
    changed(&mut config, &path, &change.key, change.value)?;
    let mut text = serde_json::to_vec_pretty(&config)
      .map_err(|source| Error::Parse { path: path.clone(), source })?;
    text.push(b'\n');
    disk::write(&path, &text, Access::Owner)?;
    let configured = Catalog::loaded().await.configure(&self.project, Some(&self.user)).await;
    let Err(refused) = configured else { return Ok(()) };
    match restored(&path, before.as_deref()) {
      Ok(()) => Err(Error::Refused(refused)),
      Err(source) => Err(Error::Restore { refused, source }),
    }
  }
}

/// The default of each key that the file of keys of the extension `name` gives, which holds when
/// no config gives the key.
pub fn defaults(name: &str, text: &str) -> Result<Map<String, Value>, Error> {
  let keys: Keys =
    serde_json::from_str(text).map_err(|source| Error::Schema { name: name.to_owned(), source })?;
  Ok(keys.properties.into_iter().map(|(one, property)| (one, property.default)).collect())
}

/// The settings of keys with no property of their own, each with the layer that gives its value.
fn settings(won: IndexMap<Vec<String>, (Value, Layer)>) -> Vec<Setting> {
  let one = |(key, (value, layer))| Setting {
    key,
    value,
    layer: Some(layer),
    property: None,
    extension: None,
  };
  won.into_iter().map(one).collect()
}

/// A config as an object, and its text, or an empty object and no text when it is not on the disk.
fn read(path: &Path) -> Result<(Map<String, Value>, Option<String>), Error> {
  let text = match fs::read_to_string(path) {
    Err(no) if no.kind() == io::ErrorKind::NotFound => return Ok((Map::new(), None)),
    got => got.map_err(|source| Error::Read { path: path.to_owned(), source })?,
  };
  let parsed =
    serde_json::from_str(&text).map_err(|source| Error::Parse { path: path.to_owned(), source })?;
  let Value::Object(config) = parsed else {
    return Err(Error::NotObject { path: path.to_owned() });
  };
  Ok((config, Some(text)))
}

fn changed(
  config: &mut Map<String, Value>,
  path: &Path,
  key: &[String],
  value: Option<Value>,
) -> Result<(), Error> {
  let (held, name) = match key {
    [name] => (config, name),
    [outer, name] => {
      let within = config.entry(outer.clone()).or_insert_with(|| Value::Object(Map::new()));
      let Value::Object(within) = within else {
        return Err(Error::Within { path: path.to_owned(), name: outer.clone() });
      };
      (within, name)
    }
    _ => return Err(Error::Key(key.to_vec())),
  };
  match (value, held.get_mut(name)) {
    (None, _) => {
      held.shift_remove(name);
    }
    (Some(Value::Bool(on)), Some(Value::Object(fields))) if key[0] == PROVIDERS => {
      switched(fields, on);
    }
    (Some(value), _) => {
      held.insert(name.clone(), value);
    }
  }
  Ok(())
}

/// A config put back as its text was, or taken away when it was not on the disk.
fn restored(path: &Path, before: Option<&str>) -> Result<(), disk::Error> {
  match before {
    Some(text) => disk::write(path, text.as_bytes(), Access::Owner),
    None => fs::remove_file(path).map_err(|source| disk::Error { path: path.to_owned(), source }),
  }
}

#[cfg(test)]
#[path = "config.test.rs"]
mod test;
