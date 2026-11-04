//! Where the program of the claude command line stands on this machine.

use std::{
  env,
  path::{Path, PathBuf},
};

/// The name of the program of the command line.
pub(super) const PROGRAM: &str = if cfg!(windows) { "claude.exe" } else { "claude" };

/// The command line of this machine: the one `FURB_CLAUDE_BIN` names, or the first program named
/// claude on the PATH, in the local programs of the home, or, on macOS, in the newest version of
/// Claude Desktop. A `FURB_CLAUDE_BIN` that is set and empty names none, and the machine is
/// searched no more.
pub(in crate::world::provider) fn located() -> Option<PathBuf> {
  if let Some(bin) = env::var_os("FURB_CLAUDE_BIN") {
    return Some(bin).filter(|one| !one.is_empty()).map(PathBuf::from);
  }
  let mut folders: Vec<PathBuf> =
    env::split_paths(&env::var_os("PATH").unwrap_or_default()).collect();
  if let Some(home) = env::home_dir() {
    folders.push(home.join(".local/bin"));
    if cfg!(target_os = "macos") {
      let desktop = home.join("Library/Application Support/Claude/claude-code");
      let mut versions: Vec<PathBuf> = std::fs::read_dir(&desktop)
        .map(|found| found.filter_map(|one| one.ok().map(|one| one.path())).collect())
        .unwrap_or_default();
      versions.sort_by_key(|one| std::cmp::Reverse(numbered(one)));
      folders.extend(versions.iter().map(|one| one.join("claude.app/Contents/MacOS")));
    }
  }
  crate::world::program("claude", folders)
}

/// The numbers of a version, in order, which sort as a version does.
fn numbered(version: &Path) -> Vec<u64> {
  let name = version.file_name().map(|one| one.to_string_lossy().into_owned()).unwrap_or_default();
  name.split(|one: char| !one.is_ascii_digit()).filter_map(|one| one.parse().ok()).collect()
}
