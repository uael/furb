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
/// runs on a runtime of its own.
#[expect(
  clippy::disallowed_methods,
  reason = "the one driver of a future in the tests: the thread of the test parks until the waker of the future wakes it"
)]
pub(crate) fn block_on<T>(fut: impl Future<Output = T>) -> T {
  let wake = waker();
  let mut cx = Context::from_waker(&wake);
  let mut fut = pin!(fut);
  loop {
    if let Poll::Ready(got) = fut.as_mut().poll(&mut cx) {
      return got;
    }
    thread::park_timeout(Duration::from_millis(20));
  }
}
