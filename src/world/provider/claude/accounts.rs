//! The accounts that the claude command line runs as, and which of them serves a turn.
//!
//! An account takes its credential from a token that `claude setup-token` made, which lives a year and never
//! refreshes, or from a config dir where the account logged in. With one account or none, nothing is parked and the
//! command line runs as it did. With several, a turn that meets a usage limit or a failed sign in parks its account,
//! and the next turn takes another: a limit parks it until its reset, and a failed sign in until a reload.

use std::{
  env, fs,
  path::{Path, PathBuf},
  process::Command,
  sync::{Mutex, MutexGuard},
  time::SystemTime,
};

/// The variable that holds a token of `claude setup-token`.
const TOKEN: &str = "CLAUDE_CODE_OAUTH_TOKEN";

/// The variable that names the config dir of the command line.
const CONFIG: &str = "CLAUDE_CONFIG_DIR";

/// The variables that give the command line another credential than the one of its account.
const OTHERS: [&str; 4] = [
  "ANTHROPIC_API_KEY",
  "ANTHROPIC_AUTH_TOKEN",
  "CLAUDE_CODE_OAUTH_TOKEN_FILE_DESCRIPTOR",
  "CLAUDE_CODE_API_KEY_FILE_DESCRIPTOR",
];

/// The words after the name of an account in each failure to read its token, which a turn takes for a failed sign in.
pub(super) const NO_TOKEN: &str = "has no token";

/// Where an account takes its credential.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
  /// A file that holds a token of `claude setup-token`, which each process reads when it starts.
  TokenFile(PathBuf),
  /// The name of a variable of the environment of furb that holds such a token.
  TokenEnv(String),
  /// A config dir of the command line, where the account logged in.
  ConfigDir(PathBuf),
}

/// One account the command line may run as.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Account {
  pub name: String,
  pub source: Source,
}

/// How a new conversation takes its account among those that are not parked.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Switch {
  /// The first account of the list.
  #[default]
  Failover,
  /// The account that a conversation took least recently.
  Spread,
}

/// Why an account cannot serve now, and until when.
#[derive(Clone, Copy)]
enum Park {
  Free,
  Until(SystemTime),
  Reload,
}

struct State {
  parked: Vec<Park>,
  given: Vec<u64>,
  tick: u64,
}

/// The accounts of one command line, and which of them are parked. An empty list is one account, the login that the
/// environment of furb gives the command line.
pub(super) struct Accounts {
  list: Vec<Account>,
  switch: Switch,
  state: Mutex<State>,
}

impl Accounts {
  pub(super) fn new(list: Vec<Account>, switch: Switch) -> Accounts {
    let count = list.len().max(1);
    let state = State { parked: vec![Park::Free; count], given: vec![0; count], tick: 0 };
    Accounts { list, switch, state: Mutex::new(state) }
  }

  fn state(&self) -> MutexGuard<'_, State> {
    self.state.lock().unwrap_or_else(|one| one.into_inner())
  }

  /// How many accounts the command line may run as.
  pub(super) fn count(&self) -> usize {
    self.list.len().max(1)
  }

  /// Whether a turn may move to another account.
  pub(super) fn switchable(&self) -> bool {
    self.list.len() > 1
  }

  /// The name of an account, which is empty for the login of the environment.
  pub(super) fn name(&self, at: usize) -> &str {
    self.list.get(at).map_or("", |one| one.name.as_str())
  }

  /// The config dir an account keeps its conversations in, none when it uses the default one.
  pub(super) fn store(&self, at: Option<usize>) -> Option<&Path> {
    match &self.list.get(at?)?.source {
      Source::ConfigDir(dir) => Some(dir.as_path()),
      _ => None,
    }
  }

  fn free_in(state: &State, at: usize, now: SystemTime) -> bool {
    match state.parked.get(at) {
      Some(Park::Until(end)) => *end <= now,
      Some(Park::Reload) => false,
      _ => true,
    }
  }

  /// Whether an account is not parked at this time.
  pub(super) fn free(&self, at: usize, now: SystemTime) -> bool {
    Accounts::free_in(&self.state(), at, now)
  }

  fn choose(&self, state: &State, now: SystemTime) -> Option<usize> {
    let mut free = (0..self.count()).filter(|at| Accounts::free_in(state, *at, now));
    match self.switch {
      Switch::Failover => free.next(),
      Switch::Spread => free.min_by_key(|at| state.given[*at]),
    }
  }

  /// The account a new conversation takes, which the policy chooses among those that are not parked.
  pub(super) fn pick(&self, now: SystemTime) -> Option<usize> {
    let mut state = self.state();
    let at = self.choose(&state, now)?;
    state.tick += 1;
    let tick = state.tick;
    state.given[at] = tick;
    Some(at)
  }

  /// The account the next conversation would take, which the policy does not count as given.
  pub(super) fn peek(&self, now: SystemTime) -> Option<usize> {
    self.choose(&self.state(), now)
  }

  /// The account that a command of no conversation runs as: the first that is not parked.
  pub(super) fn first_free(&self, now: SystemTime) -> usize {
    let state = self.state();
    (0..self.count()).find(|at| Accounts::free_in(&state, *at, now)).unwrap_or(0)
  }

  /// Park an account until a time, or until a new command line stands when no time is given.
  pub(super) fn park(&self, at: usize, until: Option<SystemTime>) {
    if let Some(slot) = self.state().parked.get_mut(at) {
      *slot = until.map_or(Park::Reload, Park::Until);
    }
  }

  /// The first time that an account parked until a limit ends.
  pub(super) fn earliest(&self, now: SystemTime) -> Option<SystemTime> {
    let state = self.state();
    state
      .parked
      .iter()
      .filter_map(|one| match one {
        Park::Until(end) if *end > now => Some(*end),
        _ => None,
      })
      .min()
  }

  /// Give a command the credential of an account, and take away each other one that would win over it. It fails with
  /// the reason when the token cannot be read, and the reason never holds a token.
  pub(super) fn apply(&self, at: usize, command: &mut Command) -> Result<(), String> {
    let Some(account) = self.list.get(at) else { return Ok(()) };
    for name in OTHERS {
      command.env_remove(name);
    }
    let name = &account.name;
    let none = |what: String| format!("The account {name} {NO_TOKEN}: {what}");
    let token = match &account.source {
      Source::ConfigDir(dir) => {
        command.env_remove(TOKEN).env(CONFIG, dir);
        return Ok(());
      }
      Source::TokenFile(file) => {
        let said = fs::read_to_string(file)
          .map_err(|no| none(format!("its token file {} cannot be read: {no}", file.display())))?;
        (said.trim().to_owned(), format!("its token file {} is empty", file.display()))
      }
      Source::TokenEnv(var) => {
        let said = env::var(var).unwrap_or_default();
        (said.trim().to_owned(), format!("its token variable {var} is empty or not set"))
      }
    };
    if token.0.is_empty() {
      return Err(none(token.1));
    }
    command.env(TOKEN, token.0);
    Ok(())
  }
}

#[cfg(test)]
#[path = "accounts.test.rs"]
mod test;
