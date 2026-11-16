//! A stream that its pipe could not read, where no door reaches it: the pipe of a command that runs
//! fails no read. The suite proves the caps and the gather of a stream through the commands of a
//! life.

use super::{HEAD, Kept, Stream, TAIL, read};
use crate::testing;
use futures::channel::mpsc;
use std::io;
use std::pin;
use std::task;
use tokio::io as tokio_io;

/// A stream whose every read fails.
struct Broken;

impl tokio_io::AsyncRead for Broken {
  fn poll_read(
    self: pin::Pin<&mut Self>,
    _: &mut task::Context<'_>,
    _: &mut tokio_io::ReadBuf<'_>,
  ) -> task::Poll<io::Result<()>> {
    task::Poll::Ready(Err(io::Error::other("the pipe broke")))
  }
}

#[test]
fn a_stream_that_cannot_be_read_is_a_fault_of_the_command_and_not_its_end() {
  let (says, _said) = mpsc::unbounded();
  let mut kept = Kept::new(HEAD, TAIL);
  let got = testing::block_on(read(Some(Broken), Stream::Out, "bash1", &says, &mut kept));
  assert_eq!(got.unwrap_err().to_string(), "its stdout could not be read: the pipe broke");
}
