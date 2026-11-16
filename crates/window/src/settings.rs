//! The settings of the session of the input, as a list part: each key with what it does, the
//! value that wins for it and the layer that gives it. Enter flips a boolean or takes the next of
//! the values of a key that takes some values only. Its skills and extensions, as a list part too:
//! enter turns an extension on or off, and adds the mention of a skill with no code to the draft.

use std::borrow::Cow;

use crate::cells::Cells;
use furb::view::Skill;
use furb::world::config::{Layer, Property, Setting};
use serde_json::Value;

use crate::{
  model::Model,
  pick::{Choice, Entry},
};

impl Model {
  /// The rows of the settings, once their machine gave them, one for each key that holds a value:
  /// an object of furb whose names come from the keys under it, as the skills, stands as those
  /// keys, each of which says what the object says.
  pub(crate) fn settings(&self) -> Vec<Entry> {
    let Some(settings) = self.client.settings.as_ref() else { return Vec::new() };
    let Some(read) = settings.read.as_ref() else {
      return vec![Entry { left: "Reading the settings…".to_owned(), ..Entry::default() }];
    };
    let under = |one: &[String]| read.iter().find(|two| two.key == one && two.property.is_some());
    let over = |one: &Setting| {
      read.iter().any(|two| two.key.len() > one.key.len() && two.key.starts_with(&one.key))
    };
    // The layers take the room of the widest, so the values and the layers each end in one column.
    let layers = read.iter().map(|one| layer(one).len()).max().unwrap_or_default();
    let row = |(place, one): (usize, &Setting)| {
      let property = one.property.as_ref().or_else(|| under(&one.key[..1])?.property.as_ref());
      let says = property.map(|one| format!("  {}", one.description));
      // Each kit names its keys, a name in an object of furb stands with furb, and a key that no
      // schema names stands apart.
      let part = match &one.skill {
        Some(kit) => Cow::Owned(kit.clone()),
        None if property.is_some() => Cow::Borrowed("furb"),
        None => Cow::Borrowed("No schema"),
      };
      Entry {
        part,
        left: one.key.join("."),
        says: says.unwrap_or_default(),
        right: stands(one, layers),
        choice: Choice::Setting(place),
        ..Entry::default()
      }
    };
    read.iter().enumerate().filter(|(_, one)| !over(one)).map(row).collect()
  }

  /// The rows of the skills, once their machine gave them: the extensions that play first, then
  /// those that play on need, then the skills with no code, each with what it is for, how it plays
  /// and where it comes from.
  pub(crate) fn played(&self) -> Vec<Entry> {
    let Some(settings) = self.client.settings.as_ref() else { return Vec::new() };
    let Some(read) = settings.played.as_ref() else {
      return vec![Entry { left: "Reading the extensions…".to_owned(), ..Entry::default() }];
    };
    // Where each comes from takes the room of the widest, so how each plays ends in one column.
    let froms = read.iter().map(|one| one.from.width()).max().unwrap_or_default();
    let row = |(place, one): (usize, &Skill)| {
      let (part, stands) = match (!one.turned_by.is_null(), one.plays, one.on) {
        (false, ..) => ("Skills", "read on need"),
        (true, true, true) if one.default => ("Play", "on by default"),
        (true, true, true) => ("Play", "on"),
        (true, true, false) => ("Play", "on · off at the next session"),
        (true, false, true) => ("On need", "off · on at the next session"),
        (true, false, false) => ("On need", "off"),
      };
      // An extension that plays reads bright, and one that changes at the next session takes the
      // accent, as the model in use does in Models, so the eye finds what plays and what changes.
      let tone = match (one.plays, one.on) {
        _ if one.turned_by.is_null() => None,
        (true, true) => Some(self.tokens.text),
        (true, false) | (false, true) => Some(self.tokens.accent),
        (false, false) => None,
      };
      Entry {
        part: Cow::Borrowed(part),
        left: one.name.clone(),
        says: format!("  {}", one.description),
        right: format!("{stands}  {:<froms$}", one.from),
        tone,
        choice: Choice::Skill(place),
        ..Entry::default()
      }
    };
    let mut rows: Vec<Entry> = read.iter().enumerate().map(row).collect();
    let order =
      |one: &Entry| ["Play", "On need", "Skills"].iter().position(|part| *part == one.part);
    rows.sort_by_key(order);
    rows
  }

  /// The row of the setting whose value he writes: its key, what it does and what it takes, and
  /// at its right the value that he writes as it would stand, as he writes it, else in the hue of a
  /// fault that its key does not take it, so he reads the refusal before he sends it.
  pub(crate) fn value(&self, place: usize) -> Vec<Entry> {
    let Some(one) = self.client.setting(place) else { return Vec::new() };
    let property = one.property.as_ref();
    let takes = property.map_or_else(|| "any value of JSON".to_owned(), Property::wanted);
    let says = property.map_or_else(String::new, |one| format!("{} ", one.description));
    let written =
      self.picker.as_ref().and_then(|one| self.client.setting_value(place, &one.filter));
    let fits = written.filter(|value| property.is_none_or(|one| one.takes(value)));
    let (right, tone) = match fits {
      Some(value) => (stands(&Setting { value, ..one.clone() }, 0), None),
      None => (format!("not {takes}"), Some(self.tokens.danger)),
    };
    vec![Entry {
      left: one.key.join("."),
      says: format!("  {says}It takes {takes}."),
      right,
      tone,
      choice: Choice::Value(place),
      ..Entry::default()
    }]
  }
}

/// The value that wins for a setting and the layer that gives it at the right of a count of
/// columns at least, as `32768  default`, each value as the control that an enter is on it: a
/// truth as a box that it ticks, as `[x] on`, and a value of a key that takes some values only
/// with the sign that an enter takes the next. An empty value reads `none`, as the description of
/// a key that takes one says it.
fn stands(one: &Setting, layers: usize) -> String {
  let value = match (&one.value, one.property.as_ref()) {
    (Value::Bool(true), _) => "[x] on".to_owned(),
    (Value::Bool(false), _) => "[ ] off".to_owned(),
    (Value::Null, _) => "none".to_owned(),
    (Value::String(text), _) if text.is_empty() => "none".to_owned(),
    (value, Some(property)) if !property.values.is_empty() => format!("{value} ›"),
    (value, _) => value.to_string(),
  };
  format!("{value}  {:>layers$}", layer(one))
}

/// The layer that gives the value of a setting.
const fn layer(one: &Setting) -> &'static str {
  match one.layer {
    Some(Layer::User) => "user",
    Some(Layer::Project) => "project",
    Some(Layer::Local) => "local",
    None => "default",
  }
}
