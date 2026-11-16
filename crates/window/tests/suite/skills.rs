//! The skills and extensions of a session as a list part, on a real daemon: how each plays, and
//! the view of an extension that enter opens, which turns it on or off in place.

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
    // The thumb of a list that scrolls stands at the right of its rows.
    row.starts_with(&said) && row.trim_end_matches([' ', '┃']).ends_with(stands)
  });
  assert!(shown, "{name} never stood as {stands:?}:\n{}", app.text());
  app.text()
}

#[test]
fn the_skills_of_a_session_say_how_each_plays_and_the_view_of_one_turns_it_on_at_once() {
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
  // Enter opens the view of the extension: what it is, how it plays, and what he does to it.
  app.press("enter");
  let view = shows(&app, "Turn it on");
  assert!(view.contains("plays not in this session") && view.contains("From      furb"), "{view}");
  assert!(view.contains("Reload the extensions of this session"), "{view}");
  // Its first action turns it on at once, and the view says so in place.
  app.press("enter");
  let view = shows(&app, "Turn it off");
  assert!(view.contains("plays in this session"), "{view}");
  // Escape goes back to the extensions, on its row.
  app.press("escape");
  plays(&app, "web", " on  furb");
  // Turned off, it plays on until the session ends.
  app.press("enter");
  shows(&app, "Turn it off");
  app.press("enter");
  shows(&app, "Next      off at the next session");
  app.press("escape");
  plays(&app, "web", "off at the next session  furb");
}
