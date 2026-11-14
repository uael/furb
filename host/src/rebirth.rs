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
//! each fact up to the fault, and a later open replays them. A life that ends with no fault writes
//! its image and no checkpoint, as [`ended`] says: the image is of the end of a life alone, so a
//! rebirth, at the tail or at the ask of the daemon, goes by a checkpoint, and the open after it
//! drops the image that this checkpoint passed.
//!
//! Each life of a daemon runs on a thread of one process, so the heap that bounds them is the heap
//! of the daemon. When it passes [`SHARE`] of its [`Heap`], the daemon asks the life that said the
//! most facts since it opened to be born again, by the same rebirth as at its tail, then the next,
//! until the heap falls under the share. The daemon reads the heap from [`Counted`], the
//! allocator of its process, which counts each byte that it holds.

use std::{
  alloc::{GlobalAlloc, Layout, System},
  collections::{HashMap, HashSet},
  mem,
  path::Path,
  sync::atomic::{AtomicUsize, Ordering},
};

use furb::{
  Engine, Fault, image,
  view::{Kind, Phase},
};
use serde::{Deserialize, Serialize};
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
  /// machine when none is given. None when the bound is zero or no size, or the machine does not
  /// say its memory.
  #[must_use]
  pub fn counted(mib: Option<u64>) -> Option<Self> {
    let bound = match mib {
      Some(0) => return None,
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
  let mut status: MEMORYSTATUSEX = unsafe { mem::zeroed() };
  status.dwLength = u32::try_from(size_of::<MEMORYSTATUSEX>()).ok()?;
  // SAFETY: status is a MEMORYSTATUSEX whose length says its size, as the call asks.
  let read = unsafe { GlobalMemoryStatusEx(&raw mut status) };
  (read != 0).then(|| usize::try_from(status.ullTotalPhys).ok()).flatten()
}

/// How many weighs the daemon waits for a life that it asked to be born again before it asks the
/// next one: a life that holds an act that a checkpoint does not keep, as a grant of its chain, may
/// not be born again for long.
pub(crate) const PATIENCE: u32 = 30;

/// The lives that the daemon asked to be born again since its heap passed its share, each with the
/// count of its births at the ask and the weighs since.
#[derive(Default)]
pub(crate) struct Asks {
  asked: HashMap<String, (u64, u32)>,
}

impl Asks {
  /// The heap fell under its share: each life asked, whose ask the daemon takes back.
  pub(crate) fn spared(&mut self) -> Vec<String> {
    mem::take(&mut self.asked).into_keys().collect()
  }

  /// The heap is past its share: of the lives that live, each with its name, its weight and the
  /// count of its births, the one to ask now. None while a life asked is not born again yet and
  /// the daemon waited for it fewer than [`PATIENCE`] weighs; else the heaviest that it did not
  /// ask since its last birth.
  pub(crate) fn next(&mut self, living: &[(String, u64, u64)]) -> Option<String> {
    // A life that ended holds no ask: one that opens again is a new life, with births of its own.
    self.asked.retain(|life, _| living.iter().any(|one| &one.0 == life));
    let mut waits = false;
    for (life, _, births) in living {
      if let Some((at, weighs)) = self.asked.get_mut(life)
        && *at == *births
        && *weighs < PATIENCE
      {
        *weighs += 1;
        waits = true;
      }
    }
    if waits {
      return None;
    }
    // A life that was born again since its ask is free to be asked again.
    let free = living.iter().filter(|(life, weight, births)| {
      *weight > 0 && self.asked.get(life).is_none_or(|(at, _)| at != births)
    });
    let (life, _, births) = free.max_by_key(|(_, weight, _)| *weight)?;
    self.asked.insert(life.clone(), (*births, 0));
    Some(life.clone())
  }
}

/// What the actor watches of a life since it opened: the facts that it said, each act that lives
/// and that a checkpoint does not keep, the rungs that say a life word of the last standing, whose
/// acts the seed makes again, and those life words. The rungs go with the birth: the next birth
/// makes its own.
#[derive(Default, Serialize, Deserialize)]
pub(crate) struct Watch {
  facts: u64,
  /// The ask of the daemon, which is of its heap now and of no later life.
  #[serde(skip)]
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

  /// The ask of the daemon, whose heap passed its share, that the life be born again, or the ask
  /// taken back once the heap fell under the share.
  pub(crate) fn ask(&mut self, asked: bool) {
    self.asked = asked;
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

/// The engine of a life dropped at the end of the life. A life that keeps its record ends with its
/// image, which holds what the host keeps of the life, so the next life of the record opens from
/// it and reads no fact of its past, and a build that reads no image of this one opens it by the
/// record, on the texts that the record names. A life that writes no image, as one that holds a
/// function of a host, ends with its checkpoint, from which the next life opens by the record. It
/// gives the fault of the end, when one came.
pub(crate) fn ended(
  mut engine: Engine,
  record: Option<&Path>,
  kept: &impl Serialize,
) -> Option<Fault> {
  let record = record?;
  match image::keep(&mut engine, record, kept) {
    Ok(true) => None,
    Ok(false) => engine.checkpoint().err(),
    Err(no) => Some(engine.checkpoint().err().unwrap_or(no)),
  }
}

#[cfg(test)]
#[path = "rebirth.test.rs"]
mod tests;
