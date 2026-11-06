//! The thought of a model in one turn: the most tokens of it, and how a model takes them.

use serde_json::{Map, Value};

use super::model::{Asked, Model};

/// The name of the environment that gives the most tokens of thought of a model in one turn, as
/// Claude Code reads it.
pub(super) const ENV_THINKING: &str = "MAX_THINKING_TOKENS";

/// The key of a config of the project that gives the most tokens of thought of a model in one turn,
/// and the setting under which the provider tells it to the claude command line.
pub(super) const KEY_THINKING: &str = "max_thinking_tokens";

/// The most tokens of thought of a model in one turn when neither the environment nor the config
/// gives one: a model that thinks more than this rarely writes a word in the turn. A model whose
/// thought the provider cuts takes a share of it at a lower effort, as [`budgeted`] says.
pub(crate) const THINKING: u64 = 32_768;

/// The most tokens of thought of a model in one turn, as Claude Code means it, 0 turning the
/// thought off, and whether the environment of furb gave it, so the claude command line inherits
/// it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Thinking {
  pub most: u64,
  pub inherited: bool,
}

impl Default for Thinking {
  fn default() -> Self {
    Self { most: THINKING, inherited: false }
  }
}

/// The most tokens of thought of a model in one turn, as the environment gives it, which wins, else
/// as the key `max_thinking_tokens` of the configs of the project in the directory of the life
/// gives it, `.furb/config.local.json` over `.furb/config.json`, else [`THINKING`]. A value that is
/// no count of tokens is as unsaid.
pub(super) fn thinking(env: Option<&str>, directory: &std::path::Path) -> Thinking {
  let given = env.and_then(|one| one.trim().parse::<u64>().ok());
  let read = |file: std::path::PathBuf| -> Option<u64> {
    let text = std::fs::read_to_string(file).ok()?;
    serde_json::from_str::<Value>(&text).ok()?.get(KEY_THINKING)?.as_u64()
  };
  let config = || crate::extension::projected(directory).into_iter().rev().find_map(read);
  Thinking { most: given.or_else(config).unwrap_or(THINKING), inherited: env.is_some() }
}

/// How a model takes the most tokens of thought of one turn.
#[derive(Clone)]
pub(super) enum Thought {
  /// Its server streams its thought one token in each part, so the provider cuts its thought at the
  /// most and asks it again with these settings, which turn its thought off, or asks it with them
  /// from the first request when the most is 0. A setting of null takes that setting off the
  /// request.
  Off(Map<String, Value>),
  /// It takes the most as the setting `max_thinking_tokens`, which the provider gives it unless the
  /// environment of furb gave the most, which it then inherits.
  Told,
}

/// The settings with these settings put in, a setting of null taking that setting off.
pub(super) fn merged(settings: &mut Map<String, Value>, with: &Map<String, Value>) {
  for (key, value) in with {
    match value {
      Value::Null => settings.remove(key),
      one => settings.insert(key.clone(), one.clone()),
    };
  }
}

/// The most tokens of thought of one turn at an effort of its actor, for a model whose thought the
/// provider cuts: an eighth of the most at `minimal` or `low`, a quarter at `medium`, and all of it
/// at `high`, above it or with no effort, since a model that thinks to the most of `high` at each
/// level spends minutes on a turn that a lower level means to keep short. A share is one token at
/// least.
pub(super) fn budgeted(most: u64, effort: Option<&str>) -> u64 {
  match effort {
    Some("minimal" | "low") => (most / 8).max(1),
    Some("medium") => (most / 4).max(1),
    _ => most,
  }
}

/// What a reply asks a model, with the most tokens of thought of the turn as the model takes it, at
/// the effort of its actor as [`budgeted`] says for a model whose thought the provider cuts.
pub(super) fn thinks((model, mut asked): (Model, Asked), thinking: Thinking) -> (Model, Asked) {
  let mut settings = match asked.request.additional_params.take() {
    Some(Value::Object(settings)) => settings,
    _ => Map::new(),
  };
  let effort =
    asked.actor.strip_prefix(model.name.as_str()).and_then(|rest| rest.strip_prefix('/'));
  asked.thought = match &model.thought {
    Some(Thought::Off(off)) if thinking.most == 0 => {
      merged(&mut settings, off);
      None
    }
    Some(Thought::Off(off)) => Some((budgeted(thinking.most, effort), off.clone())),
    Some(Thought::Told) if !thinking.inherited => {
      settings.insert(KEY_THINKING.to_owned(), Value::from(thinking.most));
      None
    }
    _ => None,
  };
  asked.request.additional_params = (!settings.is_empty()).then_some(Value::Object(settings));
  (model, asked)
}
