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
