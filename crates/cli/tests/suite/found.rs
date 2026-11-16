//! Where furb with no command finds the window, as a process: the program that `FURB_WINDOW`
//! names, then the one beside furb, then the one on PATH. The window that the checkout of the
//! build holds comes last, and its tests stand beside the code, since the checkout is fixed when
//! furb is built.

#![cfg(unix)]

use crate::yard::Yard;

use std::{
  fs,
  io::ErrorKind,
  os::unix::fs::PermissionsExt,
  path::{Path, PathBuf},
  process::{self, Command},
  thread,
  time::{Duration, Instant},
};

/// How long a start of furb waits for its file to be free.
const BUSY: Duration = Duration::from_secs(10);

/// A program in place of the window in a folder, which says its name.
fn window(folder: &Path, name: &str) -> PathBuf {
  let at = folder.join("furb-window");
  let made =
    fs::create_dir_all(folder).and_then(|()| fs::write(&at, format!("#!/bin/sh\necho {name}\n")));
  made
    .and_then(|()| fs::set_permissions(&at, fs::Permissions::from_mode(0o755)))
    .unwrap_or_else(|no| panic!("{name}: {no}"));
  at
}

/// A copy of furb in a folder of the yard, so the folder beside it is one that the test makes.
fn copied(yard: &Yard) -> PathBuf {
  let at = yard.join("furb/furb");
  let made =
    fs::create_dir_all(yard.join("furb")).and_then(|()| fs::copy(env!("CARGO_BIN_EXE_furb"), &at));
  made.unwrap_or_else(|no| panic!("furb is not copied: {no}"));
  at
}

/// What the window that furb with no command hands its terminal to says, with `FURB_WINDOW` and
/// PATH as the test gives them.
fn launched(furb: &Path, named: Option<&Path>, path: &Path) -> String {
  let mut command = Command::new(furb);
  command.env("PATH", path).env_remove("FURB_WINDOW");
  if let Some(named) = named {
    command.env("FURB_WINDOW", named);
  }
  // On Linux an exec of a file that a process holds open to write fails as busy, and a test on
  // another thread that forks while the copy of furb is open holds it until its child execs: the
  // start is tried again until that child let go of it.
  let began = Instant::now();
  let done = loop {
    match command.output() {
      Err(no) if no.kind() == ErrorKind::ExecutableFileBusy && began.elapsed() < BUSY => {
        thread::yield_now();
      }
      done => break done.unwrap_or_else(|no| panic!("furb did not start: {no}")),
    }
  };
  assert!(done.status.success(), "{}", String::from_utf8_lossy(&done.stderr));
  String::from_utf8_lossy(&done.stdout).into_owned()
}

#[test]
fn the_program_that_furb_window_names_comes_first() {
  let yard = Yard::new(format!("furb-found-named-{}", process::id()));
  let furb = copied(&yard);
  window(&yard.join("furb"), "beside");
  let named = window(&yard.join("named"), "named");
  assert_eq!(launched(&furb, Some(&named), &yard.join("bin")), "named\n");
}

#[test]
fn the_window_beside_furb_comes_before_the_one_on_path() {
  let yard = Yard::new(format!("furb-found-beside-{}", process::id()));
  let furb = copied(&yard);
  window(&yard.join("furb"), "beside");
  window(&yard.join("bin"), "path");
  assert_eq!(launched(&furb, Some(Path::new("")), &yard.join("bin")), "beside\n");
}

#[test]
fn the_window_on_path_comes_before_the_one_of_the_checkout() {
  let yard = Yard::new(format!("furb-found-path-{}", process::id()));
  let furb = copied(&yard);
  window(&yard.join("bin"), "path");
  assert_eq!(launched(&furb, None, &yard.join("bin")), "path\n");
}
