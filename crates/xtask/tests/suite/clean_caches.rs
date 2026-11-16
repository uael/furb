//! The task `clean-caches` as a process, on folders of its own.

use std::{
  fs,
  path::Path,
  process::{self, Command, Output},
};

use crate::yard::Yard;

const MARK: &str = ".furb-scratch";

#[test]
fn the_task_removes_the_scratch_directory_that_holds_only_its_mark_and_no_life_holds() {
  let yard = Yard::new(format!("xtask-clean-caches-swept-{}", process::id()));
  let root = yard.join("scratch");
  let (held, full) = ("b".repeat(16), "c".repeat(16));
  let named = ["a".repeat(16), held.clone(), full.clone(), format!("{}.new", "d".repeat(16))];
  for name in [&named[..], &["notes".to_owned()]].concat() {
    fs::create_dir_all(root.join(&name)).unwrap();
    fs::write(root.join(&name).join(MARK), "").unwrap();
  }
  fs::write(root.join(&full).join("plan.md"), "kept").unwrap();
  fs::write(root.join(".reaped"), "").unwrap();
  let life = fs::File::open(root.join(&held).join(MARK)).unwrap();
  life.lock_shared().unwrap();
  let done = cleaned(&root, &yard.join("uv"));
  drop(life);
  assert!(done.status.success(), "{}", String::from_utf8_lossy(&done.stderr));
  let mut left: Vec<String> = fs::read_dir(&root)
    .unwrap()
    .flatten()
    .map(|one| one.file_name().to_string_lossy().into_owned())
    .collect();
  left.sort();
  assert_eq!(left, [".reaped".to_owned(), held, full.clone(), "notes".to_owned()]);
  assert_eq!(fs::read_to_string(root.join(&full).join("plan.md")).unwrap(), "kept");
  let said = String::from_utf8_lossy(&done.stdout);
  assert_eq!(said.lines().next(), Some("2 scratch directories removed, of 4"));
}

#[test]
fn the_task_prunes_a_copy_of_a_wheel_that_no_entry_uses() {
  let yard = Yard::new(format!("xtask-clean-caches-pruned-{}", process::id()));
  let left = yard.join("uv/archive-v0").join(format!("leftover{}", "0".repeat(12)));
  let info = left.join("furb_cli-0.4.2.dist-info");
  fs::create_dir_all(&info).unwrap();
  fs::write(info.join("METADATA"), "Name: furb-cli\nVersion: 0.4.2\n").unwrap();
  fs::create_dir_all(left.join("furb_cli")).unwrap();
  fs::write(left.join("furb_cli/__init__.py"), "").unwrap();
  fs::create_dir_all(yard.join("scratch")).unwrap();
  let done = cleaned(&yard.join("scratch"), &yard.join("uv"));
  assert!(done.status.success(), "{}", String::from_utf8_lossy(&done.stderr));
  assert!(!left.exists());
}

fn cleaned(scratch: &Path, uv: &Path) -> Output {
  Command::new(env!("CARGO_BIN_EXE_xtask"))
    .arg("clean-caches")
    .arg("--scratch")
    .arg(scratch)
    .arg("--uv")
    .arg(uv)
    .output()
    .unwrap_or_else(|no| panic!("the task did not start: {no}"))
}
