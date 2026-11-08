//! The rebirth of a life: its actor writes its checkpoint, drops its engine and opens it again
//! from that checkpoint on the same thread, for the same readers. A life is born again when it
//! said [`TAIL`] facts since it opened while it holds no act that a checkpoint does not keep, so an
//! open after a crash replays a short tail, and the engine lets go of what its words left. A
//! checkpoint keeps each chain, each rung and each open prompt, and the seed makes again each act
//! of a life word of the standing, under a rung whose word is that life word; each other act that
//! lives, as a command, a reply, a word that runs or an act with an ear of its own, holds the
//! rebirth until it ends, however long that takes, so a rebirth never cuts work. A life whose
//! record does not hold each fact that it said, as on a full disk, holds its rebirth until the
//! record takes them, since the drop of its engine would lose them.
//!
//! A fault that stops the sandbox cuts the life with no checkpoint, and ends it: the record keeps
//! each fact up to the fault, and a later open replays them.
//!
//! Each life of a daemon runs on a thread of one process, so the heap that bounds them is the heap
//! of the daemon. When it passes [`SHARE`] of its [`Heap`], the daemon asks the life that said the
//! most facts since it opened to be born again, by the same rebirth as at its tail, then the next,
//! until the heap falls under the share. The daemon reads the heap from [`Counted`], the
//! allocator of its process, which counts each byte that it holds.

use std::{
  alloc::{GlobalAlloc, Layout, System},
  collections::HashSet,
  sync::atomic::{AtomicUsize, Ordering},
};

use furb::{
  Engine, Fault,
  view::{Kind, Phase},
};

use serde_json::Value;

use crate::event::Event;

/// How many facts a life says after it opened before it writes its checkpoint and is born again,
/// at the first moment when it holds no act that a checkpoint does not keep.
pub(crate) const TAIL: u64 = 10_000;

/// The share of the bound of the heap of the daemon, in percent, past which the daemon asks a life
/// to be born again.
pub const SHARE: usize = 75;

/// The heap of the daemon: its bound in bytes, and how the daemon reads what its process holds.
#[derive(Debug, Clone, Copy)]
pub struct Heap {
  pub bound: usize,
  pub live: fn() -> usize,
}

impl Heap {
  /// The heap that [`Counted`] reads, bound by a number of MiB, or by half the memory of the
  /// machine when none is given. None when the bound is no size, or the machine does not say its
  /// memory.
  #[must_use]
  pub fn counted(mib: Option<u64>) -> Option<Self> {
    let bound = match mib {
      Some(mib) => usize::try_from(mib).ok()?.checked_mul(1 << 20)?,
      None => machine()? / 2,
    };
    Some(Self { bound, live: Counted::live })
  }

  /// Whether the heap passes [`SHARE`] of its bound now.
  pub(crate) fn over(&self) -> bool {
    (self.live)().saturating_mul(100) > self.bound.saturating_mul(SHARE)
  }
}

/// The bytes that the process holds, which [`Counted`] counts.
static LIVE: AtomicUsize = AtomicUsize::new(0);

/// The allocator of the system, which counts each byte that the process holds, as the global
/// allocator of the daemon: one atomic add for each allocation, and an exact count of the heap,
/// where the resident size of the process keeps each page that the allocator does not give back.
pub struct Counted;

impl Counted {
  /// The bytes that the process holds now.
  #[must_use]
  pub fn live() -> usize {
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
    LIVE.fetch_add(layout.size(), Ordering::Relaxed);
    // SAFETY: the caller keeps the contract of alloc, which is that of the system.
    unsafe { System.alloc(layout) }
  }

  unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
    LIVE.fetch_add(layout.size(), Ordering::Relaxed);
    // SAFETY: the caller keeps the contract of alloc_zeroed, which is that of the system.
    unsafe { System.alloc_zeroed(layout) }
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

/// The memory of the machine, in bytes.
#[cfg(unix)]
#[expect(
  unsafe_code,
  reason = "the memory of the machine is read only through calls of the system"
)]
fn machine() -> Option<usize> {
  // SAFETY: sysconf reads a constant of the system and touches no memory of the caller.
  let pages = unsafe { libc::sysconf(libc::_SC_PHYS_PAGES) };
  // SAFETY: as above.
  let size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
  usize::try_from(pages).ok()?.checked_mul(usize::try_from(size).ok()?)
}

/// The memory of the machine, in bytes.
#[cfg(windows)]
#[expect(unsafe_code, reason = "the memory of the machine is read only through calls of Windows")]
fn machine() -> Option<usize> {
  use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
  // SAFETY: MEMORYSTATUSEX is plain data, for which each byte zero is a value.
  let mut status: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
  status.dwLength = u32::try_from(size_of::<MEMORYSTATUSEX>()).ok()?;
  // SAFETY: status is a MEMORYSTATUSEX whose length says its size, as the call asks.
  let read = unsafe { GlobalMemoryStatusEx(&raw mut status) };
  (read != 0).then(|| usize::try_from(status.ullTotalPhys).ok()).flatten()
}

/// What the actor watches of a life since it opened: the facts that it said, each act that lives
/// and that a checkpoint does not keep, the rungs that say a life word of the last standing, whose
/// acts the seed makes again, and those life words. The rungs go with the birth: the next birth
/// makes its own.
#[derive(Default)]
pub(crate) struct Watch {
  facts: u64,
  asked: bool,
  held: HashSet<String>,
  seeds: HashSet<String>,
  lives: HashSet<String>,
}

impl Watch {
  /// One event that the life told.
  pub(crate) fn heard(&mut self, event: &Event) {
    match event {
      Event::Fact(fact) => {
        self.facts += 1;
        let stood =
          fact[0] == "done" && fact[1].as_str().is_some_and(|one| one.starts_with("stand"));
        if let Some(skills) = fact[3].get(4).and_then(Value::as_array).filter(|_| stood) {
          let words = skills.iter().filter_map(|one| one.get(2)?.as_str());
          self.lives = words.filter(|one| !one.is_empty()).map(str::to_owned).collect();
        }
      }
      Event::Act(act) if matches!(act.phase, Phase::Done(_)) => {
        self.held.remove(&act.id);
      }
      Event::Act(act) if act.kind == Kind::Rung => {
        let word = act.words.first().and_then(Value::as_str).unwrap_or_default();
        if self.lives.contains(word) {
          self.seeds.insert(act.id.clone());
        }
      }
      Event::Act(act)
        if !matches!(act.kind, Kind::Chain | Kind::Prompt) && !self.seeds.contains(&act.by) =>
      {
        self.held.insert(act.id.clone());
      }
      _ => {}
    }
  }

  /// Whether the life is born again now: it said [`TAIL`] facts since it opened, or the daemon
  /// asked it to, and it holds no act that a checkpoint does not keep.
  pub(crate) fn due(&self) -> bool {
    (self.facts >= TAIL || self.asked) && self.held.is_empty()
  }

  /// The ask of the daemon, whose heap passed its share, that the life be born again.
  pub(crate) fn ask(&mut self) {
    self.asked = true;
  }

  /// The facts that the life said since it opened, by which the daemon weighs it.
  pub(crate) fn facts(&self) -> u64 {
    self.facts
  }

  /// The life opened again, with no fact said, no act held and no rung of a life word: the seed of
  /// the new birth says them again.
  pub(crate) fn reborn(&mut self) {
    self.facts = 0;
    self.asked = false;
    self.held.clear();
    self.seeds.clear();
  }
}

/// The engine of a life dropped, with its checkpoint when the life keeps its record, and the fault
/// of that checkpoint when it fails.
pub(crate) fn ended(engine: Engine, keeps: bool) -> Option<Fault> {
  keeps.then(|| engine.checkpoint()).and_then(Result::err)
}

#[cfg(test)]
#[path = "rebirth.test.rs"]
mod tests;
