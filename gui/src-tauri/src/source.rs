//! The source of a ladder: the words of its rungs as one program, each line with the acts that it made beside it.
//!
//! A word holds no trace of which line made which act, so the source places each act on the first line at or after
//! the line of the act before it that calls its verb, as read( for a read, and goes back to the top for a loop.

use serde_json::Value;

use crate::model::{Act, Life, Rung, State, fault};

/// One line of the source of a ladder.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
  /// The rung whose word holds the line, and its number in the program of the ladder, from one.
  pub rung: String,
  pub number: usize,
  pub text: String,
  /// The acts that the line made, in order.
  pub acts: Vec<String>,
  /// Whether the word runs at this line now.
  pub here: bool,
}

/// The verbs whose acts a word makes and a line shows: every act but those the engine makes for a rung.
fn shown(kind: &str) -> bool {
  !matches!(kind, "reply" | "run" | "wants" | "gate" | "merged" | "module")
}

/// The line of each act of a word: the first line at or after the line of the act before it that calls its verb,
/// or, with none, the first such line from the top, or the last line.
pub fn placed(word: &str, kinds: &[&str]) -> Vec<usize> {
  let lines: Vec<&str> = word.lines().collect();
  let calls = |line: &str, kind: &str| {
    line.match_indices(kind).any(|(at, _)| {
      let before = line[..at].chars().next_back();
      line[at + kind.len()..].starts_with('(')
        && !before.is_some_and(|one| one.is_alphanumeric() || one == '_')
    })
  };
  let mut at = 0;
  kinds
    .iter()
    .map(|kind| {
      let found = (at..lines.len()).chain(0..at).find(|one| calls(lines[*one], kind));
      at = found.unwrap_or(lines.len().saturating_sub(1));
      at
    })
    .collect()
}

/// The source of a ladder: each line of each word, with its acts, and the line that runs now.
pub fn source(life: &Life, rungs: &[Rung]) -> Vec<Line> {
  let mut lines = Vec::new();
  for rung in rungs {
    let acts: Vec<&Act> =
      rung.acts.iter().filter_map(|one| life.act(one)).filter(|act| shown(&act.kind)).collect();
    let kinds: Vec<&str> = acts.iter().map(|act| act.kind.as_str()).collect();
    let places = placed(&rung.word, &kinds);
    let first = lines.len();
    for text in rung.word.lines() {
      let number = lines.len() + 1;
      lines.push(Line {
        rung: rung.id.clone(),
        number,
        text: text.to_owned(),
        acts: Vec::new(),
        here: false,
      });
    }
    if lines.len() == first {
      continue;
    }
    for (act, place) in acts.iter().zip(places) {
      lines[first + place].acts.push(act.id.clone());
    }
    let live = matches!(rung.state, State::Runs | State::Waits | State::Asks | State::Paused)
      && !rung.thinks;
    if live {
      let waiting = rung
        .awaits
        .as_ref()
        .and_then(|one| lines[first..].iter().position(|line| line.acts.contains(one)));
      let last = lines[first..].iter().rposition(|line| !line.acts.is_empty());
      let here = waiting.or(last).unwrap_or(0);
      lines[first + here].here = true;
    }
  }
  lines
}

/// What an act came to, in a few words: what a line shows beside it.
pub fn summary(life: &Life, act: &Act) -> String {
  let Some(outcome) = &act.outcome else {
    return match act.kind.as_str() {
      "prompt" if life.owed(&act.id) => "asks you".to_owned(),
      "prompt" => format!("waits on {}", act.on),
      "bash" => last_line(life.streams.get(&act.id).map(String::as_str).unwrap_or("runs")),
      "wait" => "waits".to_owned(),
      _ => "runs".to_owned(),
    };
  };
  if let Some(name) = fault(outcome) {
    let why =
      outcome.get("args").and_then(|one| one.get(0)).and_then(Value::as_str).unwrap_or_default();
    return if why.is_empty() { name.to_owned() } else { format!("{name}: {}", clip(why, 60)) };
  }
  match act.kind.as_str() {
    "bash" => {
      let code = outcome.get("code").and_then(Value::as_i64);
      let out = outcome
        .get("stdout")
        .and_then(|one| one.get("content"))
        .and_then(Value::as_str)
        .unwrap_or_default();
      let code = code.map_or("timed out".to_owned(), |code| format!("exit {code}"));
      if out.trim().is_empty() { code } else { format!("{code}, {}", last_line(out)) }
    }
    "read" | "write" => {
      let content = outcome.get("content").and_then(Value::as_str).unwrap_or_default();
      format!("{} lines", content.lines().count())
    }
    "chain" => format!("{} {:?}", act.id, act.text(0)),
    _ => clip(&python(outcome), 60),
  }
}

/// The last line of a text that holds a word, cut short.
fn last_line(text: &str) -> String {
  clip(text.lines().rev().find(|line| !line.trim().is_empty()).unwrap_or_default().trim(), 60)
}

/// A text cut to a number of characters, with an ellipsis when it was cut.
pub fn clip(text: &str, most: usize) -> String {
  let first = text.lines().next().unwrap_or_default();
  if first.chars().count() <= most && first.len() == text.len() {
    return first.to_owned();
  }
  let mut cut: String = first.chars().take(most.saturating_sub(1)).collect();
  cut.push('…');
  cut
}

/// A value of the wire as python shows it.
pub fn python(value: &Value) -> String {
  match value {
    Value::Null => "None".to_owned(),
    Value::Bool(one) => if *one { "True" } else { "False" }.to_owned(),
    Value::Number(one) => one.to_string(),
    Value::String(one) => {
      format!("{one:?}")
        .replace("\\\"", "\"")
        .replacen('"', "'", 1)
        .trim_end_matches('"')
        .to_owned()
        + "'"
    }
    Value::Array(items) => format!("[{}]", items.iter().map(python).collect::<Vec<_>>().join(", ")),
    Value::Object(map) => match map.get("is").and_then(Value::as_str) {
      Some("Text") => {
        format!("Text({:?})", map.get("path").and_then(Value::as_str).unwrap_or_default())
      }
      Some(name) => format!("{name}(…)"),
      None => format!(
        "{{{}}}",
        map
          .iter()
          .map(|(key, one)| format!("{key:?}: {}", python(one)))
          .collect::<Vec<_>>()
          .join(", ")
      ),
    },
  }
}

#[cfg(test)]
#[path = "source.test.rs"]
mod test;
