//! An image in the input: a paste of the path of an image file puts the image at the caret as its
//! mark, and the message gives it to its session.

#![cfg(test)]

use std::fs;

use crate::{
  cards::shows,
  daemon::ANSWERS,
  input::{sent, started, told},
};

/// The bytes of a png that names its type and holds nothing else.
const PNG: &[u8] = b"\x89PNG\r\n\x1a\nrest";

#[test]
fn a_paste_of_the_path_of_an_image_file_puts_its_image_in_each_form_that_a_terminal_pastes() {
  let (app, daemon, _) = started();
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
