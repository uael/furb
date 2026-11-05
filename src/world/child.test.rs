//! A child of a life and its group.

use super::{signalled, spawn};

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
#[cfg(unix)]
fn a_slain_group_keeps_no_line_to_its_watch_so_that_free_writes_to_no_dead_reader() {
  let mut command = tokio::process::Command::new("sh");
  command.arg("-c").arg("sleep 5");
  let (_child, mut group) = spawn(&mut command).unwrap();
  assert!(group.lifeline.is_some(), "a group holds the line to its watch until it ends");
  group.slay().unwrap();
  assert!(
    group.lifeline.is_none(),
    "the watch ended with its group, and a line written to it raises SIGPIPE in a host that does not ignore it"
  );
  group.free();
}
