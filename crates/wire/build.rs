//! The build: the digest of the shapes of this crate, which a host keeps in the image of a life,
//! so an image of other shapes is not read.

use std::{fs, io};

use sha2::{Digest, Sha256};

fn main() -> io::Result<()> {
  let mut files =
    fs::read_dir("src")?.map(|one| one.map(|one| one.path())).collect::<Result<Vec<_>, _>>()?;
  // The tests of a module stand beside it, and no shape depends on them. The frames of the
  // protocol and their refusals stand in no image, and a new verb changes them often, so they
  // void no image.
  let unkept = ["protocol.rs", "error.rs"];
  files.retain(|one| {
    let name = one.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    !name.ends_with(".test.rs") && !unkept.contains(&name.as_str())
  });
  files.sort();
  println!("cargo::rerun-if-changed=src");
  let mut hash = Sha256::new();
  for file in files {
    hash.update(file.to_string_lossy().replace('\\', "/"));
    hash.update(fs::read(&file)?);
  }
  println!("cargo::rustc-env=FURB_WIRE_SHAPES={:x}", hash.finalize());
  Ok(())
}
