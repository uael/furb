//! The settings of a session as its configs give them, which the operator reads and changes from
//! the window: each key with the value that wins for it, its layer, and the property that a schema
//! gives it, from furb or from a skill, or each skill of the session with how it plays, which he
//! turns on or off.

use furb_wire::{
  card::Skill,
  config::{Kind, SKILLS, Setting},
  protocol::{Answer, Config, Frame, Verb},
};
use std::mem;

use serde_json::Value;

use crate::{Asked, Client, Target};

/// The settings of a session while the operator reads them: its machine and its record, whether
/// he reads its skills in place of its keys, whether they changed since its machine gave them, the
/// keys or the skills once it gave them, and the change of one that the client did not ask yet.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Settings {
  pub life: (usize, String),
  pub skills: bool,
  pub stale: bool,
  pub read: Option<Vec<Setting>>,
  pub played: Option<Vec<Skill>>,
  pub change: Option<Config>,
}

impl Client {
  /// The settings of the session that the input sends to, or its skills, read again, or why none
  /// shows: a new session has no life to read them from yet.
  pub fn read_settings(&mut self, skills: bool) -> Result<(), String> {
    let life = match self.target() {
      Target::Thread(key) => (key.0, key.1),
      Target::Session(at, life) => (at, life),
      Target::New(..) => {
        return Err("Send a first message: the settings are those of a session.".to_owned());
      }
    };
    self.settings = Some(Settings { life, skills, stale: true, ..Settings::default() });
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
    if settings.skills {
      return Some((at, Verb::Plays { life }, Asked::Plays));
    }
    Some((at, Verb::Config { life, config: Config::Get }, Asked::Settings))
  }

  /// What a machine said to the ask of the settings, or of a change of one: the settings that it
  /// gave, or none for a refusal; a change that it took reads them again, and the client says why
  /// it refused one. The change that turns Codex on ends its sign-in.
  pub fn configured(&mut self, asked: &Asked, heard: Option<Frame>) {
    if *asked == Asked::SignIn {
      return self.signed_in(heard);
    }
    let Some(settings) = self.settings.as_mut() else { return };
    match (asked, heard) {
      (Asked::Settings, Some(Frame::Reply { answer: Answer::Values(values), .. })) => {
        let read = values.into_iter().map(serde_json::from_value).collect::<Result<_, _>>();
        settings.read = Some(read.unwrap_or_default());
      }
      (Asked::Settings, _) => settings.read = Some(Vec::new()),
      (Asked::Plays, Some(Frame::Reply { answer: Answer::Skills(read), .. })) => {
        settings.played = Some(read);
      }
      (Asked::Plays, _) => settings.played = Some(Vec::new()),
      (_, Some(Frame::Reply { .. })) => settings.stale = true,
      (_, Some(Frame::Refused { why, .. })) => self.notice = Some(why),
      _ => {}
    }
  }

  /// The skill at a place among the skills that play, once its machine gave them.
  pub fn skill_at(&self, at: usize) -> Option<&Skill> {
    self.settings.as_ref()?.played.as_ref()?.get(at)
  }

  /// A skill with code turned off when the configs turn it on, else on, in the layer of the scope
  /// of its key, by its place among the skills. Whether it turned: a skill with no code plays no
  /// word.
  pub fn turn_skill(&mut self, at: usize) -> bool {
    let Some(one) = self.skill_at(at).filter(|one| !one.turned_by.is_null()) else { return false };
    let key = vec![SKILLS.to_owned(), one.name.clone()];
    let value = if one.on { Value::Bool(false) } else { one.turned_by.clone() };
    if let Some(settings) = self.settings.as_mut() {
      settings.change = Some(Config::Set { key, value: Some(value), layer: None });
    }
    true
  }

  /// The setting at a place among the settings, once its machine gave them.
  pub fn setting(&self, at: usize) -> Option<&Setting> {
    self.settings.as_ref()?.read.as_ref()?.get(at)
  }

  /// A setting changed by one step, by its place among the settings, in the layer of the scope of
  /// its key: a boolean flips, and a key that takes some values only takes the next one. Whether
  /// it stepped: a key of another kind takes a value that he writes.
  pub fn step_setting(&mut self, at: usize) -> bool {
    let Some(value) = self.setting(at).and_then(next) else { return false };
    self.change_setting(at, value);
    true
  }

  /// A setting given the value that he wrote, as JSON, or as the text itself for a key of text or
  /// a text that is no JSON. The machine refuses a value that the property of its key does not
  /// take, and the client says why.
  pub fn write_setting(&mut self, at: usize, text: &str) {
    let Some(setting) = self.setting(at) else { return };
    let text = text.trim();
    let string = setting.property.as_ref().is_some_and(|one| one.kind == Kind::String);
    let value = serde_json::from_str(text).ok().filter(|_| !string);
    self.change_setting(at, value.unwrap_or_else(|| Value::String(text.to_owned())));
  }

  /// The text that the value of a setting is written as: a text as it stands, any other value as
  /// JSON.
  pub fn setting_text(&self, at: usize) -> String {
    match self.setting(at).map(|one| &one.value) {
      Some(Value::String(text)) => text.clone(),
      Some(value) => value.to_string(),
      None => String::new(),
    }
  }

  /// The change of a setting to a value, which the client asks next, in the layer of the scope of
  /// its key.
  fn change_setting(&mut self, at: usize, value: Value) {
    let Some(key) = self.setting(at).map(|one| one.key.clone()) else { return };
    if let Some(settings) = self.settings.as_mut() {
      settings.change = Some(Config::Set { key, value: Some(value), layer: None });
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
