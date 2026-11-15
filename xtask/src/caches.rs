//! The task `clean-caches`, which removes what earlier runs left in the caches of the machine.

use std::{
  env,
  ffi::OsStr,
  fs,
  path::{Path, PathBuf},
  process::ExitCode,
};

use crate::{Fault, spawn};

/// The mark of the scratch directory of a record, on which a life holds a shared lock while it
/// lives.
const MARK: &str = ".furb-scratch";

/// Remove each scratch directory of a record in `scratch` that holds nothing but its mark and that
/// no life holds, then prune the cache of uv, and fail as the prune fails.
pub(crate) fn clean(
  root: &Path,
  scratch: Option<PathBuf>,
  uv: Option<&Path>,
) -> Result<ExitCode, Fault> {
  let folder = scratch.or_else(machine).ok_or(Fault::NoCache)?;
  let entries =
    fs::read_dir(&folder).map_err(|source| Fault::Read { path: folder.clone(), source })?;
  let records: Vec<PathBuf> = entries
    .flatten()
    .filter(|one| one.file_type().is_ok_and(|kind| kind.is_dir()) && record(&one.file_name()))
    .map(|one| one.path())
    .collect();
  let removed = records.iter().filter(|one| swept(one)).count();
  println!("{removed} scratch directories removed, of {}", records.len());
  let prune = [OsStr::new("--no-config"), OsStr::new("cache"), OsStr::new("prune")];
  match uv {
    Some(uv) => {
      spawn(root, "uv", &[&prune[..], &[OsStr::new("--cache-dir"), uv.as_os_str()]].concat())
    }
    None => spawn(root, "uv", &prune),
  }
}

/// The folder of the scratch directories of the records, where the crate puts it: `scratch` in the
/// cache of furb that `cache` of `src/world.rs` names.
fn machine() -> Option<PathBuf> {
  let named = |name: &str| env::var_os(name).filter(|one| !one.is_empty()).map(PathBuf::from);
  let base = named("XDG_CACHE_HOME").or_else(|| {
    if cfg!(windows) {
      named("LOCALAPPDATA")
    } else if cfg!(target_os = "macos") {
      env::home_dir().map(|home| home.join("Library/Caches"))
    } else {
      env::home_dir().map(|home| home.join(".cache"))
    }
  });
  base.map(|base| base.join("furb").join("scratch"))
}

/// Whether a name is the one of the scratch directory of a record: the digest of the record, or the
/// digest with `.new`, which a remake of the directory left.
fn record(name: &OsStr) -> bool {
  let name = name.to_string_lossy();
  let digest = name.strip_suffix(".new").unwrap_or(&name);
  digest.len() == 16 && digest.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

/// Whether the directory of a record held nothing but its mark and no life held it, and went. The
/// mark goes while the lock stands, so a life that opens on the directory then finds no mark and
/// makes it again; the lock ends before the directory goes, since Windows removes no folder that
/// holds an open file.
fn swept(folder: &Path) -> bool {
  let bare =
    fs::read_dir(folder).is_ok_and(|each| each.flatten().map(|one| one.file_name()).eq([MARK]));
  let mark = folder.join(MARK);
  let Ok(held) = fs::File::open(&mark)
    .map_err(drop)
    .and_then(|file| file.try_lock().map(|()| file).map_err(drop))
  else {
    return false;
  };
  if !bare || fs::remove_file(&mark).is_err() {
    return false;
  }
  drop(held);
  let gone = fs::remove_dir(folder).is_ok();
  // A file that a life made meanwhile keeps the directory, which keeps its mark too.
  if !gone && let Err(no) = fs::write(&mark, "") {
    eprintln!("{} stays with no mark: {no}", folder.display());
  }
  gone
}
