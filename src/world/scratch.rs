//! The scratch directory of a life: a folder of the machine for the notes and the scratch files of its words, which a
//! word names as `scratch://` and a command as `$TMPDIR`, so no text that the World answers names its place.

use std::{
  collections::HashMap,
  fs, io,
  path::{Path, PathBuf},
  sync::Arc,
  time::{SystemTime, UNIX_EPOCH},
};

use sha2::{Digest, Sha256};

use super::{
  cache,
  files::{ended, failed, stored, text},
  resolved,
};
use crate::{
  fact::named,
  value::{Fault, Object, Text},
};

/// The scheme of the paths of the scratch directory.
pub const SCRATCH: &str = "scratch://";

/// The file that marks a directory as whole: made by a write or a command, or made again from its record. No listing
/// shows it, and a directory without it is made again.
const WHOLE: &str = ".furb-scratch";

/// The scratch directory of a life, which its files and its commands share: the one of its record, which outlives
/// the life, or one of its own, which goes with the last ear that holds it. Nothing makes it until a write or a
/// command needs it, or until a life makes it again from its record.
#[derive(Clone)]
pub struct Scratch(Arc<Held>);

/// The directory, and whether it goes with the last ear that holds it.
struct Held {
  at: PathBuf,
  owned: bool,
}

impl Drop for Held {
  fn drop(&mut self) {
    if self.owned {
      let _ = fs::remove_dir_all(&self.at);
    }
  }
}

impl Scratch {
  /// The scratch directory at a path, resolved against the working directory of the process, which outlives the
  /// life.
  pub fn at(path: impl Into<PathBuf>) -> Scratch {
    let here = std::env::current_dir().unwrap_or_default();
    let at = resolved(&here, &path.into().to_string_lossy());
    Scratch(Arc::new(Held { at, owned: false }))
  }

  /// The scratch directory of a record: in the cache of furb, under the digest of the canonical path of the record,
  /// so every later life on that record finds it again, however the path is spelled, and its path stays short enough
  /// for a socket that a command makes in it. Nothing when the machine has no cache of furb.
  pub fn of(record: &Path) -> Option<Scratch> {
    let path = resolved(&std::env::current_dir().unwrap_or_default(), &record.to_string_lossy());
    let path = match (path.parent(), path.file_name()) {
      (Some(folder), Some(name)) => {
        fs::canonicalize(folder).map_or(path.clone(), |real| real.join(name))
      }
      _ => path,
    };
    let digest = Sha256::digest(path.to_string_lossy().as_bytes());
    Some(Scratch::at(cache()?.join("scratch").join(hex(&digest[..8]))))
  }

  /// A scratch directory of its own, in the temporary directory of the machine under a short name, which goes when
  /// the life does. The name is drawn, or made of the process and the time when the system draws nothing.
  pub fn own() -> Scratch {
    let mut drawn = [0u8; 4];
    if getrandom::fill(&mut drawn).is_err() {
      let now =
        SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.subsec_nanos());
      drawn = (std::process::id() ^ now).to_le_bytes();
    }
    let at = std::env::temp_dir().join(format!("furb-{}", hex(&drawn)));
    Scratch(Arc::new(Held { at, owned: true }))
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
    Ok(&self.0.at)
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
    let at = resolved(&self.0.at, path);
    let outside = || Fault::refused(format!("{SCRATCH}{path} is outside the scratch directory."));
    let under = at.strip_prefix(&self.0.at).map_err(|_| outside())?.to_path_buf();
    if !self.inside(&at) {
      return Err(outside());
    }
    let name = format!("{SCRATCH}{}", slashed(&under));
    if let Some(content) = content {
      self.made()?;
      if let Some(parent) = at.parent() {
        fs::create_dir_all(parent).map_err(|no| failed(&no, &name))?;
      }
      stored(&at, ended(&at, content).as_bytes()).map_err(|no| failed(&no, &name))?;
    }
    if under.as_os_str().is_empty() || fs::symlink_metadata(&at).is_ok_and(|one| one.is_dir()) {
      return Ok(Text::new(name, entries(&at).join("\n")));
    }
    text(&at, name, gallery)
  }

  /// Whether a path stands in the directory as the machine finds it: the deepest part of the path that stands,
  /// with every link read, is under the directory, or the directory is not made yet.
  fn inside(&self, at: &Path) -> bool {
    let Ok(root) = fs::canonicalize(&self.0.at) else { return true };
    let mut probe = at;
    while fs::symlink_metadata(probe).is_err() {
      let Some(up) = probe.parent() else { return false };
      probe = up;
    }
    fs::canonicalize(probe).is_ok_and(|real| real.starts_with(&root))
  }

  /// The directory as a life opens on its record. A new record starts with an empty directory. A directory that is
  /// missing, or that is not whole, is made again in a directory beside it, which then takes its place: each path of
  /// the scheme with the last text that a write of it came to, which the journal kept, in the order of those last
  /// writes, and a write puts aside a file or a folder that stands in its way. A file that the machine refuses is
  /// told to the log of the host, since the World renders nothing to a model.
  pub fn remade(&self, record: &[Object], log: &dyn Fn(&str)) {
    let at = &self.0.at;
    if record.is_empty() {
      let _ = fs::remove_dir_all(at);
      return;
    }
    if at.join(WHOLE).exists() {
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
    let beside = Scratch::at(at.with_file_name(name));
    let _ = fs::remove_dir_all(beside.path());
    if let Err(no) = beside.made() {
      log(&format!("the scratch directory of the record was not made again: {}", no.message()));
      return;
    }
    for (_, path, content) in files {
      if let Err(no) = put(beside.path(), &path, &content) {
        log(&format!("the scratch directory of the record leaves out {SCRATCH}{path}: {no}"));
      }
    }
    let _ = fs::remove_dir_all(at);
    if let Err(no) = fs::rename(beside.path(), at) {
      log(&format!("the scratch directory of the record was not made again: {no}"));
      let _ = fs::remove_dir_all(beside.path());
    }
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
  if let Some(parent) = at.parent() {
    fs::create_dir_all(parent)?;
  }
  fs::write(&at, content)
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

/// Bytes as the digits of base sixteen.
fn hex(bytes: &[u8]) -> String {
  bytes.iter().map(|one| format!("{one:02x}")).collect()
}

#[cfg(test)]
#[path = "scratch.test.rs"]
mod test;
