//! The image of each frame of a marker that a replay writes, drawn from its cells.

use std::{fs, io::Cursor};

use crate::{cast, frames, said, yard};

/// The replay of a recording that writes a text and marks it, once for each text, with what the
/// command said and the bytes of the image of each marker.
fn drawn(name: &str, cols: u16, texts: &[&str]) -> (String, Vec<Vec<u8>>) {
  let yard = yard(name);
  let mut events = Vec::new();
  let labels: Vec<String> = (0..texts.len()).map(|at| format!("m{at}")).collect();
  for (text, label) in texts.iter().zip(&labels) {
    events.extend([(0.0, "o", *text), (0.1, "m", label.as_str())]);
  }
  cast(&yard.join("rec.cast"), (cols, 1), &events);
  let said = said(&frames(&["replay", "rec.cast", "--out", "out"], &yard));
  let image = |label: &String| {
    let path = yard.join(format!("out/rec.{label}.png"));
    fs::read(&path).unwrap_or_else(|no| panic!("{} was not written: {no}", path.display()))
  };
  let images = labels.iter().map(image);
  (said, images.collect())
}

#[test]
fn a_glyph_that_the_face_lacks_is_named_once() {
  let (said, _) = drawn("lacks", 4, &["a⏺b⏺"]);
  assert!(said.ends_with(", the face lacks ⏺\n"), "{said}");
}

#[cfg(target_os = "macos")]
#[test]
fn a_glyph_that_the_face_lacks_is_drawn_from_a_face_of_the_system_and_not_as_a_box() {
  let (_, images) = drawn("system", 1, &["\u{10fffd}", "\r⏺", "\r⬝", "\r⎿", "\r✻"]);
  for (glyph, image) in ["⏺", "⬝", "⎿", "✻"].iter().zip(&images[1..]) {
    assert_ne!(*image, images[0], "{glyph}");
  }
}

#[test]
fn an_image_is_ten_pixels_wide_a_cell_and_the_same_each_time() {
  let (_, first) = drawn("width", 2, &["ab"]);
  let (_, again) = drawn("again", 2, &["ab"]);
  let head = png::Decoder::new(Cursor::new(&first[0])).read_info().unwrap().info().clone();
  assert_eq!(head.width, 20);
  assert_eq!(first, again);
}

#[test]
fn a_loader_that_the_face_lacks_is_drawn_as_its_shape() {
  let (said, _) = drawn("loader", 5, &["⠦◐◓◑◒"]);
  assert!(!said.contains("lacks"), "{said}");
}

#[test]
fn a_block_of_quarters_fills_its_quarters_of_the_cell_to_their_edges() {
  let (_, images) = drawn("quarters", 2, &["▀▗"]);
  let mut decoder = png::Decoder::new(Cursor::new(&images[0])).read_info().unwrap();
  let height = usize::try_from(decoder.info().height).unwrap();
  let mut pixels = vec![0; decoder.output_buffer_size().unwrap()];
  decoder.next_frame(&mut pixels).unwrap();
  // The glyph of the default color of the terminal, #d4d4d4, on its ground, #1e1e1e.
  let lit = |x: usize, y: usize| pixels[(y * 20 + x) * 3] == 0xd4;
  // The upper half of the first cell to its last row, and the lower right quarter of the second
  // from its first row.
  let rows = |x: usize| (0..height).map(|y| lit(x, y)).collect::<Vec<_>>();
  let half: Vec<bool> = (0..height).map(|y| y < height / 2).collect();
  assert_eq!((rows(0), rows(9)), (half.clone(), half.clone()));
  let lower: Vec<bool> = half.iter().map(|upper| !upper).collect();
  assert_eq!((rows(14), rows(15), rows(19)), (vec![false; height], lower.clone(), lower));
}

#[test]
fn an_emoji_is_drawn_whole_across_its_two_cells_from_the_face_that_the_program_carries() {
  let (said, images) = drawn("emoji", 2, &["\x1b[?2027h🔀"]);
  assert!(!said.contains("lacks"), "{said}");
  let mut decoder = png::Decoder::new(Cursor::new(&images[0])).read_info().unwrap();
  let mut pixels = vec![0; decoder.output_buffer_size().unwrap()];
  decoder.next_frame(&mut pixels).unwrap();
  // Each half of the glyph stands on its cell: a pixel of either cell holds more than its ground.
  let lit = |from: usize| {
    pixels
      .chunks(3)
      .enumerate()
      .any(|(at, one)| (from..from + 10).contains(&(at % 20)) && one[0] > 0x1e)
  };
  assert_eq!((lit(0), lit(10)), (true, true));
}
