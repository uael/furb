//! The shell of a command: the sh that runs it, the PATH that it runs with, and the task that reaps
//! it.

use std::{
  collections::HashSet,
  env,
  ffi::{OsStr, OsString},
  io::ErrorKind,
  path::{Path, PathBuf},
  process::{Command, Stdio},
  sync::OnceLock,
  time::Duration,
};

use tokio::{process::Child, task::JoinHandle};

use super::group::{Group, grouped};
use crate::{ear::runtime, value::Fault};

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

/// The PATH that the login shell of the user sets, read once for the process, or none on Windows,
/// with no `SHELL`, or when that shell fails to give one. A life started from a desktop or a
/// service has the short PATH of its parent, which lacks the folders that the profile of the user
/// adds, as `/opt/homebrew/bin`.
async fn login_path() -> Option<&'static OsString> {
  static FOUND: tokio::sync::OnceCell<Option<OsString>> = tokio::sync::OnceCell::const_new();
  let found = async {
    if cfg!(windows) { None } else { asked_path(Path::new(&env::var_os("SHELL")?)).await }
  };
  FOUND.get_or_init(|| found).await.as_ref()
}

/// What the login shell `shell` says its PATH is, between two marks that a profile which writes to
/// its output cannot forge by accident, or none when it fails, gives no path, or runs past five
/// seconds, when it is ended.
async fn asked_path(shell: &Path) -> Option<OsString> {
  let mut command = tokio::process::Command::new(shell);
  command
    .args(["-l", "-c", "printf '<furb-path>%s</furb-path>' \"$PATH\""])
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::null())
    .kill_on_drop(true);
  let out = tokio::time::timeout(Duration::from_secs(5), command.output()).await.ok()?.ok()?;
  let said = String::from_utf8_lossy(&out.stdout);
  let path = said.split_once("<furb-path>")?.1.split_once("</furb-path>")?.0;
  (out.status.success() && !path.is_empty()).then(|| OsString::from(path))
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
  mut command: Command,
  shown: &str,
  dir: &Path,
) -> Result<(Child, Group), Fault> {
  if let Some(login) = login_path().await {
    command.env("PATH", merged_path(login, &env::var_os("PATH").unwrap_or_default()));
  }
  let lead = grouped(&mut command)
    .map_err(|no| Fault::refused(format!("{shown} could not be held: {no}: {}", dir.display())))?;
  let mut child = tokio::process::Command::from(command).spawn().map_err(|no| {
    let why = if cfg!(windows) && no.kind() == ErrorKind::NotFound {
      format!("no POSIX sh runs it: install Git for Windows, whose sh furb runs: {no}")
    } else {
      no.to_string()
    };
    Fault::refused(format!("{shown} did not start: {why}: {}", dir.display()))
  })?;
  match Group::of(&child, lead) {
    Ok(group) => Ok((child, group)),
    Err(no) => {
      let ended = match child.start_kill() {
        Ok(()) => String::new(),
        Err(late) => format!(", and it could not be ended: {late}"),
      };
      drop(reaped(child));
      Err(Fault::refused(format!("{shown} could not be held: {no}{ended}")))
    }
  }
}

/// The shell of a command, awaited by one task of the runtime that gives its code, so the shell
/// is reaped as it exits, whether the command still waits for it or dropped before. An error of the
/// wait is given, not read as an exit with no code.
pub(super) fn reaped(mut child: Child) -> JoinHandle<std::io::Result<Option<i32>>> {
  runtime().spawn(async move { child.wait().await.map(coded) })
}

/// The code of an exit, and for a command that a signal ended, 128 and the number of the signal, as
/// a shell gives, so no code says only that the timeout ended the command.
fn coded(status: std::process::ExitStatus) -> Option<i32> {
  #[cfg(unix)]
  {
    use std::os::unix::process::ExitStatusExt;
    status.code().or_else(|| status.signal().map(|signal| 128 + signal))
  }
  #[cfg(not(unix))]
  status.code().map(signalled)
}

/// The code of an exit of the sh of Git for Windows, which a signal ends with the number of the
/// signal times 256, as 128 and that number, as a shell gives; and every other code as it is.
#[cfg_attr(all(unix, not(test)), expect(dead_code, reason = "only Windows and the tests call it"))]
fn signalled(code: i32) -> i32 {
  if code % 256 == 0 && (1..=64).contains(&(code / 256)) { 128 + code / 256 } else { code }
}

#[cfg(test)]
#[path = "shell.test.rs"]
mod test;
