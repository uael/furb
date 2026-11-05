//! The engines that this build runs: its own, and each engine of its major that a build of main
//! shipped, which the archive `src/furb/engines` keeps by the digest that a record names it by, so
//! a record replays word by word on the engine that made it, and its life goes on with that engine.

use std::sync::LazyLock;

use crate::{ENGINE, KERNEL, SYSTEM, world};

/// One engine that a life runs: the engine and the Kernel that the sandbox loads, the system prompt
/// that each model of the life reads, and the digest of the engine and of the Kernel that the fact
/// made of a record names. The sheet of the gate is this build's own for each engine, as the rest
/// of the outside is.
#[derive(Debug)]
pub struct Build {
  /// The digest of the engine and of the Kernel as they shipped, as [`world::engine`] gives it.
  pub digest: &'static str,
  /// The engine, with the adapters that make its replay exact, which `build/archive.rs` holds.
  pub engine: &'static str,
  pub kernel: &'static str,
  /// The engine minified with its program kept as it is.
  pub system: &'static str,
}

/// The engines of the archive, in the order of their digests.
static ARCHIVE: &[Build] = include!(concat!(env!("OUT_DIR"), "/archive.rs"));

/// The engine of this build.
static THIS: LazyLock<Build> = LazyLock::new(|| Build {
  digest: Box::leak(world::engine().into_boxed_str()),
  engine: ENGINE,
  kernel: KERNEL,
  system: SYSTEM,
});

/// The engine of this build.
pub fn this() -> &'static Build {
  &THIS
}

/// The engine that a digest names: this one, or one of the archive, and nothing when the build
/// keeps no engine of that digest.
pub fn named(digest: &str) -> Option<&'static Build> {
  if digest == this().digest {
    return Some(this());
  }
  ARCHIVE.iter().find(|one| one.digest == digest)
}

/// Each engine of the archive, in the order of their digests.
pub fn archive() -> &'static [Build] {
  ARCHIVE
}

impl Build {
  /// Whether this is the engine of this build.
  pub fn this(&self) -> bool {
    std::ptr::eq(self, this())
  }
}

#[cfg(test)]
#[path = "engines.test.rs"]
mod test;
