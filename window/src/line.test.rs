use opentui::style::Style;

use super::{Line, cut, run, spread, wrapped};

fn text(line: &Line) -> String {
  line.iter().map(|span| span.content.as_ref()).collect()
}

#[test]
fn a_text_past_its_room_is_cut_between_words_with_a_mark_and_a_wide_letter_counts_two_columns() {
  let cuts = [cut("the list", 8), cut("the list of cards", 8), cut("the list of", 9)];
  let alone = [cut("listofcards", 8), cut("会議の記録です", 8)];
  assert_eq!(cuts, ["the list", "the…", "the list…"].map(str::to_owned));
  assert_eq!(alone, ["listofc…", "会議の…"].map(str::to_owned));
}

#[test]
fn the_left_of_a_row_gives_way_to_its_right_and_the_row_fills_its_room() {
  let plain = Style::new();
  let row = |left: &str, right: &str| {
    text(&spread(vec![run(left, plain)], vec![run(right, plain)], 16, plain))
  };
  let rows = [row("one", "5h"), row("where does the time go", "◆ Input"), row("one", "")];
  assert_eq!(rows, ["one           5h", "where…   ◆ Input", "one             "].map(str::to_owned));
}

#[test]
fn a_text_breaks_between_words_into_lines_of_its_room() {
  assert_eq!(
    wrapped("Write below to start one.", 11),
    ["Write below", "to start", "one."].map(str::to_owned)
  );
}
