//! The settings of a session as its configs give them, which the operator reads and changes from
//! the window: each key with the value that wins for it, its layer, and the property that a schema
//! gives it, from furb or from an extension.

use furb_wire::{
  config::Setting,
  protocol::{Answer, Config, Frame, Verb},
};
use std::mem;

use serde_json::Value;

use crate::{Asked, Client, Target};

/// The settings of a session while the operator reads them: its machine and its record, whether
/// they changed since its machine gave them, the settings once it gave them, and the change of one
/// that the client did not ask yet.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Settings {
  pub life: (usize, String),
  pub stale: bool,
  pub read: Option<Vec<Setting>>,
  pub change: Option<Config>,
}

impl Client {
  /// The settings of the session that the input sends to, read again, or why none shows: a new
  /// session has no life to read them from yet.
  pub fn read_settings(&mut self) -> Result<(), String> {
    let life = match self.target() {
      Target::Thread(key) => (key.0, key.1),
      Target::Session(at, life) => (at, life),
      Target::New(..) => {
        return Err("Send a first message: the settings are those of a session.".to_owned());
      }
    };
    self.settings = Some(Settings { life, stale: true, ..Settings::default() });
    Ok(())
  }

  /// The ask of a change of a setting, else of the settings that the operator reads while they
  /// changed since their machine gave them.
  pub(crate) fn settings_due(&mut self) -> Option<(usize, Verb, Asked)> {
    let settings = self.settings.as_mut()?;
    let (at, life) = (settings.life.0, settings.life.1.clone());
    if let Some(config) = settings.change.take() {
      return Some((at, Verb::Config { life, config }, Asked::Setting));
    }
    if !mem::take(&mut settings.stale) {
      return None;
    }
    Some((at, Verb::Config { life, config: Config::Get }, Asked::Settings))
  }

  /// What a machine said to the ask of the settings, or of a change of one: the settings that it
  /// gave, or none for a refusal; a change that it took reads them again, and the client says why
  /// it refused one.
  pub fn configured(&mut self, asked: &Asked, heard: Option<Frame>) {
    let Some(settings) = self.settings.as_mut() else { return };
    match (asked, heard) {
      (Asked::Settings, Some(Frame::Reply { answer: Answer::Values(values), .. })) => {
        let read = values.into_iter().map(serde_json::from_value).collect::<Result<_, _>>();
        settings.read = Some(read.unwrap_or_default());
      }
      (Asked::Settings, _) => settings.read = Some(Vec::new()),
      (_, Some(Frame::Reply { .. })) => settings.stale = true,
      (_, Some(Frame::Refused { why, .. })) => self.notice = Some(why),
      _ => {}
    }
  }

  /// A setting changed by one step, by its place among the settings, in the layer of the project:
  /// a boolean flips, and a key that takes some values only takes the next one. A key of another
  /// kind is changed in its config, and the client says so.
  pub fn step_setting(&mut self, at: usize) {
    let Some(settings) = self.settings.as_ref() else { return };
    let Some(setting) = settings.read.as_ref().and_then(|read| read.get(at)) else { return };
    let Some(value) = next(setting) else {
      let key = setting.key.join(".");
      self.notice = Some(format!("Write {key} in the config of the project, .furb/config.json."));
      return;
    };
    let set = Config::Set { key: setting.key.clone(), value: Some(value), layer: None };
    if let Some(settings) = self.settings.as_mut() {
      settings.change = Some(set);
    }
  }
}

/// The value one step after the value of a setting: the other boolean, or the value after it
/// among the values that its property names, the first after the last.
fn next(setting: &Setting) -> Option<Value> {
  let values = setting.property.as_ref().map_or(&[][..], |one| one.values.as_slice());
  match (&setting.value, values) {
    (Value::Bool(on), _) => Some(Value::Bool(!on)),
    (_, []) => None,
    (value, values) => {
      let at = values.iter().position(|one| one == value).map_or(0, |at| at + 1);
      values.get(at % values.len()).cloned()
    }
  }
}
