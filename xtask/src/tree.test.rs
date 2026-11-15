//! The tests of the life of a worktree, on a clone of a bare origin in a yard.

#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::{
  fs,
  path::{Path, PathBuf},
  process::{Command, id},
};

use super::{Kept, close, land, new};
use crate::{Fault, yard::Yard};

fn git(dir: &Path, args: &[&str]) -> String {
  let output = Command::new("git").args(args).current_dir(dir).output().expect("git runs");
  assert!(output.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&output.stderr));
  String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// A clone of a bare origin, whose main holds one commit, on any config of the machine.
fn cloned(yard: &Yard) -> PathBuf {
  let (origin, main) = (yard.join("origin.git"), yard.join("main"));
  // The branch of a new repository is the one that the config of the machine names, as master on
  // a runner, so the clone names main itself.
  let first = ["-c", "init.defaultBranch=main"];
  git(yard, &[&first[..], &["init", "-q", "--bare", "origin.git"]].concat());
  git(yard, &[&first[..], &["clone", "-q", &origin.to_string_lossy(), "main"]].concat());
  for (key, value) in [("user.name", "t"), ("user.email", "t@t"), ("commit.gpgsign", "false")] {
    git(&main, &["config", key, value]);
  }
  committed(&main, "first");
  git(&main, &["push", "-q", "origin", "HEAD:refs/heads/main"]);
  git(&main, &["fetch", "-q", "origin"]);
  main
}

fn committed(tree: &Path, name: &str) {
  fs::write(tree.join(name), name).unwrap();
  git(tree, &["add", name]);
  git(tree, &["commit", "-q", "-m", name]);
}

#[cfg(unix)]
#[test]
fn a_new_tree_builds_in_a_copy_of_its_own_of_the_nearest_target_with_the_hooks_of_the_clone() {
  let yard = Yard::new(format!("xtask-tree-new-{}", id()));
  let main = cloned(&yard);
  let shared = yard.join("shared");
  fs::create_dir_all(&shared).unwrap();
  fs::write(shared.join("built"), "warm").unwrap();
  symlink(&shared, main.join("target")).unwrap();
  new(&main, &yard.join("one"), "first", None).unwrap();
  let target = yard.join("one/target");
  assert!(!target.is_symlink(), "the tree takes no link to a target that trees share");
  // A file system with no copy on write, as the one of a runner of Linux, copies nothing.
  if target.exists() {
    assert_eq!(fs::read_to_string(target.join("built")).unwrap(), "warm");
  }
  assert_eq!(git(&main, &["config", "core.hooksPath"]), "script/hooks");
  assert_eq!(fs::read_dir(&shared).unwrap().count(), 1, "the shared target stays as it was");
}

#[test]
fn a_new_tree_on_a_branch_that_another_tree_holds_fails_and_names_its_folder_and_the_branch() {
  let yard = Yard::new(format!("xtask-tree-held-{}", id()));
  let main = cloned(&yard);
  git(&main, &["branch", "held"]);
  git(&main, &["worktree", "add", "-q", &yard.join("first").to_string_lossy(), "held"]);
  let second = yard.join("second");
  let said = new(&main, &second, "held", None).unwrap_err().to_string();
  assert!(said.contains(&*second.to_string_lossy()) && said.contains("held"), "{said}");
}

#[test]
fn two_heads_land_on_the_branch_that_they_name_and_a_conflict_leaves_its_tree_as_it_was() {
  let yard = Yard::new(format!("xtask-tree-land-{}", id()));
  let main = cloned(&yard);
  git(&main, &["branch", "carrier"]);
  let trees: Vec<PathBuf> = ["a", "b", "c"].iter().map(|name| yard.join(name)).collect();
  for (tree, file) in trees.iter().zip(["a", "b", "a"]) {
    git(&main, &["worktree", "add", "-q", "--detach", &tree.to_string_lossy(), "carrier"]);
    fs::write(tree.join(file), tree.to_string_lossy().as_bytes()).unwrap();
    git(tree, &["add", file]);
    git(tree, &["commit", "-q", "-m", &tree.to_string_lossy()]);
  }
  land(&trees[0], "carrier").unwrap();
  land(&trees[1], "carrier").unwrap();
  assert_eq!(git(&main, &["rev-list", "--count", "main..carrier"]), "2");
  assert_eq!(git(&trees[1], &["rev-parse", "HEAD"]), git(&main, &["rev-parse", "carrier"]));
  let before = git(&trees[2], &["rev-parse", "HEAD"]);
  assert!(matches!(land(&trees[2], "carrier"), Err(Fault::Conflict(_))));
  assert_eq!(git(&trees[2], &["rev-parse", "HEAD"]), before);
  assert_eq!(git(&trees[2], &["status", "--porcelain"]), "");
}

#[test]
fn a_tree_closes_only_once_clean_landed_and_unused_or_when_its_caller_closes_its_own() {
  let yard = Yard::new(format!("xtask-tree-close-{}", id()));
  let main = cloned(&yard);
  let tree = yard.join("work");
  git(&main, &["worktree", "add", "-q", "-b", "work", &tree.to_string_lossy(), "main"]);
  let why = |own| match close(&main, &tree, "main", own) {
    Err(Fault::Kept { why, .. }) => why,
    other => panic!("the tree is kept, not {other:?}"),
  };
  fs::write(tree.join("loose"), "loose").unwrap();
  assert!(matches!(why(true), Kept::Changed(_)));
  fs::remove_file(tree.join("loose")).unwrap();
  committed(&tree, "work");
  assert!(matches!(why(true), Kept::Unlanded { .. }));
  git(&main, &["merge", "-q", "--ff-only", "work"]);
  assert!(matches!(why(false), Kept::Recent(_)), "git worked in the tree a moment ago");
  close(&main, &tree, "main", true).unwrap();
  assert!(!tree.exists());
  assert_eq!(git(&main, &["branch", "--list", "work"]), "");
}
