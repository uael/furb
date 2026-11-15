//! The integration tests of the crate, one module for each part, in one binary: cargo links the
//! crate once for them, and runs the tests of every part at the same time.

#[cfg(unix)]
#[path = "../common/pipes.rs"]
#[expect(clippy::expect_used, reason = "a test fails by a panic where its process fails")]
mod pipes;

#[cfg(unix)]
#[path = "../common/claude.rs"]
#[expect(clippy::expect_used, reason = "the fake fails the test where it is not made")]
mod claude;
#[path = "../common/drive.rs"]
mod drive;
#[path = "../common/opened.rs"]
#[expect(clippy::disallowed_methods, reason = "a test blocks on its runtime and fails by a panic")]
#[expect(clippy::unwrap_used, reason = "a helper of a test fails the test where it fails")]
#[expect(clippy::expect_used, reason = "a helper of a test fails the test where it fails")]
mod opened;
#[path = "../common/yard.rs"]
mod yard;

#[cfg(unix)]
use drive::pause;
use yard::Yard;
#[cfg(unix)]
use yard::Yard as Base;

/// The script of a life of the suite takes each word under the bound of a turn of the engine.
fn answered(_nth: usize) {}

mod async_open;
mod engine;
mod examples;
mod life;
mod promise;
mod skill;
mod tip;
mod view;
mod world;
