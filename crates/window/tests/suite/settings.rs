//! The settings of a session as a list part, on a real daemon: a tab for each group of keys, furb
//! and each kit that declares keys, which the arrows and a click show.

#![cfg(test)]

use furb_window::open;
use opentui::testing::TestApp;

use crate::{
  cards::{app, options, row, shows},
  daemon::{Daemon, PATIENCE},
};

#[test]
fn the_settings_show_one_group_of_keys_and_the_arrows_and_a_click_show_each_other() {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  let sessions = daemon.project("furb");
  daemon.served();
  let mut peer = daemon.peer();
  let life = peer.opened(&sessions.join("settings.jsonl"), true);
  peer.said(&life, "which settings hold here");
  open(&app, vec![daemon.machine("mac")], options(&daemon.file("cards.json")));
  app.click(80, row(&app, "which settings h"));
  shows(&app, "which settings hold here");
  // The kit naming, which plays by default, declares a key, so it has a tab.
  app.press("ctrl+x");
  app.press(",");
  let furb = shows(&app, "max_thinking_tokens");
  assert!(furb.contains("furb   naming") && !furb.contains("naming.actor"), "{furb}");
  app.press("right");
  let shown = app.wait_until(PATIENCE, || {
    let frame = app.text();
    frame.contains("naming.actor") && !frame.contains("max_thinking_tokens")
  });
  assert!(shown, "{}", app.text());
  let tabs = row(&app, "furb   naming");
  let column = app.text().lines().nth(usize::from(tabs)).and_then(|one| one.find("furb"));
  app.click(u16::try_from(column.unwrap_or_default()).unwrap_or_default(), tabs);
  shows(&app, "max_thinking_tokens");
}

/// The column where the part of the settings starts in a frame: that of its title.
fn left(frame: &str) -> Option<usize> {
  frame.lines().find_map(|one| one.find("Settings"))
}

#[test]
fn the_settings_keep_their_columns_while_he_types_a_search_and_changes_the_group() {
  // A wide window, where the part takes the columns that its rows ask and not those of the window.
  let app = TestApp::new(200, 60);
  let mut daemon = Daemon::bound(&app);
  let sessions = daemon.project("furb");
  daemon.served();
  let mut peer = daemon.peer();
  let life = peer.opened(&sessions.join("settings.jsonl"), true);
  peer.said(&life, "which settings hold here");
  open(&app, vec![daemon.machine("mac")], options(&daemon.file("cards.json")));
  app.click(170, row(&app, "which settings h"));
  shows(&app, "which settings hold here");
  app.press("ctrl+x");
  app.press(",");
  let opened = left(&shows(&app, "max_thinking_tokens"));
  app.press("right");
  let grouped = app.wait_until(PATIENCE, || !app.text().contains("max_thinking_tokens"));
  assert!(grouped, "{}", app.text());
  assert_eq!(left(&app.text()), opened, "{}", app.text());
  app.type_text("naming.ac");
  let found = app.wait_until(PATIENCE, || app.text().contains("1 / 1"));
  assert!(found, "{}", app.text());
  assert_eq!(left(&app.text()), opened, "{}", app.text());
}
