//! The one writer of the configs of a life, in their layers: `config.json` in the config directory
//! of the user, then `.furb/config.json` of the project, which git keeps, then
//! `.furb/config.local.json` beside it, which git ignores. Each reader reads them in this order,
//! as [`crate::skill::configs`] gives them, and a later layer wins for a key. A change that
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
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use super::{
  Catalog, catalog,
  disk::{self, Access},
  provider::{KEY_AUTOMATIC, KEY_CLASSES, KEY_THINKING, THINKING, catalog::switched},
  scratch,
};
pub use furb_wire::{
  Layer,
  config::{Kind, Property, Setting},
};

use crate::{
  skill::{self, Places, configs},
  value::Fault,
};

const LAYERS: [Layer; 3] = [Layer::User, Layer::Project, Layer::Local];

/// The keys of the file that the manifest of an extension names under `furb-config`.
#[derive(Deserialize)]
struct Keys {
  properties: IndexMap<String, Property>,
}

/// A key that a schema names, with its property and the extension that names it.
struct Named {
  key: Vec<String>,
  property: Property,
  skill: Option<String>,
}

/// The objects of furb that each reader merges name by name, which the settings give with each of
/// their names after them.
const PROVIDERS: &str = "providers";

/// The keys that furb reads.
/// Whether a name is a key of furb at the top of the configs, which no skill that gives keys of
/// its own takes as its name, since the configs hold those keys under its name.
pub(crate) fn furb_key(name: &str) -> bool {
  own().iter().any(|one| one.key[0] == name)
}

fn own() -> Vec<Named> {
  let one = |name: &str, kind, default, description: &str| Named {
    key: vec![name.to_owned()],
    property: Property {
      kind,
      default,
      description: description.to_owned(),
      values: vec![],
      scope: Layer::Project,
    },
    skill: None,
  };
  let local = |mut named: Named| {
    named.property.scope = Layer::Local;
    named
  };
  vec![
    one(
      KEY_THINKING,
      Kind::Integer,
      json!(THINKING),
      "The most tokens of thought of a model in one turn, 0 turning the thought off.",
    ),
    one(KEY_AUTOMATIC, Kind::Array, json!([]), "The models of automatic work, the first first."),
    one(
      scratch::KEY,
      Kind::Integer,
      json!(scratch::DAYS),
      "The days with no use after which the scratch directory of a record that no life holds is \
       removed: no change of any file in it, at any depth, for that many days. A directory that \
       the system refuses to remove, as one with an open file, stays. A reopen of the record \
       makes it again from the writes that the record holds; the files that commands made in it \
       are lost.",
    ),
    one(KEY_CLASSES, Kind::Object, json!({}), "The class, cheap or dear, of each model it names."),
    one(PROVIDERS, Kind::Object, json!({}), "The providers of models, each by its name."),
    // A person turns an extension on or off for this machine, in the local config of the project,
    // which git does not keep.
    local(one(
      skill::KEY,
      Kind::Object,
      json!({}),
      "The skills that a life runs, each by its name: true, false, or the path of its folder.",
    )),
  ]
}

/// The kind of the fact that a life says at each change of a key of its configs through the
/// protocol: the key, and the value that wins for it after the change. A change of `skills` that
/// turns an extension on plays it at once, as a write of a config of the project does.
pub const CONFIGURED: &str = "configured";

/// A key of a layer set to a value, or taken out when the value is none. A change that names no
/// layer writes the layer of the scope of its key. A true or a false on a provider whose value is
/// an object is [`switched`], as each reader of the providers takes it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Change {
  pub key: Vec<String>,
  #[serde(default)]
  pub value: Option<Value>,
  #[serde(default)]
  pub layer: Option<Layer>,
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
  #[error("The keys of the skills were not read: {0}")]
  Skills(#[source] Fault),
  #[error("The keys of the skill {name} are no schema of keys: {source}")]
  Schema { name: String, source: serde_json::Error },
  #[error("The key {key} takes {wanted}, and {value} is not")]
  Value { key: String, wanted: String, value: Value },
}

/// The three configs of a project, under the config directory of a user, and the claude command
/// line whose catalog a change is checked against.
#[derive(Debug, Clone)]
pub struct Layers {
  project: PathBuf,
  user: PathBuf,
  claude: Option<PathBuf>,
}

impl Layers {
  /// The configs of a project, under the config directory of the user of this process when none
  /// is given, checked against the claude command line of this machine.
  pub fn new(project: impl Into<PathBuf>, user: Option<PathBuf>) -> Self {
    let user = user.unwrap_or_else(|| Places::here().config);
    Self { project: project.into(), user, claude: None }
  }

  /// The configs checked against the claude command line at this program, as the life that holds
  /// them runs it, in place of the one of this machine.
  #[must_use]
  pub fn claude(self, bin: Option<PathBuf>) -> Self {
    Self { claude: bin, ..self }
  }

  /// The places of the extensions that these configs turn on: the config directory of their user,
  /// and the home of this process.
  pub fn places(&self) -> Places {
    Places { config: self.user.clone(), home: Places::here().home }
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
    for Named { key, property, skill } in self.schema()? {
      let (value, layer, within) = if property.kind == Kind::Object && skill.is_none() {
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
      out.push(Setting { key, value, layer, property: Some(property), skill });
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
  /// takes the name of a key of furb plays not, as [`skill::load`] says, and names no key.
  fn schema(&self) -> Result<Vec<Named>, Error> {
    let places = self.places();
    let mut named = own();
    for (name, text) in skill::contributed(&places, &self.project).map_err(Error::Skills)? {
      if furb_key(&name) {
        continue;
      }
      let Ok(keys) = serde_json::from_str::<Keys>(&text) else { continue };
      named.extend(keys.properties.into_iter().map(|(one, property)| Named {
        key: vec![name.clone(), one],
        property,
        skill: Some(name.clone()),
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
    let schema = self.schema()?;
    let named = schema.iter().find(|one| one.key == change.key);
    if let (Some(value), Some(one)) = (&change.value, named)
      && !one.property.takes(value)
    {
      let (key, wanted) = (change.key.join("."), one.property.wanted());
      return Err(Error::Value { key, wanted, value: value.clone() });
    }
    // A name in an object of furb takes the scope of its object.
    let scoped = schema.iter().filter(|one| change.key.starts_with(&one.key));
    let scope = scoped.max_by_key(|one| one.key.len()).map(|one| one.property.scope);
    let scope = scope.unwrap_or_default();
    let path = self.file(change.layer.unwrap_or(scope));
    let (mut config, before) = read(&path)?;
    changed(&mut config, &path, &change.key, change.value)?;
    let mut text = serde_json::to_vec_pretty(&config)
      .map_err(|source| Error::Parse { path: path.clone(), source })?;
    text.push(b'\n');
    disk::write(&path, &text, Access::Owner)?;
    let catalog = Catalog::loaded(self.claude.clone()).await;
    let configured = catalog.configure(&self.project, Some(&self.user)).await;
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
  let one =
    |(key, (value, layer))| Setting { key, value, layer: Some(layer), property: None, skill: None };
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
