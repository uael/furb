//! The layers of the configs of furb, which a client names when it sets a key.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A layer of the configs, from the one that wins least to the one that wins most.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Layer {
  User,
  #[default]
  Project,
  Local,
}
