//! The integration tests of the crate, one module for each part, in one binary: cargo links the
//! crate once for them, and runs the tests of every part at the same time.

#[path = "../../../tests/common/yard.rs"]
mod yard;

mod daemon;
mod life;
mod net;
mod open_probe;
mod protocol;
mod rebirth;
mod view;
