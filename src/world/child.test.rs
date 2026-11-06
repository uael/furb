//! A child of a life and its group.

use super::signalled;
#[cfg(unix)]
use crate::testing::{on_reactor, pause};
#[cfg(unix)]
use rustix::{
  io::Errno,
  process::{Pid, test_kill_process},
};
#[cfg(unix)]
use std::{process::Stdio, time::Duration};
#[cfg(unix)]
use tokio::process;
#[cfg(unix)]
use tokio::{
  io::{AsyncBufReadExt, BufReader},
  process::Command,
};

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
  let mut command = process::Command::new("sh");
  command.arg("-c").arg("sleep 5");
  let (_child, mut group) = super::spawn(&mut command).unwrap();
  assert!(group.lifeline.is_some(), "a group holds the line to its watch until it ends");
  group.slay().unwrap();
  assert!(
    group.lifeline.is_none(),
    "the watch ended with its group, and a line written to it raises SIGPIPE in a host that does not ignore it"
  );
  group.free();
}

#[test]
#[cfg(unix)]
fn the_processes_that_a_child_started_end_when_its_group_drops() {
  let mut command = Command::new("sh");
  command.arg("-c").arg("sleep 60 & echo $!; wait").stdout(Stdio::piped());
  let (mut child, group) = super::spawn(&mut command).unwrap();
  let out = child.stdout.take().unwrap();
  let line = on_reactor(async { BufReader::new(out).lines().next_line().await });
  let pid = Pid::from_raw(line.unwrap().unwrap().parse().unwrap()).unwrap();
  assert_eq!(test_kill_process(pid), Ok(()), "the process that the child started runs");
  drop(group);
  for _ in 0..200 {
    if test_kill_process(pid) == Err(Errno::SRCH) {
      return;
    }
    pause(Duration::from_millis(25));
  }
  panic!("the process that the child started outlives its group");
}
