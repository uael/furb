//! A recording of a terminal in the format 3 of asciinema: a head with the size of the terminal,
//! then one event a line, each at its time after the event before it.

use std::{borrow::Cow, fmt::Write as _, time::Duration};

use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::Error;

/// A recording.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Cast {
  pub(crate) cols: usize,
  pub(crate) rows: usize,
  pub(crate) events: Vec<Event>,
}

/// An event of a recording, at its time from the start.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Event {
  pub(crate) at: Duration,
  pub(crate) what: What,
}

/// What a recording holds at a moment: what the program wrote, what it read, as a key, a marker of
/// a step, a new size, or an event that no frame shows.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum What {
  Output(String),
  Input(String),
  Marker(String),
  Resize { cols: usize, rows: usize },
  Other,
}

#[derive(Deserialize)]
struct Head {
  version: u8,
  term: Term,
}

#[derive(Deserialize)]
struct Term {
  cols: usize,
  rows: usize,
}

impl Cast {
  /// A recording from its text. A recording of another format than 3 is refused.
  pub(crate) fn read(text: &str) -> Result<Self, Error> {
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let head: Head = serde_json::from_str(lines.next().ok_or(Error::Cast("it is empty"))?)?;
    if head.version != 3 {
      return Err(Error::Cast("its version is not 3"));
    }
    let mut at = Duration::ZERO;
    let mut events = Vec::new();
    for line in lines {
      let (interval, code, data): (f64, String, Value) = serde_json::from_str(line)?;
      at += Duration::try_from_secs_f64(interval).ok().ok_or(Error::Cast("a time is not valid"))?;
      events.push(Event { at, what: What::of(&code, &data)? });
    }
    Ok(Self { cols: head.term.cols, rows: head.term.rows, events })
  }

  /// The text of the recording. An event that no frame shows is not written.
  pub(crate) fn write(&self) -> String {
    let head = json!({ "version": 3, "term": { "cols": self.cols, "rows": self.rows } });
    let mut text = format!("{head}\n");
    let mut before = Duration::ZERO;
    for event in &self.events {
      let (code, data) = match &event.what {
        What::Output(data) => ("o", Cow::Borrowed(data.as_str())),
        What::Input(data) => ("i", Cow::Borrowed(data.as_str())),
        What::Marker(label) => ("m", Cow::Borrowed(label.as_str())),
        What::Resize { cols, rows } => ("r", Cow::Owned(format!("{cols}x{rows}"))),
        What::Other => continue,
      };
      let interval = event.at.saturating_sub(before).as_secs_f64();
      before = event.at;
      let line = json!([(interval * 1e6).round() / 1e6, code, data]);
      // A String takes each write.
      writeln!(text, "{line}").ok();
    }
    text
  }
}

impl What {
  fn of(code: &str, data: &Value) -> Result<Self, Error> {
    let text =
      || data.as_str().map(str::to_owned).ok_or(Error::Cast("the data of an event is no text"));
    Ok(match code {
      "o" => Self::Output(text()?),
      "i" => Self::Input(text()?),
      "m" => Self::Marker(text()?),
      "r" => {
        let size = text()?;
        let (cols, rows) = size.split_once('x').ok_or(Error::Cast("a size is not COLSxROWS"))?;
        let number = |text: &str| text.parse().ok().ok_or(Error::Cast("a size is not a number"));
        Self::Resize { cols: number(cols)?, rows: number(rows)? }
      }
      _ => Self::Other,
    })
  }
}
