//! The input of a client: where it sends, and what it sends.
//!
//! The input sends to its target: the open thread, a new thread of a session, or a new session in
//! the project of the folder of the client. Each message waits in one queue, and only one request
//! of the queue is asked at a time, so each message goes after the one before it, and a message
//! that goes to the thread that the message before it made knows that thread when it goes. His
//! message shows in the thread at once, and a message that the daemon refuses comes back to the
//! input with the message after it.

use std::path::Path;

use jiff::Timestamp;
use serde_json::Value;

use crate::{Client, Opened, Outgoing, Pending, Target, Thread};

/// The start of a message that asks its question aside, in a fork of the chain of the open thread.
pub const ASIDE: &str = "/btw ";

/// The text of the input, and whether it is a word of python that runs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Draft {
  pub text: String,
  pub python: bool,
}

impl Client {
  /// Where the input sends now: where the operator turned it, else the open thread, else a new
  /// session on the first machine, in the folder of the client.
  pub fn target(&self) -> Target {
    match (&self.aim, &self.thread) {
      (Some(target), _) => target.clone(),
      (None, Some(opened)) => Target::Thread(opened.thread.key.clone()),
      (None, None) => Target::New(0, self.folder.clone()),
    }
  }

  /// The input turned to a new thread of the session of the open thread, and back.
  pub fn turn(&mut self) {
    self.aim = match (&self.aim, &self.thread) {
      (None, Some(opened)) => {
        Some(Target::Session(opened.thread.key.0, opened.thread.key.1.clone()))
      }
      _ => None,
    };
  }

  /// The text of the input sent to its target: it waits in the queue, the client holds its life, and
  /// the thread that it goes to shows it at once. A message to no thread starts one, which opens, so
  /// the next message goes to it, and a new session opens on the actor that the operator chose for
  /// it. A word of python shows in no thread. A message from a moment of the past of the open thread
  /// goes as the word of a rewind, and one that starts with `/btw ` as the word of a question aside.
  /// While the input fills an input of a view, the text goes to that node alone. A machine that the
  /// client dials again, or a life that a fault ended, takes nothing, and the error says why.
  pub fn send(&mut self, text: String) -> Result<(), String> {
    if self.submit_fill(&text) {
      return Ok(());
    }
    let (at, life, thread, cwd) = match self.target() {
      Target::Thread(key) => (key.0, key.1.clone(), Some(key.2), self.cwd(key.0, &key.1)),
      Target::Session(at, life) => (at, life.clone(), None, self.cwd(at, &life)),
      Target::New(at, folder) => (at, self.record(&folder), None, Some(folder)),
    };
    if let Some(away) = self.away(at) {
      return Err(away);
    }
    if let Some(why) = self.fault_of(at, &life) {
      return Err(format!("Nothing went: {why}"));
    }
    self.hold((at, life.clone()), cwd, self.moved);
    let chosen = self.chosen.take_if(|one| one.life.is_none() && one.at == at);
    if let (Some(chosen), Some(hold)) = (chosen, self.holds.get_mut(&(at, life.clone()))) {
      hold.actor = Some(chosen.actor);
    }
    let rewound =
      if self.draft.python { None } else { self.aside(&text).or_else(|| self.rewound(&text)) };
    let (text, python) = match rewound {
      Some(Ok(word)) => (word, true),
      Some(Err(why)) => return Err(why),
      None => (text, self.draft.python),
    };
    let pending = Pending { act: None, text: text.clone() };
    match (&thread, python, self.thread.as_mut()) {
      (_, true, _) | (Some(_), false, None) => {}
      (None, false, _) => {
        let key = (at, life.clone(), String::new());
        let pending = vec![pending];
        let thread = Thread { key, above: Some(0), started: true, pending, ..Thread::default() };
        (self.selected, self.aim) = (Some(thread.key.clone()), None);
        self.thread = Some(Opened { thread, ..Opened::default() });
      }
      (Some(_), false, Some(open)) => open.thread.pending.push(pending),
    }
    self.outgoing.push_back(Outgoing { at, life, thread, text, python });
    self.walk = None;
    Ok(())
  }

  /// The word that asks a message that starts with `/btw ` aside, as his word on the open thread:
  /// a fork of the chain of the thread, with all that it holds, answers the question, so the chain
  /// of the thread keeps nothing of it, and the answer stands under the line of the word. None for
  /// any other message, and while the input sends to no thread.
  fn aside(&self, text: &str) -> Option<Result<String, String>> {
    let question = text.strip_prefix(ASIDE)?.trim();
    self.thread.as_ref().filter(|one| self.aim.is_none() && !one.thread.key.2.is_empty())?;
    let question = Value::String(question.to_owned());
    Some(Ok(format!("prompt(str, {question}, on=chain('btw', source=__name__))")))
  }

  /// The input turned to ask its text aside: it starts with `/btw ` once.
  pub fn aside_draft(&mut self) {
    let rest = self.draft.text.trim_start().trim_start_matches(ASIDE.trim_end()).trim_start();
    self.restore = Some(format!("{ASIDE}{rest}"));
  }

  /// The folder that the life of a record opens in when no life holds the record: the folder of
  /// the client for a new record, else the project of its cards.
  pub fn cwd(&self, at: usize, life: &str) -> Option<String> {
    let cards = self.machines.get(at).map(|told| &told.cards[..]).unwrap_or_default();
    match cards.iter().find(|card| card.life == life) {
      None => Some(self.folder.clone()),
      Some(card) => card.project.clone(),
    }
  }

  /// The record of a new session, in the sessions of the project of a folder, named by the second
  /// and the moment of the client, so no two clients share it, and past each record that the
  /// client knows, so no two sessions of one client share it when its clocks stand still.
  pub fn record(&self, folder: &str) -> String {
    let second = Timestamp::from_second(i64::try_from(self.second()).unwrap_or_default());
    let stamp = second.unwrap_or_default().strftime("%Y-%m-%dT%H-%M-%S");
    let sessions = Path::new(folder).join(".furb").join("sessions");
    let named = |n: u128| sessions.join(format!("{stamp}-{:04x}.jsonl", n % 0x1_0000));
    let known = |life: &str| {
      self.holds.keys().any(|held| held.1 == life)
        || self.machines.iter().any(|one| one.cards.iter().any(|card| card.life == life))
    };
    let mut n = self.moved.as_nanos();
    while known(&named(n).display().to_string()) {
      n += 1;
    }
    named(n).display().to_string()
  }

  /// The second of the clock of the machine of the client.
  pub fn second(&self) -> u64 {
    self.now / 1000
  }
}
