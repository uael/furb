//! The shell of the commands, proved on every machine.

use std::fs;

use super::{Kept, git_sh, signalled};

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
