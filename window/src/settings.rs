//! The settings of the session of the input, as a list part: each key with what it does, the
//! value that wins for it and the layer that gives it. Enter flips a boolean or takes the next of
//! the values of a key that takes some values only.

use std::borrow::Cow;

use furb::world::config::{Layer, Property, Setting};

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
    let row = |(place, one): (usize, &Setting)| {
      let layer = match one.layer {
        Some(Layer::User) => "user",
        Some(Layer::Project) => "project",
        Some(Layer::Local) => "local",
        None => "default",
      };
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
        right: format!("{}  {layer}", one.value),
        choice: Choice::Setting(place),
        ..Entry::default()
      }
    };
    read.iter().enumerate().map(row).collect()
  }

  /// The row of the setting whose value he writes: its key, what it does, and what it takes.
  pub(crate) fn value(&self, place: usize) -> Vec<Entry> {
    let Some(one) = self.client.setting(place) else { return Vec::new() };
    let property = one.property.as_ref();
    vec![Entry {
      left: one.key.join("."),
      says: property.map(|one| format!("  {}", one.description)).unwrap_or_default(),
      right: property.map_or_else(|| "any value of JSON".to_owned(), Property::wanted),
      choice: Choice::Value(place),
      ..Entry::default()
    }]
  }
}
