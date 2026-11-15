//! The tests of the opens of the records, on a build that a script stands for.

use std::{
  fs,
  os::unix::fs::PermissionsExt,
  process::{ExitCode, id},
};

use super::Opens;
use crate::yard::Yard;

#[test]
fn a_record_that_opened_before_and_opens_no_more_fails_the_run_with_its_line() {
  let yard = Yard::new(format!("xtask-opens-{}", id()));
  let furb = yard.join("furb");
  fs::write(&furb, "#!/bin/sh\nif grep -q bad \"$3\"; then echo 'furb: the record drifts'; exit 3; fi\necho 'furb: opened'\n").unwrap();
  fs::set_permissions(&furb, fs::Permissions::from_mode(0o755)).unwrap();
  let (good, bad) = (yard.join("good.jsonl"), yard.join("bad.jsonl"));
  fs::write(&good, "good\n").unwrap();
  fs::write(&bad, "bad\n").unwrap();
  fs::write(yard.join("list"), format!("{}\n{}\n", good.display(), bad.display())).unwrap();
  fs::write(
    yard.join("before.tsv"),
    format!("{}\topens\n{}\topens\n", good.display(), bad.display()),
  )
  .unwrap();
  let opens = Opens {
    furb,
    out: yard.join("out.tsv"),
    before: Some(yard.join("before.tsv")),
    list: Some(yard.join("list")),
    bound: 30,
  };
  assert_eq!(opens.run(&yard).unwrap(), ExitCode::FAILURE);
  let table = fs::read_to_string(yard.join("out.tsv")).unwrap();
  let rows: Vec<Vec<&str>> = table.lines().map(|line| line.split('\t').collect()).collect();
  let kept: Vec<(&str, &str, &str)> = rows.iter().map(|row| (row[0], row[1], row[4])).collect();
  let (good, bad) = (good.display().to_string(), bad.display().to_string());
  assert_eq!(
    kept,
    [
      (good.as_str(), "opens", "furb: opened"),
      (bad.as_str(), "refused:3", "furb: the record drifts")
    ]
  );
}
