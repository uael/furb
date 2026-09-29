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
  Ear, Fact, Fault, Object,
  ear::{Co, Next, ear, say, tell},
  world::{SHAPES, answered},
};
use futures::{Stream, stream};
use tokio::io::{AsyncBufReadExt, BufReader, Lines, Stdin};
use unsync::oneshot;

/// A prompt put to the operator: its act, the chain it is on, its shape and its message.
pub struct Asked {
  pub about: String,
  pub on: String,
  pub shape: String,
  pub message: String,
}

impl Asked {
  /// The prompt a fact asks the operator, when it is one that no ear before the console took, as a console takes it:
  /// the console says its started, and closes at once a prompt of a shape outside SHAPES, with the refusal of that
  /// shape, and gives it not. So every console of furb puts to the operator the same shapes.
  pub async fn taken<W: 'static>(co: &mut Co<W>, a: &Fact) -> Result<Option<Asked>, Fault> {
    if a.kind() != "prompt" || !a.question() {
      return Ok(None);
    }
    let word = |at: usize| a.word(at).and_then(|one| one.as_str().map(str::to_owned));
    let asked = Asked {
      about: a.about().to_owned(),
      on: a.on().to_owned(),
      shape: word(1).unwrap_or_default(),
      message: word(2).unwrap_or_default(),
    };
    say(co, Fact::says("started", &asked.about, [])).await;
    if SHAPES.contains(&asked.shape.as_str()) {
      return Ok(Some(asked));
    }
    let no = answered(&asked.shape, "").unwrap_or_else(|no| no.object());
    co.call("close", vec![no], vec![("id", Object::string(&asked.about))]).await?;
    Ok(None)
  }
}

/// The lines of stdin, which one read holds at a time.
type Input = Lines<BufReader<Stdin>>;

/// The console, as an ear: it takes each prompt to the operator and shows them in turn, and closes each with the line
/// the operator writes back, or with why no line came. A prompt that is done before its line comes, by a cancel, is
/// shown no more: a line the operator wrote before that goes nowhere, and the next line goes to the next prompt.
pub fn terminal() -> Box<dyn Ear> {
  ear(|mut co: Co<Read>| async move {
    // The lines of stdin, while no read holds them.
    let mut lines = Some(BufReader::new(tokio::io::stdin()).lines());
    let mut waiting: VecDeque<Asked> = VecDeque::new();
    // The prompt shown, and the stop of its read.
    let mut shown: Option<(Asked, oneshot::Sender<()>)> = None;
    loop {
      if shown.is_none()
        && lines.is_some()
        && let Some(next) = waiting.pop_front()
        && let Some(held) = lines.take()
      {
        eprint!("{} wants a {}: {}\n> ", next.about, next.shape, next.message);
        let _ = io::stderr().flush();
        let (stop, stopped) = oneshot::channel();
        co.work(read(held, stopped));
        shown = Some((next, stop));
      }
      match co.next().await {
        Next::Heard(a) => {
          if let Some(asked) = Asked::taken(&mut co, &a).await? {
            waiting.push_back(asked);
          } else if a.kind() == "done" {
            waiting.retain(|one| one.about != a.about());
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
          tell(&mut co, "close", vec![value], vec![("id", Object::string(&about))]).await;
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
