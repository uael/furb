//! A text in the room of a row: cut with a mark where it does not fit, or broken into lines.
#![expect(clippy::string_slice, reason = "a panic is how a test fails")]
#![cfg(test)]

use std::fs;

use furb::view::Standing;
use furb_window::{Options, open};

use crate::{
  cards::{app, options, shows},
  daemon::Daemon,
  list::{card, kept},
};

#[test]
fn a_text_past_its_room_is_cut_between_words_with_a_mark_and_a_wide_letter_counts_two_columns() {
  let titles =
    ["the list of cards that waits", "listofcardsthatwaitsforhim", "会議の記録ですから全部読んで"];
  let cards = titles.iter().enumerate().map(|(at, one)| card(one, Standing::Rests, 60 + at as u64));
  let (app, _daemon) = kept(cards.collect());
  let frame = shows(&app, "Resting 3");
  let rows: Vec<String> = frame
    .lines()
    .filter(|line| line.contains("as of"))
    .map(|line| line.chars().skip(61).collect::<String>().trim().to_owned())
    .collect();
  // Each title ends at one column, so the hour beside it stands at one column too.
  let cut = [
    "the list of cards…  as of 00:00",
    "listofcardsthatwa…  as of 00:00",
    "会議の記録ですか…   as of 00:00",
  ];
  assert_eq!(rows, cut, "{frame}");
}

#[test]
fn a_word_wider_than_a_line_breaks_where_the_line_ends() {
  let app = app();
  let mut daemon = Daemon::bound(&app);
  daemon.served();
  let name = "a-folder-whose-name-is-wider-than-each-line-of-the-home-of-the-window";
  let folder = daemon.file(name);
  fs::create_dir_all(&folder).unwrap();
  open(
    &app,
    vec![daemon.machine("mac")],
    Options { folder, ..options(&daemon.file("cards.json")) },
  );
  let frame = shows(&app, "on this computer");
  let lines: Vec<&str> = frame.lines().map(str::trim).collect();
  let at = lines.iter().position(|line| name.starts_with(*line) && line.len() > 20);
  let at = at.unwrap_or_else(|| panic!("no line holds the start of the name alone:\n{frame}"));
  let rest = &name[lines[at].len()..];
  assert!(lines[at + 1].starts_with(&format!("{rest} on this computer")), "{frame}");
}
