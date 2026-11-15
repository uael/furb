//! What the tests of the crate share.

use crate::sand::BOUND;
use std::{cell::Cell, time::Duration};

#[path = "../tests/common/drive.rs"]
mod drive;
pub(crate) use drive::{block_on, on_reactor, park, pause, waker};

#[path = "../tests/common/yard.rs"]
mod yard;
pub(crate) use yard::Yard;

/// The bound of a turn for the words that a test does not cut: a step that a loaded machine
/// runs in a minute still holds under it.
pub(crate) const LOADED: Duration = Duration::from_secs(120);

thread_local! {
  /// Whether the scripted model of a test of this thread cuts its first word alone.
  static FIRST: Cell<bool> = const { Cell::new(false) };
}

/// Each sandbox of this thread with `bound` as the bound of a turn for the first word that a
/// scripted model gives, and [`LOADED`] for each word after it, since a test proves what its first
/// word did, not how fast a loaded machine runs the others.
pub(crate) fn cut_first(bound: Duration) {
  BOUND.set(bound);
  FIRST.set(true);
}

/// Each sandbox of this thread with `bound` as the bound of a turn for every word.
pub(crate) fn bound_all(bound: Duration) {
  BOUND.set(bound);
  FIRST.set(false);
}

/// What a scripted model calls as it gives its answer number `nth`: from the second on, the
/// bound of a test that cuts its first word alone becomes [`LOADED`].
pub(crate) fn answering(nth: usize) {
  if nth > 1 && FIRST.get() {
    BOUND.set(LOADED);
  }
}
