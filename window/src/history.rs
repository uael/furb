//! The history of the input: what the operator sent in the project of the target of the input, as
//! the daemon of its machine gives it from the records, so each window of each machine walks the
//! same history. The up arrow on the first row of the input walks it from the newest, and the down
//! arrow on its last row walks back. A walk takes only the texts that start with what the input
//! held when the walk began, as fish does, and the down arrow past the newest gives that text back.
//! A walk asks its machine once, when it begins, so it shows what was sent until then.

use furb::view::Sent;
use furb_host::protocol::Verb;

use crate::{
  input::{Draft, Target},
  model::Model,
};

/// A walk through the history: its machine and the record that it asked, the history once the
/// machine gave it, the draft that it began from, and the place of the text that the input shows
/// among the texts that start with that draft, the newest first, or none while it shows the
/// draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Walk {
  at: usize,
  life: String,
  asked: bool,
  sent: Option<Vec<Sent>>,
  from: Draft,
  place: Option<usize>,
}

impl Model {
  /// An arrow on the first row of the input, up, or on its last row, down: whether the walk took
  /// it. The first up begins a walk from the draft. A down takes the newer text, and the draft
  /// past the newest; a down with no walk, or on the draft, moves in the input.
  pub(crate) fn walked(&mut self, up: bool) -> bool {
    let walk = match (self.walk.take(), up) {
      (Some(walk), _) => walk,
      (None, false) => return false,
      (None, true) => {
        let (at, life) = match self.target() {
          Target::Thread(key) => (key.0, key.1),
          Target::Session(at, life) => (at, life),
          Target::New(at) => (at, self.record()),
        };
        Walk { at, life, asked: false, sent: None, from: self.draft.clone(), place: None }
      }
    };
    let place = match (walk.place, up) {
      (None, false) => {
        self.walk = Some(walk);
        return false;
      }
      (None, true) => Some(0),
      (Some(0), false) => None,
      (Some(place), false) => Some(place - 1),
      (Some(place), true) => Some(place + 1),
    };
    self.walk = Some(Walk { place, ..walk });
    self.stepped();
    true
  }

  /// The ask of the history of a walk that did not ask it yet, with its machine.
  pub(crate) fn history_due(&mut self) -> Option<(usize, Verb)> {
    let walk = self.walk.as_mut().filter(|one| !one.asked)?;
    walk.asked = true;
    Some((walk.at, Verb::History { life: walk.life.clone() }))
  }

  /// The history that the machine of the walk gave.
  pub(crate) fn historied(&mut self, sent: Vec<Sent>) {
    if let Some(walk) = self.walk.as_mut() {
      walk.sent = Some(sent);
      self.stepped();
    }
  }

  /// The text of the editor, which the draft takes: a text that the walk did not show ends the
  /// walk.
  pub(crate) fn edited(&mut self, text: String) {
    if self.walk.as_ref().is_some_and(|walk| walk.text() != Some(&text)) {
      self.walk = None;
    }
    self.draft.text = text;
  }

  /// The text at the place of the walk given to the input, with its kind: the oldest one that
  /// starts with the draft when the walk went past it, and the draft at no place. A walk that
  /// finds none says so, and shows the draft.
  fn stepped(&mut self) {
    let Some(walk) = self.walk.as_mut() else { return };
    let Some(found) = walk.found() else { return };
    let shown = match (walk.place, found.len().checked_sub(1)) {
      (None, _) => walk.from.clone(),
      (Some(place), Some(oldest)) => {
        let place = place.min(oldest);
        let shown = Draft { text: found[place].text.clone(), python: found[place].python };
        walk.place = Some(place);
        shown
      }
      (Some(_), None) => {
        walk.place = None;
        let says = if walk.from.text.is_empty() {
          "You sent nothing here yet."
        } else {
          "Nothing that you sent here starts so."
        };
        self.notice = Some(says.to_owned());
        return;
      }
    };
    self.draft.python = shown.python;
    self.restore = Some(shown.text);
  }
}

impl Walk {
  /// The texts that start with the draft of the walk, the newest first, once its machine gave the
  /// history.
  fn found(&self) -> Option<Vec<&Sent>> {
    let from = &self.from.text;
    let sent = self.sent.as_ref()?;
    Some(
      sent.iter().filter(|one| one.text != *from && one.text.starts_with(from.as_str())).collect(),
    )
  }

  /// The text that the walk gives the input, once its machine gave the history.
  fn text(&self) -> Option<&str> {
    match self.place {
      Some(place) => self.found()?.get(place).map(|one| one.text.as_str()),
      None => Some(&self.from.text),
    }
  }
}
