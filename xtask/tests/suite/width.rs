//! The task `comment-width`, on a repository in a yard.

use std::{fs, process};

use crate::{ended, git, xtask, yard::Yard};

#[test]
fn only_a_comment_line_that_the_change_adds_past_the_width_fails() {
  let yard = Yard::new(format!("xtask-width-{}", process::id()));
  let write = |file: &str, text: String| {
    fs::write(yard.join(file), text).unwrap_or_else(|no| panic!("{file} was not written: {no}"));
  };
  let long = format!("// {}", "x".repeat(100));
  git(&yard, &["init", "-q"]);
  write("a.rs", format!("fn a() {{}}\n{long}\n"));
  git(&yard, &["add", "a.rs"]);
  git(&yard, &["commit", "-q", "-m", "base"]);
  let added = [
    "fn a() {}".to_owned(),
    long.clone(),
    format!("  /{long}"),
    "// short".to_owned(),
    format!("const FIXTURE: &str = r\"\n{long}\n\";"),
    format!("fn b() {{}} {long}"),
    format!("/* {} */", "x".repeat(100)),
  ];
  write("a.rs", added.join("\n") + "\n");
  write("b.rs", format!("{long}\n"));
  git(&yard, &["add", "b.rs"]);
  let said = "a comment line that this change adds goes past 100 characters";
  let width = |files: &[&str]| {
    let (code, out, _) =
      ended(&mut xtask(&yard, [&["comment-width", "--base", "HEAD"][..], files].concat()));
    (code, out)
  };
  assert_eq!(
    width(&[]),
    (Some(1), format!("a.rs:3: {said}\nb.rs:1: {said}\n")),
    "the line that the base holds, a short line, a string, code and a block comment pass"
  );
  assert_eq!(
    width(&["b.rs"]),
    (Some(1), format!("b.rs:1: {said}\n")),
    "given files, it reads those alone"
  );
}
