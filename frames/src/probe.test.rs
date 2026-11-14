use super::*;
use crate::frame::Run;

const BASE: u32 = 0x0a_0a0a;
const RAISED: u32 = 0x1b_1b1b;
const TEXT: u32 = 0xe4_e4e4;

/// A frame of rows, each a list of runs of a text, its color and its ground.
fn frame(rows: &[&[(&str, u32, u32)]]) -> Frame {
  let lines: Vec<Vec<Run>> = rows
    .iter()
    .map(|runs| {
      runs
        .iter()
        .map(|(text, fg, bg)| Run {
          text: (*text).to_owned(),
          fg: Rgb::hex(*fg),
          bg: Rgb::hex(*bg),
          attributes: 0,
        })
        .collect()
    })
    .collect();
  Frame { cols: rows[0].iter().map(|run| run.0.chars().count()).sum(), rows: rows.len(), lines }
}

fn says(frame: &Frame) -> Vec<String> {
  probe(frame)
    .into_iter()
    .map(|found| format!("{} {}: {}", found.row, found.col, found.says))
    .collect()
}

#[test]
fn a_color_that_no_token_gives_is_named_once_at_its_first_cell() {
  let shown = frame(&[
    &[("ab", 0x12_3456, BASE), ("c", TEXT, BASE)],
    &[("d", 0x12_3456, BASE), ("  ", TEXT, BASE)],
  ]);
  assert_eq!(says(&shown), ["0 0: the color #123456 of a glyph is no token (3 cells)"]);
}

#[test]
fn a_box_with_more_room_on_one_side_is_named() {
  let shown = frame(&[&[(" ", TEXT, BASE), ("  text ", TEXT, RAISED), (" ", TEXT, BASE)]]);
  assert_eq!(
    says(&shown),
    [
      "0 1: the box #1b1b1b of rows 0 to 0 and columns 1 to 7 is 2 cells wide inside on its left and 1 on its right"
    ]
  );
}

#[test]
fn a_bar_on_the_edge_of_a_box_is_its_border() {
  let shown = frame(&[&[("┃ text ┃", TEXT, RAISED)]]);
  assert_eq!(says(&shown), Vec::<String>::new());
}

#[test]
fn a_padding_that_the_scale_does_not_name_is_named() {
  let shown = frame(&[&[("   text   ", TEXT, RAISED)]]);
  assert_eq!(
    says(&shown),
    ["0 0: the box #1b1b1b of rows 0 to 0 and columns 0 to 9 has an inset of 3, which the scale [0, 1, 2] does not name";
      2]
  );
}

#[test]
fn a_slot_of_repeated_rows_in_no_one_column_is_named() {
  let muted = 0x94_9494;
  let rows = |gap: &str, tail: &str| {
    frame(&[
      &[("a", TEXT, BASE), ("  ", TEXT, BASE), ("one", muted, BASE), ("   ", TEXT, BASE)],
      &[("bb", TEXT, BASE), (gap, TEXT, BASE), ("two", muted, BASE), (tail, TEXT, BASE)],
    ])
  };
  assert_eq!(says(&rows(" ", "   ")), Vec::<String>::new());
  assert_eq!(
    says(&rows("  ", "  ")),
    [
      "0 3: the slot 2 of a repeated row neither starts nor ends in one column: row 0 3-5, row 1 4-6"
    ]
  );
}

#[test]
fn a_part_with_another_padding_at_another_size_is_named() {
  let part = |left: &str, right: &str| {
    let text = format!("{left}text{right}");
    let blank = " ".repeat(text.len());
    let rows: [&[(&str, u32, u32)]; 2] = [&[(&text, TEXT, RAISED)], &[(&blank, TEXT, RAISED)]];
    frame(&rows)
  };
  let found = sizes(&[
    ("120x40".to_owned(), part("  ", "        ")),
    ("80x24".to_owned(), part(" ", "     ")),
  ]);
  assert_eq!(
    found,
    [
      "the part #1b1b1b number 1 from the top has other insets at another size: 120x40 left 2 right 8 above 0 below 1, 80x24 left 1 right 5 above 0 below 1"
    ]
  );
}
