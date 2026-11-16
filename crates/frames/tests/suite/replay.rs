//! The command `replay`: the key frames of a recording, at each marker and at each step of time.

use std::fs;

use serde_json::{Value, json};

use crate::{cast, frames, json_of, read, said, yard};

/// The key frames of a recording, replayed into the folder `out` of its yard, with what the
/// command said.
fn replayed(
  name: &str,
  size: (u16, u16),
  events: &[(f64, &str, &str)],
  every: &[&str],
) -> (crate::Yard, String) {
  let yard = yard(name);
  cast(&yard.join("rec.cast"), size, events);
  let done = frames(&[&["replay", "rec.cast", "--out", "out"][..], every].concat(), &yard);
  let said = said(&done);
  (yard, said)
}

#[test]
fn a_marker_shows_what_was_written_before_it_and_a_label_again_takes_its_count() {
  let events = [(0.0, "o", "x"), (0.1, "m", "a"), (0.05, "o", "y"), (0.15, "m", "a")];
  let (yard, said) = replayed("marker", (2, 1), &events, &[]);
  assert!(said.ends_with(": 2 markers, 0 steps of time in out\n"), "{said}");
  let out = yard.join("out");
  assert_eq!(
    (read(&out.join("rec.a.txt")), read(&out.join("rec.a-2.txt"))),
    ("x\n".into(), "xy\n".into())
  );
  for each in ["rec.a.json", "rec.a.png", "rec.a-2.json", "rec.a-2.png"] {
    assert!(out.join(each).is_file(), "{each}");
  }
}

#[test]
fn a_step_of_time_gives_a_frame_only_where_the_screen_changed_and_an_unknown_event_shows_nothing() {
  let events = [(0.05, "o", "x"), (0.37, "o", "y"), (0.08, "x", "0")];
  let (yard, said) = replayed("steps", (2, 1), &events, &["--every", "100"]);
  assert!(said.ends_with(": 0 markers, 2 steps of time in out\n"), "{said}");
  let mut written: Vec<String> = fs::read_dir(yard.join("out"))
    .unwrap()
    .map(|one| one.unwrap().file_name().to_string_lossy().into_owned())
    .collect();
  written.sort();
  assert_eq!(
    written,
    ["rec.t000100.json", "rec.t000100.txt", "rec.t000500.json", "rec.t000500.txt"]
  );
  let out = yard.join("out");
  assert_eq!(
    (read(&out.join("rec.t000100.txt")), read(&out.join("rec.t000500.txt"))),
    ("x\n".into(), "xy\n".into())
  );
}

#[test]
fn a_synchronized_update_shows_whole_at_its_end() {
  let events = [
    (0.0, "o", "x"),
    (0.01, "o", "\x1b[?2026h\ry"),
    (0.01, "m", "a"),
    (0.01, "o", "z\x1b[?2026l"),
    (0.01, "m", "a"),
  ];
  let (yard, _) = replayed("synchronized", (2, 1), &events, &[]);
  let out = yard.join("out");
  assert_eq!(
    (read(&out.join("rec.a.txt")), read(&out.join("rec.a-2.txt"))),
    ("x\n".into(), "yz\n".into())
  );
}

#[test]
fn a_recording_replays_at_the_size_that_it_resized_to_and_its_input_shows_nothing() {
  let events = [(0.0, "o", "ab"), (0.1, "i", "<Enter>"), (0.1, "r", "4x2"), (0.1, "m", "a")];
  let (yard, _) = replayed("resize", (3, 1), &events, &[]);
  let frame = json_of(&yard.join("out/rec.a.json"));
  assert_eq!((&frame["cols"], &frame["rows"]), (&json!(4), &json!(2)));
  assert_eq!(read(&yard.join("out/rec.a.txt")), "ab\n\n");
}

#[test]
fn a_recording_of_another_version_is_refused() {
  let yard = yard("version");
  fs::write(yard.join("rec.cast"), "{\"version\":2,\"term\":{\"cols\":3,\"rows\":2}}\n").unwrap();
  let done = frames(&["replay", "rec.cast"], &yard);
  assert_eq!(
    (done.status.code(), String::from_utf8_lossy(&done.stderr).as_ref()),
    (Some(1), "furb-frames: the recording is not valid: its version is not 3\n")
  );
}

/// The runs of the first row of the frame of a marker, with their colors and their looks, as the
/// JSON of a frame writes them.
fn runs(name: &str, size: (u16, u16), text: &str) -> Value {
  let (yard, _) = replayed(name, size, &[(0.0, "o", text), (0.1, "m", "a")], &[]);
  json_of(&yard.join("out/rec.a.json"))["lines"][0].clone()
}

#[test]
fn a_frame_holds_each_run_of_one_look_with_its_colors_in_the_json_of_the_captures_of_the_other_programs()
 {
  let (yard, _) = replayed("json", (1, 2), &[(0.0, "o", "\x1b[1ma"), (0.1, "m", "a")], &[]);
  assert_eq!(
    read(&yard.join("out/rec.a.json")),
    r##"{"cols":1,"rows":2,"lines":[[["a","#d4d4d4","#1e1e1e",1]],[[" ","#d4d4d4","#1e1e1e",0]]]}"##
  );
  let shown = runs("looks", (6, 2), "\x1b[1;38;2;1;2;3mab\x1b[0m\x1b[31;44mc\x1b[7md\x1b[0;2me");
  assert_eq!(
    shown,
    json!([
      ["ab", "#010203", "#1e1e1e", 1],
      ["c", "#cd3131", "#2472c8", 0],
      ["d", "#2472c8", "#cd3131", 0],
      ["e", "#797979", "#1e1e1e", 2],
      [" ", "#d4d4d4", "#1e1e1e", 0]
    ])
  );
}

#[test]
fn a_wide_glyph_takes_two_cells_and_shows_once_in_its_text() {
  let (yard, _) = replayed("wide", (4, 2), &[(0.0, "o", "日x"), (0.1, "m", "a")], &[]);
  assert_eq!(read(&yard.join("out/rec.a.txt")), "日x\n\n");
  let frame = json_of(&yard.join("out/rec.a.json"));
  assert_eq!(frame["lines"][0], json!([["日x ", "#d4d4d4", "#1e1e1e", 0]]));
}

#[test]
fn a_joined_emoji_takes_one_cell_of_its_width_whole_as_the_window_wrote_it() {
  // The rows of the list of emojis as the window writes them in a terminal that joins characters,
  // which it asks with the mode 2027: each emoji moves to the cell after its two, where the name
  // starts in the same column on each row.
  let row = |at: usize, emoji: &str, name: &str| {
    let look = "\x1b[38;2;228;228;228m\x1b[48;2;27;27;27m";
    format!("\x1b[{at};1H{look}┃\x1b[{at};2H {emoji}\x1b[{at};5H  :{name}\x1b[0m")
  };
  let shown = [
    ("✡\u{fe0f}", "star_of_david"),
    ("🔀", "twisted_rightwards_arrows"),
    ("🏳\u{fe0f}\u{200d}⚧\u{fe0f}", "transgender_flag"),
    ("💁\u{200d}♂\u{fe0f}", "tipping_hand_man"),
  ];
  let rows = shown.iter().enumerate().map(|(at, (emoji, name))| row(at + 1, emoji, name));
  let text = format!("\x1b[?2027h\x1b[48;2;27;27;27m\x1b[2J{}", rows.collect::<String>());
  let (yard, _) = replayed("joined", (40, 4), &[(0.0, "o", &text), (0.1, "m", "a")], &[]);
  let wanted: Vec<String> =
    shown.iter().map(|(emoji, name)| format!("┃ {emoji}  :{name}\n")).collect();
  assert_eq!(read(&yard.join("out/rec.a.txt")), wanted.concat());
  // The probes find each name of the list in one column, on the ground of the window.
  let done = frames(&["probe", "out/rec.a.json"], &yard);
  assert_eq!(said(&done), "");
}

#[test]
fn the_256_colors_take_the_cube_and_the_ramp() {
  let shown = runs("palette", (2, 1), "\x1b[38;5;196ma\x1b[38;5;244mb");
  assert_eq!((&shown[0][1], &shown[1][1]), (&json!("#ff0000"), &json!("#808080")));
}
