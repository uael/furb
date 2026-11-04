//! The group of a command, proved on each machine that has a watch of its group.

#![cfg(unix)]

use super::{Group, grouped};

#[test]
fn a_slain_group_keeps_no_line_to_its_watch_so_that_free_writes_to_no_dead_reader() {
  let _inside = crate::ear::runtime().enter();
  let mut command = std::process::Command::new("sh");
  command.arg("-c").arg("sleep 5");
  let lead = grouped(&mut command).unwrap();
  let child = tokio::process::Command::from(command).spawn().unwrap();
  let mut group = Group::of(&child, lead).unwrap();
  assert!(group.lifeline.is_some(), "a group holds the line to its watch until it ends");
  group.slay().unwrap();
  assert!(
    group.lifeline.is_none(),
    "the watch ended with its group, and a line written to it raises SIGPIPE in a host that does not ignore it"
  );
  group.free();
}
