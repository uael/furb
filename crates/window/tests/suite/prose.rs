//! The prose of a model in its thread, which is markdown: a table and a block of code in the room
//! of the feed.
#![expect(clippy::string_slice, reason = "a panic is how a test fails")]
#![cfg(test)]

use crate::{
  cards::shows,
  daemon::{MARKDOWN, TABLES},
  question::opened,
  screens::screen,
  thread::LEFT,
};

/// The rows of the feed from the first row that holds a text, a count of them, each with no blank
/// at its end.
fn from(frame: &str, text: &str, count: usize) -> Vec<String> {
  let feed = frame.lines().map(|line| line.chars().take(LEFT).collect::<String>());
  let rows = feed.skip_while(|line| !line.contains(text)).take(count);
  rows.map(|line| line.trim_end().to_owned()).collect()
}

#[test]
fn a_table_puts_each_cell_in_its_column_on_its_side_under_a_head_over_a_rule() {
  let (app, _daemon, ..) = opened(&format!("where does the open go\n{TABLES}"), "Stage");
  let frame = shows(&app, "boot");
  screen("prose-table", &app);
  let table = from(&frame, "Stage    Time", 4);
  let table: Vec<&str> = table.iter().map(|row| row.trim_start()).collect();
  assert_eq!(
    table,
    [
      "Stage    Time   Share",
      "─────────────────────",
      "seed    181 s    55%",
      "boot    104 s    3%"
    ],
    "{frame}"
  );
}

#[test]
fn a_table_wider_than_the_prose_cuts_its_widest_columns() {
  let (app, _daemon, ..) = opened(&format!("where does the open go\n{TABLES}"), "Stage");
  let frame = shows(&app, "the seed of each chain");
  let table = from(&frame, "Stage of the open", 3);
  let table: Vec<&str> = table.iter().map(|row| row.trim_start()).collect();
  assert_eq!(
    table,
    [
      "Stage of the open of a…   Where its time goes in…   Share",
      "─────────────────────────────────────────────────────────",
      "the seed of each chain    each entry sent to the…   55%",
    ],
    "{frame}"
  );
}

#[test]
fn a_block_of_code_keeps_on_its_right_the_columns_that_its_numbers_leave_blank_on_its_left() {
  let (app, _daemon, ..) = opened(&format!("where does the open go\n{TABLES}"), "Stage");
  shows(&app, "print('a line of code");
  let frame = app.render();
  let text = frame.text();
  let y = text.lines().position(|line| line.contains("print('a line of code")).unwrap();
  let row: Vec<char> = text.lines().nth(y).unwrap().chars().collect();
  let width = frame.width as usize;
  let ground = |x: usize| frame.cells[y * width + x].bg;
  let number = row.iter().position(|one| *one == '1').unwrap();
  let shown: String = row.iter().take(LEFT).collect();
  let end = shown[..shown.rfind("+23").unwrap()].chars().count() + 3;
  // The ground of the block runs from its left to its right, around the number and the code.
  let left = (0..number).rev().take_while(|at| ground(*at) == ground(number)).count();
  let right = (end..LEFT).take_while(|at| ground(*at) == ground(number)).count();
  assert_eq!((left, right), (2, 2), "{text}");
}

#[test]
fn each_part_of_the_markdown_of_github_draws_as_github_draws_it() {
  let (app, _daemon, ..) = opened(&format!("draw some markdown\n{MARKDOWN}"), "Alerts");
  app.resize(200, 140);
  shows(&app, "What a step can break.");
  screen("prose-markdown", &app);
}
