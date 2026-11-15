use opentui::style::Color;
use unicode_width::UnicodeWidthStr;

use super::{INNER, Kind, bar};

#[test]
fn the_bar_fills_its_row_and_a_part_that_holds_tokens_takes_a_cell() {
  let kind = |tokens| Kind { name: "New", tokens, color: Color::hex(0) };
  let line = bar(&[kind(620_000), kind(10), kind(379_990)], 1_000_000);
  let cells: Vec<usize> = line.iter().skip(1).map(|one| one.content.width()).collect();
  assert_eq!(cells, [22, 1, INNER - 23]);
}
