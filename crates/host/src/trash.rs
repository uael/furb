//! The trash of a session: the folder `trash` beside the folder of the sessions of a project,
//! where a delete moves the record of a session and each file beside it, so the session can come
//! back.

use std::{
  fs, io,
  path::{Path, PathBuf},
  time::{SystemTime, UNIX_EPOCH},
};

use furb::world::{self, Scratch};
use serde_json::json;

/// Why a session did not move to the trash.
#[derive(Debug, thiserror::Error)]
pub(crate) enum TrashError {
  #[error("another process holds the record {}: {why}", record.display())]
  Held { record: PathBuf, why: String },
  #[error("there is no record at {}", .0.display())]
  Missing(PathBuf),
  #[error("{} cannot move to the trash: {source}", path.display())]
  Move { path: PathBuf, source: io::Error },
}

/// The record of a session and each file beside it that bears its name moved to a folder of their
/// own in the trash, under the lease of the record, with its scratch directory and `restore.json`,
/// which names where each came from: the folder in the trash. The lock file stays at its path,
/// empty, since a lookup of a renamed path can go on giving the moved file on macOS, so the lease
/// holds only while no holder moves the lock. A move that fails puts back each file that moved.
pub(crate) fn trashed(record: &Path) -> Result<PathBuf, TrashError> {
  let missing = || TrashError::Missing(record.to_path_buf());
  let (Some(sessions), Some(name)) = (record.parent(), record.file_name()) else {
    return Err(missing());
  };
  let name = name.to_string_lossy().into_owned();
  if !record.is_file() {
    return Err(missing());
  }
  let held = |why: String| TrashError::Held { record: record.to_path_buf(), why };
  let lease = world::Lease::of(record).map_err(|no| held(no.message()))?;
  let scratch = Scratch::of(record).ok().flatten().map(|one| one.path().to_path_buf());
  let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_millis());
  let folder = sessions.parent().unwrap_or(sessions).join("trash").join(format!("{name}-{stamp}"));
  fs::create_dir_all(&folder).map_err(failed(&folder))?;
  let (dotted, lock) = (format!("{name}."), format!("{name}.lock"));
  let entries = fs::read_dir(sessions).map_err(failed(sessions))?;
  let mut names: Vec<String> = entries
    .flatten()
    .map(|one| one.file_name().to_string_lossy().into_owned())
    .filter(|one| (*one == name || one.starts_with(&dotted)) && *one != lock)
    .collect();
  names.sort();
  let mut moved: Vec<(PathBuf, PathBuf)> = Vec::new();
  for one in names {
    let (from, to) = (sessions.join(&one), folder.join(&one));
    if let Err(source) = fs::rename(&from, &to) {
      // A file that cannot go back stays in the trash, where `restore.json` is not, and the
      // refusal names the first file that did not move.
      for (back, there) in moved.iter().rev() {
        fs::rename(there, back).ok();
      }
      return Err(TrashError::Move { path: from, source });
    }
    moved.push((from, to));
  }
  let kept = folder.join("scratch");
  let scratch = scratch.filter(|one| one.is_dir() && fs::rename(one, &kept).is_ok());
  let files: Vec<[String; 2]> =
    moved.iter().map(|(from, to)| [from, to].map(|one| one.display().to_string())).collect();
  let scratch = scratch.map(|one| [one.display().to_string(), kept.display().to_string()]);
  let restore = json!({ "files": files, "scratch": scratch }).to_string();
  fs::write(folder.join("restore.json"), restore).map_err(failed(&folder))?;
  drop(lease);
  Ok(folder)
}

/// The refusal of a move of the trash at a path.
fn failed(path: &Path) -> impl FnOnce(io::Error) -> TrashError {
  let path = path.to_path_buf();
  move |source| TrashError::Move { path, source }
}
