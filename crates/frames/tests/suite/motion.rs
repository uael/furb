//! The command `motion`: the measures of the motion of a span of a recording.

use std::path::MAIN_SEPARATOR;

use crate::{cast, frames, said, yard};

#[test]
fn the_measures_say_how_soon_a_key_shows_and_each_row_that_flickers() {
  let yard = yard("motion");
  let events = [
    (0.0, "o", "ab"),
    (0.1, "i", "<Enter>"),
    (0.03, "o", "\rcd"),
    (0.02, "o", "\rab"),
    (0.25, "o", "\rc"),
  ];
  cast(&yard.join("rec.cast"), (4, 1), &events);
  let said =
    said(&frames(&["motion", "rec.cast", "--from", "0", "--until", "500", "--out", "out"], &yard));
  // The command names the files of the motion by their path, with the separator of the system.
  let head = format!("rec.cast: out{MAIN_SEPARATOR}rec.0-500");
  let measures = [
    head.as_str(),
    "span 0.000 s to 0.500 s: 5 screens",
    "moving cells: rows 0 to 0, columns 0 to 1",
    "key \"<Enter>\" at 0.100 s: first change after 30 ms",
    "changes of the moving cells: 4 from 0.000 s to 0.400 s, 7.5 per second while they move",
    "intervals: mean 133 ms, least 20, most 250, spread 93 ms (69% of the mean)",
    "flicker: 2 cells on 1 row",
    "  row 0 columns 0 to 1, 2 cells: \"cd\" for 20 ms at 0.130 s",
  ];
  assert_eq!(said, measures.map(|line| line.to_owned() + "\n").concat());
}
