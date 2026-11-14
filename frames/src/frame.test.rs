use avt::Terminal;

use super::*;

fn frame(text: &str, cols: usize) -> Frame {
  let mut terminal = Terminal::new(cols, 2).unwrap();
  terminal.feed([text]);
  Frame::of(&terminal)
}

fn run(text: &str, fg: u32, bg: u32, attributes: u8) -> Run {
  Run { text: text.to_owned(), fg: Rgb::hex(fg), bg: Rgb::hex(bg), attributes }
}

#[test]
fn a_frame_holds_each_run_of_one_look_with_its_colors() {
  let shown = frame("\x1b[1;38;2;1;2;3mab\x1b[0m\x1b[31;44mc\x1b[7md\x1b[0;2me", 6);
  let first = vec![
    run("ab", 0x01_0203, 0x1e_1e1e, BOLD),
    run("c", 0xcd_3131, 0x24_72c8, 0),
    run("d", 0x24_72c8, 0xcd_3131, REVERSE),
    run("e", 0x79_7979, 0x1e_1e1e, DIM),
    run(" ", 0xd4_d4d4, 0x1e_1e1e, 0),
  ];
  assert_eq!(shown.lines[0], first);
  assert_eq!(shown.text(), "abcde\n\n");
}

#[test]
fn a_wide_glyph_takes_two_cells_and_shows_once_in_its_text() {
  let shown = frame("日x", 4);
  assert_eq!(shown.lines[0], vec![run("日x ", 0xd4_d4d4, 0x1e_1e1e, 0)]);
  let cells = shown.cells();
  let glyphs: Vec<&str> = cells[0].iter().map(|cell| cell.glyph.as_str()).collect();
  assert_eq!(glyphs, ["日", "", "x", " "]);
}

#[test]
fn the_256_colors_take_the_cube_and_the_ramp() {
  let shown = frame("\x1b[38;5;196ma\x1b[38;5;244mb", 2);
  assert_eq!(
    (shown.lines[0][0].fg, shown.lines[0][1].fg),
    (Rgb::hex(0xff_0000), Rgb::hex(0x80_8080))
  );
}

#[test]
fn the_json_of_a_frame_is_the_one_of_the_captures_of_the_other_programs() {
  let shown = frame("\x1b[1ma", 1);
  let json = serde_json::to_string(&shown).unwrap();
  assert_eq!(
    json,
    r##"{"cols":1,"rows":2,"lines":[[["a","#d4d4d4","#1e1e1e",1]],[[" ","#d4d4d4","#1e1e1e",0]]]}"##
  );
  assert_eq!(serde_json::from_str::<Frame>(&json).unwrap(), shown);
}

#[test]
fn a_color_that_is_not_rrggbb_is_refused() {
  assert_eq!(
    Rgb::try_from("#12345".to_owned()).unwrap_err().to_string(),
    "\"#12345\" is not a color #rrggbb"
  );
}
