//! The voice of the operator in the input, on a program of the speech that says a script: a held
//! space bar writes what he says at the caret as he says it, and its release gives the final text.
//! The program is a script of the shell, so the suite runs on unix alone.

#![cfg(test)]
#![cfg(unix)]

use std::{fs, os::unix::fs::PermissionsExt, time::Duration};

use furb_window::{Options, open};
use opentui::{
  input::KeyEventKind,
  prelude::{KeyCode, KeyEvent, Modifiers},
  testing::TestApp,
};

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
  // The bar of the time, the last row of the window.
  let same = |frame: &str| frame.lines().last().map(str::to_owned);
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

/// A space as a terminal tells it with the keys of Kitty that tell each kind of event.
fn space(app: &TestApp, kind: KeyEventKind) {
  let code = KeyCode::Char(' ');
  app.key(KeyEvent { code, modifiers: Modifiers::NONE, kind, base_layout: None, keypad: false });
}

#[test]
fn where_the_terminal_tells_each_release_a_hold_never_plays_the_bar_and_a_tap_plays_it_at_once() {
  let (app, mut daemon) = listened();
  let sessions = daemon.project("furb");
  let mut peer = daemon.peer();
  let life = peer.opened(&sessions.join("time.jsonl"), true);
  peer.said(&life, &format!("how long does an open take {ANSWERS}"));
  app.click(80, row(&app, "how long does an"));
  shows(&app, "The open takes");
  app.press("ctrl+x");
  app.press("b");
  space(&app, KeyEventKind::Release);
  shows(&app, "▶  ◆");
  // The bar steps back to the first moment, so a play has moments to show.
  for _ in 0..5 {
    app.press("left");
  }
  let bar = app.text();
  // The bar of the time, the last row of the window.
  let same = |frame: &str| frame.lines().last().map(str::to_owned);
  // A tap plays the bar at its release, and a second tap stops it.
  space(&app, KeyEventKind::Press);
  space(&app, KeyEventKind::Release);
  app.advance(Duration::from_millis(40));
  assert_ne!(same(&app.text()), same(&bar));
  space(&app, KeyEventKind::Press);
  space(&app, KeyEventKind::Release);
  let bar = app.text();
  app.advance(Duration::from_secs(1));
  assert_eq!(same(&app.text()), same(&bar));
  // The bar again at now, where the input records.
  app.press("escape");
  app.press("ctrl+x");
  app.press("b");
  let bar = shows(&app, "▶  ◆");
  // The press of a hold, its pause, its repeats and its release leave the bar as it stood.
  space(&app, KeyEventKind::Press);
  app.advance(Duration::from_millis(250));
  assert_eq!(same(&app.text()), same(&bar));
  space(&app, KeyEventKind::Repeat);
  app.advance(Duration::from_millis(30));
  space(&app, KeyEventKind::Repeat);
  typed(&app, "read the engine");
  assert_eq!(same(&app.text()), same(&bar));
  app.advance(Duration::from_millis(400));
  space(&app, KeyEventKind::Release);
  typed(&app, "read the engine.");
  assert_eq!(same(&app.text()), same(&bar));
}

#[test]
fn a_speech_that_gives_no_final_text_ends_its_record_and_a_tap_types_a_space_again() {
  // The program says its guesses and never answers the stop.
  let mute = "#!/bin/sh\necho '{\"ready\":true}'\nwhile read -r line; do\n  case $line in\n    start) echo '{\"text\":\"read the\"}' ;;\n  esac\ndone\n";
  let (app, _daemon) = listened_to(mute);
  app.paste("please");
  hold(&app);
  typed(&app, "please read the");
  // He let go: the record ends by itself with the text so far, and a space types again.
  app.advance(Duration::from_millis(400));
  app.advance(Duration::from_secs(4));
  shows(&app, "The speech gave no final text");
  assert!(!app.text().contains("● Listening"), "{}", app.text());
  app.type_text(" ok");
  typed(&app, "please read the ok");
}

#[test]
fn a_key_that_is_no_space_ends_a_record_with_its_text_so_far() {
  let mute = "#!/bin/sh\necho '{\"ready\":true}'\nwhile read -r line; do\n  case $line in\n    start) echo '{\"text\":\"read the\"}' ;;\n  esac\ndone\n";
  let (app, _daemon) = listened_to(mute);
  hold(&app);
  typed(&app, "read the");
  shows(&app, "● Listening");
  app.type_text("x");
  typed(&app, "read thex");
  assert!(!app.text().contains("● Listening"), "{}", app.text());
}

#[test]
fn a_bar_held_through_a_reload_records_from_its_repeats_in_the_new_window() {
  // The new window hears no press: the terminal goes on with the repeats of the bar.
  let (app, _daemon) = listened();
  space(&app, KeyEventKind::Repeat);
  app.advance(Duration::from_millis(30));
  space(&app, KeyEventKind::Repeat);
  typed(&app, "read the engine");
  shows(&app, "● Listening");
  app.advance(Duration::from_millis(400));
  space(&app, KeyEventKind::Release);
  typed(&app, "read the engine.");
}
