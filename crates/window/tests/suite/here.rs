//! The allocator of the suite: the one of the process, which counts each byte and each call, and a
//! count of the calls of each thread. The window runs on the thread of its test, and the daemon of
//! the test and the transport of the window on threads of their own, so the calls of the thread of
//! a test are the cost of the window alone, which no frame of the daemon moves.

use std::{
  alloc::{GlobalAlloc, Layout},
  cell::Cell,
};

use crate::counted::Counted;

thread_local! {
  /// The calls of this thread that asked the allocator for memory, a growth in place among them.
  static CALLS: Cell<usize> = const { Cell::new(0) };
}

/// The allocator of the process, with the count of the calls of each thread.
pub(crate) struct Here;

impl Here {
  /// The calls that this thread made since it started.
  pub(crate) fn calls() -> usize {
    CALLS.get()
  }
}

/// One more call of this thread.
fn called() {
  CALLS.set(CALLS.get() + 1);
}

#[expect(
  unsafe_code,
  reason = "a count of the calls of a thread wraps the allocator of the process, whose trait is unsafe"
)]
// SAFETY: each call goes to the allocator of the process with its own arguments, and only counts.
unsafe impl GlobalAlloc for Here {
  unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
    called();
    // SAFETY: the caller keeps the contract of alloc.
    unsafe { Counted.alloc(layout) }
  }

  unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
    called();
    // SAFETY: the caller keeps the contract of alloc_zeroed.
    unsafe { Counted.alloc_zeroed(layout) }
  }

  unsafe fn dealloc(&self, at: *mut u8, layout: Layout) {
    // SAFETY: at came from this allocator with this layout.
    unsafe { Counted.dealloc(at, layout) }
  }

  unsafe fn realloc(&self, at: *mut u8, layout: Layout, size: usize) -> *mut u8 {
    called();
    // SAFETY: at came from this allocator with this layout, and the caller keeps the contract of
    // realloc.
    unsafe { Counted.realloc(at, layout, size) }
  }
}
