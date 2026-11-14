//! The thread that the operator opened, at the left of the list: its head, which never scrolls,
//! the feed of its exchanges, the dock of its agents, the input, and the row of hints. The pages
//! of the thread come from the daemon, with each rule of what a line and a block show: the window
//! lays them out in its columns.
//!
//! The feed shows from an anchor: an exchange, and a count of rows from its first row. So a page
//! of older exchanges, which comes above, moves no row that the operator reads, and a frame builds
//! the rows of the exchanges that show and of no other.

use std::{
  collections::{BTreeMap, HashMap, HashSet},
  mem,
  ops::Range,
};

use furb::view::{Card as Agent, Exchange, Page};
use furb_host::protocol::{Card, Verb};
use opentui::style::{Color, Style};
use unicode_width::UnicodeWidthStr;

use crate::{
  line::Line,
  list::hash,
  model::{Key, Model},
  tokens::Loader,
};

/// How many exchanges a page holds.
pub(crate) const PAGE: usize = 20;

/// The rows under the feed that stand in each frame: the input, the row under it, and the hints.
const INPUT: usize = 3;

/// The thread that the operator reads: its card, the exchanges that the daemon gave, how many
/// stand before them, its agents, where the operator reads it, and what he opened.
#[derive(Debug, Default)]
pub(crate) struct Opened {
  pub(crate) key: Key,
  pub(crate) exchanges: BTreeMap<usize, Exchange>,
  /// How many exchanges stand before the first that the window holds, and none before the first
  /// page came.
  pub(crate) above: Option<usize>,
  pub(crate) agents: Vec<Agent>,
  /// The row at the top of the feed while the operator reads away from its end, and none while
  /// the feed follows its end.
  pub(crate) top: Option<Anchor>,
  /// The blocks and the thoughts that the operator opened.
  pub(crate) unfolded: HashSet<Fold>,
  /// Each page that the window asked and that did not come, by the id of its request: the place
  /// of the exchange that it stands before, or none for the page of the end.
  pub(crate) asked: HashMap<u64, Option<usize>>,
  /// Why the daemon gave no page.
  pub(crate) fault: Option<String>,
}

/// A row of the feed as a place that holds when rows come above it: a count of rows from the
/// first row of an exchange, which is negative for a row above that exchange.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Anchor {
  pub(crate) at: usize,
  pub(crate) row: isize,
}

/// What a click opens or folds in the feed: the block or the thought of the answer to a message.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Fold {
  Block(String),
  Thought(String),
}

/// What a click on a row does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Click {
  Fold(Fold),
  Jump,
}

/// A row of the left side: its runs, the columns that go bright under the pointer and what a click
/// there does, and the column of its loader, a blank cell that a widget of its own draws.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Row {
  pub(crate) line: Line,
  pub(crate) hot: Option<(Range<usize>, Option<Click>)>,
  pub(crate) loader: Option<(usize, Loader)>,
}

impl Row {
  pub(crate) fn of(line: Line) -> Self {
    Self { line, ..Self::default() }
  }

  /// A row that takes a click in each column of its text.
  pub(crate) fn clicked(line: Line, click: Click) -> Self {
    let wide = width(&line);
    Self { line, hot: Some((0..wide, Some(click))), loader: None }
  }

  /// The row with its text bright where the pointer is on a thing that takes a click: its hot
  /// columns, or a link.
  fn lit(mut self, x: usize, hover: Color) -> Self {
    let hot = self.hot.as_ref().map(|hot| hot.0.clone()).filter(|hot| hot.contains(&x));
    let mut start = 0;
    for one in &mut self.line {
      let end = start + one.content.width();
      let on = |range: &Range<usize>| range.start < end && start < range.end;
      if hot.as_ref().map_or(one.link.is_some() && (start..end).contains(&x), on) {
        one.style = one.style.fg(hover);
      }
      start = end;
    }
    self
  }
}

/// The columns that a line takes.
pub(crate) fn width(line: &Line) -> usize {
  line.iter().map(|one| one.content.width()).sum()
}

/// What the left side shows of an open thread: its rows, each loader of which stands blank, and
/// the cell of each loader.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Left {
  pub(crate) rows: Vec<Line>,
  pub(crate) loaders: Vec<Spot>,
}

/// The cell of a loader: its column, its row, its marks and its look.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Spot {
  pub(crate) x: u16,
  pub(crate) y: u16,
  pub(crate) loader: Loader,
  pub(crate) style: Style,
}

/// One part of the feed in its order: the head of the feed, which says what stands before the
/// exchanges that the window holds, then each of those exchanges by its place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Item {
  Head,
  At(usize),
}

/// The parts of a feed in order, from the exchanges that the window holds.
pub(crate) struct Items<'a, V> {
  pub(crate) held: &'a BTreeMap<usize, V>,
  /// Whether the head of the feed shows, above the first exchange.
  pub(crate) head: bool,
}

/// The rows of the feed that show: from the top, the place of the top, and whether the last row of
/// the feed shows, so the feed follows its end.
#[derive(Debug)]
pub(crate) struct Shown {
  pub(crate) rows: Vec<Row>,
  pub(crate) top: Option<Anchor>,
  pub(crate) end: bool,
}

impl<V> Items<'_, V> {
  fn before(&self, item: Item) -> Option<Item> {
    match item {
      Item::Head => None,
      Item::At(at) => {
        let earlier = self.held.range(..at).next_back().map(|one| Item::At(*one.0));
        earlier.or_else(|| self.head.then_some(Item::Head))
      }
    }
  }

  fn after(&self, item: Item) -> Option<Item> {
    let from = match item {
      Item::Head => self.held.iter().next(),
      Item::At(at) => self.held.range(at + 1..).next(),
    };
    from.map(|one| Item::At(*one.0))
  }

  fn last(&self) -> Option<Item> {
    let last = self.held.keys().next_back().map(|at| Item::At(*at));
    last.or_else(|| self.head.then_some(Item::Head))
  }

  /// The anchor of a row of an item: a row of the head stands above the first exchange.
  fn anchor(&self, item: Item, row: usize, rows: usize) -> Option<Anchor> {
    let row = isize::try_from(row).unwrap_or_default();
    match item {
      Item::At(at) => Some(Anchor { at, row }),
      Item::Head => {
        let first = self.held.keys().next()?;
        Some(Anchor { at: *first, row: row - isize::try_from(rows).unwrap_or_default() })
      }
    }
  }

  /// The rows that show in a count of rows, from an anchor or at the end. Only the rows of the
  /// items that show, and of those between the anchor and them, are made.
  pub(crate) fn shown(
    &self,
    top: Option<Anchor>,
    room: usize,
    mut rows_of: impl FnMut(Item) -> Vec<Row>,
  ) -> Shown {
    let Some(Anchor { at, mut row }) = top.filter(|one| self.held.contains_key(&one.at)) else {
      return self.end(room, rows_of);
    };
    let mut item = Item::At(at);
    let mut rows = rows_of(item);
    // A row above the item: the items before it give their last rows, up to the first row. The
    // rows of each item that it passes wait for their turn.
    let mut passed: Vec<(Item, Vec<Row>)> = Vec::new();
    while row < 0 {
      let Some(earlier) = self.before(item) else {
        row = 0;
        break;
      };
      let before = rows_of(earlier);
      row += isize::try_from(before.len()).unwrap_or_default();
      passed.push((item, mem::replace(&mut rows, before)));
      item = earlier;
    }
    // A row under the item: the items after it give their first rows, up to the end.
    let mut row = usize::try_from(row).unwrap_or_default();
    while row >= rows.len() {
      row -= rows.len();
      match self.after(item) {
        Some(next) => (item, rows) = (next, rows_of(next)),
        None => return self.end(room, rows_of),
      }
    }
    let top = self.anchor(item, row, rows.len());
    let mut shown: Vec<Row> = rows.drain(row..).collect();
    while shown.len() < room {
      match passed.pop().or_else(|| self.after(item).map(|next| (next, rows_of(next)))) {
        Some((next, rows)) => {
          shown.extend(rows);
          item = next;
        }
        None => return self.end(room, rows_of),
      }
    }
    shown.truncate(room);
    Shown { rows: shown, top, end: false }
  }

  /// The last rows of the feed, from the last item up.
  fn end(&self, room: usize, mut rows_of: impl FnMut(Item) -> Vec<Row>) -> Shown {
    let mut items = Vec::new();
    let (mut item, mut count) = (self.last(), 0);
    while let Some(one) = item.filter(|_| count < room) {
      let rows = rows_of(one);
      count += rows.len();
      items.push((one, rows));
      item = self.before(one);
    }
    let skip = count.saturating_sub(room);
    let top = items.last().and_then(|(one, rows)| self.anchor(*one, skip, rows.len()));
    let rows = items.into_iter().rev().flat_map(|one| one.1).skip(skip).collect();
    Shown { rows, top, end: true }
  }

  /// How many rows stand above an anchor, counted up to a bound.
  fn above(&self, top: Anchor, bound: usize, mut rows_of: impl FnMut(Item) -> Vec<Row>) -> usize {
    let mut count = usize::try_from(top.row).unwrap_or_default();
    let mut deficit = usize::try_from(-top.row).unwrap_or_default();
    let mut item = self.before(Item::At(top.at));
    while let Some(one) = item.filter(|_| count < bound) {
      let rows = rows_of(one).len();
      count += rows.saturating_sub(deficit);
      deficit = deficit.saturating_sub(rows);
      item = self.before(one);
    }
    count
  }
}

impl Opened {
  /// A page that the daemon gave, or a change of the thread that it told: each exchange takes the
  /// place of the one before it, and the agents are those of the whole thread now.
  fn took(&mut self, page: Page) {
    self.above = Some(self.above.map_or(page.above, |above| above.min(page.above)));
    self.agents = page.agents;
    self.fault = None;
    self.exchanges.extend(page.exchanges.into_iter().map(|one| (one.at, one)));
  }

  /// The page of the thread before the exchange at a place, or the page of its end.
  fn page(&self, before: Option<usize>) -> Verb {
    let (life, thread) = (self.key.1.clone(), self.key.2.clone());
    Verb::Thread { life, thread, before, limit: PAGE }
  }

  /// The parts of the feed. The head shows until the first exchange of the thread is held.
  pub(crate) fn items(&self) -> Items<'_, Exchange> {
    let head = self.fault.is_some() || self.above != Some(0);
    Items { held: &self.exchanges, head }
  }
}

impl Model {
  /// The thread of the selected card opened, and the page of its end, which the window asks of its
  /// machine.
  pub(crate) fn open_thread(&mut self) -> Option<Verb> {
    let key = self.selected.clone()?;
    if key.2.is_empty() || self.thread.as_ref().is_some_and(|one| one.key == key) {
      return None;
    }
    let opened = Opened { key, ..Opened::default() };
    let verb = opened.page(None);
    self.thread = Some(opened);
    Some(verb)
  }

  /// The open thread on a machine, while it waits for the page of a request.
  pub(crate) fn waits(&self, at: usize, id: u64) -> bool {
    self.thread.as_ref().is_some_and(|one| one.key.0 == at && one.asked.contains_key(&id))
  }

  /// The answer to a page that the open thread asked: the page, or why the daemon gave none.
  /// Whether the open thread changed.
  pub(crate) fn answered(&mut self, at: usize, id: u64, page: Result<Page, String>) -> bool {
    let Some(opened) = self.thread.as_mut().filter(|one| one.key.0 == at) else { return false };
    if opened.asked.remove(&id).is_none() {
      return false;
    }
    match page {
      Ok(page) => opened.took(page),
      Err(why) => opened.fault = Some(why),
    }
    true
  }

  /// A change of a thread that a machine told: whether it is the open thread.
  pub(crate) fn changed(&mut self, at: usize, (life, thread): (&str, &str), page: Page) -> bool {
    let open = |one: &&mut Opened| one.key.0 == at && one.key.1 == life && one.key.2 == thread;
    let Some(opened) = self.thread.as_mut().filter(open) else { return false };
    opened.took(page);
    true
  }

  /// The feed moved by a count of rows, up for a negative count: at its end it follows its end,
  /// and when less than a screen of it stands above, it asks the page before.
  pub(crate) fn scroll_feed(&mut self, by: isize) -> Option<Verb> {
    let room = self.feed_room();
    let feed = self.feed()?;
    let items = feed.opened.items();
    let now = items.shown(feed.opened.top, room, |item| feed.rows(item));
    let moved = now.top.map(|top| Anchor { at: top.at, row: top.row + by });
    let then = items.shown(moved, room, |item| feed.rows(item));
    let near = then.top.is_none_or(|top| items.above(top, room, |item| feed.rows(item)) < room);
    let opened = self.thread.as_mut()?;
    opened.top = then.top.filter(|_| !then.end);
    let older = opened.asked.values().any(Option::is_some);
    let first = opened.exchanges.keys().next().copied();
    let before = opened.above.is_some_and(|above| above > 0);
    (near && before && !older).then(|| opened.page(first))
  }

  /// The feed at its end, which it follows from now on.
  pub(crate) fn jump(&mut self) {
    if let Some(opened) = self.thread.as_mut() {
      opened.top = None;
    }
  }

  /// A click on a cell of the left side: a block or a thought opens or folds, or the feed goes to
  /// its end.
  pub(crate) fn clicked_feed(&mut self, x: usize, y: usize) {
    let Some(rows) = self.left_rows() else { return };
    let click = rows.into_iter().nth(y).and_then(|row| row.hot).filter(|hot| hot.0.contains(&x));
    let Some(opened) = self.thread.as_mut() else { return };
    match click.and_then(|hot| hot.1) {
      Some(Click::Fold(fold)) => {
        if !opened.unfolded.remove(&fold) {
          opened.unfolded.insert(fold);
        }
      }
      Some(Click::Jump) => opened.top = None,
      None => {}
    }
  }

  /// The rows of the left side and the cell of each loader, while a thread is open. The text of a
  /// thing under the pointer that takes a click goes bright, and nothing else changes.
  pub(crate) fn left(&self) -> Option<Left> {
    let pointer = self.pointer.filter(|(x, _)| !self.on_list(*x));
    let mut rows = self.left_rows()?;
    if let Some((x, y)) = pointer
      && let Some(row) = rows.get_mut(usize::from(y))
    {
      *row = mem::take(row).lit(usize::from(x), self.tokens.hover);
    }
    let loaders = rows.iter().enumerate().filter_map(|(y, row)| {
      let (x, loader) = row.loader?;
      let style = spot(&row.line, x)?;
      Some(Spot { x: u16::try_from(x).ok()?, y: u16::try_from(y).ok()?, loader, style })
    });
    let loaders = loaders.collect();
    Some(Left { rows: rows.into_iter().map(|row| row.line).collect(), loaders })
  }

  /// Each row of the left side: the head, the feed in its room, the dock, the input and the hints.
  fn left_rows(&self) -> Option<Vec<Row>> {
    let feed = self.feed()?;
    let room = self.feed_room();
    let shown = feed.opened.items().shown(feed.opened.top, room, |item| feed.rows(item));
    let mut rows = vec![feed.head()];
    rows.extend(shown.rows);
    rows.resize(1 + room, Row::default());
    if !shown.end
      && let Some(last) = rows.last_mut()
    {
      *last = feed.jump();
    }
    rows.extend(feed.dock());
    rows.extend(feed.input());
    Some(rows)
  }

  /// How many rows the feed takes: the window less the head, the dock, the input and the hints.
  pub(crate) fn feed_room(&self) -> usize {
    let docked = self.feed().map_or(0, |feed| feed.dock().len());
    usize::from(self.size.1).saturating_sub(1 + docked + INPUT)
  }

  /// The card of a key, as its machine last told it.
  pub(crate) fn told_card(&self, key: &Key) -> Option<&Card> {
    let told = |one: &&Card| one.life == key.1 && one.thread.prompt == key.2;
    self.machines.get(key.0)?.cards.iter().find(told)
  }

  /// The hue of a thread, from its id.
  pub(crate) fn hue(&self, key: &Key) -> Color {
    self.tokens.hues[hash(&format!("{}{}", key.1, key.2)) % self.tokens.hues.len()]
  }
}

/// The look of the run of a line that stands at a column.
fn spot(line: &Line, x: usize) -> Option<Style> {
  let mut start = 0;
  line.iter().find_map(|one| {
    let found = (start..start + one.content.width().max(1)).contains(&x).then_some(one.style);
    start += one.content.width();
    found
  })
}

#[cfg(test)]
#[path = "thread.test.rs"]
mod test;
