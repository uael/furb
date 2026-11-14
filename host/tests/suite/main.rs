//! The integration tests of the crate, one module for each part, in one binary: cargo links the
//! crate once for them, and runs the tests of every part at the same time.

#[path = "../../../tests/common/yard.rs"]
mod yard;

/// The allocator that counts each byte that the process holds, which the daemon reads as its heap.
#[global_allocator]
static COUNTED: furb_host::Counted = furb_host::Counted;

mod apart;
mod daemon;
mod heap;
mod image;
mod life;
mod memory;
mod net;
mod open_probe;
mod protocol;
mod rebirth;
mod unkept;
mod view;
