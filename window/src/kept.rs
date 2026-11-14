//! The cards that the window saw, in a file of its own. The next window shows them in its first
//! frame, before a machine answers, and each machine then says what stands.

use std::{fs, io, path::Path};

use crate::model::Told;

/// What each machine told the window of before. A file that is missing, or that this build cannot
/// read, keeps nothing: the machines tell their cards again.
pub(crate) fn kept(file: &Path) -> Vec<Told> {
  fs::read(file).ok().and_then(|text| serde_json::from_slice(&text).ok()).unwrap_or_default()
}

/// What each machine told, kept. The file takes its place whole, so no window reads half of it.
pub(crate) fn keep(file: &Path, machines: &[Told]) -> io::Result<()> {
  if let Some(folder) = file.parent() {
    fs::create_dir_all(folder)?;
  }
  let new = file.with_extension("new");
  fs::write(&new, serde_json::to_vec(machines)?)?;
  fs::rename(new, file)
}
