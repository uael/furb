//! The settings of the session of the input, as a list part: each key with what it does, the
//! value that wins for it and the layer that gives it. Enter flips a boolean or takes the next of
//! the values of a key that takes some values only.

use furb::world::config::{Layer, Setting};

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
      Entry {
        part: if one.skill.is_some() { "Extensions" } else { "furb" },
        left: one.key.join("."),
        says: says.unwrap_or_default(),
        right: format!("{}  {layer}", one.value),
        choice: Choice::Setting(place),
        ..Entry::default()
      }
    };
    read.iter().enumerate().map(row).collect()
  }
}
