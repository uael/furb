//! The task `memwatch` as a process, on a machine whose `ps` and `sysctl` are scripts that list
//! real processes that sleep.

#![cfg(unix)]

use std::{
  env, fs,
  os::unix::{fs::PermissionsExt, process::ExitStatusExt},
  path::{Path, PathBuf},
  process::{self, Child, Command, Output},
  time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::{shimmed, yard::Yard};
use wait_timeout::ChildExt;

const MB: u64 = 1024;
const WORKER: &str = "python -u -c import sys;exec(eval(sys.stdin.readline()))";

/// Real processes that the fake ps lists, so that the watch ends a real pid; each is ended at the
/// end of the test.
struct Fakes(Vec<Child>);

impl Fakes {
  fn new(count: usize) -> Self {
    Self(
      (0..count)
        .map(|_| {
          Command::new("sleep").arg("60").spawn().unwrap_or_else(|no| panic!("sleep starts: {no}"))
        })
        .collect(),
    )
  }

  fn pid(&self, at: usize) -> u32 {
    self.0[at].id()
  }

  /// Whether each process still runs, in order.
  fn running(&mut self) -> Vec<bool> {
    self
      .0
      .iter_mut()
      .map(|one| {
        one.wait_timeout(Duration::ZERO).unwrap_or_else(|no| panic!("a wait: {no}")).is_none()
      })
      .collect()
  }

  /// Whether the watch ended the process by a kill, waited for since a signal lands when the
  /// process next runs.
  fn killed(&mut self, at: usize) -> bool {
    let status =
      self.0[at].wait_timeout(Duration::from_secs(10)).unwrap_or_else(|no| panic!("a wait: {no}"));
    status.is_some_and(|status| status.signal() == Some(9))
  }
}

impl Drop for Fakes {
  fn drop(&mut self) {
    for one in &mut self.0 {
      if one.kill().is_ok() {
        one.wait().unwrap_or_else(|no| panic!("an ended process is reaped: {no}"));
      }
    }
  }
}

fn me() -> u64 {
  let out = Command::new("id").arg("-u").output().unwrap_or_else(|no| panic!("id runs: {no}"));
  String::from_utf8_lossy(&out.stdout)
    .trim()
    .parse()
    .unwrap_or_else(|no| panic!("id gives a number: {no}"))
}

/// A program of the shell in the folder `shim` of a yard, which the watch finds before the program
/// of the machine of that name.
fn shim(yard: &Path, name: &str, script: &str) {
  let path = yard.join("shim").join(name);
  let made = fs::create_dir_all(yard.join("shim"))
    .and_then(|()| fs::write(&path, format!("#!/bin/sh\n{script}")))
    .and_then(|()| fs::set_permissions(&path, fs::Permissions::from_mode(0o755)));
  made.unwrap_or_else(|no| panic!("the shim {name} was not made: {no}"));
}

/// A fake machine: ps lists the given processes as pid, size in KB, command and user, with no
/// process that ended, and on macOS sysctl gives the given pressure level of the kernel.
fn machine(yard: &Path, level: Option<u64>, rows: &[(u32, u64, &str, u64)]) {
  let table: Vec<String> =
    rows.iter().map(|(pid, kb, command, uid)| format!("{uid} {pid} {kb} {command}\n")).collect();
  fs::write(yard.join("table"), table.concat()).unwrap_or_else(|no| panic!("{no}"));
  let lister = format!(
    "while read -r uid pid rest; do kill -0 \"$pid\" 2>/dev/null && echo \"$uid $pid $rest\"; done < {}\n",
    yard.join("table").display()
  );
  let sysctl = level.map_or("exit 1\n".to_owned(), |level| format!("echo {level}\n"));
  shim(yard, "ps", &lister);
  shim(yard, "sysctl", &sysctl);
}

/// The watch, once or in its loop, in a folder, with a test process of 1 KB as a runaway unless a
/// test gives the limit.
fn watch(yard: &Path, folder: &Path, once: bool, more: &[(&str, &str)]) -> Command {
  let mut run = Command::new(env!("CARGO_BIN_EXE_xtask"));
  run.arg("memwatch").args(once.then_some("--once")).current_dir(folder);
  for (name, _) in env::vars().filter(|(name, _)| name.starts_with("FURB_WATCH_")) {
    run.env_remove(name);
  }
  run
    .env("PATH", shimmed(yard))
    .env("FURB_WATCH_LOG", yard.join("watch.log"))
    .env("FURB_WATCH_MIN", "1");
  run.envs(more.iter().copied());
  run
}

fn once(yard: &Path, more: &[(&str, &str)]) -> String {
  let out: Output =
    watch(yard, yard, true, more).output().unwrap_or_else(|no| panic!("the watch runs: {no}"));
  assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
  String::from_utf8_lossy(&out.stdout).into_owned()
}

fn yard(name: &str) -> Yard {
  // A path that names the guard would hide each process of the test from the census.
  Yard::new(format!("xtask-guard-{name}-{}", process::id()))
}

#[test]
fn a_tight_memory_ends_the_largest_test_process_of_the_user_and_no_other_process() {
  let yard = yard("tight");
  let mut fakes = Fakes::new(8);
  let mine = me();
  let commands = [
    ("/w/furb/target/debug/deps/furb-1a2b3c4d5e6f7a8b --exact life::test", 800, mine),
    ("/opt/bun/bin/bun test ./bind/typescript/test/session.test.ts", 2000, mine),
    ("/w/furb/.venv/bin/python -m pytest -q test/boot", 1000, mine),
    ("/w/furb/target/debug/furb host --grace 5", 9000, mine),
    ("/Applications/Terminal.app/Contents/MacOS/Terminal", 6000, mine),
    ("/opt/bun/bin/bun test ./bind/typescript/test/engine.test.ts", 1500, mine),
    (WORKER, 2500, mine),
    ("/opt/bun/bin/bun test ./bind/typescript/test/other.test.ts", 9999, mine + 1),
  ];
  let rows: Vec<_> = commands
    .iter()
    .enumerate()
    .map(|(at, (command, size, uid))| (fakes.pid(at), size * MB, *command, *uid))
    .collect();
  machine(&yard, Some(1), &rows);
  assert_eq!(once(&yard, &[]), "", "a memory that is not tight ends nothing");
  assert_eq!(fakes.running(), [true; 8]);
  machine(&yard, Some(2), &rows);
  let said = once(&yard, &[]);
  assert!(fakes.killed(6));
  assert_eq!(
    said,
    format!("memwatch: pressure level 2, ended pid {} (2500 MB): {WORKER}\n", fakes.pid(6))
  );
  let mut left = fakes.running();
  left.remove(6);
  assert_eq!(left, [true; 7]);
  let logged = fs::read_to_string(yard.join("watch.log")).unwrap();
  let (when, rest) = logged.trim_end().split_once(' ').unwrap();
  let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
  assert!(now - when.parse::<u64>().unwrap() < 60);
  assert_eq!(rest, format!("{} {WORKER}", fakes.pid(6)), "each end is a line of the log");
}

#[test]
fn a_machine_with_no_reading_or_no_test_process_ends_nothing() {
  let yard = yard("nothing");
  let mut fakes = Fakes::new(2);
  let mine = me();
  machine(&yard, None, &[(fakes.pid(0), 500 * MB, "bun test ./a.test.ts", mine)]);
  assert_eq!(
    once(&yard, &[("FURB_WATCH_FREE", "0")]),
    "",
    "a machine that gives no reading is never tight"
  );
  let rows = [
    (fakes.pid(0), 9000 * MB, "/w/furb/target/debug/furb host", mine),
    (fakes.pid(1), 5000 * MB, "Mail", mine),
  ];
  machine(&yard, Some(4), &rows);
  assert_eq!(once(&yard, &[]), "memwatch: pressure level 4, and no test process to end\n");
  assert_eq!(fakes.running(), [true; 2]);
}

#[test]
fn a_program_that_only_names_a_test_binary_or_pytest_is_no_test_process() {
  let yard = yard("named");
  let mut fakes = Fakes::new(5);
  let mine = me();
  let binary = "/w/target/debug/deps/furb-3bfcea354560c7bc";
  let link = format!("/usr/bin/ld -demangle -o {binary} {binary}.o");
  let pytest = "/w/furb/.venv/bin/python3 /w/furb/.venv/bin/pytest -q test/boot";
  let rows = [
    (fakes.pid(0), 3000 * MB, link.as_str(), mine),
    (fakes.pid(1), 2999 * MB, "/w/target/debug/xtask budget pytest -- uv run pytest -q", mine),
    (fakes.pid(2), 2998 * MB, "sh -c uv run pytest -q", mine),
    (fakes.pid(3), 2997 * MB, "uv run pytest -q", mine),
    (fakes.pid(4), 400 * MB, pytest, mine),
  ];
  machine(&yard, Some(2), &rows);
  assert_eq!(
    once(&yard, &[]),
    format!("memwatch: pressure level 2, ended pid {} (400 MB): {pytest}\n", fakes.pid(4))
  );
  assert!(fakes.killed(4));
}

#[test]
fn the_work_is_the_cause_only_with_a_runaway_test_process_or_a_work_past_its_limit() {
  let yard = yard("cause");
  let mut fakes = Fakes::new(3);
  let mine = me();
  let rows = [
    (fakes.pid(0), 1500 * MB, "/opt/bun/bin/bun test ./a.test.ts", mine),
    (fakes.pid(1), 20000 * MB, "Chrome", mine),
    (fakes.pid(2), 9000 * MB, "/elsewhere/furb/target/debug/rustc --crate-name y", mine),
  ];
  machine(&yard, Some(2), &rows);
  let not =
    "the work is not the cause (it holds 1500 MB, its largest test process 1500 MB): none ended";
  assert_eq!(
    once(&yard, &[("FURB_WATCH_MIN", "")]),
    format!("memwatch: pressure level 2, and {not}\n")
  );
  assert_eq!(fakes.running(), [true; 3]);
  let said = once(&yard, &[("FURB_WATCH_MIN", ""), ("FURB_WATCH_LANE", "1")]);
  assert_eq!(
    said,
    format!(
      "memwatch: pressure level 2, ended pid {} (1500 MB): /opt/bun/bin/bun test ./a.test.ts\n",
      fakes.pid(0)
    )
  );
  assert!(fakes.killed(0));
  machine(&yard, Some(2), &[(fakes.pid(1), 4500 * MB, "bun test ./b.test.ts", mine)]);
  once(&yard, &[("FURB_WATCH_MIN", "")]);
  assert!(fakes.killed(1), "a test process of 4 GB is a runaway by default");
}

#[test]
fn the_loop_ends_one_test_process_at_each_look_while_the_memory_stays_tight() {
  let yard = yard("loop");
  let mut fakes = Fakes::new(2);
  let mine = me();
  machine(
    &yard,
    Some(2),
    &[
      (fakes.pid(0), 900 * MB, "bun test ./a.test.ts", mine),
      (fakes.pid(1), 300 * MB, "bun test ./b.test.ts", mine),
    ],
  );
  let mut looped =
    watch(&yard, &yard, false, &[("FURB_WATCH_POLL", "0.05"), ("FURB_WATCH_AFTER", "0.05")])
      .spawn()
      .unwrap();
  let both = fakes.killed(0) && fakes.killed(1);
  looped.kill().unwrap();
  looped.wait().unwrap();
  assert!(both);
}

/// A git clone with a second tree beside it: the clone, and the second tree.
fn clone(yard: &Path) -> (PathBuf, PathBuf) {
  let root = yard.join("clone");
  fs::create_dir_all(&root).unwrap_or_else(|no| panic!("the clone: {no}"));
  let git = |args: &[&str]| {
    let mut run = Command::new("git");
    run.args([
      "-c",
      "user.name=t",
      "-c",
      "user.email=t@t",
      "-c",
      "commit.gpgsign=false",
      "-c",
      "core.hooksPath=/dev/null",
    ]);
    assert!(
      run
        .args(args)
        .current_dir(&root)
        .status()
        .unwrap_or_else(|no| panic!("git runs: {no}"))
        .success()
    );
  };
  git(&["init", "-q"]);
  fs::write(root.join("a"), "a").unwrap_or_else(|no| panic!("a file of the clone: {no}"));
  git(&["add", "-A"]);
  git(&["commit", "-qm", "x"]);
  let other = yard.join("other");
  git(&["worktree", "add", "-q", &other.to_string_lossy(), "-b", "other"]);
  (
    fs::canonicalize(root).unwrap_or_else(|no| panic!("the clone: {no}")),
    fs::canonicalize(other).unwrap_or_else(|no| panic!("the tree: {no}")),
  )
}

#[test]
fn the_work_is_each_test_process_and_each_process_that_names_a_tree_less_a_daemon_of_furb() {
  let yard = yard("trees");
  let (root, other) = clone(&yard);
  let mine = me();
  for (size, name, ended) in [
    (3000, "target/debug/rustc --crate-name x", true),
    (2000, "target/debug/rustc --crate-name x", false),
    (3000, "target/debug/furb host", false),
  ] {
    let mut fakes = Fakes::new(3);
    let names = [
      format!("{}/target/debug/deps/furb-0123456789abcdef --test-threads 4", other.display()),
      format!("{}/node_modules/.bin/esbuild --serve", other.display()),
      format!("{}/{name}", root.display()),
    ];
    let rows: Vec<_> = names
      .iter()
      .enumerate()
      .map(|(at, one)| (fakes.pid(at), size * MB, one.as_str(), mine))
      .collect();
    machine(&yard, Some(2), &rows);
    let out = watch(&yard, &root, true, &[("FURB_WATCH_MIN", "")]).output().unwrap();
    let said = String::from_utf8_lossy(&out.stdout);
    assert_eq!(said.contains("ended pid"), ended, "{size} {name}: {said}");
    if ended {
      assert!(fakes.killed(0));
    } else {
      assert_eq!(fakes.running(), [true; 3]);
    }
  }
}
