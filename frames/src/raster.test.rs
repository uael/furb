use std::io::Cursor;

use super::*;
use crate::frame::Run;

fn frame(text: &str) -> Frame {
  let run =
    Run { text: text.to_owned(), fg: Rgb::hex(0xe4_e4e4), bg: Rgb::hex(0x0a_0a0a), attributes: 0 };
  Frame { cols: text.chars().count(), rows: 1, lines: vec![vec![run]] }
}

#[test]
fn a_glyph_that_no_face_holds_is_named_once() {
  let (_, missing) = png(&frame("a⏺b⏺")).unwrap();
  assert_eq!(missing, ['⏺']);
}

#[test]
fn an_image_is_ten_pixels_wide_a_cell_and_the_same_each_time() {
  let (image, _) = png(&frame("ab")).unwrap();
  let head = png::Decoder::new(Cursor::new(&image)).read_info().unwrap().info().clone();
  assert_eq!((head.width, head.height), (20, u32::try_from(rows().0).unwrap()));
  assert_eq!(png(&frame("ab")).unwrap().0, image);
}

#[test]
fn a_loader_that_the_face_lacks_is_drawn_as_its_shape() {
  let (_, missing) = png(&frame("⠦◐◓◑◒")).unwrap();
  assert_eq!(missing, Vec::<char>::new());
}

#[test]
fn a_block_of_quarters_fills_its_quarters_of_the_cell_to_their_edges() {
  let (image, _) = png(&frame("▀▗")).unwrap();
  let mut decoder = png::Decoder::new(Cursor::new(&image)).read_info().unwrap();
  let mut pixels = vec![0; decoder.output_buffer_size().unwrap()];
  decoder.next_frame(&mut pixels).unwrap();
  let height = rows().0;
  let lit = |x: usize, y: usize| pixels[(y * 20 + x) * 3] == 0xe4;
  // The upper half of the first cell to its last row, and the lower right quarter of the second
  // from its first row.
  let rows = |x: usize| (0..height).map(|y| lit(x, y)).collect::<Vec<_>>();
  let half: Vec<bool> = (0..height).map(|y| y < height / 2).collect();
  assert_eq!((rows(0), rows(9)), (half.clone(), half.clone()));
  let lower: Vec<bool> = half.iter().map(|upper| !upper).collect();
  assert_eq!((rows(14), rows(15), rows(19)), (vec![false; height], lower.clone(), lower));
}
