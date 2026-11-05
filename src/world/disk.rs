//! The one write of a file of the World, which lands whole: the bytes go to a part file beside the
//! target, and a rename puts them in its place, so a reader sees the old file or the new one and
//! never a part of either, and a failure midway, as on a full disk, leaves the old file whole.

use std::{
  fs::{self, OpenOptions},
  io::{self, ErrorKind, Write},
  path::{Path, PathBuf},
  sync::atomic::{AtomicU32, Ordering},
};

/// The count of the part files that this process made, which names the next one.
static PARTS: AtomicU32 = AtomicU32::new(0);

/// Why a file was not written.
#[derive(Debug, thiserror::Error)]
#[error("{} was not written: {source}", path.display())]
pub struct Error {
  pub path: PathBuf,
  pub source: io::Error,
}

/// Who may read the file that lands. It binds on a system with modes of files.
#[derive(Clone, Copy)]
pub(crate) enum Access {
  /// The file keeps the mode that it had, or takes the one that the system gives a new file.
  Shared,
  /// Only the user reads the file, whatever mode it had.
  Owner,
}

/// Writes `bytes` to the file that `path` names after its links, and the folders above it when
/// they are missing, so a link stays a link. A file that the user cannot write is refused as a
/// plain write refuses it. A failure leaves the old file whole and no part file beside it.
pub(crate) fn write(path: &Path, bytes: &[u8], access: Access) -> Result<(), Error> {
  filled(path, bytes, access, Write::write_all)
}

/// [`write`] with the fill of the part file given, so a test can make it fail midway.
fn filled(
  path: &Path,
  bytes: &[u8],
  access: Access,
  fill: impl FnOnce(&mut fs::File, &[u8]) -> io::Result<()>,
) -> Result<(), Error> {
  let failed = |source| Error { path: path.to_owned(), source };
  let target = linked(path).map_err(failed)?;
  if let Some(parent) = target.parent() {
    fs::create_dir_all(parent).map_err(failed)?;
  }
  let kept = match fs::metadata(&target) {
    Ok(info) => {
      OpenOptions::new().write(true).open(&target).map_err(failed)?;
      matches!(access, Access::Shared).then(|| info.permissions())
    }
    Err(no) if no.kind() == ErrorKind::NotFound => None,
    Err(no) => return Err(failed(no)),
  };
  let (part, mut opened) = beside(&target, access).map_err(failed)?;
  let done = fill(&mut opened, bytes)
    .and_then(|()| opened.sync_all())
    .and_then(|()| kept.map_or(Ok(()), |mode| opened.set_permissions(mode)));
  drop(opened);
  done.and_then(|()| fs::rename(&part, &target)).map_err(|no| {
    drop(fs::remove_file(&part));
    failed(no)
  })
}

/// A new part file beside the target: hidden, and named by the target, this process and a count
/// that passes each name that a part file left by a crash still holds.
fn beside(target: &Path, access: Access) -> io::Result<(PathBuf, fs::File)> {
  let name = target.file_name().unwrap_or_default().to_string_lossy();
  loop {
    let count = PARTS.fetch_add(1, Ordering::Relaxed);
    let part = target.with_file_name(format!(".{name}.{}.{count}.part", std::process::id()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    restrict(&mut options, access);
    match options.open(&part) {
      Err(no) if no.kind() == ErrorKind::AlreadyExists => {}
      opened => return opened.map(|file| (part, file)),
    }
  }
}

/// The file that a path names after its links, each relative link from the folder that holds it,
/// or a refusal for a chain of more than 40 links, as a plain write gives.
fn linked(path: &Path) -> io::Result<PathBuf> {
  let mut at = path.to_path_buf();
  for _ in 0..40 {
    match fs::read_link(&at) {
      Ok(to) => at = at.parent().unwrap_or(Path::new("")).join(to),
      Err(_) => return Ok(at),
    }
  }
  Err(io::Error::other("it is a chain of more than 40 links"))
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
#[path = "disk.test.rs"]
mod test;
