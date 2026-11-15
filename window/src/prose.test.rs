use opentui::style::Span;

use super::lines;
use crate::{code::Colors, tokens::Tokens};

/// The text of each line of a text of markdown in a count of columns, with no blank at its end.
fn texts(markdown: &str, room: usize) -> Vec<String> {
  let (tokens, colors) = (Tokens::on(None), Colors::new());
  let text = |line: &Vec<Span<'_>>| line.iter().map(|one| one.content.as_ref()).collect::<String>();
  lines(markdown, room, &tokens, &colors)
    .iter()
    .map(|line| text(line).trim_end().to_owned())
    .collect()
}

/// A table with a column on each side.
const TABLE: &str =
  "| Stage | Time | Share |\n| --- | ---: | :---: |\n| seed | 181 s | 55% |\n| boot | 104 s | 3% |";

#[test]
fn a_table_puts_each_cell_in_its_column_on_its_side_under_a_head_over_a_rule() {
  let drawn = [
    "Stage    Time   Share",
    "─────────────────────",
    "seed    181 s    55%",
    "boot    104 s    3%",
  ];
  assert_eq!(texts(TABLE, 60), drawn);
}

#[test]
fn a_table_wider_than_the_prose_cuts_its_widest_columns() {
  let drawn = ["St…   Ti…   Sh…", "───────────────", "se…   18…   55%", "bo…   10…   3%"];
  assert_eq!(texts(TABLE, 15), drawn);
}
