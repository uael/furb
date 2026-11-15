//! The task `tree`, which makes, lands and closes a worktree, on a clone of a bare origin in a
//! yard.

#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::{
  fs,
  path::{Path, PathBuf},
  process,
};

use crate::{ended, git, xtask, yard::Yard};

/// A clone of a bare origin, whose main holds one commit, on any config of the machine.
fn cloned(yard: &Yard) -> PathBuf {
  let (origin, main) = (yard.join("origin.git"), yard.join("main"));
  // The branch of a new repository is the one that the config of the machine names, as master on
  // a runner, so the clone names main itself.
  let first = ["-c", "init.defaultBranch=main"];
  git(yard, &[&first[..], &["init", "-q", "--bare", "origin.git"]].concat());
  git(yard, &[&first[..], &["clone", "-q", &origin.to_string_lossy(), "main"]].concat());
  // A runner names no person, so the clone names one for each commit that a task makes, as a
  // rebase does.
  for (key, value) in [("user.name", "t"), ("user.email", "t@t"), ("commit.gpgsign", "false")] {
    git(&main, &["config", key, value]);
  }
  committed(&main, "first");
  git(&main, &["push", "-q", "origin", "HEAD:refs/heads/main"]);
  git(&main, &["fetch", "-q", "origin"]);
  main
}

fn committed(tree: &Path, name: &str) {
  fs::write(tree.join(name), name).unwrap_or_else(|no| panic!("{name} was not written: {no}"));
  git(tree, &["add", name]);
  git(tree, &["commit", "-q", "-m", name]);
}

/// The task tree with these words, run in a folder: its code, its output and its errors.
fn tree(folder: &Path, words: &[&str]) -> (Option<i32>, String, String) {
  ended(&mut xtask(folder, [&["tree"][..], words].concat()))
}

#[cfg(unix)]
#[test]
fn a_new_tree_builds_in_a_copy_of_its_own_of_the_nearest_target_with_the_hooks_of_the_clone() {
  let yard = Yard::new(format!("xtask-tree-new-{}", process::id()));
  let main = cloned(&yard);
  let shared = yard.join("shared");
  let made = fs::create_dir_all(&shared).and_then(|()| fs::write(shared.join("built"), "warm"));
  made.and_then(|()| symlink(&shared, main.join("target"))).unwrap_or_else(|no| panic!("{no}"));
  let one = yard.join("one");
  let (code, out, err) = tree(&main, &["new", &one.to_string_lossy(), "first"]);
  assert_eq!(code, Some(0), "{err}");
  assert!(out.ends_with(&format!("{} is ready on first.\n", one.display())), "{out}");
  let target = one.join("target");
  assert!(!target.is_symlink(), "the tree takes no link to a target that trees share");
  // A file system with no copy on write, as the one of a runner of Linux, copies nothing.
  if target.exists() {
    assert_eq!(fs::read_to_string(target.join("built")).ok().as_deref(), Some("warm"));
  }
  assert_eq!(git(&main, &["config", "core.hooksPath"]), "script/hooks");
  assert_eq!(fs::read_dir(&shared).map(Iterator::count).ok(), Some(1), "the shared target stays");
}

#[test]
fn a_new_tree_on_a_branch_that_another_tree_holds_fails_and_names_its_folder_and_the_branch() {
  let yard = Yard::new(format!("xtask-tree-held-{}", process::id()));
  let main = cloned(&yard);
  git(&main, &["branch", "held"]);
  git(&main, &["worktree", "add", "-q", &yard.join("first").to_string_lossy(), "held"]);
  let second = yard.join("second");
  let (code, _, err) = tree(&main, &["new", &second.to_string_lossy(), "held"]);
  assert_eq!(code, Some(1), "{err}");
  assert!(err.contains(&*second.to_string_lossy()) && err.contains("held"), "{err}");
  assert!(!second.exists());
}

#[test]
fn two_heads_land_on_the_branch_that_they_name_and_a_conflict_leaves_its_tree_as_it_was() {
  let yard = Yard::new(format!("xtask-tree-land-{}", process::id()));
  let main = cloned(&yard);
  git(&main, &["branch", "carrier"]);
  let trees: Vec<PathBuf> = ["a", "b", "c"].iter().map(|name| yard.join(name)).collect();
  for (one, file) in trees.iter().zip(["a", "b", "a"]) {
    git(&main, &["worktree", "add", "-q", "--detach", &one.to_string_lossy(), "carrier"]);
    fs::write(one.join(file), one.to_string_lossy().as_bytes())
      .unwrap_or_else(|no| panic!("{file} was not written: {no}"));
    git(one, &["add", file]);
    git(one, &["commit", "-q", "-m", &one.to_string_lossy()]);
  }
  for one in &trees[..2] {
    let (code, out, err) = tree(one, &["land", "carrier"]);
    let head = git(one, &["rev-parse", "HEAD"]);
    let landed = format!("landed: carrier is {} (1 commits)\n", &head[..9]);
    assert_eq!((code, out), (Some(0), landed), "{err}");
    assert_eq!(head, git(&main, &["rev-parse", "carrier"]));
  }
  assert_eq!(git(&main, &["rev-list", "--count", "main..carrier"]), "2");
  let before = git(&trees[2], &["rev-parse", "HEAD"]);
  let (code, out, err) = tree(&trees[2], &["land", "carrier"]);
  assert_eq!(
    (code, out.as_str(), err.as_str()),
    (
      Some(1),
      "",
      "cargo xtask: the head conflicts with carrier, so the rebase is aborted: rebase the tree \
       there by hand, then land again\n"
    )
  );
  assert_eq!(git(&trees[2], &["rev-parse", "HEAD"]), before);
  assert_eq!(git(&trees[2], &["status", "--porcelain"]), "");
}

#[test]
fn a_tree_closes_only_once_clean_landed_and_unused_or_when_its_caller_closes_its_own() {
  let yard = Yard::new(format!("xtask-tree-close-{}", process::id()));
  let main = cloned(&yard);
  let work = yard.join("work");
  git(&main, &["worktree", "add", "-q", "-b", "work", &work.to_string_lossy(), "main"]);
  let at = work.to_string_lossy();
  let close = |own: &[&str]| tree(&main, &[&["close", &at, "--into", "main"][..], own].concat());
  let kept = |why: &str| (Some(1), String::new(), format!("cargo xtask: {at} stays: {why}\n"));
  fs::write(work.join("loose"), "loose").unwrap_or_else(|no| panic!("loose: {no}"));
  assert_eq!(close(&["--own"]), kept("it holds changes:\n?? loose\n"));
  fs::remove_file(work.join("loose")).unwrap_or_else(|no| panic!("loose: {no}"));
  committed(&work, "work");
  assert_eq!(close(&["--own"]), kept("1 of its commits stand neither on main nor on a remote"));
  git(&main, &["merge", "-q", "--ff-only", "work"]);
  assert_eq!(
    close(&[]),
    kept("git worked in it 0 minutes ago, under the two hours of an unused tree"),
    "git worked in the tree a moment ago"
  );
  let closed = format!("{at} is closed, with its build and the branch work.\n");
  assert_eq!(close(&["--own"]), (Some(0), closed, String::new()));
  assert!(!work.exists());
  assert_eq!(git(&main, &["branch", "--list", "work"]), "");
}
