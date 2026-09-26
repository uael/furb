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

use crate::Place;

/// How to get a TUI when none is found.
const NONE: &str = "no TUI is found: set FURB_TUI to the program of the furb TUI, put furb-tui on PATH, or run furb \
                    from a checkout of furb after `bun install && bun run build`";

/// The TUI, run with the words it takes, in the place of furb: on the demo session, on the record and in the
/// directory furb is given, and with the words after `--`.
pub fn launch(demo: bool, place: &Place, more: &[String]) -> Result<Infallible, String> {
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
  words.extend(more.iter().map(OsString::from));
  let mut command = Command::new(&words[0]);
  command.args(&words[1..]);
  handed(command).map_err(|no| format!("{} did not start: {no}", words[0].display()))
}

/// The checkout this furb was built from: the root of the repository, above the crate of the command line.
fn checkout() -> PathBuf {
  let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
  crate_dir.parent().unwrap_or(crate_dir).to_path_buf()
}

/// The words that run the TUI, or why none runs: the program a name gives, the `furb-tui` on a PATH, or bun on that
/// PATH with the TUI of a checkout that is built.
fn found(named: Option<OsString>, path: &OsStr, checkout: &Path) -> Result<Vec<OsString>, String> {
  if let Some(named) = named.filter(|one| !one.is_empty()) {
    return Ok(vec![named]);
  }
  if let Some(tui) = program("furb-tui", path) {
    return Ok(vec![tui.into()]);
  }
  let cli = checkout.join("tui/src/cli.ts");
  if !cli.is_file() {
    return Err(NONE.to_owned());
  }
  let at = checkout.display();
  let Some(bun) = program("bun", path) else {
    return Err(format!(
      "the TUI of {at} runs on bun 1.4.2 or later, and no bun is on PATH: see https://bun.sh"
    ));
  };
  if !checkout.join("bind/typescript/dist").is_dir() {
    return Err(format!("the TUI of {at} is not built: run `bun install && bun run build` there"));
  }
  Ok(vec![bun.into(), "run".into(), cli.into()])
}

/// The first program of a name on a PATH, with the extensions of a program on Windows.
fn program(name: &str, path: &OsStr) -> Option<PathBuf> {
  let names: &[&str] = if cfg!(windows) { &[".exe", ".cmd", ""] } else { &[""] };
  env::split_paths(path)
    .filter(|dir| !dir.as_os_str().is_empty())
    .flat_map(|dir| names.iter().map(move |end| dir.join(format!("{name}{end}"))))
    .find(|one| runs(one))
}

/// Whether a path is a program that this machine runs.
fn runs(path: &Path) -> bool {
  let Ok(info) = path.metadata() else { return false };
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    info.is_file() && info.permissions().mode() & 0o111 != 0
  }
  #[cfg(not(unix))]
  {
    info.is_file()
  }
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
