//! The task `opens`, on a build of furb that a script of the shell stands for, as the old test did:
//! the task reads the memory of furb with ps, so it runs on unix.

#![cfg(unix)]

use std::{fs, os::unix::fs::PermissionsExt, process};

use crate::{ended, xtask, yard::Yard};

#[test]
fn a_record_that_opened_before_and_opens_no_more_fails_the_run_with_its_line() {
  let yard = Yard::new(format!("xtask-opens-{}", process::id()));
  let furb = yard.join("furb");
  let (good, bad) = (yard.join("good.jsonl"), yard.join("bad.jsonl"));
  let (good, bad) = (good.display().to_string(), bad.display().to_string());
  let made = fs::write(
    &furb,
    "#!/bin/sh\nif grep -q bad \"$3\"; then echo 'furb: the record drifts'; exit 3; fi\n\
     echo 'furb: opened'\n",
  )
  .and_then(|()| fs::set_permissions(&furb, fs::Permissions::from_mode(0o755)))
  .and_then(|()| fs::write(&good, "good\n"))
  .and_then(|()| fs::write(&bad, "bad\n"))
  .and_then(|()| fs::write(yard.join("list"), format!("{good}\n{bad}\n")))
  .and_then(|()| fs::write(yard.join("before.tsv"), format!("{good}\topens\n{bad}\topens\n")));
  made.unwrap_or_else(|no| panic!("the build and the records were not made: {no}"));
  let (code, _, err) = ended(&mut xtask(
    &yard,
    ["opens", "furb", "out.tsv", "before.tsv", "--list", "list", "--bound", "30"],
  ));
  assert_eq!(code, Some(1), "{err}");
  let table = fs::read_to_string(yard.join("out.tsv")).unwrap_or_default();
  let rows: Vec<Vec<&str>> = table.lines().map(|line| line.split('\t').collect()).collect();
  let kept: Vec<(&str, &str, &str)> = rows.iter().map(|row| (row[0], row[1], row[4])).collect();
  assert_eq!(
    kept,
    [
      (good.as_str(), "opens", "furb: opened"),
      (bad.as_str(), "refused:3", "furb: the record drifts")
    ]
  );
}
