//! The settings of the session of the input, as a list part: each key with what it does, the
//! value that wins for it and the layer that gives it. Enter flips a boolean or takes the next of
//! the values of a key that takes some values only. Its skills and extensions, as a list part too:
//! enter turns an extension on or off, and adds the mention of a skill with no code to the draft.

use std::borrow::Cow;

use furb::view::Skill;
use furb::world::config::{Layer, Property, Setting};
use serde_json::Value;
use unicode_width::UnicodeWidthStr;

use crate::{
  model::Model,
  pick::{Choice, Entry},
};

impl Model {
  /// The rows of the settings, once their machine gave them.
  pub(crate) fn settings(&self) -> Vec<Entry> {
    let Some(settings) = self.client.settings.as_ref() else { return Vec::new() };
    let Some(read) = settings.read.as_ref() else {
      return vec![Entry { left: "Reading the settings…".to_owned(), ..Entry::default() }];
    };
    // The layers take the room of the widest, so the values end in one column.
    let layers = read.iter().map(|one| layer(one, read).len()).max().unwrap_or_default();
    let row = |(place, one): (usize, &Setting)| {
      let says = one.property.as_ref().map(|one| format!("  {}", one.description));
      // Each kit names its keys, a name in an object of furb stands with furb, and a key that no
      // schema names stands apart.
      let furb =
        |key: &[String]| read.iter().any(|two| two.key == key[..1] && two.property.is_some());
      let part = match &one.skill {
        Some(kit) => Cow::Owned(kit.clone()),
        None if one.property.is_some() || furb(&one.key) => Cow::Borrowed("furb"),
        None => Cow::Borrowed("No schema"),
      };
      Entry {
        part,
        left: one.key.join("."),
        says: says.unwrap_or_default(),
        right: stands(one, (read, layers)),
        choice: Choice::Setting(place),
        ..Entry::default()
      }
    };
    read.iter().enumerate().map(row).collect()
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
  /// the value that wins for it now with the layer that gives it.
  pub(crate) fn value(&self, place: usize) -> Vec<Entry> {
    let Some(one) = self.client.setting(place) else { return Vec::new() };
    let read =
      self.client.settings.as_ref().and_then(|one| one.read.as_deref()).unwrap_or_default();
    let property = one.property.as_ref();
    let takes = property.map_or_else(|| "any value of JSON".to_owned(), Property::wanted);
    let says = property.map_or_else(String::new, |one| format!("{} ", one.description));
    vec![Entry {
      left: one.key.join("."),
      says: format!("  {says}It takes {takes}."),
      right: stands(one, (read, 0)),
      choice: Choice::Value(place),
      ..Entry::default()
    }]
  }
}

/// The value that wins for a setting and the layer that gives it in a count of columns at least,
/// as `32768  default`, among the settings that it stands with. An empty value reads `none`, as the
/// description of a key that takes one says it.
fn stands(one: &Setting, (read, layers): (&[Setting], usize)) -> String {
  let value = match &one.value {
    Value::Null => "none".to_owned(),
    Value::String(text) if text.is_empty() => "none".to_owned(),
    value => value.to_string(),
  };
  format!("{value}  {:<layers$}", layer(one, read))
}

/// The layer that gives the value of a setting. An object of furb whose names come from the keys
/// under it, as the skills, takes the one layer that gives each of them, and no word when they
/// come from more than one, since each of their rows says its own.
fn layer(one: &Setting, read: &[Setting]) -> &'static str {
  let under =
    read.iter().filter(|two| two.key.len() > one.key.len() && two.key.starts_with(&one.key));
  let under: Vec<Option<Layer>> = under.map(|two| two.layer).collect();
  let layer = match under.split_first() {
    Some((first, rest)) if one.layer.is_none() => {
      rest.iter().all(|two| two == first).then_some(*first)
    }
    _ => Some(one.layer),
  };
  match layer {
    None => "",
    Some(Some(Layer::User)) => "user",
    Some(Some(Layer::Project)) => "project",
    Some(Some(Layer::Local)) => "local",
    Some(None) => "default",
  }
}
