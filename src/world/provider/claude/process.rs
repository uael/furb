//! The process of the claude command line that a conversation holds: how it starts, with a program
//! that an update may replace for a moment, and how a turn reads its stdout and its stderr.

use std::{
  process::Stdio,
  time::{Duration, Instant},
};

use futures::FutureExt;
use rig_core::completion::CompletionError;
use serde_json::Value;
use tokio::{
  io::{AsyncBufReadExt, AsyncReadExt, BufReader},
  process::{Child, ChildStderr, ChildStdin, ChildStdout, Command},
};

use super::{MISSING, Program, failed, protocol::Event};

/// How long a turn reads what a process that ended still holds on its stdout and its stderr, since
/// a process that it started may hold them open.
pub(super) const GRACE: Duration = Duration::from_secs(1);

/// The longest line of the stream that the command line may write.
pub(super) const LINE: usize = 32 * 1024 * 1024;

/// How many characters of what a process wrote on its stderr a failure keeps.
pub(super) const TAIL: usize = 2000;

/// The error of the system that says a program is open to be written, which is ETXTBSY.
pub(super) const BUSY: i32 = 26;

/// The first pause before a process tries its program again, which doubles at each try up to
/// [`PAUSE`].
const FIRST: Duration = Duration::from_millis(100);

/// The longest pause before a process tries its program again.
const PAUSE: Duration = Duration::from_secs(5);

/// What the stdout of a process says: an event of the stream, a line that is none, or its end, with
/// why when the end is no exit.
pub(super) enum Line {
  Event(Box<Event>),
  Odd(String),
  Over(Option<String>),
}

/// What a process of the command line spends on each of its turns, which it started at: its effort,
/// and the most tokens of thought that it sets as `MAX_THINKING_TOKENS` when its turns name one.
#[derive(Clone, PartialEq)]
pub(super) struct Spend {
  pub(super) effort: Option<String>,
  pub(super) thinking: Option<u64>,
}

/// One process of the command line: what it spends, the child, its stdin, its stdout, the tail of
/// its stderr, and the running cost it said last, since it says the cost of all its turns.
pub(super) struct Process {
  /// What each turn of the process spends, which it started at.
  pub(super) spend: Spend,
  pub(super) child: Child,
  pub(super) stdin: ChildStdin,
  pub(super) out: Out,
  pub(super) tail: Tail,
  pub(super) paid: f64,
}

impl Process {
  /// A process of this command, with its pipes, which spends what it started at. When its program
  /// is missing or does not run, it tries again after a pause that grows, until the program starts
  /// or the wait for it passed.
  pub(super) async fn spawn(
    mut command: Command,
    program: &Program,
    spend: Spend,
    gone: Duration,
  ) -> Result<Process, CompletionError> {
    command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
    // A program that a process holds open to write it is busy, as a new program is for a moment
    // while a fork of another thread holds it, so it starts at a later try.
    let mut tries = 0;
    let due = Instant::now() + gone;
    let mut pause = FIRST;
    let mut child = loop {
      let wait = match command.spawn() {
        Err(no) if no.raw_os_error() == Some(BUSY) && tries < 50 => {
          tries += 1;
          Duration::from_millis(10)
        }
        Err(no)
          if matches!(
            no.kind(),
            std::io::ErrorKind::NotFound | std::io::ErrorKind::PermissionDenied
          ) && Instant::now() < due =>
        {
          let wait = pause.min(due.saturating_duration_since(Instant::now()));
          pause = (pause * 2).min(PAUSE);
          wait
        }
        got => break got,
      };
      tokio::time::sleep(wait).await;
    }
    .map_err(|no| {
      let absent =
        matches!(no.kind(), std::io::ErrorKind::NotFound | std::io::ErrorKind::PermissionDenied);
      let missing = if absent { format!("{MISSING}: ") } else { String::new() };
      failed(format!("{missing}{} did not start: {no}", program.name()))
    })?;
    let (Some(stdin), Some(stdout), Some(stderr)) =
      (child.stdin.take(), child.stdout.take(), child.stderr.take())
    else {
      return Err(failed("claude started with no pipes"));
    };
    let out = Out { reader: BufReader::new(stdout), read: Vec::new() };
    let tail = Tail { stderr: Some(stderr), kept: String::new() };
    Ok(Process { spend, child, stdin, out, tail, paid: 0.0 })
  }

  /// Whether the process still stands: what an earlier turn left unread goes, and an end among it
  /// says no.
  pub(super) fn stands(&mut self) -> bool {
    while let Some(line) = self.out.line().now_or_never() {
      if let Line::Over(_) = line {
        return false;
      }
    }
    matches!(self.child.try_wait(), Ok(None))
  }
}

/// The stdout of a process, read a line at a time, and the part of a line read so far, which stays
/// when a read is dropped before its line is whole.
pub(super) struct Out {
  reader: BufReader<ChildStdout>,
  read: Vec<u8>,
}

impl Out {
  /// The next line of the stdout, as it comes.
  pub(super) async fn line(&mut self) -> Line {
    loop {
      let limit = u64::try_from((LINE + 1).saturating_sub(self.read.len())).unwrap_or(u64::MAX);
      let got = (&mut self.reader).take(limit).read_until(b'\n', &mut self.read).await;
      return match got {
        Ok(0) if self.read.is_empty() => Line::Over(None),
        Ok(_) if self.read.len() > LINE => {
          Line::Over(Some(format!("Claude wrote a line over {LINE} bytes.")))
        }
        Ok(_) => {
          let read = std::mem::take(&mut self.read);
          match serde_json::from_slice::<Value>(read.trim_ascii()) {
            Ok(value) => match Event::read(value) {
              Ok(event) => Line::Event(Box::new(event)),
              Err(no) => {
                Line::Over(Some(format!("Claude sent an event its protocol forbids: {no}")))
              }
            },
            Err(_) if read.trim_ascii().is_empty() => continue,
            Err(_) => Line::Odd(String::from_utf8_lossy(read.trim_ascii()).into_owned()),
          }
        }
        Err(no) => Line::Over(Some(format!("The stdout of claude failed: {no}"))),
      };
    }
  }
}

/// The stderr of a process, until its end, and the last characters it wrote, kept for the failure
/// that comes next.
pub(super) struct Tail {
  stderr: Option<ChildStderr>,
  pub(super) kept: String,
}

impl Tail {
  /// What the stderr writes next, kept, until its end.
  async fn heard(&mut self) {
    let Some(stderr) = self.stderr.as_mut() else { return std::future::pending().await };
    let mut chunk = [0u8; 4096];
    match stderr.read(&mut chunk).await {
      Ok(n) if n > 0 => {
        self.kept.push_str(&String::from_utf8_lossy(&chunk[..n]));
        let cut = self.kept.len() - last(&self.kept).len();
        self.kept.drain(..cut);
      }
      _ => self.stderr = None,
    }
  }

  /// What a work of the process comes to, with its stderr read meanwhile, so the process never
  /// stops for it.
  pub(super) async fn beside<T>(&mut self, work: impl Future<Output = T>) -> T {
    let mut work = std::pin::pin!(work);
    loop {
      tokio::select! {
        got = &mut work => return got,
        () = self.heard() => {}
      }
    }
  }

  /// All that the stderr writes until its end.
  pub(super) async fn drained(&mut self) {
    while self.stderr.is_some() {
      self.heard().await;
    }
  }

  /// The last of what the process wrote on its stderr, after a colon, or nothing.
  pub(super) fn said(&self) -> String {
    let kept = self.kept.trim();
    if kept.is_empty() { String::new() } else { format!(": {kept}") }
  }
}

/// The last characters of a text that a failure keeps.
pub(super) fn last(text: &str) -> &str {
  let cut = text.char_indices().rev().nth(TAIL - 1).map_or(0, |(at, _)| at);
  &text[cut..]
}

/// The failure of a turn that waited its stall with no progress, with the last line it read that
/// was no event.
pub(super) fn stalled(stall: Duration, odd: &str) -> CompletionError {
  let seconds = stall.as_secs_f64();
  let odd =
    if odd.is_empty() { String::new() } else { format!(" The last line it wrote was {odd}") };
  failed(format!("Claude made no progress for {seconds}s.{odd}"))
}

/// The failure of a turn whose process ended, with its code or by a signal, and all that it wrote
/// on its stderr, which the stderr gives until its end or for a grace; and whether the process
/// found no program to run: it wrote no line of its stream, and its stderr says so, as a prefix of
/// the command line says while an update replaces claude.
pub(super) async fn exited(
  code: Option<i32>,
  tail: &mut Tail,
  program: &Program,
  spoke: bool,
) -> (CompletionError, bool) {
  tokio::time::timeout(GRACE, tail.drained()).await.ok();
  let code = code.map_or_else(|| "a signal".to_owned(), |code| code.to_string());
  if !spoke && absent(&tail.kept) {
    let said = tail.said();
    return (failed(format!("{MISSING}: {} exited ({code}){said}", program.name())), true);
  }
  (failed(format!("Claude exited ({code}){}", tail.said())), false)
}

/// Whether what a process wrote on its stderr says that it found no program.
pub(super) fn absent(said: &str) -> bool {
  let said = said.to_lowercase();
  said.contains("not found") || said.contains("no such file or directory")
}
