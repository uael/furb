//! The tests of the tasks of a developer, each through the program that `cargo xtask` runs, on a
//! yard of its own, in one binary: cargo links the program once for them.

#[path = "../../../tests/common/yard.rs"]
mod yard;

mod clean_caches;
mod links;
mod memwatch;
mod pack;
mod paths;
mod tree;
mod unused;
mod width;

use std::{
  env,
  ffi::OsStr,
  fs,
  path::Path,
  process::{Command, Output},
};

/// A program in a folder, with no variable of git: a hook that runs the test sets them, and they
/// would point each git of the program at the repository of the hook.
fn program(name: impl AsRef<OsStr>, folder: &Path) -> Command {
  let mut command = Command::new(name);
  command.current_dir(folder);
  for (name, _) in env::vars_os().filter(|(name, _)| name.to_string_lossy().starts_with("GIT_")) {
    command.env_remove(name);
  }
  command
}

/// The program of the tasks with these words, in a folder.
fn xtask<I: AsRef<OsStr>>(folder: &Path, words: impl IntoIterator<Item = I>) -> Command {
  let mut command = program(env!("CARGO_BIN_EXE_xtask"), folder);
  command.args(words);
  command
}

/// The end of a command: its code, and what it wrote on its output and on its errors.
fn ended(command: &mut Command) -> (Option<i32>, String, String) {
  let Output { status, stdout, stderr } =
    command.output().unwrap_or_else(|no| panic!("the command did not start: {no}"));
  let text = |bytes: Vec<u8>| String::from_utf8_lossy(&bytes).into_owned();
  (status.code(), text(stdout), text(stderr))
}

/// What git printed in a folder, on any config of the machine and with no variable of a hook; a git
/// that fails fails the test.
fn git(folder: &Path, args: &[&str]) -> String {
  let config = ["-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false"];
  let (code, out, err) = ended(program("git", folder).args(config).args(args));
  assert_eq!(code, Some(0), "git {args:?} in {}: {err}", folder.display());
  out.trim().to_owned()
}

/// A repository of git in a folder, which holds these files, added and not committed.
fn held(folder: &Path, files: &[(&str, &str)]) {
  for (file, text) in files {
    let path = folder.join(file);
    let made =
      path.parent().map_or(Ok(()), fs::create_dir_all).and_then(|()| fs::write(&path, text));
    made.unwrap_or_else(|no| panic!("{file} was not made: {no}"));
  }
  git(folder, &["init", "-q"]);
  git(folder, &["add", "-A"]);
}
