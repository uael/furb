//! The history of the input: what the operator sent in the project of the target of the input, as
//! the daemon of its machine gives it from the records, so each client of each machine walks the
//! same history. The client asks it at each change of the target, after each message or word that
//! went, and again at the start of each walk, so a walk holds what was sent until then.
//!
//! A step back walks it from the newest, and a step forward walks back. A walk takes only the texts
//! that start with what the input held when the walk began, as fish does, and a step forward past
//! the newest gives that text back. While no walk runs, the input offers the rest of the newest
//! text that starts with what he wrote.

use furb_wire::{Sent, protocol::Verb};

use crate::{Client, Draft, Target};

/// The history of the input: the target that it was asked for, whether the client asked it, and
/// the history once the machine gave it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct History {
  target: Target,
  asked: bool,
  sent: Option<Vec<Sent>>,
}

/// A walk through the history: the draft that it began from, and the place of the text that the
/// input shows among the texts that start with that draft, the newest first, or none while it
/// shows the draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Walk {
  from: Draft,
  place: Option<usize>,
}

impl Client {
  /// A step through the history, back to an older text or forward: whether the walk took it. The
  /// first step back begins a walk from the draft, and asks the history again. A step forward
  /// takes the newer text, and the draft past the newest; a step forward with no walk, or on the
  /// draft, is no step of the history.
  pub fn walked(&mut self, back: bool) -> bool {
    let walk = match (self.walk.take(), back) {
      (Some(walk), _) => walk,
      (None, false) => return false,
      (None, true) => {
        self.history = Some(History { target: self.target(), asked: false, sent: None });
        Walk { from: self.draft.clone(), place: None }
      }
    };
    let place = match (walk.place, back) {
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

  /// The ask of the history of the target of the input, when the client did not ask it yet, with
  /// its machine and that target.
  pub fn history_due(&mut self) -> Option<(usize, Verb, Target)> {
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
  pub fn ask_history(&mut self) {
    if let Some(history) = self.history.as_mut() {
      history.asked = false;
    }
  }

  /// The history that a machine gave for a target: the input takes it while that target stands.
  pub fn historied(&mut self, target: &Target, sent: Vec<Sent>) {
    let Some(heard) = self.history.as_mut().filter(|heard| heard.target == *target) else { return };
    heard.sent = Some(sent);
    self.stepped();
  }

  /// The text of the editor of the input, which the draft takes: a text that the walk did not show
  /// ends the walk.
  pub fn edited(&mut self, text: String) {
    if self.walk.as_ref().is_some_and(|walk| self.walk_text(walk) != Some(&text)) {
      self.walk = None;
    }
    self.draft.text = text;
  }

  /// The rest of the newest text that he sent here that starts with what he wrote, while no walk
  /// runs and no form of a question takes the input.
  pub fn suggested(&self) -> Option<&str> {
    let typed = &self.draft.text;
    let form = self.thread.as_ref().is_some_and(|opened| opened.form.is_some());
    if self.walk.is_some() || typed.is_empty() || form {
      return None;
    }
    let sent = self.history.as_ref()?.sent.as_ref()?;
    let newest = sent
      .iter()
      .find(|one| one.text.len() > typed.len() && one.text.starts_with(typed.as_str()))?;
    newest.text.get(typed.len()..)
  }

  /// What he sent where the input sends, the newest first, once the machine gave it.
  pub fn sent(&self) -> &[Sent] {
    self.history.as_ref().and_then(|one| one.sent.as_deref()).unwrap_or_default()
  }

  /// A text that he sent, which the input takes in the place of its draft, as a step of a walk
  /// does.
  pub fn recall(&mut self, sent: &Sent) {
    self.walk = None;
    self.restore = Some(sent.text.clone());
    self.draft = Draft { text: sent.text.clone(), python: sent.python };
  }

  /// Where the walk stands: the place of the text that the input shows, from one, among the texts
  /// that start with the draft that the walk began from, and how many they are.
  pub fn walking(&self) -> Option<(usize, usize)> {
    let walk = self.walk.as_ref()?;
    let found = self.found(walk)?.len();
    walk.place.map(|place| (place + 1, found))
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
    // An editor tells no change of a text that the client gives it, so the draft takes it here.
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
