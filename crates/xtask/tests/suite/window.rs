//! The task `window`, on a repository in a yard whose cargo is a stand-in.

use std::{fs, process};

use crate::{ended, git, held, shimmed, stand_in, xtask, yard::Yard};

#[test]
fn a_build_of_the_window_waits_until_a_commit_holds_each_change_of_the_tree() {
  let yard = Yard::new(format!("xtask-window-{}", process::id()));
  held(&yard, &[("src/lib.rs", "one\n")]);
  git(&yard, &["commit", "-q", "-m", "one"]);
  let built = yard.join("built.txt");
  fs::write(&built, "built\n").unwrap_or_else(|no| panic!("built.txt was not made: {no}"));
  stand_in(&yard, "cargo", &[("build", &built)]);
  fs::write(yard.join("src/lib.rs"), "two\n").unwrap_or_else(|no| panic!("no change: {no}"));
  let refused = ended(xtask(&yard, ["window", "-j", "4"]).env("PATH", shimmed(&yard)));
  let why = "cargo xtask: the window would take changes that no commit holds: commit them, or \
             build in a worktree of your own\n M src/lib.rs\n";
  assert_eq!(refused, (Some(1), String::new(), why.to_owned()));
  git(&yard, &["commit", "-q", "-a", "-m", "two"]);
  let done = ended(xtask(&yard, ["window", "-j", "4"]).env("PATH", shimmed(&yard)));
  assert_eq!(done, (Some(0), "built\n".to_owned(), String::new()));
}
