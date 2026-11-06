//! What the tests of the crate share.

use std::{
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
  crate::ear::reactor().block_on(fut)
}

#[path = "../tests/common/yard.rs"]
mod yard;
pub(crate) use yard::Yard;
