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
  ear::{Co, call, ear, hear, say, tell},
  world::{SHAPES, answered},
};
use tokio::io::{AsyncBufReadExt, BufReader};

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
  pub async fn taken(co: &Co, a: &Fact) -> Result<Option<Asked>, Fault> {
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
    call("close", vec![no], vec![("id", Object::string(&asked.about))])?;
    Ok(None)
  }
}

/// The console, as an ear: it takes each prompt to the operator and shows them in turn, and closes each with the line
/// the operator writes back, or with why no line came. A prompt that is done before its line comes, by a cancel, is
/// shown no more, and the line goes to the next.
pub fn terminal() -> Box<dyn Ear> {
  ear(|co| async move {
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    let mut waiting: VecDeque<Asked> = VecDeque::new();
    let mut shown: Option<Asked> = None;
    loop {
      if shown.is_none()
        && let Some(next) = waiting.pop_front()
      {
        eprint!("{} wants a {}: {}\n> ", next.about, next.shape, next.message);
        let _ = io::stderr().flush();
        shown = Some(next);
      }
      tokio::select! {
        biased;
        a = hear(&co) => {
          if let Some(asked) = Asked::taken(&co, &a).await? {
            waiting.push_back(asked);
          } else if a.kind() == "done" {
            waiting.retain(|one| one.about != a.about());
            shown = shown.filter(|one| one.about != a.about());
          }
        }
        line = co.working(lines.next_line()), if shown.is_some() => {
          let Some(Asked { about, shape, .. }) = shown.take() else { continue };
          let value = match line {
            Ok(Some(line)) => answered(&shape, line.trim_end_matches('\r')),
            Ok(None) => Err(Fault::refused("the operator cannot be read: the input is over")),
            Err(no) => Err(Fault::refused(format!("the operator cannot be read: {no}"))),
          };
          let value = value.unwrap_or_else(|no| no.object());
          tell(&co, "close", vec![value], vec![("id", Object::string(&about))]).await;
        }
      }
    }
  })
}
