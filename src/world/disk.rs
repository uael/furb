//! The one write of a file of the World, which lands whole: the bytes go to a part file beside the
//! target, and a rename puts them in its place, so a reader sees the old file or the new one and
//! never a part of either, and a failure midway, as on a full disk, leaves the old file whole. A
//! part file that a kill left, which no write of its process removes, goes at the first write of a
//! later process into its folder.

#[cfg(unix)]
use rustix::{
  io::Errno,
  process::{Pid, test_kill_process},
};
use std::process;
use std::{
  collections::BTreeSet,
  fs::{self, OpenOptions},
  io::{self, ErrorKind, Write},
  path::{Path, PathBuf},
  sync::{
    Mutex, PoisonError,
    atomic::{AtomicU32, Ordering},
  },
};
#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::{FILE_SHARE_READ, FILE_SHARE_WRITE};

/// The count of the part files that this process made, which names the next one.
static PARTS: AtomicU32 = AtomicU32::new(0);

/// The folders that this process swept, as [`swept`] says.
static SWEPT: Mutex<BTreeSet<PathBuf>> = Mutex::new(BTreeSet::new());

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
  filled(path, access, |file| file.write_all(bytes))
}

/// [`write`] with the fill of the part file given, for a file whose bytes no one buffer holds: the
/// fill writes them in turn, and may go back in the part file to write what it knows last.
pub(crate) fn filled(
  path: &Path,
  access: Access,
  fill: impl FnOnce(&mut fs::File) -> io::Result<()>,
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
  swept(&target);
  let (part, mut opened) = beside(&target, access, &PARTS).map_err(failed)?;
  let done = fill(&mut opened)
    .and_then(|()| opened.sync_all())
    .and_then(|()| kept.map_or(Ok(()), |mode| opened.set_permissions(mode)));
  drop(opened);
  done.and_then(|()| fs::rename(&part, &target)).map_err(|no| {
    drop(fs::remove_file(&part));
    failed(no)
  })
}

/// A new part file beside the target: hidden, and named by the target, this process and the next
/// of `counts`, which passes each name that a part file left by a crash still holds.
fn beside(target: &Path, access: Access, counts: &AtomicU32) -> io::Result<(PathBuf, fs::File)> {
  let name = target.file_name().unwrap_or_default().to_string_lossy();
  loop {
    let count = counts.fetch_add(1, Ordering::Relaxed);
    let part = target.with_file_name(format!(".{name}.{}.{count}.part", process::id()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    restrict(&mut options, access);
    match options.open(&part) {
      Err(no) if no.kind() == ErrorKind::AlreadyExists => {}
      opened => return opened.map(|file| (part, file)),
    }
  }
}

/// Removes each part file of the folder of a target whose writer runs no more, as one that a kill
/// cut midway, at the first write of this process into that folder. A part is left only by a
/// process that ended, so a later process finds it, and one listing for each folder keeps the cost
/// of a write the same in a large folder as in a small one. A part that does not go, as one whose
/// write goes on, stays until a later process writes there.
fn swept(target: &Path) {
  let Some(folder) = target.parent() else { return };
  if !SWEPT.lock().unwrap_or_else(PoisonError::into_inner).insert(folder.to_owned()) {
    return;
  }
  let Ok(entries) = fs::read_dir(folder) else { return };
  for entry in entries.flatten() {
    let file = entry.file_name();
    if file.to_str().and_then(writer).is_some_and(gone) {
      drop(fs::remove_file(entry.path()));
    }
  }
}

/// The process that wrote a part file, as [`beside`] names it: `.<target>.<pid>.<count>.part`.
fn writer(file: &str) -> Option<u32> {
  let rest = file.strip_prefix('.')?.strip_suffix(".part")?;
  let (rest, count) = rest.rsplit_once('.')?;
  let (target, pid) = rest.rsplit_once('.')?;
  count.parse::<u32>().ok()?;
  (!target.is_empty()).then(|| pid.parse().ok()).flatten()
}

/// Whether the writer that a pid names runs no more. A pid that a new process took again keeps the
/// part of the process before it until that one ends too.
#[cfg(unix)]
fn gone(pid: u32) -> bool {
  let pid = i32::try_from(pid).ok().and_then(Pid::from_raw);
  pid.is_none_or(|pid| test_kill_process(pid) == Err(Errno::SRCH))
}

/// On Windows each writer holds its part open with no share of delete, as [`restrict`] opens it,
/// so the remove of a part whose write goes on fails, and no pid is asked.
#[cfg(windows)]
fn gone(_: u32) -> bool {
  true
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

#[cfg(windows)]
fn restrict(options: &mut OpenOptions, _: Access) {
  use std::os::windows::fs::OpenOptionsExt;
  options.share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE);
}

#[cfg(test)]
#[path = "disk.test.rs"]
mod test;
