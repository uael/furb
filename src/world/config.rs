//! The one writer of the configs of a life, in their layers: `config.json` in the config directory
//! of the user, then `.furb/config.json` of the project, which git keeps, then
//! `.furb/config.local.json` beside it, which git ignores. Each reader reads them in this order,
//! as [`crate::extension::configs`] gives them, and a later layer wins for a key. A change that
//! the catalog cannot read stays on no disk: the writer puts its file back as it was.

use std::{
  fs, io,
  path::{Path, PathBuf},
};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::{
  Catalog, catalog,
  disk::{self, Access},
};
use crate::extension::{Places, configs};

/// A layer of the configs, from the one that wins least to the one that wins most.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
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
  pub layer: Layer,
}

/// A key of a layer set to a value, or taken out when the value is none. A true or a false on a
/// key whose value is an object sets its field `enabled`, so a provider that is off keeps its
/// settings.
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

  /// Each key of the configs with the value that wins for it, in the order in which the layers
  /// first give the keys. A config that is not on the disk gives no key.
  pub fn get(&self) -> Result<Vec<Setting>, Error> {
    let mut won: IndexMap<Vec<String>, (Value, Layer)> = IndexMap::new();
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
    Ok(won.into_iter().map(|(key, (value, layer))| Setting { key, value, layer }).collect())
  }

  /// A change written to its layer, then the catalog of this machine configured on the layers, as
  /// a stand of a life configures it. A change that the catalog refuses puts the file back as it
  /// was, and gives why.
  pub async fn set(&self, change: Change) -> Result<(), Error> {
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
    (Some(Value::Bool(on)), Some(Value::Object(fields))) => {
      fields.insert("enabled".to_owned(), Value::Bool(on));
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
