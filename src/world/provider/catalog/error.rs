//! Why the provider settings of the configs are refused.

use std::{io, path::PathBuf};

/// Why the settings of the providers cannot apply to a catalog.
/// Each text names the setting that fails.
#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("Could not read {}: {source}", .path.display())]
  Read { path: PathBuf, source: io::Error },
  #[error("Invalid config at {}: {source}", .path.display())]
  Parse { path: PathBuf, source: serde_json::Error },
  #[error("The config at {} must be an object", .path.display())]
  NotObject { path: PathBuf },
  #[error("The providers at {} must be an object", .path.display())]
  ProvidersNotObject { path: PathBuf },
  #[error("A provider name must be nonempty and contain no space, colon or slash")]
  ProviderName,
  #[error("No built-in provider is {0}")]
  Builtin(String),
  #[error("Invalid provider {name}: {source}")]
  Invalid { name: String, source: serde_json::Error },
  #[error("Invalid codex settings: {0}")]
  CodexSettings(serde_json::Error),
  #[error("The codex model_context_window must be positive")]
  CodexWindow,
  #[error("The URL of {name} is invalid")]
  Url { name: String, source: url::ParseError },
  #[error("The URL of {0} must use HTTP or HTTPS with no credential")]
  UrlScheme(String),
  #[error("The prices of each model of {0} must be nonnegative")]
  Prices(String),
  #[error(
    "The efforts of a model of {0} need the protocol chat and one or more of off, low, medium and high"
  )]
  Efforts(String),
  #[error("Each model of {0} needs an id, a positive window and a positive output limit")]
  Model(String),
  #[error("A custom provider {0} needs a URL and at least one model")]
  Custom(String),
  #[error("The claude-cli command needs a nonempty executable")]
  CommandEmpty,
  #[error("The claude-cli command must not contain a null byte")]
  CommandNull,
  #[error("The claude-cli accounts need at least one account")]
  NoAccounts,
  #[error("The claude-cli switch needs accounts")]
  SwitchWithoutAccounts,
  #[error("An account of claude-cli needs a name with no space: {0:?}")]
  AccountName(String),
  #[error("Two accounts of claude-cli are named {0}")]
  AccountTwice(String),
  #[error("The account {0} of claude-cli names a variable that is empty or holds = or a null byte")]
  AccountVariable(String),
  #[error("The account {0} of claude-cli names ~, and there is no home")]
  NoHome(String),
  #[error("A path of the account {name} of claude-cli must be absolute or start with ~: {path}")]
  Path { name: String, path: String },
}

/// Why the environment does not say how to reach a model of a provider of the network.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Unreached {
  #[error("no credential of it stands in the environment: set {}", .env.join(" or "))]
  NoCredential { env: Vec<String> },
  #[error("no project of Google stands in the environment: set GOOGLE_CLOUD_PROJECT")]
  NoProject,
  #[error("its address needs {name}: set it in the environment")]
  Unfilled { name: String },
}
