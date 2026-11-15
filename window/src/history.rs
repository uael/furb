//! The history of the input: what the operator sent in the project of the target of the input, as
//! the daemon of its machine gives it from the records, so each window of each machine walks the
//! same history. The window asks it at each change of the target, after each message or word that
//! went, and again at the start of each walk, so a walk holds what was sent until then.
//!
//! The up arrow on the first row of the input walks it from the newest, and the down arrow on its
//! last row walks back. A walk takes only the texts that start with what the input held when the
//! walk began, as fish does, and the down arrow past the newest gives that text back. While no walk
//! runs, the input shows in grey the rest of the newest text that starts with what he wrote, which
//! the right arrow takes.

use furb::view::Sent;
use furb_host::protocol::Verb;
use unicode_width::UnicodeWidthStr;

use furb_client::Target;

use crate::{input::Draft, line::cut, model::Model};

/// The history of the input: the target that it was asked for, whether the window asked it, and
/// the history once the machine gave it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct History {
  target: Target,
  asked: bool,
  sent: Option<Vec<Sent>>,
}

/// A walk through the history: the draft that it began from, and the place of the text that the
/// input shows among the texts that start with that draft, the newest first, or none while it
/// shows the draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Walk {
  from: Draft,
  place: Option<usize>,
}

impl Model {
  /// An arrow on the first row of the input, up, or on its last row, down: whether the walk took
  /// it. The first up begins a walk from the draft, and asks the history again. A down takes the
  /// newer text, and the draft past the newest; a down with no walk, or on the draft, moves in the
  /// input.
  pub(crate) fn walked(&mut self, up: bool) -> bool {
    let walk = match (self.walk.take(), up) {
      (Some(walk), _) => walk,
      (None, false) => return false,
      (None, true) => {
        self.history = Some(History { target: self.target(), asked: false, sent: None });
        Walk { from: self.draft.clone(), place: None }
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

  /// The ask of the history of the target of the input, when the window did not ask it yet, with
  /// its machine and that target.
  pub(crate) fn history_due(&mut self) -> Option<(usize, Verb, Target)> {
    let target = self.target();
    if self.history.as_ref().is_none_or(|heard| heard.target != target) {
      self.history = Some(History { target: target.clone(), asked: false, sent: None });
    }
    let heard = self.history.as_mut().filter(|heard| !heard.asked)?;
    heard.asked = true;
    let (at, life) = match target.clone() {
      Target::Thread(key) => (key.0, key.1),
      Target::Session(at, life) => (at, life),
      Target::New(at, folder) => (at, self.record(&folder)),
    };
    Some((at, Verb::History { life }, target))
  }

  /// The history asked again, after a message or a word went, while the input keeps the history
  /// before it until the answer comes.
  pub(crate) fn ask_history(&mut self) {
    if let Some(history) = self.history.as_mut() {
      history.asked = false;
    }
  }

  /// The history that a machine gave for a target: the input takes it while that target stands.
  pub(crate) fn historied(&mut self, target: &Target, sent: Vec<Sent>) {
    let Some(heard) = self.history.as_mut().filter(|heard| heard.target == *target) else { return };
    heard.sent = Some(sent);
    self.stepped();
  }

  /// The text of the editor, which the draft takes: a text that the walk did not show ends the
  /// walk.
  pub(crate) fn edited(&mut self, text: String) {
    if self.walk.as_ref().is_some_and(|walk| self.walk_text(walk) != Some(&text)) {
      self.walk = None;
    }
    self.draft.text = text;
  }

  /// The rest of the newest text that he sent here that starts with what he wrote, while no walk
  /// runs and no form of a question takes the input.
  pub(crate) fn suggested(&self) -> Option<&str> {
    let typed = &self.draft.text;
    if self.walk.is_some() || typed.is_empty() || self.form().is_some() {
      return None;
    }
    let sent = self.history.as_ref()?.sent.as_ref()?;
    let newest = sent
      .iter()
      .find(|one| one.text.len() > typed.len() && one.text.starts_with(typed.as_str()))?;
    newest.text.get(typed.len()..)
  }

  /// The rest that the input shows in grey, at the end of what he wrote: its column, its row, and
  /// its text on one row, cut at the end of the box of the input.
  pub(crate) fn ghost(&self) -> Option<(u16, u16, String)> {
    let rest = self.suggested()?;
    let area = self.area();
    let width = usize::from(area.width);
    if width == 0 {
      return None;
    }
    // The editor breaks a line that passes its columns at any character.
    let mut lines = self.draft.text.split('\n').map(UnicodeWidthStr::width);
    let last = lines.next_back()?;
    let above: usize = lines.map(|one| one.div_ceil(width).max(1)).sum();
    let (row, column) = (above + last / width, last % width);
    let rest = cut(&rest.replace('\n', "↵"), width - column);
    let number = |one: usize| u16::try_from(one).ok();
    let (x, y) = (area.x + number(column)?, area.y + number(row)?);
    (row < usize::from(area.height)).then_some((x, y, rest))
  }

  /// The text at the place of the walk given to the input, with its kind: the oldest one that
  /// starts with the draft when the walk went past it, and the draft at no place. A walk that
  /// finds none says so, and shows the draft.
  fn stepped(&mut self) {
    let Some(walk) = self.walk.as_ref() else { return };
    let Some(found) = self.found(walk) else { return };
    let (shown, place) = match (walk.place, found.len().checked_sub(1)) {
      (None, _) => (walk.from.clone(), None),
      (Some(place), Some(oldest)) => {
        let one = found[place.min(oldest)];
        (Draft { text: one.text.clone(), python: one.python }, Some(place.min(oldest)))
      }
      (Some(_), None) => {
        let says = if walk.from.text.is_empty() {
          "You sent nothing here yet."
        } else {
          "Nothing that you sent here starts so."
        };
        self.notice = Some(says.to_owned());
        (walk.from.clone(), None)
      }
    };
    if let Some(walk) = self.walk.as_mut() {
      walk.place = place;
    }
    // The editor tells no change of a text that the window gives it, so the draft takes it here.
    self.restore = Some(shown.text.clone());
    self.draft = shown;
  }

  /// The texts that start with the draft of a walk, the newest first, once the machine gave the
  /// history.
  fn found(&self, walk: &Walk) -> Option<Vec<&Sent>> {
    let from = &walk.from.text;
    let sent = self.history.as_ref()?.sent.as_ref()?;
    Some(
      sent.iter().filter(|one| one.text != *from && one.text.starts_with(from.as_str())).collect(),
    )
  }

  /// The text that a walk gives the input, once the machine gave the history.
  fn walk_text<'a>(&'a self, walk: &'a Walk) -> Option<&'a str> {
    match walk.place {
      Some(place) => self.found(walk)?.get(place).map(|one| one.text.as_str()),
      None => Some(&walk.from.text),
    }
  }
}
