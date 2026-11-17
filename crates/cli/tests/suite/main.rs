//! The integration tests of the crate, one module for each part, in one binary: cargo links the
//! crate once for them, and runs the tests of every part at the same time.

use std::{
  fs,
  path::{Path, PathBuf},
};

#[cfg(unix)]
#[path = "../../../../tests/common/pipes.rs"]
#[expect(clippy::expect_used, reason = "a test fails by a panic where its process fails")]
mod pipes;

#[path = "../../../../tests/common/tailscale.rs"]
mod tailscale;
#[path = "../../../../tests/common/yard.rs"]
mod yard;

mod found;
mod furb;
mod held;
mod host;
mod kills;
mod window;

/// The program furb of this build of the suite, through a hard link of its own in the folder `bin`
/// of the folder of a test. A build of the tree, as `cargo build` or the wheel that `uv sync`
/// builds, puts another program at the path that cargo gives while the suite runs, and a daemon
/// follows a new program at its path: it would hand its lives over in the middle of the test. A
/// system that refuses the link, as across two disks, runs the program at its path.
fn program(folder: &Path) -> PathBuf {
  let built = Path::new(env!("CARGO_BIN_EXE_furb"));
  let Some(name) = built.file_name() else { return built.to_owned() };
  let linked = folder.join("bin").join(name);
  let made = || fs::create_dir_all(folder.join("bin")).and_then(|()| fs::hard_link(built, &linked));
  if linked.is_file() || made().is_ok() {
    return linked;
  }
  built.to_owned()
}
