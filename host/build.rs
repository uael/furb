//! The build: the digest of the files that give their shape to what the host of a life keeps in
//! the image of the life, which the host writes beside it, so a host of another shape takes no part
//! of an image and opens the life by its record.

use std::fs;

use sha2::{Digest, Sha256};

/// The files that define the part of the host in an image: `Kept` and each type that it holds. The
/// types of the crate that it holds, as `Held`, are in the digest of the crate, which the key of an
/// image names.
const KEPT: [&str; 5] =
  ["src/life.rs", "src/event.rs", "src/queue.rs", "src/backlog.rs", "src/rebirth.rs"];

fn main() {
  let mut hash = Sha256::new();
  for path in KEPT {
    println!("cargo::rerun-if-changed={path}");
    hash.update(path);
    match fs::read(path) {
      Ok(text) => hash.update(text),
      Err(no) => panic!("the build reads {path}, which gives a shape to what the host keeps: {no}"),
    }
  }
  println!("cargo::rustc-env=FURB_KEPT={:x}", hash.finalize());
}
