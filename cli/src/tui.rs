//! The TUI: furb with no command hands the terminal to it.
//!
//! The TUI is a program of bun, which furb finds and runs with the words it takes: the program that `FURB_TUI` names,
//! the `furb-tui` on PATH, or the TUI of the checkout this furb was built from, which the bun on PATH runs. On Unix
//! furb becomes the TUI, so the TUI holds the terminal and its signals alone. On Windows, which has no such call,
//! furb waits for the TUI and ends with its code.

use std::{
  convert::Infallible,
  env,
  ffi::{OsStr, OsString},
  path::{Path, PathBuf},
  process::Command,
};

use furb::world;

use crate::{Place, Stand, error::CliError, life};

/// The TUI, run with the words it takes, in the place of furb: on the demo session, on the record and in the
/// directory furb is given, on the default actor and the roster furb is given, and with the words after `--`. The
/// TUI takes the model and the effort of the actor apart.
pub(crate) async fn launch(
  demo: bool,
  place: &Place,
  stand: &Stand,
  more: &[String],
) -> Result<Infallible, CliError> {
  let path = env::var_os("PATH").unwrap_or_default();
  let mut words = found(env::var_os("FURB_TUI"), &path, &checkout())?;
  if demo {
    words.push("--demo".into());
  }
  if let Some(record) = &place.record {
    words.extend(["--record".into(), record.into()]);
  }
  if let Some(cwd) = &place.cwd {
    words.extend(["--cwd".into(), cwd.into()]);
  }
  if let Some(to) = &stand.model {
    let actor = life::actor(to).await;
    let model = actor.model.unwrap_or_else(|| actor.name.clone());
    words.extend(["--model".into(), model.as_str().into()]);
    if let Some(effort) = actor.name.strip_prefix(&format!("{model}/")) {
      words.extend(["--effort".into(), effort.into()]);
    }
  }
  for one in &stand.roster {
    words.extend(["--roster".into(), one.into()]);
  }
  words.extend(more.iter().map(OsString::from));
  let mut command = Command::new(&words[0]);
  command.args(&words[1..]);
  handed(command).map_err(|source| CliError::Started { program: PathBuf::from(&words[0]), source })
}

/// The checkout this furb was built from: the root of the repository, above the crate of the command line.
fn checkout() -> PathBuf {
  let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
  crate_dir.parent().unwrap_or(crate_dir).to_path_buf()
}

/// The words that run the TUI, or why none runs: the program a name gives, the `furb-tui` on a PATH, or bun on that
/// PATH with the TUI of a checkout that is built.
fn found(
  named: Option<OsString>,
  path: &OsStr,
  checkout: &Path,
) -> Result<Vec<OsString>, CliError> {
  if let Some(named) = named.filter(|one| !one.is_empty()) {
    return Ok(vec![named]);
  }
  if let Some(tui) = world::program("furb-tui", env::split_paths(path)) {
    return Ok(vec![tui.into()]);
  }
  let cli = checkout.join("tui/src/cli.ts");
  if !cli.is_file() {
    return Err(CliError::NoTui);
  }
  let Some(bun) = world::program("bun", env::split_paths(path)) else {
    return Err(CliError::NoBun(checkout.to_owned()));
  };
  if !checkout.join("binding/typescript/dist").is_dir() {
    return Err(CliError::Unbuilt(checkout.to_owned()));
  }
  Ok(vec![bun.into(), "run".into(), cli.into()])
}

/// The TUI given the terminal: furb becomes it, so it holds the terminal and its signals alone.
#[cfg(unix)]
fn handed(mut command: Command) -> std::io::Result<Infallible> {
  use std::os::unix::process::CommandExt;
  Err(command.exec())
}

/// The TUI given the terminal: furb lets the console give Ctrl+C and Ctrl+Break to the TUI alone, waits for it, and
/// ends with its code.
#[cfg(windows)]
fn handed(mut command: Command) -> std::io::Result<Infallible> {
  unsafe extern "system" fn held(_: u32) -> windows_sys::core::BOOL {
    1
  }
  // SAFETY: the handler is a function of this program that reads nothing and holds for as long as the process lives.
  unsafe {
    windows_sys::Win32::System::Console::SetConsoleCtrlHandler(Some(held), 1);
  }
  let status = command.status()?;
  std::process::exit(status.code().unwrap_or(1))
}

#[cfg(test)]
#[path = "tui.test.rs"]
mod test;
