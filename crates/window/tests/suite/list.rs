//! The list of the cards, on cards that a window kept while their machine does not answer: the
//! keys that step through it, and its rows whatever the count of its cards.

#![cfg(test)]

use std::{fs, path::Path};

use furb::view::{self, Standing};
use furb_client::Told;
use furb_host::protocol::Card;
use furb_window::open;
use opentui::testing::TestApp;

use crate::{
  cards::{CLOCK, app, options, shows},
  daemon::Daemon,
};

/// A card of a thread that the operator read, which stood since a count of seconds before the
/// clock of the window.
pub(crate) fn card(title: &str, standing: Standing, before: u64) -> Card {
  let thread = view::Card {
    prompt: "prompt1".into(),
    title: title.to_owned(),
    standing,
    model: Some("claude-cli:opus".to_owned()),
    agents: 0,
    readers: vec!["operator".to_owned()],
  };
  Card {
    life: format!("/home/ada/src/furb/.furb/sessions/{title}.jsonl"),
    session: None,
    project: None,
    refused: None,
    held: true,
    viewers: Vec::new(),
    pins: Vec::new(),
    since: CLOCK - before,
    thread,
  }
}

/// A window whose machine does not answer, on the cards that the window before it kept.
pub(crate) fn kept(cards: Vec<Card>) -> (TestApp, Daemon) {
  let app = app();
  let daemon = Daemon::bound(&app);
  let file = daemon.file("cards.json");
  let told = Told { name: "mac".to_owned(), login: "ada".to_owned(), cards, ..Told::default() };
  write(&file, &[told]);
  open(&app, vec![daemon.machine("mac")], options(&file));
  (app, daemon)
}

fn write(file: &Path, told: &[Told]) {
  fs::write(file, serde_json::to_vec(told).unwrap()).unwrap();
}

/// The title of the thread that the selected card opened, which the head of the window names.
fn selected(app: &TestApp) -> String {
  let frame = app.text();
  let head = frame.lines().next().unwrap_or_default();
  let title = head.split(" · ").nth(1).unwrap_or_default();
  title.split_whitespace().next().unwrap_or_default().to_owned()
}

#[test]
fn the_keys_step_through_the_cards_and_to_each_thread_that_needs_the_operator() {
  let cards = vec![
    card("asks", Standing::Asks { of: None }, 60),
    card("failed", Standing::Failed, 50),
    card("works", Standing::Thinks, 40),
    card("done", Standing::Resolved, 30),
  ];
  let (app, _daemon) = kept(cards);
  // The operator read each thread, so the one that failed rests.
  let frame = shows(&app, "Resolved 1 ▸");
  let heads = ["Needs you 1", "Working 1", "Resting 1"];
  assert!(heads.iter().all(|head| frame.contains(head)), "{frame}");
  app.press("alt+down");
  app.press("alt+down");
  assert_eq!(selected(&app), "works");
  // The resolved threads are folded, so the last card of the keys is the one before them, and a
  // step past it shows them and goes to the first.
  app.press("alt+down");
  assert_eq!(selected(&app), "failed");
  assert!(app.text().contains("Resolved 1 ▸"), "{}", app.text());
  app.press("alt+down");
  shows(&app, "Resolved 1 ▾");
  assert_eq!(selected(&app), "done");
  app.press("alt+a");
  assert_eq!(selected(&app), "asks");
}

#[test]
fn the_list_shows_its_rows_whatever_the_count_of_the_cards() {
  let cards = (0..600).map(|n| card(&format!("thread{n:03}"), Standing::Rests, 600 + n));
  let (app, _daemon) = kept(cards.collect());
  shows(&app, "Resting 600");
  // The selected card stays in the rows that show.
  for _ in 0..40 {
    app.press("alt+down");
  }
  assert_eq!(selected(&app), "thread039");
  let list: Vec<String> = app.text().lines().map(|line| line.chars().skip(61).collect()).collect();
  let list = list.join("\n");
  assert!(!list.contains("Resting 600") && list.contains("thread039"), "{list}");
  // The list goes back to its head.
  for _ in 0..40 {
    app.scroll(70, 10, false);
  }
  shows(&app, "Resting 600");
}

#[test]
fn a_machine_that_is_lost_keeps_its_cards() {
  let (app, mut daemon) = kept(vec![card("asks", Standing::Asks { of: None }, 60)]);
  daemon.lost();
  let frame = shows(&app, "mac offline");
  assert!(frame.contains("asks") && frame.contains("Needs you 1"), "{frame}");
}
