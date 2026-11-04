//! Who may reach the daemon over the network, and with which role (#983, Q5): the file
//! `host.json` in the config folder of furb, which turns on the daemon on the tailnet and gives
//! each login its role. The daemon refuses a login that the file does not name.

use std::{
  collections::HashMap,
  fs, io,
  path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::{error::AccessError, protocol::Role};

/// The settings of the daemon that the operator writes in `host.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
  /// The ports of the daemon on the address of the machine on the tailnet. With none, the daemon
  /// listens on the machine alone.
  #[serde(default)]
  pub tailnet: Option<Ports>,
  #[serde(default)]
  pub roles: Access,
}

/// The port of each transport of the network that the daemon listens on, if it does.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ports {
  pub tcp: Option<u16>,
  pub websocket: Option<u16>,
}

/// The role of each login of the network.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct Access(HashMap<String, Role>);

impl Config {
  /// The settings in a file, or the settings that turn nothing on when no file stands there.
  pub fn load(path: &Path) -> Result<Config, AccessError> {
    let shown = || PathBuf::from(path);
    match fs::read_to_string(path) {
      Err(no) if no.kind() == io::ErrorKind::NotFound => Ok(Config::default()),
      Err(source) => Err(AccessError::Read { path: shown(), source }),
      Ok(text) => {
        serde_json::from_str(&text).map_err(|source| AccessError::Parse { path: shown(), source })
      }
    }
  }
}

impl Access {
  /// The role of a login, or its refusal when the list does not name it.
  pub fn role(&self, login: &str) -> Result<Role, AccessError> {
    self.0.get(login).copied().ok_or_else(|| AccessError::Unlisted { login: login.to_owned() })
  }
}

impl FromIterator<(String, Role)> for Access {
  fn from_iter<I: IntoIterator<Item = (String, Role)>>(roles: I) -> Self {
    Access(roles.into_iter().collect())
  }
}
