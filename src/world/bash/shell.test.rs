//! The shell of the commands, proved on every machine.

#![cfg_attr(
  unix,
  expect(clippy::disallowed_methods, reason = "a test blocks on its runtime and fails by a panic")
)]

use std::fs;
#[cfg(unix)]
use std::time::Duration;

#[cfg(unix)]
use super::asked_path;
use super::{git_sh, merged_path};

/// A program at a path, which this machine runs.
fn made(at: &std::path::Path) {
  fs::create_dir_all(at.parent().unwrap()).unwrap();
  fs::write(at, "").unwrap();
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(at, fs::Permissions::from_mode(0o755)).unwrap();
  }
}

#[test]
fn the_sh_of_git_is_the_sh_on_path_else_the_one_beside_the_git_on_path_else_the_one_of_the_programs()
 {
  let at = crate::testing::Yard::unmade(format!("furb-git-sh-{}", std::process::id()));
  let (cmd, programs) = (at.join("Git").join("cmd"), at.join("Programs"));
  assert_eq!(git_sh(std::slice::from_ref(&cmd), Some(programs.clone())), None);
  made(&programs.join("Git").join("bin").join("sh"));
  assert_eq!(
    git_sh(std::slice::from_ref(&cmd), Some(programs.clone())),
    Some(programs.join("Git").join("bin").join("sh"))
  );
  made(&cmd.join("git"));
  made(&at.join("Git").join("usr").join("bin").join("sh"));
  assert_eq!(
    git_sh(std::slice::from_ref(&cmd), Some(programs.clone())),
    Some(at.join("Git").join("usr").join("bin").join("sh"))
  );
  made(&at.join("Git").join("bin").join("sh"));
  assert_eq!(
    git_sh(std::slice::from_ref(&cmd), Some(programs.clone())),
    Some(at.join("Git").join("bin").join("sh"))
  );
  made(&cmd.join("sh"));
  assert_eq!(git_sh(std::slice::from_ref(&cmd), Some(programs)), Some(cmd.join("sh")));
}

#[cfg(unix)]
fn fake_shell(name: &str, body: &str) -> crate::testing::Yard {
  use std::os::unix::fs::PermissionsExt;
  let path =
    crate::testing::Yard::new(format!("furb-login-{}-{name}", std::process::id())).within("sh");
  fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
  fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
  path
}

/// What the login shell says its PATH is, awaited on the runtime of the ears.
#[cfg(unix)]
fn asked(shell: &std::path::Path) -> Option<std::ffi::OsString> {
  crate::ear::reactor().block_on(asked_path(shell, Duration::from_secs(120)))
}

#[test]
#[cfg(unix)]
fn the_path_of_the_login_shell_is_read_between_its_marks() {
  let shell = fake_shell(
    "marks",
    "echo noise of a profile; printf '<furb-path>/opt/homebrew/bin:/usr/bin</furb-path>'",
  );
  assert_eq!(asked(&shell), Some("/opt/homebrew/bin:/usr/bin".into()));
}

#[test]
#[cfg(unix)]
fn the_path_of_a_login_shell_whose_profile_leaves_a_process_on_its_output_is_read_when_the_shell_ends()
 {
  let held = crate::testing::Yard::new(format!("furb-login-{}-release", std::process::id()));
  let release = held.join("release");
  let held = format!(
    "(n=0; while [ ! -e '{}' ] && [ $n -lt 300 ]; do sleep 0.1; n=$((n+1)); done) & printf '<furb-path>/a:/b</furb-path>'",
    release.display()
  );
  let got =
    crate::ear::reactor().block_on(asked_path(&fake_shell("held", &held), Duration::from_secs(20)));
  fs::write(&release, "").unwrap();
  assert_eq!(got, Some("/a:/b".into()));
}

#[test]
#[cfg(unix)]
fn a_login_shell_that_fails_or_gives_no_path_gives_none() {
  assert_eq!(asked(&fake_shell("fails", "printf '<furb-path>/a</furb-path>'; exit 1")), None);
  assert_eq!(asked(&fake_shell("empty", "printf '<furb-path></furb-path>'")), None);
  assert_eq!(asked(&fake_shell("silent", "true")), None);
  assert_eq!(asked(std::path::Path::new("/no/such/shell")), None);
}

#[test]
fn the_folders_of_the_login_path_come_first_and_the_current_ones_follow_with_no_repeat() {
  let join = |parts: &[&str]| std::env::join_paths(parts).unwrap();
  let got = merged_path(
    &join(&["/opt/homebrew/bin", "/usr/bin"]),
    &join(&["/usr/bin", "/bin", "/opt/homebrew/bin"]),
  );
  assert_eq!(got, join(&["/opt/homebrew/bin", "/usr/bin", "/bin"]));
}
