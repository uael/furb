//! The integration tests of the crate, one module for each part, in one binary: cargo links the
//! crate once for them, and runs the tests of every part at the same time.

#[path = "../../../tests/common/yard.rs"]
mod yard;

/// The allocator that counts each byte that the process holds, which the memory bench reads.
#[global_allocator]
static COUNTED: counted::Counted = counted::Counted;

mod counted;

mod apart;
mod daemon;
mod equal;
mod image;
mod life;
mod memory;
mod net;
mod open_probe;
mod protocol;
mod rebirth;
mod unkept;
mod view;
