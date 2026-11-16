//! The console of the operator at a terminal: an ear of the World that takes each prompt put to the operator, shows it
//! on stderr, and closes it with the line the operator writes back on stdin, read as the shape the prompt wants.
//!
//! There is one terminal and one operator, so the prompts are shown and answered one at a time, in the order they
//! asked, and never two at once on one stream. Stdout is left to what the command prints.

use std::{
  collections::VecDeque,
  io::{self, Write},
};

use furb::{
  Ear, Fault, Object,
  ear::{Co, Next, hearing},
  world::{Asked, answered},
};
use futures::{Stream, stream};
use tokio::io::{AsyncBufReadExt, BufReader, Lines, Stdin, stdin};
use unsync::oneshot;

/// The lines of stdin, which one read holds at a time.
type Input = Lines<BufReader<Stdin>>;

/// The console, as an ear: it takes each prompt to the operator and shows them in turn, and closes each with the line
/// the operator writes back, or with why no line came. A notice, which the console closed as it took it, is shown in
/// its turn as one line, and reads no line. A prompt that is done before its line comes, by a cancel, is
/// shown no more: a line the operator wrote before that goes nowhere, and the next line goes to the next prompt.
pub(crate) fn terminal() -> Box<dyn Ear> {
  hearing(&["prompt"], |mut co: Co<Read>| async move {
    // The lines of stdin, while no read holds them.
    let mut lines = Some(BufReader::new(stdin()).lines());
    let mut waiting: VecDeque<Asked> = VecDeque::new();
    // The prompt shown, and the stop of its read.
    let mut shown: Option<(Asked, oneshot::Sender<()>)> = None;
    loop {
      if shown.is_none()
        && lines.is_some()
        && let Some(next) = waiting.pop_front()
      {
        if next.shape == "None" {
          eprintln!("{} tells: {}", next.about, next.message);
          continue;
        }
        let Some(held) = lines.take() else { continue };
        eprint!("{} wants a {}: {}\n> ", next.about, next.shape, next.message);
        if let Err(no) = io::stderr().flush() {
          // The prompt shows when stderr takes it, or not at all: the read waits for its line
          // either way.
          eprintln!("furb: {} may not show: {no}", next.about);
        }
        let (stop, stopped) = oneshot::channel();
        co.work(read(held, stopped));
        shown = Some((next, stop));
      }
      match co.next().await {
        Next::Heard(a) | Next::Resumed(a) => {
          if let Some(asked) = Asked::taken(&mut co, &a).await? {
            waiting.push_back(asked);
          } else if a.kind() == "done" {
            waiting.retain(|one| one.about != a.about() || one.shape == "None");
            shown = shown.filter(|(one, _)| one.about != a.about());
          }
        }
        Next::Worked((held, line)) => {
          lines = Some(held);
          let (Some(line), Some((Asked { about, shape, .. }, _))) = (line, shown.take()) else {
            continue;
          };
          let value = match line {
            Ok(Some(line)) => answered(&shape, line.trim_end_matches('\r')),
            Ok(None) => Err(Fault::refused("the operator cannot be read: the input is over")),
            Err(no) => Err(Fault::refused(format!("the operator cannot be read: {no}"))),
          };
          let value = value.unwrap_or_else(|no| no.object());
          let id = vec![("id", Object::string(&about))];
          if let Err(no) = co.call("close", vec![value], id).await {
            // The prompt went first, as by a cancel, or the engine refused the line: the
            // operator reads why, and the console goes on to the next prompt.
            eprintln!("{about} takes no line: {no}");
          }
        }
      }
    }
  })
}

/// What a read of stdin gives back: the lines, and the line read, or nothing when its prompt went first.
type Read = (Input, Option<io::Result<Option<String>>>);

/// One read of a line of stdin, which ends at its stop, and gives the lines back either way.
fn read(mut lines: Input, stopped: oneshot::Receiver<()>) -> impl Stream<Item = Read> {
  stream::once(async move {
    // A line that came before the stop is read, so its prompt takes it and not the next.
    let line = tokio::select! {
      biased;
      line = lines.next_line() => Some(line),
      _ = stopped => None,
    };
    (lines, line)
  })
}
