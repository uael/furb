//! What a command writes: each stream kept at its head and its tail, and read to its end as it
//! comes.

use std::fmt;

use futures::channel::mpsc::UnboundedSender;
use tokio::io::{AsyncRead, AsyncReadExt};

use super::Said;

/// A stream that a command writes.
#[derive(Clone, Copy, Debug)]
pub(super) enum Stream {
  Out,
  Err,
}

impl fmt::Display for Stream {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(match self {
      Self::Out => "stdout",
      Self::Err => "stderr",
    })
  }
}

/// How much of a stream a command keeps: its first `HEAD` bytes, which it says as they come, and
/// its last `TAIL` bytes, which its exit gives with the first. A command that writes more has the
/// rest of it cut, whatever it writes, so no command fills the memory of the host or the record.
pub(super) const HEAD: usize = 4 << 20;
pub(super) const TAIL: usize = 4 << 20;

/// The line that says a stream passed its head, which the text said as it comes holds after the
/// head.
fn notice(head: usize) -> String {
  format!("\n[output over {head} bytes: the exit keeps its first and its last bytes]\n")
}

/// What a stream keeps of its text: the first bytes whole, then the last bytes, and how many bytes
/// lie between.
pub(super) struct Kept {
  head: String,
  tail: String,
  full: bool,
  cut: usize,
  caps: (usize, usize),
}

impl Kept {
  pub(super) fn new(head: usize, tail: usize) -> Self {
    Self { head: String::new(), tail: String::new(), full: false, cut: 0, caps: (head, tail) }
  }

  /// The text to say now: the part of a new text that the head takes, and, once the head is full, a
  /// line that says the cut. The rest of a text goes to the tail, which keeps its last bytes.
  fn push(&mut self, text: &str) -> String {
    let mut said = String::new();
    let mut rest = text;
    if !self.full {
      let mut at = (self.caps.0 - self.head.len()).min(text.len());
      while !text.is_char_boundary(at) {
        at -= 1;
      }
      self.head.push_str(&text[..at]);
      said.push_str(&text[..at]);
      rest = &text[at..];
      if rest.is_empty() {
        return said;
      }
      self.full = true;
      said.push_str(&notice(self.caps.0));
    }
    self.tail.push_str(rest);
    if self.tail.len() > 2 * self.caps.1 {
      self.trim();
    }
    said
  }

  /// The tail cut to its cap, at a character.
  fn trim(&mut self) {
    let mut at = self.tail.len().saturating_sub(self.caps.1);
    while !self.tail.is_char_boundary(at) {
      at += 1;
    }
    self.tail.drain(..at);
    self.cut += at;
  }

  /// The text of the exit: all of the stream, or its head, a line that counts the bytes cut, and
  /// its tail.
  pub(super) fn finish(mut self) -> String {
    if !self.full {
      return self.head;
    }
    if self.tail.len() > self.caps.1 {
      self.trim();
    }
    match self.cut {
      0 => self.head + &self.tail,
      cut => format!("{}\n[... {cut} bytes cut ...]\n{}", self.head, self.tail),
    }
  }
}

/// A stream of a command that its pipe could not read.
#[derive(Debug, thiserror::Error)]
#[error("its {stream} could not be read: {source}")]
pub(super) struct Unread {
  stream: Stream,
  source: std::io::Error,
}

/// One stream of a command: each text said as it comes, whole in utf-8, as far as its head holds,
/// and kept in `all` until the stream ends. The stream is read to its end, whatever it holds, so
/// the command never blocks on a full pipe. A read that fails is no end of the stream: it is the
/// fault of the command.
pub(super) async fn read(
  stream: Option<impl AsyncRead + Unpin>,
  name: Stream,
  about: &str,
  says: &UnboundedSender<Said>,
  all: &mut Kept,
) -> Result<(), Unread> {
  let Some(mut stream) = stream else { return Ok(()) };
  let (mut held, mut chunk) = (Vec::new(), [0u8; 8192]);
  loop {
    let n = stream.read(&mut chunk).await.map_err(|source| Unread { stream: name, source })?;
    held.extend_from_slice(&chunk[..n]);
    let text = whole(&mut held, n == 0);
    let text = if text.is_empty() { text } else { all.push(&text) };
    if !text.is_empty() {
      // The stream that holds the receiver is going, and this read goes with it.
      if says.unbounded_send(Said::Out { about: about.to_owned(), text, stream: name }).is_err() {
        return Ok(());
      }
    }
    if n == 0 {
      return Ok(());
    }
  }
}

/// The text of the bytes held that is whole in utf-8, which leaves them: a character cut at the end
/// of a read waits for the next, unless the stream is over, and a byte that no utf-8 holds reads as
/// the character that replaces it.
pub(in crate::world) fn whole(held: &mut Vec<u8>, over: bool) -> String {
  let mut text = String::new();
  loop {
    match std::str::from_utf8(held) {
      Ok(all) => {
        text.push_str(all);
        held.clear();
        return text;
      }
      Err(no) => {
        let good = no.valid_up_to();
        text.push_str(std::str::from_utf8(&held[..good]).unwrap_or_default());
        match no.error_len() {
          Some(bad) => {
            text.push(char::REPLACEMENT_CHARACTER);
            held.drain(..good + bad);
          }
          None if over => {
            text.push(char::REPLACEMENT_CHARACTER);
            held.clear();
            return text;
          }
          None => {
            held.drain(..good);
            return text;
          }
        }
      }
    }
  }
}

#[cfg(test)]
#[path = "kept.test.rs"]
mod test;
