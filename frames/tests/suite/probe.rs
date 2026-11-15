//! The command `probe`: the findings of the probes on frames of the window, from their cells as
//! JSON, on each frame and across the sizes of a step.

use std::{fs, path::Path};

use furb_window::tokens::Tokens;
use serde_json::json;

use crate::{frames, put, said, yard};

const BASE: u32 = 0x0a_0a0a;
const RAISED: u32 = 0x1b_1b1b;
const TEXT: u32 = 0xe4_e4e4;
const MUTED: u32 = 0x94_9494;

/// The rows of a frame, each a list of runs of a text, its color and its ground.
type Rows<'a> = &'a [&'a [(&'a str, u32, u32)]];

/// The first hue of a thread that the tokens give, as the bar of a card takes it.
fn hue() -> u32 {
  let hue = Tokens::on(None).hues[0];
  u32::from_be_bytes([0, hue.r, hue.g, hue.b])
}

/// A frame of rows, each a list of runs of a text, its color and its ground, written as the JSON
/// of a frame at a path.
fn frame(path: &Path, rows: Rows<'_>) {
  let color = |one: u32| format!("#{one:06x}");
  let lines: Vec<Vec<_>> = rows
    .iter()
    .map(|runs| {
      runs.iter().map(|(text, fg, bg)| json!([text, color(*fg), color(*bg), 0])).collect()
    })
    .collect();
  let cols: usize = rows[0].iter().map(|run| run.0.chars().count()).sum();
  put(path, &json!({ "cols": cols, "rows": rows.len(), "lines": lines }).to_string());
}

/// The findings of the probes on frames, each written at its path in a yard, one for each line.
fn found(name: &str, written: &[(&str, Rows<'_>)]) -> Vec<String> {
  let yard = yard(name);
  for (path, rows) in written {
    frame(&yard.join(path), rows);
  }
  let paths: Vec<&str> = written.iter().map(|one| one.0).collect();
  said(&frames(&[&["probe"][..], &paths].concat(), &yard)).lines().map(str::to_owned).collect()
}

/// The findings of the probes on one frame.
fn says(name: &str, rows: Rows<'_>) -> Vec<String> {
  found(name, &[("f.json", rows)])
}

#[test]
fn a_color_that_no_token_gives_is_named_once_at_its_first_cell() {
  let rows: &[&[_]] =
    &[&[("ab", 0x12_3456, BASE), ("c", TEXT, BASE)], &[("d", 0x12_3456, BASE), ("  ", TEXT, BASE)]];
  assert_eq!(
    says("color", rows),
    ["f.json row 0 col 0: the color #123456 of a glyph is no token (3 cells)"]
  );
}

#[test]
fn a_box_with_more_room_on_one_side_is_named() {
  let rows: &[&[_]] = &[&[(" ", TEXT, BASE), ("  text ", TEXT, RAISED), (" ", TEXT, BASE)]];
  assert_eq!(
    says("room", rows),
    [
      "f.json row 0 col 1: the box #1b1b1b of rows 0 to 0 and columns 1 to 7 is 2 cells wide inside on its left and 1 on its right"
    ]
  );
}

#[test]
fn a_bar_on_the_edge_of_a_box_is_its_border() {
  assert_eq!(says("bar", &[&[("┃ text ┃", TEXT, RAISED)]]), Vec::<String>::new());
}

#[test]
fn a_half_block_of_the_ground_of_a_box_carries_its_side() {
  // A card with a bar on its left and a half block of its ground on its right: the text stands
  // one cell inside each.
  let rows: &[&[_]] = &[&[
    (" ", TEXT, BASE),
    ("▌", hue(), RAISED),
    ("text", TEXT, RAISED),
    ("▌", RAISED, BASE),
    (" ", TEXT, BASE),
  ]];
  assert_eq!(says("half", rows), Vec::<String>::new());
}

#[test]
fn a_card_on_half_rows_with_its_bar_in_half_a_column_is_one_box_with_its_corners() {
  // The selected card of the list: half a row of its ground above and under its text, its bar in
  // the right half of the column before it, and its ground in the left half of the column after.
  let hue = hue();
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
  let rows: &[&[_]] = &[&band(["▗", "▄▄▄▄▄▄", "▖"]), &line, &line, &band(["▝", "▀▀▀▀▀▀", "▘"])];
  assert_eq!(says("card", rows), Vec::<String>::new());
}

#[test]
fn a_padding_that_the_scale_does_not_name_is_named() {
  let said = "f.json row 0 col 0: the box #1b1b1b of rows 0 to 0 and columns 0 to 9 has an inset of 3, which the scale [0, 1, 2] does not name";
  assert_eq!(says("padding", &[&[("   text   ", TEXT, RAISED)]]), [said; 2]);
}

#[test]
fn a_slot_of_repeated_rows_in_no_one_column_is_named() {
  let rows = |gap: &'static str, tail: &'static str| -> [[(&'static str, u32, u32); 4]; 2] {
    [
      [("a", TEXT, BASE), ("  ", TEXT, BASE), ("one", MUTED, BASE), ("   ", TEXT, BASE)],
      [("bb", TEXT, BASE), (gap, TEXT, BASE), ("two", MUTED, BASE), (tail, TEXT, BASE)],
    ]
  };
  let [one, two] = rows(" ", "   ");
  assert_eq!(says("column", &[&one, &two]), Vec::<String>::new());
  let [one, two] = rows("  ", "  ");
  assert_eq!(
    says("slot", &[&one, &two]),
    [
      "f.json row 0 col 3: the slot 2 of a repeated row neither starts nor ends in one column: row 0 3-5, row 1 4-6"
    ]
  );
}

#[test]
fn a_slot_that_follows_the_slot_before_it_with_no_blank_flows_with_it() {
  // Two lines of code of the same colors: the parenthesis after a string stands where the string
  // ends, as in prose, and in no column.
  let line = |text: &'static str, tail: &'static str| -> [(&'static str, u32, u32); 4] {
    [("f(", MUTED, BASE), (text, TEXT, BASE), (")", MUTED, BASE), (tail, TEXT, BASE)]
  };
  assert_eq!(
    says("flows", &[&line("'slept'", "     "), &line("'the bench'", " ")]),
    Vec::<String>::new()
  );
}

#[test]
fn a_part_with_another_padding_at_another_size_is_named() {
  let (wide, narrow) = (format!("  text{}", " ".repeat(8)), format!(" text{}", " ".repeat(5)));
  let (wide_blank, narrow_blank) = (" ".repeat(wide.len()), " ".repeat(narrow.len()));
  let found = found(
    "sizes",
    &[
      (
        "120x40/a.json",
        &[&[(wide.as_str(), TEXT, RAISED)], &[(wide_blank.as_str(), TEXT, RAISED)]],
      ),
      (
        "80x24/a.json",
        &[&[(narrow.as_str(), TEXT, RAISED)], &[(narrow_blank.as_str(), TEXT, RAISED)]],
      ),
    ],
  );
  assert_eq!(
    found,
    [
      "sizes a.json: the part #1b1b1b that starts `text` has other insets at another size: 120x40 left 2 right 8 above 0 below 1, 80x24 left 1 right 5 above 0 below 1"
    ]
  );
}

#[test]
fn the_inset_above_a_part_that_the_scroll_of_the_feed_cuts_says_nothing() {
  // Under the head of the window, a part whose first row the feed scrolled away at one size.
  let blank = "             ";
  let head: &[(&str, u32, u32)] = &[(blank, TEXT, BASE)];
  let whole: &[&[_]] =
    &[head, &[(blank, TEXT, RAISED)], &[("  one        ", TEXT, RAISED)], &[(blank, TEXT, RAISED)]];
  let cut: &[&[_]] = &[head, &[("  one        ", TEXT, RAISED)], &[(blank, TEXT, RAISED)], head];
  assert_eq!(
    found("cut", &[("120x40/a.json", whole), ("80x24/a.json", cut)]),
    Vec::<String>::new()
  );
}

#[test]
fn a_part_is_the_same_part_at_each_size_by_its_text_and_not_by_its_place() {
  // Two parts of one ground, in one order at one size and in the other at another, as a feed that
  // scrolled shows them: each keeps its insets.
  let blank = "             ";
  let parts = |first: &'static str, second: &'static str| -> [[(&'static str, u32, u32); 1]; 5] {
    [
      [(first, TEXT, RAISED)],
      [(blank, TEXT, RAISED)],
      [(blank, TEXT, BASE)],
      [(second, TEXT, RAISED)],
      [(blank, TEXT, RAISED)],
    ]
  };
  let (one, other) = ("  one        ", " other       ");
  let (first, second) = (parts(one, other), parts(other, one));
  let (first, second) =
    (first.each_ref().map(|row| &row[..]), second.each_ref().map(|row| &row[..]));
  let found = found("place", &[("120x40/a.json", &first), ("80x24/a.json", &second)]);
  assert_eq!(found, Vec::<String>::new());
}

#[test]
fn a_color_that_is_not_rrggbb_is_refused() {
  let yard = yard("refused");
  fs::write(yard.join("f.json"), r##"{"cols":1,"rows":1,"lines":[[["a","#12345","#000000",0]]]}"##)
    .unwrap();
  let done = frames(&["probe", "f.json"], &yard);
  let why = String::from_utf8_lossy(&done.stderr);
  assert_eq!(done.status.code(), Some(1));
  assert!(why.contains("\"#12345\" is not a color #rrggbb"), "{why}");
}
