//! The skills and extensions of a session as a list part, on a real daemon: how each plays, and
//! enter, which turns an extension on or off.

#![cfg(test)]

use furb_window::open;
use opentui::testing::TestApp;

use crate::{
  cards::{app, options, row, shows},
  daemon::{Daemon, PATIENCE},
};

/// The row of the list that names a skill, from its name: its name, then what it is for. The
/// filter above the rows may hold the name too.
fn line(frame: &str, name: &str) -> String {
  let head = format!("{name}  ");
  let at = frame.lines().rev().find_map(|line| line.find(&head).map(|at| &line[at..]));
  at.unwrap_or_default().to_owned()
}

/// The frame once the row of a skill says how it plays, from its name and what follows it.
fn plays(app: &TestApp, name: &str, stands: &str) -> String {
  let said = format!("{name}  ");
  let shown = app.wait_until(PATIENCE, || {
    let frame = app.text();
    let row = line(&frame, name);
    row.starts_with(&said) && row.trim_end().ends_with(stands)
  });
  assert!(shown, "{name} never stood as {stands:?}:\n{}", app.text());
  app.text()
}

#[test]
fn the_skills_of_a_session_say_how_each_plays_and_enter_turns_one_on_at_once() {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  let sessions = daemon.project("furb");
  daemon.served();
  let mut peer = daemon.peer();
  let life = peer.opened(&sessions.join("skills.jsonl"), true);
  peer.said(&life, "which skills play here");
  open(&app, vec![daemon.machine("mac")], options(&daemon.file("cards.json")));
  app.click(80, row(&app, "which skills pl"));
  shows(&app, "which skills play here");
  app.press("ctrl+x");
  app.press("x");
  // The life of the test opens with no skill, so each default one plays at the next session. Each
  // says what it does under its name, so a filter brings a row in view.
  app.type_text("memory");
  plays(&app, "memory", "on at the next session  furb");
  for _ in 0.."memory".len() {
    app.press("backspace");
  }
  app.type_text("web");
  plays(&app, "web", "off  furb");
  app.press("enter");
  plays(&app, "web", " on  furb");
  // A second enter turns it off, and it plays on until the session ends.
  app.press("enter");
  plays(&app, "web", "off at the next session  furb");
}
