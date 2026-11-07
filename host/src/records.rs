//! The records of the sessions of the workspaces that `workspaces.json` in a config folder names,
//! which the verb `list` gives: the `.furb/sessions` of the main checkout of each workspace and of
//! each worktree of it, as a window lists them.

use std::{
  fs,
  path::{Path, PathBuf},
};

use serde_json::Value;

/// A folder of the sessions of a checkout of a workspace of the list: its path, the main checkout
/// of its workspace, and each record that it holds.
pub(crate) struct Sessions {
  pub(crate) folder: PathBuf,
  pub(crate) workspace: String,
  pub(crate) records: Vec<String>,
}

/// Each folder of the sessions of each workspace of the list in a config folder. A list that
/// cannot be read names no workspace.
pub(crate) fn recorded(config: &Path) -> Vec<Sessions> {
  let text = fs::read_to_string(config.join("workspaces.json")).unwrap_or_default();
  let list = serde_json::from_str::<Value>(&text).unwrap_or_default();
  let saved = list["workspaces"].as_array().into_iter().flatten();
  let mut found = Vec::new();
  for main in saved.filter_map(|one| one["directory"].as_str()) {
    for checkout in checkouts(Path::new(main)) {
      let folder = checkout.join(".furb").join("sessions");
      let entries = fs::read_dir(&folder).into_iter().flatten().flatten();
      let records = entries.map(|entry| entry.path()).filter(|path| record(path));
      let records = records.map(|path| path.display().to_string()).collect();
      found.push(Sessions { folder, workspace: main.to_owned(), records });
    }
  }
  found
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
