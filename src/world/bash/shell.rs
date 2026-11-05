//! The shell of a command: the sh that runs it, the PATH that it runs with, and the task that reaps
//! it.

use std::{
  collections::HashSet,
  env,
  ffi::{OsStr, OsString},
  fs,
  io::ErrorKind,
  path::{Path, PathBuf},
  process::{Command, Stdio},
  sync::{
    OnceLock,
    atomic::{AtomicUsize, Ordering},
  },
  time::Duration,
};

use tokio::process::Child;

use crate::{
  value::Fault,
  world::child::{self, Group},
};

/// The POSIX sh that runs a command: `/bin/sh`, and on Windows, which has none of its own, the sh
/// of Git for Windows, which [`git_sh`] finds, or `sh` when it finds none, which a command then
/// names in the refusal of its start.
pub fn shell() -> &'static Path {
  static FOUND: OnceLock<PathBuf> = OnceLock::new();
  FOUND.get_or_init(|| {
    if !cfg!(windows) {
      return PathBuf::from("/bin/sh");
    }
    let folders: Vec<PathBuf> =
      env::split_paths(&env::var_os("PATH").unwrap_or_default()).collect();
    git_sh(&folders, env::var_os("ProgramFiles").map(PathBuf::from))
      .unwrap_or_else(|| PathBuf::from("sh"))
  })
}

/// How long the login shell of the user may take to say its PATH, which a slow profile needs.
const LOGIN_LIMIT: Duration = Duration::from_secs(5);

/// The PATH that the login shell of the user sets, read once for the process, or none on Windows,
/// with no `SHELL`, or when that shell fails to give one. A life started from a desktop or a
/// service has the short PATH of its parent, which lacks the folders that the profile of the user
/// adds, as `/opt/homebrew/bin`.
async fn login_path() -> Option<&'static OsString> {
  static FOUND: tokio::sync::OnceCell<Option<OsString>> = tokio::sync::OnceCell::const_new();
  let found = async {
    if cfg!(windows) {
      None
    } else {
      asked_path(Path::new(&env::var_os("SHELL")?), LOGIN_LIMIT).await
    }
  };
  FOUND.get_or_init(|| found).await.as_ref()
}

/// What the login shell `shell` says its PATH is, between two marks that a profile which writes to
/// its output cannot forge by accident, or none when it fails, gives no path, or runs past `limit`,
/// when it is ended. The shell writes to a file and not to a pipe, since a pipe ends only
/// when each process that holds its write end is gone, and a process that a profile starts in the
/// background, or that another thread forks while the pipe is made, holds it past the shell.
async fn asked_path(shell: &Path, limit: Duration) -> Option<OsString> {
  static MADE: AtomicUsize = AtomicUsize::new(0);
  let file = env::temp_dir().join(format!(
    "furb-login-path-{}-{}",
    std::process::id(),
    MADE.fetch_add(1, Ordering::Relaxed)
  ));
  let stdout = fs::OpenOptions::new().write(true).create_new(true).open(&file).ok()?;
  let ran = async {
    let mut command = tokio::process::Command::new(shell);
    command.args(["-l", "-c", "printf '<furb-path>%s</furb-path>' \"$PATH\""]);
    command.stdin(Stdio::null()).stdout(stdout).stderr(Stdio::null());
    let (mut child, mut group) = child::spawn(&mut command).ok()?;
    let status = child.wait().await.ok();
    // A process that the profile left in the background stays, as it would in a terminal.
    group.free();
    status
  };
  let status = tokio::time::timeout(limit, ran).await.ok().flatten();
  let said = fs::read(&file);
  drop(fs::remove_file(&file));
  let said = String::from_utf8_lossy(&said.ok()?).into_owned();
  let path = said.split_once("<furb-path>")?.1.split_once("</furb-path>")?.0;
  (status?.success() && !path.is_empty()).then(|| OsString::from(path))
}

/// The folders of `login` in order, then those of `current` that it lacks.
fn merged_path(login: &OsStr, current: &OsStr) -> OsString {
  let mut seen = HashSet::new();
  let kept: Vec<PathBuf> = env::split_paths(login)
    .chain(env::split_paths(current))
    .filter(|one| seen.insert(one.clone()))
    .collect();
  env::join_paths(kept).unwrap_or_else(|_| current.to_owned())
}

/// The sh of Git for Windows: the `sh` in these folders, else the one of the Git whose `git` stands
/// in them, in a `bin` or a `usr/bin` of a folder above that program, as a Git that puts only its
/// `cmd` on PATH holds it, else the one of the Git in the folder of the programs of the machine.
fn git_sh(folders: &[PathBuf], programs: Option<PathBuf>) -> Option<PathBuf> {
  program("sh", folders.iter().cloned())
    .or_else(|| {
      let git = program("git", folders.iter().cloned())?;
      let roots = git.ancestors().skip(2).take(3).map(Path::to_path_buf).collect::<Vec<_>>();
      program("sh", roots.iter().flat_map(|one| [one.join("bin"), one.join("usr").join("bin")]))
    })
    .or_else(|| program("sh", [programs?.join("Git").join("bin")]))
}

/// The first program of a name in these folders, with the endings of a program on Windows.
pub fn program(name: &str, folders: impl IntoIterator<Item = PathBuf>) -> Option<PathBuf> {
  let endings: &[&str] = if cfg!(windows) { &[".exe", ".cmd", ""] } else { &[""] };
  let folders = folders.into_iter().filter(|one| !one.as_os_str().is_empty());
  folders
    .flat_map(|one| endings.iter().map(move |end| one.join(format!("{name}{end}"))))
    .find(|one| runs(one))
}

/// Whether a path is a program that this machine runs.
fn runs(path: &Path) -> bool {
  let Ok(info) = path.metadata() else { return false };
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    info.is_file() && info.permissions().mode() & 0o111 != 0
  }
  #[cfg(not(unix))]
  {
    info.is_file()
  }
}

/// The shell of a command spawned with the PATH of the login shell of the user, and the group of
/// its processes, or why the machine would not start it or hold it.
pub(super) async fn spawned(
  command: Command,
  shown: &str,
  dir: &Path,
) -> Result<(Child, Group), Fault> {
  let mut command = tokio::process::Command::from(command);
  if let Some(login) = login_path().await {
    command.env("PATH", merged_path(login, &env::var_os("PATH").unwrap_or_default()));
  }
  child::spawn(&mut command).map_err(|no| {
    let why = if cfg!(windows) && no.kind() == ErrorKind::NotFound {
      format!("no POSIX sh runs it: install Git for Windows, whose sh furb runs: {no}")
    } else {
      no.to_string()
    };
    Fault::refused(format!("{shown} did not start: {why}: {}", dir.display()))
  })
}

#[cfg(test)]
#[path = "shell.test.rs"]
mod test;
