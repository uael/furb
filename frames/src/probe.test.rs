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
fn a_half_block_of_the_ground_of_a_box_carries_its_side() {
  // A card with a bar on its left and a half block of its ground on its right: the text stands
  // one cell inside each.
  let shown = frame(&[&[
    (" ", TEXT, BASE),
    ("▌", 0xec_b081, RAISED),
    ("text", TEXT, RAISED),
    ("▌", RAISED, BASE),
    (" ", TEXT, BASE),
  ]]);
  assert_eq!(says(&shown), Vec::<String>::new());
}

#[test]
fn a_card_on_half_rows_with_its_bar_in_half_a_column_is_one_box_with_its_corners() {
  // The selected card of the list: half a row of its ground above and under its text, its bar in
  // the right half of the column before it, and its ground in the left half of the column after.
  let hue = 0xec_b081;
  let band = |[bar, fill, end]: [&'static str; 3]| -> [(&'static str, u32, u32); 5] {
    [
      (" ", TEXT, BASE),
      (bar, hue, BASE),
      (fill, RAISED, BASE),
      (end, RAISED, BASE),
      (" ", TEXT, BASE),
    ]
  };
  let line = [
    (" ", TEXT, BASE),
    ("▐", hue, BASE),
    (" text ", TEXT, RAISED),
    ("▌", RAISED, BASE),
    (" ", TEXT, BASE),
  ];
  let shown = frame(&[&band(["▗", "▄▄▄▄▄▄", "▖"]), &line, &line, &band(["▝", "▀▀▀▀▀▀", "▘"])]);
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
fn a_slot_that_follows_the_slot_before_it_with_no_blank_flows_with_it() {
  // Two lines of code of the same colors: the parenthesis after a string stands where the string
  // ends, as in prose, and in no column.
  let muted = 0x94_9494;
  let line = |text: &'static str, tail: &'static str| -> [(&'static str, u32, u32); 4] {
    [("f(", muted, BASE), (text, TEXT, BASE), (")", muted, BASE), (tail, TEXT, BASE)]
  };
  let shown = frame(&[&line("'slept'", "     "), &line("'the bench'", " ")]);
  assert_eq!(says(&shown), Vec::<String>::new());
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
      "the part #1b1b1b that starts `text` has other insets at another size: 120x40 left 2 right 8 above 0 below 1, 80x24 left 1 right 5 above 0 below 1"
    ]
  );
}

#[test]
fn the_inset_above_a_part_that_the_scroll_of_the_feed_cuts_says_nothing() {
  // Under the head of the window, a part whose first row the feed scrolled away at one size.
  let blank = "             ";
  let head: &[(&str, u32, u32)] = &[(blank, TEXT, BASE)];
  let whole = frame(&[
    head,
    &[(blank, TEXT, RAISED)],
    &[("  one        ", TEXT, RAISED)],
    &[(blank, TEXT, RAISED)],
  ]);
  let cut = frame(&[head, &[("  one        ", TEXT, RAISED)], &[(blank, TEXT, RAISED)], head]);
  assert_eq!(
    sizes(&[("120x40".to_owned(), whole), ("80x24".to_owned(), cut)]),
    Vec::<String>::new()
  );
}

#[test]
fn a_part_is_the_same_part_at_each_size_by_its_text_and_not_by_its_place() {
  // Two parts of one ground, in one order at one size and in the other at another, as a feed that
  // scrolled shows them: each keeps its insets.
  let parts = |first: &'static str, second: &'static str| {
    let blank = "             ";
    frame(&[
      &[(first, TEXT, RAISED)],
      &[(blank, TEXT, RAISED)],
      &[(blank, TEXT, BASE)],
      &[(second, TEXT, RAISED)],
      &[(blank, TEXT, RAISED)],
    ])
  };
  let (one, other) = ("  one        ", " other       ");
  let found =
    sizes(&[("120x40".to_owned(), parts(one, other)), ("80x24".to_owned(), parts(other, one))]);
  assert_eq!(found, Vec::<String>::new());
}
