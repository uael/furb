//! The map of a part of the file of an image, where no door reaches it: an open from an image
//! loads the session through it, and a load proves only that its bytes are whole, not that they
//! are the bytes of that part.

use std::{fs, fs::File, process};

use super::mapped;
use crate::testing::Yard;

#[test]
fn a_map_of_a_part_of_a_file_gives_the_bytes_of_that_part() {
  let at = Yard::new(format!("furb-image-map-{}", process::id())).within("image");
  let bytes: Vec<u8> = (0..20_000u32).map(|n| u8::try_from(n % 251).unwrap()).collect();
  fs::write(&at, &bytes).unwrap();
  // A part that starts inside a page of memory and ends past the next one.
  let map = mapped(&File::open(&at).unwrap(), &(4_097..13_001)).unwrap();
  assert_eq!(map[..], bytes[4_097..13_001]);
}
