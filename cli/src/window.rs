//! The window: furb with no command hands the terminal to it.
//!
//! The window is the program `furb-window`, which furb finds and runs with no word: the program
//! that `FURB_WINDOW` names, the `furb-window` beside furb, the one on PATH, or the one that the
//! checkout this furb was built from builds with `cargo build -p furb-window`. On Unix furb becomes
//! the window, so the window holds the terminal and its signals alone. On Windows, which has no
//! such call, furb waits for the window and ends with its code.

#[cfg(windows)]
use std::process::exit;
use std::{
  convert::Infallible,
  env,
  ffi::{OsStr, OsString},
  io,
  path::{Path, PathBuf},
  process::Command,
};

use furb::world;
#[cfg(windows)]
use windows_sys::{Win32::System::Console::SetConsoleCtrlHandler, core::BOOL};

use crate::error::CliError;

/// The name of the program of the window.
const PROGRAM: &str = "furb-window";

/// The window, in the place of furb.
pub(crate) fn launch() -> Result<Infallible, CliError> {
  let path = env::var_os("PATH").unwrap_or_default();
  // A furb that cannot name its own path looks for the window on PATH and in its checkout alone.
  let furb = env::current_exe().ok();
  let beside = furb.as_deref().and_then(Path::parent);
  let program = found(env::var_os("FURB_WINDOW"), beside, &path, &checkout())?;
  handed(Command::new(&program)).map_err(|source| CliError::Started { program, source })
}

/// The checkout this furb was built from: the root of the repository, above the crate of the
/// command line.
fn checkout() -> PathBuf {
  let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
  crate_dir.parent().unwrap_or(crate_dir).to_path_buf()
}

/// The program of the window, or why none runs: the program a name gives, the `furb-window` in the
/// folder of furb or on a PATH, or the one that a checkout built.
fn found(
  named: Option<OsString>,
  beside: Option<&Path>,
  path: &OsStr,
  checkout: &Path,
) -> Result<PathBuf, CliError> {
  if let Some(named) = named.filter(|one| !one.is_empty()) {
    return Ok(named.into());
  }
  let near = beside.map(Path::to_path_buf).into_iter().chain(env::split_paths(path));
  if let Some(window) = world::program(PROGRAM, near) {
    return Ok(window);
  }
  if let Some(built) = world::program(PROGRAM, [checkout.join("target").join("debug")]) {
    return Ok(built);
  }
  if checkout.join("window").join("Cargo.toml").is_file() {
    return Err(CliError::Unbuilt(checkout.to_owned()));
  }
  Err(CliError::NoWindow)
}

/// The window given the terminal: furb becomes it, so it holds the terminal and its signals alone.
#[cfg(unix)]
fn handed(mut command: Command) -> io::Result<Infallible> {
  use std::os::unix::process::CommandExt;
  Err(command.exec())
}

/// The window given the terminal: furb lets the console give Ctrl+C and Ctrl+Break to the window
/// alone, waits for it, and ends with its code.
#[cfg(windows)]
#[expect(
  unsafe_code,
  reason = "the handler of the console is a call of Windows that no safe crate of the lock makes"
)]
fn handed(mut command: Command) -> io::Result<Infallible> {
  unsafe extern "system" fn held(_: u32) -> BOOL {
    1
  }
  // SAFETY: the handler is a function of this program that reads nothing and holds for as long as
  // the process lives.
  unsafe {
    SetConsoleCtrlHandler(Some(held), 1);
  }
  let status = command.status()?;
  exit(status.code().unwrap_or(1))
}

#[cfg(test)]
#[path = "window.test.rs"]
mod test;
