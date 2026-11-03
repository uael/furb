//! The wait of a test for a process whose output is piped.

use std::{
  io::Read,
  process::{Child, Output},
  sync::mpsc,
  thread::{self, JoinHandle},
  time::{Duration, Instant},
};

/// What a process did, or nothing when it still ran after the bound and was killed.
///
/// Both streams are read while the process runs: a process that fills one pipe waits for a reader, so a wait
/// that reads none waits for ever.
pub(crate) fn finished(mut child: Child, bound: Duration) -> Option<Output> {
  let (done, ended) = mpsc::channel();
  let out = drained(child.stdout.take().expect("the stdout is piped"), done.clone());
  let err = drained(child.stderr.take().expect("the stderr is piped"), done);
  let end = Instant::now() + bound;
  let whole =
    (0..2).all(|_| ended.recv_timeout(end.saturating_duration_since(Instant::now())).is_ok());
  if !whole {
    child.kill().expect("the process ends");
  }
  let status = child.wait().expect("the process is gone");
  whole.then(|| Output {
    status,
    stdout: out.join().expect("the stdout is read"),
    stderr: err.join().expect("the stderr is read"),
  })
}

fn drained<R: Read + Send + 'static>(mut from: R, done: mpsc::Sender<()>) -> JoinHandle<Vec<u8>> {
  thread::spawn(move || {
    let mut bytes = Vec::new();
    from.read_to_end(&mut bytes).expect("the stream is read");
    done.send(()).expect("the wait is heard");
    bytes
  })
}
