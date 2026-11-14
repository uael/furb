//! The allocator of the system, which counts each byte that the process of the suite holds, so the
//! memory bench reads the heap of a life exactly.

use std::{
  alloc::{GlobalAlloc, Layout, System},
  sync::atomic::{AtomicUsize, Ordering},
};

/// The bytes that the process holds, which [`Counted`] counts.
static LIVE: AtomicUsize = AtomicUsize::new(0);

/// The allocator of the system, which counts each byte that the process holds, as the global
/// allocator of the daemon: one atomic add for each allocation, and an exact count of the heap,
/// where the resident size of the process keeps each page that the allocator does not give back.
pub(crate) struct Counted;

impl Counted {
  /// The bytes that the process holds now.
  #[must_use]
  pub(crate) fn live() -> usize {
    LIVE.load(Ordering::Relaxed)
  }
}

#[expect(
  unsafe_code,
  reason = "a count of the heap wraps the allocator of the system, whose trait is unsafe"
)]
// SAFETY: each call goes to the allocator of the system with its own arguments, and only counts
// the bytes.
unsafe impl GlobalAlloc for Counted {
  unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
    // SAFETY: the caller keeps the contract of alloc, which is that of the system.
    let at = unsafe { System.alloc(layout) };
    if !at.is_null() {
      LIVE.fetch_add(layout.size(), Ordering::Relaxed);
    }
    at
  }

  unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
    // SAFETY: the caller keeps the contract of alloc_zeroed, which is that of the system.
    let at = unsafe { System.alloc_zeroed(layout) };
    if !at.is_null() {
      LIVE.fetch_add(layout.size(), Ordering::Relaxed);
    }
    at
  }

  unsafe fn dealloc(&self, at: *mut u8, layout: Layout) {
    LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
    // SAFETY: at came from the allocator of the system with this layout.
    unsafe { System.dealloc(at, layout) }
  }

  unsafe fn realloc(&self, at: *mut u8, layout: Layout, size: usize) -> *mut u8 {
    // SAFETY: at came from the allocator of the system with this layout, and the caller keeps the
    // contract of realloc.
    let moved = unsafe { System.realloc(at, layout, size) };
    if !moved.is_null() {
      LIVE.fetch_add(size, Ordering::Relaxed);
      LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
    }
    moved
  }
}
