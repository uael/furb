//! The configs of furb as a client reads them: the layers, which a client names when it sets a
//! key, and each setting with the property that a schema gives it.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The key of a config that turns the skills on and off, each by its name: true, false, or the
/// path of its folder.
pub const SKILLS: &str = "skills";

/// A layer of the configs, from the one that wins least to the one that wins most.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Layer {
  User,
  #[default]
  Project,
  Local,
}

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
  pub skill: Option<String>,
}

/// A key that a schema names, in the form of a setting of VS Code: the kind of its value, the
/// value that holds when no config gives one, what it does, under `enum` the values that it takes
/// when it takes only some, and under `scope` the layer that a change of it writes, the project
/// unless it names another, as the scope of a setting of VS Code says where it applies.
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
  #[serde(default)]
  pub scope: Layer,
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
  /// Whether the property takes a value: of its kind, and among its values when it names some.
  pub fn takes(&self, value: &Value) -> bool {
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
  pub fn wanted(&self) -> String {
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
