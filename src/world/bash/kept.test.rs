//! What a command writes, kept at its head and its tail and read to its end.

use super::{HEAD, Kept, Stream, TAIL, read};
use crate::testing;
use futures::channel::mpsc;
use std::io;
use std::pin;
use std::task;
use tokio::io as tokio_io;

#[test]
fn a_stream_past_its_caps_keeps_its_head_and_its_tail_whole_and_counts_the_cut() {
  let mut kept = Kept::new(4, 4);
  let said = kept.push("ééé");
  assert!(said.starts_with("éé\n[output over 4 bytes"), "{said:?}");
  assert!(kept.push("abcdefghijkl").is_empty());
  assert_eq!(kept.finish(), "éé\n[... 10 bytes cut ...]\nijkl");
  let mut kept = Kept::new(4, 3);
  kept.push("abcdé");
  kept.push("éé");
  assert_eq!(kept.finish(), "abcd\n[... 4 bytes cut ...]\né");
}

#[test]
fn a_stream_within_its_caps_is_kept_whole() {
  let mut kept = Kept::new(4, 4);
  assert_eq!(kept.push("ab"), "ab");
  assert_eq!(kept.push("cd"), "cd");
  assert_eq!(kept.finish(), "abcd");
  let mut kept = Kept::new(2, 4);
  kept.push("abcd");
  assert_eq!(kept.finish(), "abcd");
}

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
