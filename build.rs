//! The build of the crate, which a dump is stamped with: one hash of every file the crate is compiled from, but the
//! engine, which a dump is stamped with apart. A dump holds the stand-in, the sheet and what the crate made of the
//! session, and a build that differs in any of them could not read that session again.

use std::{
  env, fs,
  path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

fn main() {
  #[cfg(feature = "typescript")]
  napi_build::setup();
  let root =
    PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("cargo names the directory of the crate"));
  let mut files: Vec<PathBuf> =
    ["Cargo.toml", "Cargo.lock", "build.rs", "src/preamble.py", "src/furb/sheet.py"]
      .iter()
      .map(|name| root.join(name))
      .collect();
  sources(&root.join("src"), &mut files);
  files.sort();
  let mut hash = Sha256::new();
  for file in &files {
    println!("cargo:rerun-if-changed={}", file.display());
    let name = file.strip_prefix(&root).expect("a file of the crate").components();
    let name: Vec<String> =
      name.map(|part| part.as_os_str().to_string_lossy().into_owned()).collect();
    let bytes =
      fs::read(file).unwrap_or_else(|no| panic!("the build reads {}: {no}", file.display()));
    hash.update(name.join("/").as_bytes());
    hash.update((bytes.len() as u64).to_le_bytes());
    hash.update(&bytes);
  }
  // A source file added later is a new build too.
  println!("cargo:rerun-if-changed=src");
  println!("cargo:rustc-env=FURB_BUILD={:x}", hash.finalize());
}

/// Every rust source under a directory, which is what the crate compiles.
fn sources(at: &Path, into: &mut Vec<PathBuf>) {
  let listed =
    fs::read_dir(at).unwrap_or_else(|no| panic!("the build lists {}: {no}", at.display()));
  for one in listed {
    let path = one.unwrap_or_else(|no| panic!("the build lists {}: {no}", at.display())).path();
    if path.is_dir() {
      sources(&path, into);
    } else if path.extension().is_some_and(|ext| ext == "rs") {
      into.push(path);
    }
  }
}
