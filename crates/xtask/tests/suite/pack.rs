//! The task `pack` as a process, on a workspace of its own.

use std::{
  fs,
  process::{self, Command, Output},
};

use crate::yard::Yard;

/// The manifest of a crate that reads a file of its folder at build time, with the include of its
/// pack.
fn manifest(include: &str) -> String {
  format!(
    "[package]\nname = \"read\"\nversion = \"0.1.0\"\nedition = \"2024\"\ninclude = [{include}]\n\n\
     [workspace]\n"
  )
}

#[test]
fn a_file_that_the_build_of_a_crate_reads_and_its_pack_leaves_out_is_named() {
  let yard = Yard::new(format!("xtask-pack-{}", process::id()));
  fs::create_dir_all(yard.join("src")).unwrap();
  fs::write(yard.join("src/lib.rs"), "pub const READ: &str = include_str!(\"../read.txt\");\n")
    .unwrap();
  fs::write(yard.join("read.txt"), "read").unwrap();
  fs::write(yard.join("Cargo.toml"), manifest("\"/src/**\"")).unwrap();
  let done = packed(&yard);
  let said = String::from_utf8_lossy(&done.stdout);
  assert_eq!(
    (done.status.code(), said.as_ref()),
    (Some(1), "read: read.txt is read by the build of the crate and left out of its pack\n")
  );
  fs::write(yard.join("Cargo.toml"), manifest("\"/src/**\", \"/read.txt\"")).unwrap();
  let done = packed(&yard);
  assert_eq!((done.status.code(), done.stdout.as_slice()), (Some(0), &b""[..]));
}

fn packed(yard: &Yard) -> Output {
  let mut command = Command::new(env!("CARGO_BIN_EXE_xtask"));
  command.arg("pack").current_dir(yard).env_remove("CARGO_TARGET_DIR");
  command.output().unwrap_or_else(|no| panic!("the task did not start: {no}"))
}
