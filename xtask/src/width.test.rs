//! The tests of the width of the comments of Rust, on a clone in a yard.

use std::{
  fs,
  path::PathBuf,
  process::{Command, id},
};

use super::failures;
use crate::yard::Yard;

fn git(dir: &Yard, args: &[&str]) {
  let output = Command::new("git").args(args).current_dir(dir).output().expect("git runs");
  assert!(output.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn only_a_comment_line_that_the_change_adds_past_the_width_fails() {
  let yard = Yard::new(format!("xtask-width-{}", id()));
  let long = format!("// {}", "x".repeat(100));
  git(&yard, &["init", "-q"]);
  fs::write(yard.join("a.rs"), format!("fn a() {{}}\n{long}\n")).unwrap();
  git(&yard, &["add", "a.rs"]);
  git(&yard, &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", "base"]);
  let added = [
    "fn a() {}".to_owned(),
    long.clone(),
    format!("  /{long}"),
    "// short".to_owned(),
    format!("const FIXTURE: &str = r\"\n{long}\n\";"),
    format!("fn b() {{}} {long}"),
    format!("/* {} */", "x".repeat(100)),
  ];
  fs::write(yard.join("a.rs"), added.join("\n") + "\n").unwrap();
  fs::write(yard.join("b.rs"), format!("{long}\n")).unwrap();
  git(&yard, &["add", "b.rs"]);
  let said = "a comment line that this change adds goes past 100 characters";
  assert_eq!(
    failures(&yard, Some("HEAD"), &[]).unwrap(),
    [format!("a.rs:3: {said}"), format!("b.rs:1: {said}")],
    "the line that the base holds, a short line, a string, code and a block comment pass"
  );
  assert_eq!(
    failures(&yard, Some("HEAD"), &[PathBuf::from("b.rs")]).unwrap(),
    [format!("b.rs:1: {said}")],
    "given files, it reads those alone"
  );
}
