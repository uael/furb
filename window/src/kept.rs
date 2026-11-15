//! What the window keeps in files of its own: the cards that it saw, which the next window shows
//! in its first frame before a machine answers, and the drafts that the operator emptied.

use std::{
  fs, io,
  path::{Path, PathBuf},
};

use serde::{Serialize, de::DeserializeOwned};

/// What a file kept. A file that is missing, or that this build cannot read, keeps nothing: the
/// machines tell their cards again.
pub(crate) fn kept<T: DeserializeOwned + Default>(file: &Path) -> T {
  fs::read(file).ok().and_then(|text| serde_json::from_slice(&text).ok()).unwrap_or_default()
}

/// A value kept in a file. The file takes its place whole, so no window reads half of it.
pub(crate) fn keep(file: &Path, value: &impl Serialize) -> io::Result<()> {
  if let Some(folder) = file.parent() {
    fs::create_dir_all(folder)?;
  }
  let new = file.with_extension("new");
  fs::write(&new, serde_json::to_vec(value)?)?;
  fs::rename(new, file)
}

/// The file of the drafts that the operator emptied, beside the file of the cards.
pub(crate) fn stash_of(cards: &Path) -> PathBuf {
  cards.with_file_name("stash.json")
}
