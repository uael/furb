//! The voice of the operator in the input, on a program of the speech that says a script: a held
//! space bar writes what he says at the caret as he says it, and its release gives the final text.
//! The program is a script of the shell, so the suite runs on unix alone.

#![cfg(test)]
#![cfg(unix)]

use std::{fs, os::unix::fs::PermissionsExt, time::Duration};

use furb_window::{Options, open};
use opentui::testing::TestApp;

use crate::{
  cards::{app, options, row, shows},
  daemon::{ANSWERS, Daemon},
  input::typed,
};

/// What the program of a test says: that it is ready, then for each record a level of the
/// microphone and two guesses of the text as he speaks, and the final text once the record stops.
const SAYS: &str = r#"#!/bin/sh
echo '{"ready":true}'
while read -r line; do
  case $line in
    start) echo '{"level":40}'; echo '{"text":"read the"}'; echo '{"text":"read the engine"}' ;;
    stop) echo '{"text":"read the engine.","final":true}' ;;
  esac
done
"#;

/// A window on a program of the speech that says the script, once its models came.
fn listened() -> (TestApp, Daemon) {
  listened_to(SAYS)
}

/// A window on a program of the speech that says a script, once its models came.
fn listened_to(says: &str) -> (TestApp, Daemon) {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  daemon.project("furb");
  daemon.served();
  let listen = daemon.file("listen");
  fs::write(&listen, says).unwrap();
  fs::set_permissions(&listen, fs::Permissions::from_mode(0o755)).unwrap();
  let options = Options { listen: Some(listen), ..options(&daemon.file("cards.json")) };
  open(&app, vec![daemon.machine("mac")], options);
  shows(&app, "opus  high");
  (app, daemon)
}

/// The bar held down as a terminal tells it: a press, its first repeat after a pause, then the
/// repeats that come at once, the first of which starts the record.
fn hold(app: &TestApp) {
  app.type_text(" ");
  app.advance(Duration::from_millis(250));
  app.type_text(" ");
  app.advance(Duration::from_millis(30));
  app.type_text(" ");
}

#[test]
fn a_held_space_writes_his_voice_at_the_caret_live_and_a_tap_stays_a_space() {
  let (app, _daemon) = listened();
  app.paste("please");
  // The spaces of the hold leave the input, and the guesses come at the caret as he speaks, while
  // the row under the input says that it listens.
  hold(&app);
  typed(&app, "please read the engine");
  shows(&app, "● Listening ");
  // The repeats of the bar hold the record, and their end ends it with the final text.
  app.advance(Duration::from_millis(30));
  app.type_text(" ");
  app.advance(Duration::from_millis(400));
  typed(&app, "please read the engine.");
  assert!(!app.text().contains("● Listening"), "{}", app.text());
  // A tap of the bar types a space.
  app.advance(Duration::from_millis(400));
  app.type_text(" ok");
  typed(&app, "please read the engine. ok");
}

#[test]
fn a_held_space_records_over_the_bar_of_the_time_which_a_tap_plays() {
  let (app, mut daemon) = listened();
  let sessions = daemon.project("furb");
  let mut peer = daemon.peer();
  let life = peer.opened(&sessions.join("time.jsonl"), true);
  peer.said(&life, &format!("how long does an open take {ANSWERS}"));
  app.click(80, row(&app, "how long does an"));
  shows(&app, "The open takes");
  app.press("ctrl+x");
  app.press("b");
  shows(&app, "Space play");
  // The track of the bar, which stands at now and does not play.
  let bar = shows(&app, "▶  ◆");
  // The hold leaves the bar as it stood, though its taps played it and paused it, and records.
  hold(&app);
  typed(&app, "read the engine");
  app.advance(Duration::from_millis(400));
  typed(&app, "read the engine.");
  let same = |frame: &str| frame.lines().take(3).map(str::to_owned).collect::<Vec<_>>();
  assert_eq!(same(&app.text()), same(&bar));
}

#[test]
fn a_speech_that_ends_before_a_hold_says_nothing_and_the_hold_says_why() {
  // The window starts the program with its first frame, before he asks for his voice, as on a
  // machine whose speech cannot run.
  let (app, _daemon) = listened_to("#!/bin/sh\nexit 0\n");
  app.advance(Duration::from_millis(400));
  assert!(!app.text().contains("The speech ended."), "{}", app.text());
  hold(&app);
  shows(&app, "The speech ended.");
}
