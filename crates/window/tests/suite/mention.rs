//! A mention of a file in the input: how it stands out in the input, and how the files that he
//! named come first among those that a mention offers.

#![cfg(test)]

use std::fs;

use furb_window::tokens::Tokens;
use opentui::testing::TestApp;

use crate::{
  cards::shows,
  daemon::ANSWERS,
  input::{sent, started},
};

/// The row of each line of the window that holds a text, in the frame that the test reads.
fn rows(app: &TestApp, text: &str) -> Vec<usize> {
  let frame = app.text();
  frame.lines().enumerate().filter(|(_, line)| line.contains(text)).map(|(at, _)| at).collect()
}

#[test]
fn a_file_that_he_named_comes_first_among_the_files_that_match_as_well() {
  let (app, daemon, _) = started();
  let folder = daemon.file("furb-try");
  for file in ["src/a.rs", "src/b.rs", "docs/z y.md", "docs/c.md"] {
    fs::create_dir_all(folder.join(file).parent().unwrap()).unwrap();
    fs::write(folder.join(file), "").unwrap();
  }
  // A mark that ends a sentence is no part of a path, a quoted path keeps its blank, and an at sign
  // with no path names nothing.
  sent(&app, &format!("Read @src/b.rs, then @\"docs/z y.md\" and mail me@ once. {ANSWERS}"));
  shows(&app, "Resolved 1");
  app.type_text("@rs");
  shows(&app, "a.rs  src/");
  let (b, a) = (rows(&app, "b.rs  src/"), rows(&app, "a.rs  src/"));
  assert!(b.len() == 1 && a.len() == 1 && b[0] < a[0], "{}", app.text());
  for _ in 0..3 {
    app.press("backspace");
  }
  app.type_text("@md");
  shows(&app, "c.md    docs/");
  let (named, other) = (rows(&app, "z y.md  docs/"), rows(&app, "c.md    docs/"));
  assert!(named.len() == 1 && named[0] < other[0], "{}", app.text());
}

#[test]
fn a_mention_lights_from_its_mark_to_the_end_of_its_path_and_its_quotes() {
  let (app, _daemon, _) = started();
  let text = "Read @src/lib.rs, and @\"a b.md\" now.";
  app.type_text(text);
  shows(&app, text);
  let accent = Tokens::on(None).accent;
  let frame = app.render();
  let width = frame.width as usize;
  let y = frame.text().lines().position(|line| line.contains(text)).unwrap();
  let x = frame.text().lines().nth(y).unwrap().chars().position(|one| one == 'R').unwrap();
  let lit: String = text
    .chars()
    .enumerate()
    .map(|(at, one)| if frame.cells[y * width + x + at].fg == accent { one } else { '.' })
    .collect();
  assert_eq!(lit, ".....@src/lib.rs......@\"a b.md\".....");
}
