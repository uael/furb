//! The window on each system of a client: how it names each key, and how it shows the folder of
//! the home.

#![cfg(test)]

use std::path::PathBuf;

use furb_client::HOLD;
use furb_window::{Options, System, open};

use crate::{
  cards::{app, options, row, shows},
  daemon::{Daemon, TICKS},
  screens::screen,
};

#[test]
fn on_another_system_a_key_shows_each_key_that_it_holds_as_a_word() {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  let sessions = daemon.project("furb");
  daemon.served();
  let mut peer = daemon.peer();
  let life = peer.opened(&sessions.join("side.jsonl"), true);
  peer.said(&life, &format!("which keys does the form take\n{TICKS}"));
  let options = Options { system: System::Other, ..options(&daemon.file("cards.json")) };
  open(&app, vec![daemon.machine("mac")], options);
  app.click(80, row(&app, "which keys does the"));
  shows(&app, "[ ] tab");
  app.advance(HOLD);
  let frame = shows(&app, "↑↓ choose · Space tick · Enter answer");
  assert!(frame.contains("Ctrl+C refuse"), "{frame}");
  app.press("ctrl+x");
  app.press("?");
  let frame = shows(&app, "Search a key");
  for keys in ["Ctrl+P or Ctrl+O", "Alt+A or Ctrl+X A", "Alt+↓ or Ctrl+X J", "Ctrl+X L", "Esc Esc"]
  {
    assert!(frame.contains(keys), "{keys}: {frame}");
  }
  screen("system-keys", &app);
}

/// The folder that a window shows under its input, in a folder of a home, at the left of the hints.
fn shown(folder: &str, home: &str) -> String {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  daemon.served();
  let options = Options {
    folder: PathBuf::from(folder),
    home: Some(PathBuf::from(home)),
    ..options(&daemon.file("cards.json"))
  };
  open(&app, vec![daemon.machine("mac")], options);
  let frame = shows(&app, "No thread yet");
  let last = frame.lines().last().unwrap_or_default().trim();
  last.split("  ").next().unwrap_or_default().to_owned()
}

#[test]
fn a_folder_of_the_home_shows_as_a_path_of_the_home_with_a_slash_on_each_system() {
  assert_eq!(shown(r"C:\Users\ada\src\furb", r"C:\Users\ada"), "~/src/furb");
  assert_eq!(shown(r"C:\Users\adam\furb", r"C:\Users\ada"), r"C:\Users\adam\furb");
  assert_eq!(shown("/srv/furb", "/home/ada"), "/srv/furb");
}
