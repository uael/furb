//! The settings of a session as a list part, on a real daemon: a tab for each group of keys, furb
//! and each kit that declares keys, which the arrows and a click show.

#![cfg(test)]

use furb_window::open;

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
