//! The integration tests of the crate, one module for each part, in one binary: cargo links the
//! crate once for them, and runs the tests of every part at the same time.

#[cfg(unix)]
#[path = "../common/pipes.rs"]
#[expect(clippy::expect_used, reason = "a test fails by a panic where its process fails")]
mod pipes;

#[path = "../common/yard.rs"]
mod yard;

mod async_open;
mod examples;
mod promise;
mod tip;
