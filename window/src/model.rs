//! What the window knows: the cards that each machine told, and where the hand of the operator
//! is.
//!
//! A card shows its state at once. Its place is the one of the last time that the list settled,
//! which the window does only while the hand is away from the list, so no card moves under a
//! click. The rules of a card, its part and its place among the others, come from the daemon and
//! the crate: the window only draws them.

use std::{path::PathBuf, time::Duration};

use furb::view::Part;
use furb_host::protocol::{Answer, Card, Frame, Verb};
use jiff::{Timestamp, tz::TimeZone};
use serde::{Deserialize, Serialize};

use crate::{Options, code::Colors, link::Heard, list::Row, thread::Opened, tokens::Tokens};

/// The width of the list of cards.
pub(crate) const LIST: u16 = 38;

/// The columns of the list and of the line at its left.
pub(crate) const SIDE: u16 = LIST + 1;

/// The least width of the window that shows the list beside the thread.
const BESIDE: u16 = 100;

/// The rows at the top of the list that never scroll.
const PINNED: usize = 2;

/// Where the connection to a machine stands: the window dials it, its daemon gave its cards, or
/// the connection ended, with why.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) enum Link {
  #[default]
  Dialing,
  Listed,
  Lost(String),
}

/// What one machine told the window, which the window keeps in a file for its next first frame:
/// the name of the machine, the login of the operator there, its cards, and the second at which
/// it last told them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Told {
  pub(crate) name: String,
  #[serde(default)]
  pub(crate) login: String,
  #[serde(default)]
  pub(crate) heard: u64,
  #[serde(default)]
  pub(crate) cards: Vec<Card>,
  #[serde(skip)]
  pub(crate) link: Link,
}

/// A card among those of each machine: its machine, its record and its prompt.
pub(crate) type Key = (usize, String, String);

pub(crate) struct Model {
  pub(crate) machines: Vec<Told>,
  /// The cards in the order of the list as it last settled, each with the part that it had then
  /// and with its state as its machine last told it.
  pub(crate) settled: Vec<(Part, Key, Card)>,
  pub(crate) selected: Option<Key>,
  /// Whether the resolved threads show their rows.
  pub(crate) open: bool,
  /// Whether the operator changed what the width of the window does with the list: he hid the
  /// list that stands beside the thread, or he showed the list over it.
  pub(crate) toggled: bool,
  /// Whether a machine made the window wait long enough to say so.
  pub(crate) slow: bool,
  /// The cell that the pointer is on, its column and its row.
  pub(crate) pointer: Option<(u16, u16)>,
  /// How many rows of the list stand above the first row that shows.
  pub(crate) scroll: usize,
  /// The second of the clock of the machine.
  pub(crate) now: u64,
  /// For how long the loaders moved.
  pub(crate) moved: Duration,
  pub(crate) size: (u16, u16),
  /// The folder of the window, whose project a new session starts in.
  pub(crate) folder: PathBuf,
  pub(crate) zone: TimeZone,
  pub(crate) tokens: Tokens,
  /// Why the window could not keep its cards.
  pub(crate) fault: Option<String>,
  /// The thread that the operator opened.
  pub(crate) thread: Option<Opened>,
  /// The colors of each text of python that the window drew.
  pub(crate) colors: Colors,
}

impl Model {
  /// The model of a window that opens, with what each machine told the window of before.
  pub(crate) fn new(
    machines: Vec<Told>,
    size: (u16, u16),
    tokens: Tokens,
    options: &Options,
  ) -> Self {
    Self {
      machines,
      settled: Vec::new(),
      selected: None,
      open: false,
      toggled: false,
      slow: false,
      pointer: None,
      scroll: 0,
      now: 0,
      moved: Duration::ZERO,
      size,
      folder: options.folder.clone(),
      zone: options.zone.clone(),
      tokens,
      fault: None,
      thread: None,
      colors: Colors::new(),
    }
  }

  /// One frame of the daemon of a machine, or the end of its connection: whether what the machine
  /// told changed. The first cause of a loss stays.
  pub(crate) fn hear(&mut self, at: usize, heard: Heard) -> bool {
    match heard {
      Ok(Frame::Reply { id, answer: Answer::Page(page) }) => {
        return self.answered(at, id, Ok(page));
      }
      Ok(Frame::Refused { id: Some(id), why, .. }) if self.waits(at, id) => {
        return self.answered(at, id, Err(why));
      }
      Ok(Frame::Thread { life, thread, page }) => return self.changed(at, (&life, &thread), page),
      _ => {}
    }
    let (now, told) = (self.now, &mut self.machines[at]);
    if matches!(told.link, Link::Lost(_)) {
      return false;
    }
    match heard {
      Ok(Frame::Reply { answer: Answer::Welcome { peer, .. }, .. }) => told.login = peer.login,
      Ok(Frame::Reply { answer: Answer::Cards(cards), .. }) => {
        (told.cards, told.heard, told.link) = (cards, now, Link::Listed);
      }
      Ok(Frame::Cards { life, changed, gone }) => {
        let stays = |one: &Card| {
          let told = |prompt: &String| *prompt == one.thread.prompt;
          one.life != life
            || !(gone.iter().any(told) || changed.iter().any(|new| told(&new.thread.prompt)))
        };
        told.cards.retain(stays);
        told.cards.extend(changed);
        told.heard = now;
      }
      Ok(Frame::Refused { why, .. }) => told.link = Link::Lost(why),
      Err(fault) => told.link = Link::Lost(fault.to_string()),
      // A frame that the list does not show.
      Ok(_) => return false,
    }
    for (_, key, card) in self.settled.iter_mut().filter(|one| one.1.0 == at) {
      let told = |one: &&Card| one.life == key.1 && one.thread.prompt == key.2;
      if let Some(now) = self.machines[at].cards.iter().rfind(told) {
        card.clone_from(now);
      }
    }
    true
  }

  /// The list settled: each card of each machine at its place.
  pub(crate) fn settle(&mut self) {
    let cards = self.machines.iter().enumerate().flat_map(|(at, told)| {
      told.cards.iter().map(move |card| (card.place(&told.login), &card.thread.title, at, card))
    });
    let mut cards: Vec<_> = cards.collect();
    cards.sort_by(|one, other| (&one.0, one.1, one.2).cmp(&(&other.0, other.1, other.2)));
    let keyed = |((part, ..), _, at, card): ((Part, u8, i64), _, usize, &Card)| {
      (part, (at, card.life.clone(), card.thread.prompt.clone()), card.clone())
    };
    self.settled = cards.into_iter().map(keyed).collect();
    let stands = |key: &Key| self.settled.iter().any(|one| one.1 == *key);
    self.selected = self.selected.take().filter(stands);
    self.scrolled(0);
  }

  /// The card that stands a count of cards after the selected one, selected. The resolved threads
  /// take part only while they show.
  pub(crate) fn step(&mut self, by: isize) {
    let shown = |(part, ..): &&(Part, Key, Card)| self.open || *part != Part::Resolved;
    let keys: Vec<&Key> = self.settled.iter().filter(shown).map(|one| &one.1).collect();
    let at = self.selected.as_ref().and_then(|key| keys.iter().position(|one| *one == key));
    let next = match at {
      Some(at) => at.saturating_add_signed(by).min(keys.len().saturating_sub(1)),
      None if by > 0 => 0,
      None => keys.len().saturating_sub(1),
    };
    self.selected = keys.get(next).map(|key| (*key).clone());
    self.reveal();
  }

  /// The next card that needs the operator after the selected one, selected, from the first one
  /// again after the last.
  pub(crate) fn needed(&mut self) {
    let needs: Vec<&Key> =
      self.settled.iter().filter(|one| self.needs(&one.1, &one.2)).map(|one| &one.1).collect();
    let at = self.selected.as_ref().and_then(|key| needs.iter().position(|one| *one == key));
    let next = at.map_or(0, |at| (at + 1) % needs.len().max(1));
    if let Some(key) = needs.get(next) {
      self.selected = Some((*key).clone());
      self.reveal();
    }
  }

  /// A click on a row of the window, in the list: a card is selected and its thread opens, and the
  /// head of the resolved threads shows or folds their rows. The page that the thread asks.
  pub(crate) fn clicked(&mut self, y: usize) -> Option<Verb> {
    match self.row(y) {
      Some(Row::Card(at, _) | Row::Resolved(at)) => {
        self.selected = self.settled.get(at).map(|one| one.1.clone());
        self.open_thread()
      }
      Some(Row::Head(Part::Resolved)) => {
        self.open = !self.open;
        // The resolved threads end the list, so the list moves to its end to show them.
        self.scrolled(isize::MAX);
        None
      }
      _ => None,
    }
  }

  /// The list moved by a count of rows, as far as it has rows.
  pub(crate) fn scrolled(&mut self, by: isize) {
    let most = (self.rows().len().saturating_sub(PINNED)).saturating_sub(self.room());
    self.scroll = self.scroll.saturating_add_signed(by).min(most);
  }

  /// The window at a new size. A list that the operator showed or hid goes back to what the width
  /// of the window gives when the width crosses the bound.
  pub(crate) fn resized(&mut self, size: (u16, u16)) {
    if (size.0 < BESIDE) != (self.size.0 < BESIDE) {
      self.toggled = false;
    }
    self.size = size;
    self.scrolled(0);
  }

  /// The list shown when it was hidden, and hidden when it showed.
  pub(crate) fn toggle(&mut self) {
    self.toggled = !self.toggled;
  }

  /// The list closed, when it stands over the home.
  pub(crate) fn close(&mut self) {
    self.toggled &= self.size.0 >= BESIDE;
  }

  /// Whether the list shows.
  pub(crate) fn listed(&self) -> bool {
    (self.size.0 >= BESIDE) != self.toggled
  }

  /// Whether a column of the window stands on the list.
  pub(crate) fn on_list(&self, x: u16) -> bool {
    self.listed() && x >= self.size.0.saturating_sub(LIST)
  }

  /// How many columns the home has: those that the list leaves it while the list shows.
  pub(crate) fn wide(&self) -> usize {
    usize::from(self.size.0.saturating_sub(if self.listed() { SIDE } else { 0 }))
  }

  /// Whether each machine answered or was lost, so the window knows what stands.
  pub(crate) fn known(&self) -> bool {
    self.machines.iter().all(|one| one.link != Link::Dialing)
  }

  /// Whether a mark of the list moves.
  pub(crate) fn moves(&self) -> bool {
    let works = |one: &(Part, Key, Card)| one.2.thread.works() && self.confirmed(&one.1);
    self.slow && !self.known() || self.settled.iter().any(works)
  }

  pub(crate) fn needs(&self, key: &Key, card: &Card) -> bool {
    card.thread.needs(&self.machines[key.0].login)
  }

  /// Whether the machine of a card told the card in this connection.
  pub(crate) fn confirmed(&self, key: &Key) -> bool {
    self.machines[key.0].link == Link::Listed
  }

  /// The hour and the minute of a second, in the zone of the window.
  pub(crate) fn hour(&self, second: u64) -> String {
    let at = Timestamp::from_second(i64::try_from(second).unwrap_or_default()).unwrap_or_default();
    at.to_zoned(self.zone.clone()).strftime("%H:%M").to_string()
  }

  /// How long ago a second was, from one minute on.
  pub(crate) fn age(&self, since: u64) -> Option<String> {
    match self.now.saturating_sub(since) {
      ..60 => None,
      age @ ..3600 => Some(format!("{}m", age / 60)),
      age @ ..86400 => Some(format!("{}h", age / 3600)),
      age => Some(format!("{}d", age / 86400)),
    }
  }

  /// The rows of the list that scroll, and how many of them show.
  fn room(&self) -> usize {
    usize::from(self.size.1).saturating_sub(PINNED)
  }

  /// The row of the list at a row of the window.
  pub(crate) fn row(&self, y: usize) -> Option<Row> {
    let rows = self.rows();
    rows.get(if y < PINNED { y } else { y + self.scroll }).copied()
  }

  /// The rows of the list that show, from the top of the window.
  pub(crate) fn shown(&self) -> impl Iterator<Item = Row> {
    let rows = self.rows();
    let pinned = rows.len().min(PINNED);
    let scrolled: Vec<Row> =
      rows[pinned..].iter().skip(self.scroll).take(self.room()).copied().collect();
    rows.into_iter().take(pinned).chain(scrolled)
  }

  /// The list moved as far as the selected card needs to show, with the row above it and the row
  /// under it.
  fn reveal(&mut self) {
    let Some(key) = &self.selected else { return };
    let card = |row: &Row| match row {
      Row::Card(at, _) | Row::Resolved(at) => self.settled[*at].1 == *key,
      _ => false,
    };
    let rows = self.rows();
    let (Some(first), Some(last)) = (rows.iter().position(card), rows.iter().rposition(card))
    else {
      return;
    };
    let (above, under) = (first.saturating_sub(PINNED + 1), (last + 2).saturating_sub(PINNED));
    self.scroll = self.scroll.max(under.saturating_sub(self.room())).min(above);
  }
}

#[cfg(test)]
#[path = "model.test.rs"]
mod test;
