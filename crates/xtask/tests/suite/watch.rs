//! The task `watch` as a process, on a repository in a yard.
#![expect(clippy::panic, reason = "a panic is how a test fails")]

use std::{
  fs,
  path::Path,
  process::{self, Command},
};

use crate::yard::Yard;

/// What git says in a folder, or a panic with why it failed.
fn git(at: &Path, args: &[&str]) -> String {
  let done = Command::new("git")
    .args(args)
    .current_dir(at)
    .output()
    .unwrap_or_else(|no| panic!("git did not start: {no}"));
  assert!(done.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&done.stderr));
  String::from_utf8_lossy(&done.stdout).trim().to_owned()
}

#[test]
fn a_check_that_cannot_run_is_a_red_of_its_head_and_the_watch_goes_on() {
  let yard = Yard::new(format!("xtask-watch-{}", process::id()));
  git(&yard, &["init", "--quiet", "--initial-branch", "main"]);
  for (name, text) in [("a", "1"), ("b", "2")] {
    fs::write(yard.join(name), text).unwrap();
    git(&yard, &["add", name]);
    git(&yard, &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "--quiet", "-m", name]);
  }
  let head = git(&yard, &["rev-parse", "--short=9", "main"]);
  git(&yard, &["checkout", "--quiet", "--detach", "HEAD~1"]);
  // A lock of the index that no git holds: the tree reads as clean, and its checkout fails.
  fs::write(yard.join(".git/index.lock"), "").unwrap();
  let (table, inbox) = (yard.join("checks.tsv"), yard.join("inbox.md"));
  let done = Command::new(env!("CARGO_BIN_EXE_xtask"))
    .args(["watch", "--branch", "main", "--once", "--every", "0", "--tree"])
    .arg(&*yard)
    .arg("--table")
    .arg(&table)
    .arg("--inbox")
    .arg(&inbox)
    .output()
    .unwrap_or_else(|no| panic!("the task did not start: {no}"));
  assert_eq!(done.status.code(), Some(0), "{}", String::from_utf8_lossy(&done.stderr));
  let line = fs::read_to_string(&table).unwrap();
  let fields: Vec<&str> = line.trim_end().split('\t').skip(1).collect();
  assert_eq!(fields, [head.as_str(), "red", "0", "1"]);
  let told = fs::read_to_string(&inbox).unwrap();
  assert!(told.contains(&format!("the check of {head} could not run: ")), "{told}");
}
