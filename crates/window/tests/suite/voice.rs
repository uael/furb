//! The voice of the operator in the input, on a program of the speech that says a script: a held
//! space bar writes what he says at the caret as he says it, and its release gives the final text.
//! The program is a script of the shell, so the suite runs on unix alone.

#![cfg(unix)]

use std::{fs, os::unix::fs::PermissionsExt, time::Duration};

use furb_window::{Options, open};

use crate::{
  cards::{app, options, shows},
  daemon::Daemon,
  input::typed,
};

/// What the program of a test says: a level of the microphone, two guesses of the text as he
/// speaks, then the final text once its input ends.
const SAYS: &str = r#"#!/bin/sh
echo '{"level":40}'
echo '{"text":"read the"}'
echo '{"text":"read the engine"}'
cat > /dev/null
echo '{"text":"read the engine.","final":true}'
"#;

#[test]
fn a_held_space_writes_his_voice_at_the_caret_live_and_a_tap_stays_a_space() {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  daemon.project("furb");
  daemon.served();
  let listen = daemon.file("listen");
  fs::write(&listen, SAYS).unwrap();
  fs::set_permissions(&listen, fs::Permissions::from_mode(0o755)).unwrap();
  let options = Options { listen: Some(listen), ..options(&daemon.file("cards.json")) };
  open(&app, vec![daemon.machine("mac")], options);
  shows(&app, "opus  high");
  app.paste("please");
  // One tap of the bar types a space, and so does its first repeat, which a terminal sends after
  // a pause; the repeats that follow at once start the record, and the spaces of the hold go.
  app.type_text(" ");
  typed(&app, "please");
  app.advance(Duration::from_millis(250));
  app.type_text(" ");
  app.advance(Duration::from_millis(30));
  app.type_text(" ");
  typed(&app, "please read the engine");
  // The repeats of the bar hold the record, and their end ends it with the final text.
  app.advance(Duration::from_millis(30));
  app.type_text(" ");
  app.advance(Duration::from_millis(400));
  typed(&app, "please read the engine.");
  app.advance(Duration::from_millis(400));
  app.type_text(" ok");
  typed(&app, "please read the engine. ok");
}
