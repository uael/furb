//! What the tests of the crate share.

use crate::{ear, sand::BOUND};
use std::{
  cell::Cell,
  future::Future,
  pin::pin,
  sync::Arc,
  task::{Context, Poll, Wake, Waker},
  thread,
  time::Duration,
};

/// A waker that unparks the thread of the test, so what an ear waits for wakes the poll when it
/// is ready.
struct Parked(thread::Thread);

impl Wake for Parked {
  fn wake(self: Arc<Self>) {
    self.0.unpark();
  }
}

/// The waker of the thread of the test.
pub(crate) fn waker() -> Waker {
  Waker::from(Arc::new(Parked(thread::current())))
}

/// One future driven to its end on this thread, which sleeps until its waker unparks it or twenty
/// milliseconds pass. It enters no runtime: a future that needs the timers or the sockets of tokio
/// runs by [`on_reactor`].
pub(crate) fn block_on<T>(fut: impl Future<Output = T>) -> T {
  let wake = waker();
  let mut cx = Context::from_waker(&wake);
  let mut fut = pin!(fut);
  loop {
    if let Poll::Ready(got) = fut.as_mut().poll(&mut cx) {
      return got;
    }
    park(Duration::from_millis(20));
  }
}

/// The thread of the test parked until its waker unparks it or the time passes.
#[expect(
  clippy::disallowed_methods,
  reason = "the one park of the tests: the thread of the test waits for the waker of a future or of an engine"
)]
pub(crate) fn park(time: Duration) {
  thread::park_timeout(time);
}

/// The thread of the test asleep for a time, for a test that waits for a process or a file of the
/// machine.
#[expect(
  clippy::disallowed_methods,
  reason = "the one sleep of the tests: what a test waits for is outside the engine and wakes nothing"
)]
pub(crate) fn pause(time: Duration) {
  thread::sleep(time);
}

/// One future driven to its end on the reactor of the ears, which a future that needs the timers or
/// the sockets of tokio needs.
#[expect(
  clippy::disallowed_methods,
  reason = "the one block on the reactor in the tests: a test fails by a panic, and the reactor waits for nothing else"
)]
pub(crate) fn on_reactor<T>(fut: impl Future<Output = T>) -> T {
  ear::reactor().block_on(fut)
}

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
