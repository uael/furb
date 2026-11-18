//! The search of the sh of Git for Windows, proved on every machine: its one door, the shell of a
//! command, searches only on Windows. The suite proves the login shell through the commands of a
//! life.

use std::{fs, path, process, slice};

use super::git_sh;
use crate::testing;

/// A program at a path, which this machine runs.
fn made(at: &path::Path) {
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
  let at = testing::Yard::unmade(format!("furb-git-sh-{}", process::id()));
  let (cmd, programs) = (at.join("Git").join("cmd"), at.join("Programs"));
  assert_eq!(git_sh(slice::from_ref(&cmd), Some(programs.clone())), None);
  made(&programs.join("Git").join("bin").join("sh"));
  assert_eq!(
    git_sh(slice::from_ref(&cmd), Some(programs.clone())),
    Some(programs.join("Git").join("bin").join("sh"))
  );
  made(&cmd.join("git"));
  made(&at.join("Git").join("usr").join("bin").join("sh"));
  assert_eq!(
    git_sh(slice::from_ref(&cmd), Some(programs.clone())),
    Some(at.join("Git").join("usr").join("bin").join("sh"))
  );
  made(&at.join("Git").join("bin").join("sh"));
  assert_eq!(
    git_sh(slice::from_ref(&cmd), Some(programs.clone())),
    Some(at.join("Git").join("bin").join("sh"))
  );
  made(&cmd.join("sh"));
  assert_eq!(git_sh(slice::from_ref(&cmd), Some(programs)), Some(cmd.join("sh")));
}
