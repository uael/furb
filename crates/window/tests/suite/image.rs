//! An image in the input: a paste of the path of an image file puts the image at the caret as its
//! mark, and the message gives it to its session.

#![cfg(test)]

use std::fs;

use crate::{
  cards::shows,
  daemon::{ANSWERS, PATIENCE, WORKS},
  input::{sent, started, told},
  thread::clicked,
};

/// The bytes of a png that names its type and holds nothing else.
const PNG: &[u8] = b"\x89PNG\r\n\x1a\nrest";

#[test]
fn a_paste_of_the_path_of_an_image_file_puts_its_image_in_each_form_that_a_terminal_pastes() {
  let (app, daemon, _) = started();
  // The room of the four marks on one row of the input.
  app.resize(102, 30);
  let folder = daemon.file("furb-try");
  fs::write(folder.join("a shot.png"), PNG).unwrap();
  fs::write(folder.join("notes.txt"), "no image").unwrap();
  let full = folder.join("a shot.png").display().to_string();
  let forms =
    ["a shot.png".to_owned(), format!("'{full}'"), format!("{}\n", full.replace(' ', "\\ "))];
  for form in forms.iter().chain([&format!("file://{full}")]) {
    app.paste(form);
  }
  shows(&app, "▣ image-1.png ▣ image-2.png ▣ image-3.png ▣ image-4.png");
  // A path of a file that is no image, or of no file, is text.
  app.paste("notes.txt ");
  app.paste("missing.png");
  let frame = shows(&app, "notes.txt missing.png");
  assert!(!frame.contains("image-5"), "{frame}");
}

#[test]
fn each_image_of_the_input_takes_the_next_name_and_a_message_names_it_as_its_life_does() {
  let (app, daemon, sessions) = started();
  let folder = daemon.file("furb-try");
  fs::write(folder.join("one.png"), PNG).unwrap();
  app.type_text("see ");
  app.paste("one.png");
  app.paste("one.png");
  shows(&app, "see ▣ image-1.png ▣ image-2.png");
  sent(&app, ANSWERS);
  shows(&app, "Two stages hold most");
  shows(&app, "Resolved 1");
  let said = told(&daemon, &sessions);
  let named = |at: usize| format!("![image-{at}.png](furb-image://");
  assert!(said.len() == 1 && said[0].matches(&named(1)).count() == 1, "{said:?}");
  assert!(said[0].contains(&named(2)) && said[0].starts_with("see "), "{said:?}");
}

#[test]
fn an_image_that_no_frame_holds_stays_out_of_the_input_and_says_why() {
  let (app, daemon, _) = started();
  let large = [PNG, &vec![0; 13 << 20]].concat();
  fs::write(daemon.file("furb-try").join("large.png"), large).unwrap();
  app.paste("large.png");
  let frame = shows(&app, "An image of at most 12 MiB goes to a session.");
  assert!(!frame.contains("▣"), "{frame}");
}

#[test]
fn an_image_that_a_message_names_draws_under_its_text_and_one_that_does_not_draw_says_why() {
  let (app, daemon, _) = started();
  let folder = daemon.file("furb-try");
  fs::write(folder.join("shot.png"), include_bytes!("../../../frames/src/shot.png")).unwrap();
  fs::write(folder.join("broken.png"), PNG).unwrap();
  app.type_text("see ");
  app.paste("shot.png");
  app.paste("broken.png");
  sent(&app, &format!(" {WORKS}"));
  shows(&app, "┃ ▣ image-2.png does not show: its bytes are no whole");
  // The image of 192 by 108 pixels takes 7 rows of 16 pixels and the columns of its shape, a
  // blank row under the text, once its bytes came, which a loaded machine reads after the line of
  // the other image.
  let boxed = |frame: &str| {
    let rows: Vec<&str> =
      frame.lines().skip_while(|row| !row.contains("(the model works")).collect();
    let drawn = |row: &&str| row.chars().nth(4).is_some_and(|one| ('▀'..='▟').contains(&one));
    let image = rows.iter().skip(3).copied().take_while(drawn).count();
    (image, rows.get(2).and_then(|row| row.chars().nth(4)))
  };
  let drawn = app.wait_until(PATIENCE, || boxed(&app.text()) == (7, Some(' ')));
  assert!(drawn, "{}", app.text());
}

#[test]
fn an_image_that_a_message_names_twice_draws_twice() {
  let (app, daemon, _) = started();
  let shot = include_bytes!("../../../frames/src/shot.png");
  fs::write(daemon.file("furb-try").join("shot.png"), shot).unwrap();
  app.paste("shot.png");
  app.paste("shot.png");
  sent(&app, &format!(" {WORKS}"));
  shows(&app, "(the model works");
  // Both boxes draw once the one image of the two marks came, each in 7 rows.
  let drawn = |frame: &str| {
    let pixels = |row: &&str| row.chars().nth(4).is_some_and(|one| ('▀'..='▟').contains(&one));
    frame.lines().filter(pixels).count()
  };
  let twice = app.wait_until(PATIENCE, || drawn(&app.text()) == 14);
  assert!(twice, "{}", app.text());
}

#[test]
fn an_image_that_a_word_reads_draws_under_the_line_that_read_it() {
  let (app, daemon, _) = started();
  let shot = include_bytes!("../../../frames/src/shot.png");
  fs::write(daemon.file("furb-try").join("shot.png"), shot).unwrap();
  sent(&app, &format!("look {ANSWERS}"));
  shows(&app, "Resolved 1");
  clicked(&app, " Python ", 0, 1);
  shows(&app, "┃ >>> A word of Python to run");
  sent(&app, "shot = read('shot.png')");
  // The image draws at the column of the code, under its line, in 7 rows.
  let under = |frame: &str| {
    let rows = frame.lines().skip_while(|row| !row.contains("shot = read('shot.png')")).skip(1);
    let at = |row: &&str| row.chars().nth(7).is_some_and(|one| ('▀'..='▟').contains(&one));
    rows.take_while(at).count()
  };
  let drawn = app.wait_until(PATIENCE, || under(&app.text()) == 7);
  assert!(drawn, "{}", app.text());
}

#[test]
fn an_image_whose_box_the_edge_of_the_feed_cuts_names_it_there() {
  let (app, daemon, _) = started();
  let shot = include_bytes!("../../../frames/src/shot.png");
  fs::write(daemon.file("furb-try").join("shot.png"), shot).unwrap();
  sent(&app, &format!("first {ANSWERS}"));
  shows(&app, "Resolved 1");
  sent(&app, &format!("second {ANSWERS}"));
  app.paste("shot.png");
  sent(&app, &format!(" {ANSWERS}"));
  // The block of the third answer, whose lines follow the four of the two before it.
  shows(&app, "6  close('The open takes");
  // The wheel takes the feed back until the image draws whole, then on back until its box crosses
  // the foot of the feed, where the image cannot draw a part of itself: its first row names it
  // there. Then down until its box crosses the head of the feed, where its last row names it.
  let drawn = |frame: &str| {
    frame.lines().any(|row| row.chars().nth(4).is_some_and(|one| ('▀'..='▟').contains(&one)))
  };
  // A row of the feed alone, as the list stands at its right.
  let feed = |row: &str| row.chars().take(50).collect::<String>().trim_end().to_owned();
  let named = |frame: &str| frame.lines().any(|row| feed(row) == "  ┃ ▣ image-1.png");
  let (x, y) = (10, 5);
  let turned = |down: bool, found: &dyn Fn(&str) -> bool| {
    let reached = app.wait_until(PATIENCE, || {
      app.scroll(x, y, down);
      found(&app.text())
    });
    assert!(reached, "{}", app.text());
  };
  turned(false, &drawn);
  turned(false, &named);
  turned(true, &drawn);
  turned(true, &named);
}
