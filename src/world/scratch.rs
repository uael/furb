//! The scratch directory of a life: a folder of the machine for the notes and the scratch files of its words, which a
//! word names as `scratch://` and a command as `$TMPDIR`, so no text that the World answers names its place.

#![expect(
  clippy::too_many_arguments,
  reason = "debt: a function of more than 4 parameters takes a struct, rule 8 of the skill rust"
)]
use std::{
  collections::HashMap,
  fs, io,
  path::{Path, PathBuf},
  sync::{Arc, OnceLock},
  thread,
  time::{Duration, SystemTime, UNIX_EPOCH},
};

use sha2::{Digest, Sha256};

use super::{
  cache,
  config::Layers,
  disk::{self, Access},
  files::{filed, text},
  resolved,
};
use crate::{
  fact::named,
  value::{Fault, Object, Text, hex},
};
use std::env;
use std::process;

/// The scheme of the paths of the scratch directory.
pub const SCRATCH: &str = "scratch://";

/// The file that marks a directory as whole: made by a write or a command, or made again from its record. No listing
/// shows it, and a directory without it is made again.
const WHOLE: &str = ".furb-scratch";

/// The key of a config that says after how many days with no use the scratch directory of a record
/// that no life holds is removed, as `{"scratch_days": 7}`.
pub(super) const KEY: &str = "scratch_days";

/// The days of [`KEY`] when no config names it.
pub(super) const DAYS: u64 = 7;

/// The file beside the scratch directories of the records whose time says when they were last
/// reaped, so a life reaps them at most once a day.
const REAPED: &str = ".reaped";

/// How long a reap of the scratch directories of the records stands before the next one.
const DAY: Duration = Duration::from_hours(24);

/// The scratch directory of a life, which its files and its commands share: the one of its record, which outlives
/// the life, or one of its own, which goes with the last ear that holds it. Nothing makes it until a write or a
/// command needs it, or until a life makes it again from its record.
#[derive(Clone)]
pub struct Scratch(Arc<Held>);

/// The directory, whether it goes with the last ear that holds it, and the mark of the directory,
/// which a life holds locked while it lives, so no reap removes the directory of a live life.
struct Held {
  at: PathBuf,
  owned: bool,
  lock: OnceLock<fs::File>,
}

impl Drop for Held {
  fn drop(&mut self) {
    // A directory that cannot go stays under the temp of the machine, which the machine clears.
    if self.owned {
      fs::remove_dir_all(&self.at).unwrap_or_default();
    }
  }
}

impl Scratch {
  /// The scratch directory at a path, resolved against the working directory of the process, which outlives the
  /// life. It is refused when the path is relative and the process has no working directory.
  pub fn at(path: impl Into<PathBuf>) -> Result<Self, Fault> {
    absolute(&path.into()).map(Self::kept)
  }

  /// The scratch directory at an absolute path, which outlives the life.
  fn kept(at: PathBuf) -> Self {
    Self(Arc::new(Held { at, owned: false, lock: OnceLock::new() }))
  }

  /// The scratch directory of a record: in the cache of furb, under the digest of the canonical path of the record,
  /// so every later life on that record finds it again, however the path is spelled, and its path stays short enough
  /// for a socket that a command makes in it. Nothing when the machine has no cache of furb, and
  /// a refusal when the record is a relative path and the process has no working directory.
  pub fn of(record: &Path) -> Result<Option<Self>, Fault> {
    let path = absolute(record)?;
    let path = match (path.parent(), path.file_name()) {
      (Some(folder), Some(name)) => {
        fs::canonicalize(folder).map_or(path.clone(), |real| real.join(name))
      }
      _ => path,
    };
    let digest = Sha256::digest(path.to_string_lossy().as_bytes());
    Ok(cache().map(|cache| Self::kept(cache.join("scratch").join(hex(&digest[..8])))))
  }

  /// A scratch directory of its own, in the temporary directory of the machine under a short name, which goes when
  /// the life does. The name is drawn, or made of the process and the time when the system draws nothing.
  pub fn own() -> Self {
    let mut drawn = [0u8; 4];
    if getrandom::fill(&mut drawn).is_err() {
      let now =
        SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.subsec_nanos());
      drawn = (process::id() ^ now).to_le_bytes();
    }
    let at = env::temp_dir().join(format!("furb-{}", hex(&drawn)));
    Self(Arc::new(Held { at, owned: true, lock: OnceLock::new() }))
  }

  /// The directory on the machine.
  pub fn path(&self) -> &Path {
    &self.0.at
  }

  /// The directory, made whole when a write or a command first needs it.
  pub(crate) fn made(&self) -> Result<&Path, Fault> {
    let failed = |no| Fault::refused(format!("The scratch directory could not be made: {no}"));
    let mark = self.0.at.join(WHOLE);
    if !mark.exists() {
      fs::create_dir_all(&self.0.at).and_then(|()| fs::write(&mark, "")).map_err(failed)?;
    }
    self.hold();
    Ok(&self.0.at)
  }

  /// The mark of the directory locked for as long as the life lives, once the mark stands, and its
  /// time set to now, so a reap counts each life as a use. A mark that a reap locks or removes now,
  /// or that cannot be locked, is not held: the next make holds the mark that it makes again.
  fn hold(&self) {
    if self.0.lock.get().is_some() {
      return;
    }
    let mark = self.0.at.join(WHOLE);
    let Ok(file) = fs::OpenOptions::new().write(true).open(&mark) else { return };
    if file.try_lock_shared().is_ok() && mark.exists() {
      // A time that cannot be set only lets the bound of days count from the last make.
      drop(file.set_modified(SystemTime::now()));
      drop(self.0.lock.set(file));
    }
  }

  /// The days of the configs of the user and of the project, a later layer winning, after which a
  /// reap removes the scratch directory of a record that no life holds: [`DAYS`] unless [`KEY`]
  /// says another number.
  fn days(user: &Path, project: &Path) -> u64 {
    let layers = Layers::new(project, Some(user.to_path_buf()));
    layers.values().ok().and_then(|values| values.get(KEY)?.as_u64()).unwrap_or(DAYS)
  }

  /// The scratch directories beside this one, of the records, reaped in the background when no
  /// reap ran for a day: each that no life holds and that no write or command used for the days
  /// that the configs of the user and of the project say.
  /// What a reap loses: a reopen of the record makes the directory again with each file that a
  /// write of the record left, but the files that a command made in it are gone. A process that a
  /// life left running holds no lock, so the bound is its margin. A directory of its own goes with
  /// its life, and a reap leaves it alone.
  pub fn reap(&self, user: &Path, project: &Path) {
    let bound = DAY * u32::try_from(Self::days(user, project)).unwrap_or(u32::MAX);
    let Some(root) = self.0.at.parent().filter(|_| !self.0.owned).map(Path::to_path_buf) else {
      return;
    };
    let stamp = root.join(REAPED);
    let last = fs::metadata(&stamp).and_then(|one| one.modified()).ok();
    if last.and_then(|one| one.elapsed().ok()).is_some_and(|since| since < DAY) {
      return;
    }
    // The stamp is written once the reap ends, so a life that ends first leaves the reap to the
    // next life; a stamp that cannot be written only lets the next life reap again.
    thread::spawn(move || {
      reaped(&root, bound, SystemTime::now());
      drop(fs::write(&stamp, ""));
    });
  }

  /// What a read or a write of a path of the scheme came to: the file at that path, with the content of the write
  /// put there first, or the entries of a folder, one on each line, each folder with `/` at its end. Each text and
  /// each refusal names the path under the scheme, and never as the machine names it, and a path that leaves the
  /// directory, by `..` or by a link, is refused.
  pub(crate) fn served(
    &self,
    path: &str,
    content: Option<String>,
    gallery: Option<&Path>,
  ) -> Result<Text, Fault> {
    let at = &self.0.at;
    beneath(
      Beneath { root: at, scheme: SCRATCH, place: "the scratch directory" },
      path,
      content,
      gallery,
      || self.made().map(|_| ()),
    )
  }

  /// The directory as a life opens on its record. A new record starts with an empty directory. A directory that is
  /// missing, or that is not whole, is made again in a directory beside it, which then takes its place: each path of
  /// the scheme with the last text that a write of it came to, which the journal kept, in the order of those last
  /// writes, and a write puts aside a file or a folder that stands in its way. A file that the machine refuses is
  /// told to the log of the host, since the World renders nothing to a model.
  pub fn remade(&self, record: &[Object], log: &dyn Fn(&str)) {
    let at = &self.0.at;
    if record.is_empty() {
      gone(at, log);
      return;
    }
    if at.join(WHOLE).exists() {
      self.hold();
      return;
    }
    let mut last: HashMap<String, (usize, String)> = HashMap::new();
    for (place, entry) in record.iter().enumerate() {
      if let Some(Text { path, content }) = written(entry)
        && let Some(path) = path.strip_prefix(SCRATCH)
      {
        last.insert(path.to_owned(), (place, content));
      }
    }
    let mut files: Vec<_> =
      last.into_iter().map(|(path, (place, content))| (place, path, content)).collect();
    files.sort();
    let mut name = at.file_name().unwrap_or_default().to_os_string();
    name.push(".new");
    let beside = Self::kept(at.with_file_name(name));
    gone(beside.path(), log);
    if let Err(no) = beside.made() {
      log(&format!("the scratch directory of the record was not made again: {}", no.message()));
      return;
    }
    for (_, path, content) in files {
      if let Err(no) = put(beside.path(), &path, &content) {
        log(&format!("the scratch directory of the record leaves out {SCRATCH}{path}: {no}"));
      }
    }
    // The mark of the new directory stays locked while `beside` lives, and Windows refuses to
    // rename a directory that holds an open file, so the lock goes before the rename.
    let from = beside.path().to_path_buf();
    drop(beside);
    gone(at, log);
    if let Err(no) = fs::rename(&from, at) {
      log(&format!("the scratch directory of the record was not made again: {no}"));
      gone(&from, log);
    }
    self.hold();
  }
}

/// The scratch directories of the records under `root` that no life holds and whose last use is
/// older than `bound` at `now`, removed; it gives the ones that it removed. A directory is one
/// whose name is the digest of a record, or such a name with `.new` that a remake left. Its last
/// use is the latest time of a change of any entry of the directory, at any depth, and of the
/// directory itself, so a process that still writes in it keeps it; each life that holds it sets
/// the time of its mark. A directory that the system refuses to remove, as one that holds an open
/// file on Windows, stays, and the reap goes on with the others.
pub(crate) fn reaped(root: &Path, bound: Duration, now: SystemTime) -> Vec<PathBuf> {
  let Ok(entries) = fs::read_dir(root) else { return Vec::new() };
  let mut gone = Vec::new();
  for entry in entries.flatten() {
    let name = entry.file_name().to_string_lossy().into_owned();
    let digest = name.get(..16).is_some_and(|one| one.bytes().all(|b| b.is_ascii_hexdigit()));
    if !digest || !matches!(&name[16..], "" | ".new") {
      continue;
    }
    let at = entry.path();
    let mark = at.join(WHOLE);
    if latest(&at).is_none_or(|used| now.duration_since(used).unwrap_or_default() < bound) {
      continue;
    }
    // The lock of the mark, held while the directory goes: a life that opens on it meanwhile
    // takes no lock, and its next make makes the directory again.
    let free = fs::File::open(&mark).map_or(Ok(None), |file| file.try_lock().map(|()| Some(file)));
    let Ok(free) = free else { continue };
    // Windows removes no directory that holds an open file, so everything but the mark goes while
    // the lock stands, then the lock ends and the mark goes with the directory.
    let kept = fs::read_dir(&at).is_ok_and(|each| {
      each.flatten().filter(|one| one.path() != mark).any(|one| {
        let path = one.path();
        let gone = if one.file_type().is_ok_and(|kind| kind.is_dir()) {
          fs::remove_dir_all(&path)
        } else {
          fs::remove_file(&path)
        };
        gone.is_err()
      })
    });
    drop(free);
    if !kept && fs::remove_dir_all(&at).is_ok() {
      gone.push(at);
    }
  }
  gone
}

/// The latest time of a change of a directory and of each entry under it, at any depth, never
/// through a link; none when the directory cannot be read.
fn latest(at: &Path) -> Option<SystemTime> {
  let mut latest = fs::symlink_metadata(at).and_then(|one| one.modified()).ok()?;
  let mut folders = vec![at.to_path_buf()];
  while let Some(folder) = folders.pop() {
    for entry in fs::read_dir(&folder).into_iter().flatten().flatten() {
      let Ok(info) = entry.metadata() else { continue };
      if let Ok(changed) = info.modified() {
        latest = latest.max(changed);
      }
      if info.is_dir() {
        folders.push(entry.path());
      }
    }
  }
  Some(latest)
}

fn gone(at: &Path, log: &dyn Fn(&str)) {
  if let Err(no) = fs::remove_dir_all(at)
    && no.kind() != io::ErrorKind::NotFound
  {
    log(&format!("{} stays: {no}", at.display()));
  }
}

/// The text that the done of a write in an entry of a record came to, read before anything of the entry is copied.
fn written(entry: &Object) -> Option<Text> {
  let entry = entry.as_ref();
  let held = entry.items()?;
  let fact = held.first()?.items()?;
  let kind = fact.first()?.as_str()? == "done";
  (kind && named(fact.get(1)?.as_str()?, "write")).then(|| Text::of(*fact.get(3)?)).flatten()
}

/// A file put at a path of a directory, with its folders, where a file or a folder that stands in its way goes.
fn put(root: &Path, path: &str, content: &str) -> io::Result<()> {
  let at = resolved(root, path);
  if at == root || !at.starts_with(root) {
    return Err(io::Error::other("it is outside the scratch directory"));
  }
  for up in at.ancestors().skip(1).take_while(|up| *up != root) {
    if fs::symlink_metadata(up).is_ok_and(|one| !one.is_dir()) {
      fs::remove_file(up)?;
    }
  }
  if fs::symlink_metadata(&at).is_ok_and(|one| one.is_dir()) {
    fs::remove_dir_all(&at)?;
  }
  disk::write(&at, content.as_bytes(), Access::Shared).map_err(|no| no.source)
}

/// A directory that a scheme names: its place on the machine, the scheme, and how a refusal names
/// the directory.
#[derive(Clone, Copy)]
pub(crate) struct Beneath<'a> {
  /// The place of the directory on the machine.
  pub root: &'a Path,
  /// The scheme of its paths, as `scratch://`.
  pub scheme: &'a str,
  /// The directory as a refusal names it, as `the scratch directory`.
  pub place: &'a str,
}

/// What a read or a write of a path of a scheme came to: the file at that path under the directory
/// of the scheme, with the content of the write put there first once `make` made the directory, or
/// the entries of a folder, one on each line, each folder with `/` at its end. Each text and each
/// refusal names the path under the scheme, and never as the machine names it, and a path that
/// leaves the directory, by `..` or by a link, is refused.
pub(crate) fn beneath(
  dir: Beneath<'_>,
  path: &str,
  content: Option<String>,
  gallery: Option<&Path>,
  make: impl FnOnce() -> Result<(), Fault>,
) -> Result<Text, Fault> {
  let Beneath { root, scheme, place } = dir;
  let at = resolved(root, path);
  let outside = || Fault::refused(format!("{scheme}{path} is outside {place}."));
  let Ok(under) = at.strip_prefix(root) else { return Err(outside()) };
  let under = under.to_path_buf();
  if !inside(root, &at) {
    return Err(outside());
  }
  let name = format!("{scheme}{}", slashed(&under));
  if let Some(content) = content {
    make()?;
    filed(&at, content, &name)?;
  }
  // The scratch directory is a folder before anything made it, and a directory of another scheme
  // only when it stands.
  let unmade = under.as_os_str().is_empty() && scheme == SCRATCH;
  if unmade || fs::symlink_metadata(&at).is_ok_and(|one| one.is_dir()) {
    return Ok(Text::new(name, entries(&at).join("\n")));
  }
  text(&at, name, gallery)
}

/// Whether a path stands in a directory as the machine finds it: the deepest part of the path that
/// stands, with every link read, is under the directory, or the directory is not made yet.
fn inside(root: &Path, at: &Path) -> bool {
  let Ok(root) = fs::canonicalize(root) else { return true };
  let mut probe = at;
  while fs::symlink_metadata(probe).is_err() {
    let Some(up) = probe.parent() else { return false };
    probe = up;
  }
  fs::canonicalize(probe).is_ok_and(|real| real.starts_with(&root))
}

/// The entries of a folder, in order, each folder with `/` at its end, and no link read as a folder, as `ls` gives
/// them; the mark of a whole directory is none of them.
fn entries(folder: &Path) -> Vec<String> {
  let Ok(held) = fs::read_dir(folder) else { return Vec::new() };
  let mut out: Vec<String> = held
    .flatten()
    .filter(|one| one.file_name() != WHOLE)
    .map(|one| {
      let name = one.file_name().to_string_lossy().into_owned();
      if one.file_type().is_ok_and(|kind| kind.is_dir()) { format!("{name}/") } else { name }
    })
    .collect();
  out.sort();
  out
}

/// A path under the directory, with a slash between its parts on every machine.
fn slashed(under: &Path) -> String {
  let parts: Vec<_> = under.components().map(|one| one.as_os_str().to_string_lossy()).collect();
  parts.join("/")
}

/// A path placed against the working directory of the process, which it needs only for a
/// relative path.
fn absolute(path: &Path) -> Result<PathBuf, Fault> {
  let here = if path.is_absolute() {
    PathBuf::new()
  } else {
    env::current_dir().map_err(|no| {
      Fault::refused(format!(
        "The working directory could not be read for {}: {no}",
        path.display()
      ))
    })?
  };
  Ok(resolved(&here, &path.to_string_lossy()))
}

#[cfg(test)]
#[path = "scratch.test.rs"]
mod test;
