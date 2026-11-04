//! The one write of a file that lands whole: the bytes go to a file beside the target, and a
//! rename puts them in its place, so a reader sees the old file or the new one and never a part of
//! either.

use std::{
  fs::{self, OpenOptions},
  io::{self, Write},
  path::{Path, PathBuf},
};

/// Why a file was not written.
#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("{} was not written: {source}", path.display())]
  Write { path: PathBuf, source: io::Error },
  #[error("the machine gave no random bytes: {0}")]
  Random(#[from] getrandom::Error),
}

/// Who may read the file that lands. It binds on a system with modes of files.
#[derive(Clone, Copy)]
pub(crate) enum Access {
  Shared,
  Owner,
}

/// Writes `bytes` to `path`, and the folders above it when they are missing. A failure leaves the
/// old file as it was and no part file beside it.
pub(crate) fn write(path: &Path, bytes: &[u8], access: Access) -> Result<(), Error> {
  let beside = beside(path)?;
  put(&beside, path, bytes, access).map_err(|source| {
    drop(fs::remove_file(&beside));
    Error::Write { path: path.to_owned(), source }
  })
}

fn put(beside: &Path, path: &Path, bytes: &[u8], access: Access) -> io::Result<()> {
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent)?;
  }
  let mut options = OpenOptions::new();
  options.write(true).create_new(true);
  restrict(&mut options, access);
  let mut file = options.open(beside)?;
  file.write_all(bytes)?;
  file.sync_all()?;
  fs::rename(beside, path)
}

/// The path of the part file: the name of the target and a random suffix, so two writes of one
/// target, or of two targets that differ in their extension alone, never meet in one file.
fn beside(path: &Path) -> Result<PathBuf, getrandom::Error> {
  let mut bytes = [0; 8];
  getrandom::fill(&mut bytes)?;
  let suffix = format!("{:016x}", u64::from_ne_bytes(bytes));
  let mut name = path.file_name().unwrap_or_default().to_owned();
  name.push(format!(".{suffix}.part"));
  Ok(path.with_file_name(name))
}

#[cfg(unix)]
fn restrict(options: &mut OpenOptions, access: Access) {
  use std::os::unix::fs::OpenOptionsExt;
  if matches!(access, Access::Owner) {
    options.mode(0o600);
  }
}

#[cfg(not(unix))]
fn restrict(_: &mut OpenOptions, _: Access) {}

#[cfg(test)]
#[path = "atomic.test.rs"]
mod test;
