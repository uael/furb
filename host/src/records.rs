//! The records of the sessions of the workspaces that `workspaces.json` in a config folder names,
//! which the verb `list` gives: the `.furb/sessions` of the main checkout of each workspace and of
//! each worktree of it, as a window lists them, and those of each session with no project.

use std::{
  env, fs,
  path::{Path, PathBuf},
};

use serde_json::Value;

/// A folder of the sessions of a checkout: its path, the main checkout of its project, none for a
/// session with no project, and each record that it holds.
pub(crate) struct Sessions {
  pub(crate) folder: PathBuf,
  pub(crate) project: Option<String>,
  pub(crate) records: Vec<String>,
}

/// Each folder of the sessions of each workspace of the list in a config folder, then of each
/// session with no project in a data folder. A list that cannot be read names no workspace.
pub(crate) fn recorded(config: &Path, data: &Path) -> Vec<Sessions> {
  let text = fs::read_to_string(config.join("workspaces.json")).unwrap_or_default();
  let list = serde_json::from_str::<Value>(&text).unwrap_or_default();
  let saved = list["workspaces"].as_array().into_iter().flatten();
  let mut found = Vec::new();
  for main in saved.filter_map(|one| one["directory"].as_str()) {
    for checkout in checkouts(Path::new(main)) {
      found.push(sessions(&checkout, Some(main.to_owned())));
    }
  }
  let alone = fs::read_dir(detached(data)).into_iter().flatten().flatten();
  found.extend(alone.map(|entry| sessions(&entry.path(), None)));
  found
}

/// The data folder of furb of the user of the machine: `furb` in `XDG_DATA_HOME` when it names
/// one, else in the data folder of the system, as `~/.local/share` on Linux, `~/Library/Application
/// Support` on macOS and the local `AppData` on Windows.
pub(crate) fn data() -> PathBuf {
  let named = env::var_os("XDG_DATA_HOME").filter(|one| !one.is_empty()).map(PathBuf::from);
  named.or_else(dirs::data_local_dir).unwrap_or_default().join("furb")
}

/// The folder of the sessions with no project in a data folder: each stands in a folder of its
/// own there, which a window names, so a session of work with no code has a folder to run in.
pub(crate) fn detached(data: &Path) -> PathBuf {
  PathBuf::from(key(&data.join("sessions").display().to_string()))
}

/// The folder of the sessions of a checkout, of a project or of none.
fn sessions(checkout: &Path, project: Option<String>) -> Sessions {
  // The folder with no link in it, so each record has the one key that a life of it has.
  let folder = checkout.join(".furb").join("sessions");
  let folder = dunce::canonicalize(&folder).unwrap_or(folder);
  let entries = fs::read_dir(&folder).into_iter().flatten().flatten();
  let records = entries.map(|entry| entry.path()).filter(|path| record(path));
  let records = records.map(|path| path.display().to_string()).collect();
  Sessions { folder, project, records }
}

/// The one name of a record: its path with no link in it, as the system resolves it, so that each
/// path of one record, as `/var` and `/private/var` on macOS or a short name on Windows, gives one
/// key. The part of the path that the disk does not hold yet, as the folder of the sessions of a
/// new project and the record in it, joins the resolved part that it holds, so the key of a record
/// stays the same once the daemon made it. A relative path stays as it is given.
pub(crate) fn key(life: &str) -> String {
  let path = Path::new(life);
  let real = || {
    let (held, real) =
      path.ancestors().find_map(|one| Some((one, dunce::canonicalize(one).ok()?)))?;
    let rest = path.strip_prefix(held).ok()?;
    // A join of an empty rest would end the key with a separator.
    Some(if rest.as_os_str().is_empty() { real } else { real.join(rest) })
  };
  let real = if path.is_absolute() { real() } else { None };
  real.map_or_else(|| life.to_owned(), |one| one.display().to_string())
}

/// The checkout whose `.furb/sessions` holds a record, where a window makes the record of a new
/// session of a project that the list does not name.
pub(crate) fn checkout(record: &Path) -> Option<String> {
  let sessions = record.parent()?;
  let furb = sessions.parent()?;
  let made = sessions.file_name()? == "sessions" && furb.file_name()? == ".furb";
  made.then(|| furb.parent().map(|one| one.display().to_string())).flatten()
}

/// Whether a file is the record of a session: a `.jsonl`, and no journal of its changes.
fn record(path: &Path) -> bool {
  let kind = |path: &Path, wanted: &str| path.extension().is_some_and(|one| one == wanted);
  kind(path, "jsonl") && !path.file_stem().is_some_and(|stem| kind(Path::new(stem), "changes"))
}

/// The main checkout, then each worktree that its git folder keeps and that stands on the disk,
/// read from the `gitdir` of each, with no process of git.
fn checkouts(main: &Path) -> Vec<PathBuf> {
  let mut found = vec![main.to_path_buf()];
  let Ok(entries) = fs::read_dir(main.join(".git").join("worktrees")) else { return found };
  for entry in entries.flatten() {
    let Ok(mark) = fs::read_to_string(entry.path().join("gitdir")) else { continue };
    let Some(tree) = Path::new(mark.trim()).parent() else { continue };
    if let Ok(real) = dunce::canonicalize(tree)
      && !found.contains(&real)
    {
      found.push(real);
    }
  }
  found
}
