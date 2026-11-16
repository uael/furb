//! What a command writes: each stream kept at its head and its tail, and read to its end as it
//! comes.
#![expect(clippy::indexing_slicing, reason = "debt: an index or a slice that can panic")]
#![expect(clippy::string_slice, reason = "debt: a slice of a string that can cut a character")]
#![expect(
  clippy::too_many_arguments,
  reason = "debt: a function of more than 4 parameters takes a struct, rule 8 of the skill rust"
)]
use std::{fmt, mem, time::Duration};

use futures::channel::mpsc::UnboundedSender;
use tokio::{
  io::{AsyncRead, AsyncReadExt},
  time::{self, Instant},
};

use super::Said;
use std::io;
use std::str;

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
  source: io::Error,
}

/// How long a text that a command writes waits before the ear says it, with each text that comes
/// in that time: the window shows it within a tenth of a second, under which a person reads an
/// answer as at once, and a command that writes a line at a time says one fact for each tenth of a
/// second that it writes, and not one for each line.
const GATHER: Duration = Duration::from_millis(100);

/// One stream of a command: the texts that come within [`GATHER`] of the first said as one, whole
/// in utf-8, as far as its head holds, and kept in `all` until the stream ends. The stream is read
/// to its end, whatever it holds, so the command never blocks on a full pipe. A read that fails is
/// no end of the stream: it is the fault of the command, after the ear said what came before it.
pub(super) async fn read(
  stream: Option<impl AsyncRead + Unpin>,
  name: Stream,
  about: &str,
  says: &UnboundedSender<Said>,
  all: &mut Kept,
) -> Result<(), Unread> {
  let Some(mut stream) = stream else { return Ok(()) };
  let (mut held, mut chunk) = (Vec::new(), [0u8; 8192]);
  let (mut gathered, mut due) = (String::new(), None);
  // Whether the receiver still hears, once it heard what was gathered: the stream that holds the
  // receiver is going, and this read goes with it.
  let said = |gathered: &mut String| {
    let text = mem::take(gathered);
    text.is_empty()
      || says.unbounded_send(Said::Out { about: about.to_owned(), text, stream: name }).is_ok()
  };
  loop {
    let reading = stream.read(&mut chunk);
    // A read that the end of the gathering stops loses no byte, since a read of tokio is cancel
    // safe.
    let n = match due {
      Some(at) => {
        let Ok(read) = time::timeout_at(at, reading).await else {
          due = None;
          if !said(&mut gathered) {
            return Ok(());
          }
          continue;
        };
        read
      }
      None => reading.await,
    };
    let n = match n {
      Ok(n) => n,
      Err(source) => {
        said(&mut gathered);
        return Err(Unread { stream: name, source });
      }
    };
    held.extend_from_slice(&chunk[..n]);
    let text = whole(&mut held, n == 0);
    if !text.is_empty() {
      gathered.push_str(&all.push(&text));
    }
    if n == 0 {
      said(&mut gathered);
      return Ok(());
    }
    if !gathered.is_empty() && due.is_none() {
      due = Some(Instant::now() + GATHER);
    }
  }
}

/// The text of the bytes held that is whole in utf-8, which leaves them: a character cut at the end
/// of a read waits for the next, unless the stream is over, and a byte that no utf-8 holds reads as
/// the character that replaces it.
fn whole(held: &mut Vec<u8>, over: bool) -> String {
  let mut text = String::new();
  loop {
    match str::from_utf8(held) {
      Ok(all) => {
        text.push_str(all);
        held.clear();
        return text;
      }
      Err(no) => {
        let good = no.valid_up_to();
        text.push_str(str::from_utf8(&held[..good]).unwrap_or_default());
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
