//! The shell of the commands, proved on every machine.

use std::fs;

#[cfg(unix)]
use super::asked_path;
use super::{Kept, git_sh, merged_path, signalled};

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
  let at = std::env::temp_dir().join(format!("furb-git-sh-{}", std::process::id()));
  let _ = fs::remove_dir_all(&at);
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
  fs::remove_dir_all(&at).unwrap();
}

#[test]
fn a_signal_that_ends_the_sh_of_git_gives_128_and_the_number_of_the_signal() {
  assert_eq!(signalled(9 * 256), 137);
  assert_eq!(signalled(15 * 256), 143);
  assert_eq!(signalled(0), 0);
  assert_eq!(signalled(1), 1);
  assert_eq!(signalled(65 * 256), 65 * 256);
  assert_eq!(signalled(259), 259);
}

#[test]
fn a_stream_past_its_caps_keeps_its_head_and_its_tail_whole_and_counts_the_cut() {
  let mut kept = Kept::new(4, 4);
  let said = kept.push("ééé");
  assert!(said.starts_with("éé\n[output over 4 bytes"), "{said:?}");
  assert!(kept.push("abcdefghijkl").is_empty());
  assert_eq!(kept.finish(), "éé\n[... 10 bytes cut ...]\nijkl");
  let mut kept = Kept::new(4, 3);
  kept.push("abcdé");
  kept.push("éé");
  assert_eq!(kept.finish(), "abcd\n[... 4 bytes cut ...]\né");
}

#[test]
fn a_stream_within_its_caps_is_kept_whole() {
  let mut kept = Kept::new(4, 4);
  assert_eq!(kept.push("ab"), "ab");
  assert_eq!(kept.push("cd"), "cd");
  assert_eq!(kept.finish(), "abcd");
  let mut kept = Kept::new(2, 4);
  kept.push("abcd");
  assert_eq!(kept.finish(), "abcd");
}

#[cfg(unix)]
fn fake_shell(name: &str, body: &str) -> std::path::PathBuf {
  use std::os::unix::fs::PermissionsExt;
  let dir = std::env::temp_dir().join(format!("furb-login-{}-{name}", std::process::id()));
  fs::create_dir_all(&dir).unwrap();
  let path = dir.join("sh");
  fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
  fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
  path
}

#[test]
#[cfg(unix)]
fn the_path_of_the_login_shell_is_read_between_its_marks() {
  let shell = fake_shell(
    "marks",
    "echo noise of a profile; printf '<furb-path>/opt/homebrew/bin:/usr/bin</furb-path>'",
  );
  assert_eq!(asked_path(&shell), Some("/opt/homebrew/bin:/usr/bin".into()));
}

#[test]
#[cfg(unix)]
fn a_login_shell_that_fails_or_gives_no_path_gives_none() {
  assert_eq!(asked_path(&fake_shell("fails", "printf '<furb-path>/a</furb-path>'; exit 1")), None);
  assert_eq!(asked_path(&fake_shell("empty", "printf '<furb-path></furb-path>'")), None);
  assert_eq!(asked_path(&fake_shell("silent", "true")), None);
  assert_eq!(asked_path(std::path::Path::new("/no/such/shell")), None);
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
