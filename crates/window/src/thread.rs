//! The thread that the operator opened, at the left of the list: its head, which never scrolls,
//! the feed of its exchanges, the dock of its agents, the input, and the row of hints. The pages
//! of the thread come from the daemon, with each rule of what a line and a block show: the window
//! lays them out in its columns.
//!
//! The feed shows from a place: an exchange, and a count of rows from its first row. So a page
//! of older exchanges, which comes above, moves no row that the operator reads, and a frame builds
//! the rows of the exchanges that show and of no other.

use std::{
  collections::{BTreeMap, HashMap},
  mem,
  ops::Range,
  time::Duration,
};

use crate::cells::Cells;
use furb::{
  engine::OPERATOR,
  view::{self, Page, Standing},
};
use furb_host::protocol::{Card, Gesture, Verb, ViewEvent};
use opentui::style::{Color, Span, Style};

use furb_client::{Filled, Fold, Hold, Opened, Place, Thread};

use crate::{
  keys::Does,
  line::{Line, styled},
  list::hash,
  model::{Key, Model},
  opens::QUIET,
  tokens::Loader,
};

/// What a click on a row does: it opens or folds a part of the feed, it does what a key does, it
/// picks an entry of the picker of the sessions, it answers a question with a value of its form,
/// or it opens the thread of an agent, its prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Click {
  Fold(Fold),
  Key(Does),
  Pick(usize),
  /// A group of the settings, by its name, which shows.
  Tab(String),
  Answer(usize),
  Agent(String),
  /// The feed moved to the line of the thread that binds a name, by its number, and the name.
  Line(usize, String),
  Context,
  /// The thread taken to a moment of its time, by its place among the moments.
  Time(usize),
  /// The play of the time of the thread started or stopped.
  Play,
  /// The thread of a chain under the bar of the time opened, by the row of the chain and a place
  /// among the moments of the bar.
  Chain {
    row: usize,
    at: usize,
  },
  Link(String),
  /// A follow-up that waits on the thread taken out of its queue, by its entry.
  Unqueue(String),
  /// A press of a button of a view, by its viewer and its node.
  Press {
    viewer: String,
    node: String,
  },
  /// The input of the window turned to fill an input of a view, from the value that it holds.
  Fill {
    viewer: String,
    node: String,
    value: String,
  },
}

/// The columns of a row that go bright under the pointer, and what a click there does.
pub(crate) type Hot = (Range<usize>, Option<Click>);

/// A row of the left side: its runs, the parts that go bright under the pointer, the columns among
/// them that keep their color, as the word of a state, the column of each of its loaders, which a
/// widget of its own draws over the cells of the row, and the runs that it shows in place of its
/// own while the pointer is on it, as the value beside a line of code that ran.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Row {
  pub(crate) line: Line,
  pub(crate) hot: Vec<Hot>,
  pub(crate) keeps: Option<Range<usize>>,
  pub(crate) loaders: Vec<(usize, Loader)>,
  pub(crate) pointed: Option<Line>,
  /// The acts that the line of code of the row awaits, which the pointer on it links to.
  pub(crate) awaits: Vec<String>,
  /// The number of the line of code that the row shows first.
  pub(crate) code: Option<usize>,
  /// The columns of each name of the code of the row, with the name.
  pub(crate) names: Vec<(Range<usize>, String)>,
  /// For the head of a block, the column of its code and what the head says at its right, beside
  /// which it shows what the name under the pointer in the block holds, from that column.
  pub(crate) inspector: Option<(usize, Line)>,
  /// The box of an image whose first row the row is, which the image fills.
  pub(crate) picture: Option<Boxed>,
}

/// The box of an image on the left side: the uri by which a message names the image, its column
/// and its row, and its size in cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Boxed {
  pub(crate) source: String,
  pub(crate) x: u16,
  pub(crate) y: u16,
  pub(crate) wide: u16,
  pub(crate) high: u16,
}

impl Row {
  pub(crate) fn of(line: Line) -> Self {
    Self { line, ..Self::default() }
  }

  /// A row that takes a click in each column of its text.
  pub(crate) fn clicked(line: Line, click: Click) -> Self {
    let wide = width(&line);
    Self { line, hot: vec![(0..wide, Some(click))], ..Self::default() }
  }

  /// What a click on a column of the row does: what its hot columns say, else the open of the
  /// link there.
  pub(crate) fn click(&self, x: usize) -> Option<Click> {
    match self.hot.iter().find(|hot| hot.0.contains(&x)) {
      Some(hot) => hot.1.clone(),
      None => self.link(x).map(|link| Click::Link(link.to_owned())),
    }
  }

  /// The address of the link at a column of the row.
  fn link(&self, x: usize) -> Option<&str> {
    let at = columns(&self.line).iter().position(|one| one.contains(&x))?;
    self.line[at].link.as_deref()
  }

  /// The row as it stands under the pointer: the runs that it shows there, with its text bright
  /// where the pointer is on a thing that takes a click: its hot columns but those that keep their
  /// color, or each word of the link there.
  pub(crate) fn lit(mut self, x: usize, hover: Color) -> Self {
    if let Some(pointed) = self.pointed.take() {
      self.line = pointed;
    }
    let hot = self.hot.iter().map(|hot| hot.0.clone()).find(|hot| hot.contains(&x));
    let link = self.link(x).map(str::to_owned);
    let mut start = 0;
    for one in &mut self.line {
      let end = start + one.content.width();
      let on = |range: &Range<usize>| range.start < end && start < range.end;
      let kept = self.keeps.as_ref().is_some_and(on);
      if !kept && hot.as_ref().map_or(link.is_some() && one.link.as_deref() == link.as_deref(), on)
      {
        one.style = one.style.fg(hover);
      }
      start = end;
    }
    self
  }
}

/// The columns that a line takes.
pub(crate) fn width(line: &[Span<'_>]) -> usize {
  line.iter().map(|one| one.content.width()).sum()
}

/// The columns of each run of a line.
pub(crate) fn columns(line: &[Span<'_>]) -> Vec<Range<usize>> {
  let mut start = 0;
  let column = |one: &Span<'_>| {
    let end = start + one.content.width();
    (mem::replace(&mut start, end))..end
  };
  line.iter().map(column).collect()
}

/// What the left side shows of an open thread: its rows, each loader of which stands blank, and
/// the cell of each loader.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Left {
  pub(crate) rows: Vec<Line>,
  pub(crate) loaders: Vec<Spot>,
  /// The box of each image, whose rows all show.
  pub(crate) pictures: Vec<Boxed>,
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
  pub(crate) top: Option<Place>,
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

  pub(crate) fn last(&self) -> Option<Item> {
    let last = self.held.keys().next_back().map(|at| Item::At(*at));
    last.or_else(|| self.head.then_some(Item::Head))
  }

  /// The place of a row of an item: a row of the head stands above the first exchange.
  fn place(&self, item: Item, row: usize, rows: usize) -> Option<Place> {
    let row = isize::try_from(row).unwrap_or_default();
    match item {
      Item::At(at) => Some(Place { at, row }),
      Item::Head => {
        let first = self.held.keys().next()?;
        Some(Place { at: *first, row: row - isize::try_from(rows).unwrap_or_default() })
      }
    }
  }

  /// The rows that show in a count of rows, from a place or at the end. Only the rows of the
  /// items that show, and of those between the place and them, are made.
  pub(crate) fn shown(
    &self,
    top: Option<Place>,
    room: usize,
    mut rows_of: impl FnMut(Item) -> Vec<Row>,
  ) -> Shown {
    let Some(Place { at, mut row }) = top.filter(|one| self.held.contains_key(&one.at)) else {
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
    let top = self.place(item, row, rows.len());
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
    let top = items.last().and_then(|(one, rows)| self.place(*one, skip, rows.len()));
    let rows = items.into_iter().rev().flat_map(|one| one.1).skip(skip).collect();
    Shown { rows, top, end: true }
  }

  /// How many rows stand above a place, counted up to a bound.
  fn above(&self, top: Place, bound: usize, mut rows_of: impl FnMut(Item) -> Vec<Row>) -> usize {
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

impl Model {
  /// The thread of the selected card opened, and the input turned to it. The window holds its
  /// life, which opens when no life holds its record.
  pub(crate) fn open_thread(&mut self) {
    let Some(key) = self.client.selected.clone() else { return };
    if key.2.is_empty()
      || self.client.thread.as_ref().is_some_and(|one| one.root().thread.key == key)
    {
      return;
    }
    self.client.hold((key.0, key.1.clone()), self.client.cwd(key.0, &key.1), self.client.moved);
    self.client.leave();
    let thread = Thread { key, since: self.client.moved, ..Thread::default() };
    self.client.thread = Some(Opened { thread, ..Opened::default() });
    self.client.aim = None;
  }

  /// The thread of an agent of the open thread opened in its place, and the input turned to it.
  /// The thread that it came from waits as the operator read it, and asks its end again when it
  /// opens again, since the daemon tells a window the changes of one thread at a time.
  pub(crate) fn open_agent(&mut self, prompt: String) {
    let Some(mut from) = self.client.thread.take() else { return };
    self.client.went_on();
    let key = (from.thread.key.0, from.thread.key.1.clone(), prompt);
    (from.thread.asked, from.thread.again, from.ahead) = (HashMap::new(), true, None);
    let thread = Thread { key, since: self.client.moved, ..Thread::default() };
    self.client.thread = Some(Opened { thread, from: Some(Box::new(from)), ..Opened::default() });
    (self.client.aim, self.armed) = (None, None);
  }

  /// The thread of the next agent of the dock opened in place: the first from the thread that
  /// they came from, the one after it from the thread of an agent, and the first again after the
  /// last.
  pub(crate) fn next_agent(&mut self) {
    let Some(opened) = self.client.thread.as_ref() else { return };
    let (from, at) =
      opened.from.as_deref().map_or((opened, None), |from| (from, Some(&opened.thread.key.2)));
    let shown = from.docked().1;
    let after = at.and_then(|at| shown.iter().position(|one| one.card.prompt == *at));
    let Some(next) = shown.get(after.map_or(0, |at| at + 1)).or(shown.first()) else {
      self.client.notice = Some("No agent of this thread works now.".to_owned());
      return;
    };
    let prompt = next.card.prompt.clone();
    if at.is_some() {
      self.back();
    }
    self.open_agent(prompt);
  }

  /// An escape: from a message about a line of a file, to a plain message; from the input of a
  /// view, back to the target of the input; from the open thread: from the thread of an agent, back
  /// to the thread that it came
  /// from; from a thread that opens or whose open failed, back to the list, while the open goes on.
  /// Whether it went back.
  pub(crate) fn back(&mut self) -> bool {
    if self.client.draft.anchor.take().is_some() {
      return true;
    }
    if self.client.filled.take().is_some() {
      self.client.restore = Some(String::new());
      return true;
    }
    if self.up() {
      return true;
    }
    let leaves = self.leaves();
    if leaves {
      self.client.leave();
      (self.client.selected, self.client.aim) = (None, None);
    }
    leaves
  }

  /// A step back or forward, as the side keys of a mouse step in a browser: back from a part
  /// that stands over the feed, from the thread of an agent back to the thread that it came from,
  /// forward to the agent that he went back from, and else to the place that he left before, or
  /// that he went back from.
  pub(crate) fn walk(&mut self, back: bool) {
    // A part that stands over the feed was the last move, so a step back closes it first.
    let over = self.picker.is_some() || self.client.review.is_some();
    if back && over {
      (self.picker, self.client.review) = (None, None);
      return;
    }
    let went = if back { self.up() } else { self.down() };
    if !went && !self.client.walk_trail(back) {
      let side = if back { "behind" } else { "ahead" };
      self.client.notice = Some(format!("No place stands {side}."));
    }
    self.armed = None;
  }

  /// From the thread of an agent back to the thread that it came from, where he read it, with the
  /// agent ahead. Whether the open thread was the thread of an agent.
  fn up(&mut self) -> bool {
    let Some(mut agent) = self.client.thread.take_if(|one| one.from.is_some()) else {
      return false;
    };
    let Some(mut from) = agent.from.take() else { return false };
    (agent.thread.asked, agent.thread.again) = (HashMap::new(), true);
    from.ahead = Some(Box::new(agent));
    (self.client.thread, self.client.aim) = (Some(*from), None);
    true
  }

  /// Forward to the thread of the agent that the operator went back from, where he read it.
  /// Whether one stood ahead.
  fn down(&mut self) -> bool {
    let Some(mut from) = self.client.thread.take_if(|one| one.ahead.is_some()) else {
      return false;
    };
    let Some(mut agent) = from.ahead.take() else { return false };
    (from.thread.asked, from.thread.again) = (HashMap::new(), true);
    agent.from = Some(Box::new(from));
    (self.client.thread, self.client.aim) = (Some(*agent), None);
    true
  }

  /// The feed moved to the line of the open thread that binds a name, which shows the name in the
  /// accent: its block opens when the line stands out of the rows that roll, and the feed keeps its
  /// place when the line shows.
  pub(crate) fn go_to(&mut self, number: usize, name: String) {
    let Some(opened) = self.client.thread.as_mut() else { return };
    let mut exchanges = opened.thread.exchanges.iter();
    let Some((at, exchange, block)) = exchanges.find_map(|(at, exchange)| {
      let block = exchange.block.as_ref()?;
      block.lines.iter().any(|one| one.number == number).then_some((*at, exchange, block))
    }) else {
      return;
    };
    let place = block.lines.iter().position(|one| one.number == number).unwrap_or_default();
    let fold = Fold::Block(exchange.message.clone());
    if !block.shown(opened.unfolded.contains(&fold)).contains(&place) {
      opened.unfolded.insert(fold);
    }
    opened.went = Some((number, name));
    if self.left_rows().iter().any(|one| one.code == Some(number)) {
      return;
    }
    let Some(feed) = self.feed() else { return };
    let row = feed.rows(Item::At(at)).iter().position(|one| one.code == Some(number));
    let row = isize::try_from(row.unwrap_or_default()).unwrap_or_default() - 3;
    if let Some(opened) = self.client.thread.as_mut() {
      opened.top = Some(Place { at, row });
    }
  }

  /// Why the open thread says that it went wrong: the daemon gave no page, or a fault ended its
  /// life.
  pub(crate) fn why<'a>(&'a self, opened: &'a Opened) -> Option<&'a str> {
    opened
      .thread
      .fault
      .as_deref()
      .or_else(|| self.client.fault_of(opened.thread.key.0, &opened.thread.key.1))
  }

  /// Whether the life of an open thread opens and its page did not come. A life whose open ended,
  /// or that a fault ended, opens no more.
  pub(crate) fn opens(&self, opened: &Opened) -> bool {
    let hold = self.client.holds.get(&(opened.thread.key.0, opened.thread.key.1.clone()));
    opened.thread.above.is_none() && hold.is_some_and(Hold::runs)
  }

  /// Whether the window says that the open thread opens: from [`QUIET`] on, as the bar of the open,
  /// so an open that takes less than that shows no text that comes and goes.
  pub(crate) fn says_opens(&self, opened: &Opened) -> bool {
    let hold = self.client.holds.get(&(opened.thread.key.0, opened.thread.key.1.clone()));
    let waited = hold.is_some_and(|hold| self.client.moved.saturating_sub(hold.since) >= QUIET);
    self.opens(opened) && waited
  }

  /// Whether an escape goes back from the open thread, as the row of hints says: it is the thread
  /// of an agent, a fault ended its life, or the window says that it opens. An open that the
  /// window does not say yet takes the escape as a thread that works does, so the key does what
  /// the row of hints shows.
  pub(crate) fn leaves(&self) -> bool {
    let says = |one: &Opened| one.from.is_some() || self.why(one).is_some() || self.says_opens(one);
    self.client.thread.as_ref().is_some_and(says)
  }

  /// The open thread on a machine, while it waits for the page of a request.
  pub(crate) fn waits(&self, at: usize, id: u64) -> bool {
    self
      .client
      .thread
      .as_ref()
      .is_some_and(|one| one.thread.key.0 == at && one.thread.asked.contains_key(&id))
  }

  /// The answer to a page that the open thread asked: the page, or why the daemon gave none. A
  /// page refused while the open of its life runs says why the open failed, as the refusal of the
  /// open says it, so the window holds that life no more. Whether the open thread changed.
  pub(crate) fn answered(&mut self, at: usize, id: u64, page: Result<Page, String>) -> bool {
    let Some(opened) = self.client.thread.as_mut().filter(|one| one.thread.key.0 == at) else {
      return false;
    };
    if opened.thread.asked.remove(&id).is_none() {
      return false;
    }
    let life = (at, opened.thread.key.1.clone());
    match page {
      Ok(page) => opened.thread.took(page, self.client.moved),
      Err(why) => {
        opened.thread.fault = Some(why);
        if self.client.holds.get(&life).is_some_and(Hold::runs) {
          self.client.holds.remove(&life);
        }
      }
    }
    true
  }

  /// A change of a thread that a machine told: whether it is the open thread.
  pub(crate) fn changed(&mut self, at: usize, (life, thread): (&str, &str), page: Page) -> bool {
    let open = |one: &&mut Opened| {
      one.thread.key.0 == at && one.thread.key.1 == life && one.thread.key.2 == thread
    };
    let Some(opened) = self.client.thread.as_mut().filter(open) else { return false };
    // A change of now gives the bar of the time new moments, and reaches no moment of the past.
    if let Some(time) = opened.thread.time.as_mut() {
      time.stale = true;
      if let Some(chains) = time.chains.as_mut() {
        chains.stale = true;
      }
    }
    if opened.thread.past() {
      return false;
    }
    opened.thread.took(page, self.client.moved);
    self.client.refollow();
    true
  }

  /// The feed moved by a count of rows, up for a negative count: at its end it follows its end,
  /// and when less than a screen of it stands above, it asks the page before.
  pub(crate) fn scroll_feed(&mut self, by: isize) -> Option<Verb> {
    let room = self.feed_room();
    let feed = self.feed()?;
    let items = feed.items();
    let now = items.shown(feed.opened.top, room, |item| feed.rows(item));
    let moved = now.top.map(|top| Place { at: top.at, row: top.row + by });
    let then = items.shown(moved, room, |item| feed.rows(item));
    let near = then.top.is_none_or(|top| items.above(top, room, |item| feed.rows(item)) < room);
    let opened = self.client.thread.as_mut()?;
    opened.top = then.top.filter(|_| !then.end);
    let older = opened.thread.asked.values().any(Option::is_some);
    let first = opened.thread.exchanges.keys().next().copied();
    let before = opened.thread.above.is_some_and(|above| above > 0);
    (near && before && !older).then(|| opened.thread.page(first))
  }

  /// The feed moved to the start of the message at its top, or of the one before it once it stands
  /// there, or to the start of the next message, and to its end after the last. The verb of an
  /// older page once the top nears the first message that the window holds.
  pub(crate) fn step_message(&mut self, back: bool) -> Option<Verb> {
    let room = self.feed_room();
    let feed = self.feed()?;
    let top = feed.items().shown(feed.opened.top, room, |item| feed.rows(item)).top?;
    let held = &feed.opened.thread.exchanges;
    let to = match back {
      true if top.row > 0 => Some(top.at),
      true => held.range(..top.at).next_back().map(|one| *one.0),
      false => held.range(top.at + 1..).next().map(|one| *one.0),
    };
    let Some(at) = to else {
      if !back {
        self.jump();
      }
      return None;
    };
    self.client.thread.as_mut()?.top = Some(Place { at, row: 0 });
    self.scroll_feed(0)
  }

  /// The feed at its end, which it follows from now on.
  pub(crate) fn jump(&mut self) {
    if let Some(opened) = self.client.thread.as_mut() {
      opened.top = None;
    }
  }

  /// A click on a cell of the left side: a block or a thought opens or folds, or a row of a list
  /// part is taken. The key that the click does the thing of.
  pub(crate) fn clicked_left(&mut self, x: u16, y: usize) -> Option<Does> {
    let x = usize::from(x);
    let click = self.left_rows().into_iter().nth(y).and_then(|row| row.click(x));
    match click? {
      Click::Fold(fold) => {
        self.client.thread.as_mut()?.fold(fold);
        None
      }
      Click::Pick(at) => self.took(at),
      Click::Tab(tab) => {
        self.tabbed(tab);
        None
      }
      Click::Key(does) => Some(does),
      Click::Answer(at) => self.choose(at),
      Click::Agent(prompt) => {
        self.open_agent(prompt);
        None
      }
      Click::Line(number, name) => {
        self.go_to(number, name);
        None
      }
      Click::Context => {
        self.context_step();
        None
      }
      Click::Time(at) => {
        self.client.reach(at);
        None
      }
      Click::Play => {
        self.client.play();
        None
      }
      Click::Chain { row, at } => {
        self.open_chain(row, at);
        None
      }
      Click::Unqueue(entry) => {
        if let Some(text) = self.client.unqueue(&entry) {
          self.client.restore = Some(self.before_draft(text));
        }
        None
      }
      Click::Press { viewer, node } => {
        self.client.gesture(ViewEvent { viewer, node, kind: Gesture::Press, value: None });
        None
      }
      Click::Fill { viewer, node, value } => {
        self.client.fill(Some(Filled { viewer, node }), value);
        None
      }
      Click::Link(address) => {
        self.client.notice = Some(match (self.browse)(&address) {
          Ok(()) => format!("Opened {address} in the browser."),
          Err(no) => format!("The browser did not open {address}: {no}"),
        });
        None
      }
    }
  }

  /// The rows of the left side and the cell of each loader. The text of a thing under the pointer
  /// that takes a click goes bright, and a row that shows more under the pointer shows it. A line
  /// under the pointer that awaits agents lights the row of each, so the eye goes from the await to
  /// what it waits for, and a name under the pointer shows what it holds on the head of its block,
  /// which covers no code.
  pub(crate) fn left(&self) -> Left {
    let mut rows = self.left_rows();
    let named = self.pointed().and_then(|at| Self::named(&rows, at));
    if let Some((_, y)) = self.pointed()
      && let Some(name) = &named
      && let Some(head) =
        rows[..usize::from(y)].iter_mut().rev().find(|one| one.inspector.is_some())
      && let Some(feed) = self.feed()
    {
      head.line = feed.inspector(name, head.inspector.take().unwrap_or_default());
    }
    if let Some((x, y)) = self.pointed()
      && let Some(row) = rows.get_mut(usize::from(y))
    {
      let awaits = mem::take(&mut row.awaits);
      let fold = row.click(usize::from(x)).filter(|click| matches!(click, Click::Fold(_)));
      *row = mem::take(row).lit(usize::from(x), self.tokens.hover);
      // A name that takes a click stands as a link while the pointer is on it, since the light of
      // the text of the code is near the white of the pointer.
      let link = row.hot.iter().find(|hot| hot.0.contains(&usize::from(x)) && hot.1.is_some());
      if let Some(range) = link.map(|hot| hot.0.clone()).filter(|_| named.is_some()) {
        row.line =
          styled(mem::take(&mut row.line), &range, |one| one.fg(self.tokens.hover).underline());
      }
      let awaited = |row: &Row| {
        let agent = row.hot.iter().find_map(|hot| match &hot.1 {
          Some(Click::Agent(prompt)) => Some((hot.0.start, prompt)),
          _ => None,
        });
        agent.filter(|(_, prompt)| awaits.contains(prompt)).map(|(x, _)| x)
      };
      // A fold that stands on many rows lights whole, as one thing that takes a click.
      let folded = |row: &Row| {
        let hot = row.hot.iter().find(|hot| fold.is_some() && hot.1 == fold);
        hot.map(|hot| hot.0.start)
      };
      for (at, row) in rows.iter_mut().enumerate() {
        if let Some(x) = awaited(row).or_else(|| folded(row).filter(|_| at != usize::from(y))) {
          *row = mem::take(row).lit(x, self.tokens.hover);
        }
      }
    }
    let loaders = rows.iter().enumerate().flat_map(|(y, row)| {
      row.loaders.iter().filter_map(move |&(x, loader)| {
        let style = spot(&row.line, x)?;
        Some(Spot { x: u16::try_from(x).ok()?, y: u16::try_from(y).ok()?, loader, style })
      })
    });
    let loaders = loaders.collect();
    let pictures = rows
      .iter()
      .enumerate()
      .filter_map(|(y, row)| Some(Boxed { y: u16::try_from(y).ok()?, ..row.picture.clone()? }));
    let pictures = pictures.collect();
    Left { rows: rows.into_iter().map(|row| row.line).collect(), loaders, pictures }
  }

  /// The name of the code at a place of the left side, among its rows.
  fn named(rows: &[Row], (x, y): (u16, u16)) -> Option<String> {
    let row = rows.get(usize::from(y))?;
    row.names.iter().find(|one| one.0.contains(&usize::from(x))).map(|one| one.1.clone())
  }

  /// The name under the pointer, which the machine of the open thread reads, or none.
  pub(crate) fn pointed_name(&mut self) {
    let rows = self.left_rows();
    let name = self.pointed().and_then(|at| Self::named(&rows, at));
    let key = self.client.thread.as_ref().map(|one| one.thread.key.clone());
    let pointed = key.as_ref().zip(name.as_deref());
    self.client.inspect(pointed);
  }

  /// Each row of the left side: the home, or the head of the open thread, the feed in its room and
  /// the dock; then the input.
  pub(crate) fn left_rows(&self) -> Vec<Row> {
    // The bar of the time says what the pointer is on, so its rows show the place of the pointer.
    let bar = self.client.thread.as_ref().is_some_and(|one| one.thread.time.is_some());
    let stamp = (self.changes, self.clock(), self.pointed().filter(|_| bar));
    if let Some((at, rows)) = &*self.laid.borrow()
      && *at == stamp
    {
      return rows.clone();
    }
    let rows = self.laid_rows();
    #[cfg(test)]
    self.layouts.set(self.layouts.get() + 1);
    self.laid.replace(Some((stamp, rows.clone())));
    rows
  }

  /// The clock that the rows of the left side show: the second of each timer, or the moment
  /// itself while they change with each moment, as [`Model::steps`] says.
  fn clock(&self) -> Duration {
    if self.steps() { self.client.moved } else { Duration::from_secs(self.second()) }
  }

  /// Each row of the left side, laid out again.
  fn laid_rows(&self) -> Vec<Row> {
    let mut rows = match self.feed() {
      None => self.home(),
      Some(feed) => {
        let room = self.feed_room();
        let shown = feed.items().shown(feed.opened.top, room, |item| feed.rows(item));
        let mut rows = vec![feed.head()];
        rows.extend(self.open_rows());
        rows.extend(self.time_rows());
        let above = rows.len();
        rows.extend(shown.rows);
        rows.resize(above + room, Row::default());
        if !shown.end
          && let Some(last) = rows.last_mut()
        {
          *last = feed.jump();
        }
        // An image draws only where each row of its box shows, as it cannot draw a part of itself.
        let bottom = above + room - usize::from(!shown.end);
        for (y, row) in rows.iter_mut().enumerate().skip(above) {
          row.picture.take_if(|one| y + usize::from(one.high) > bottom);
        }
        rows.extend(feed.dock());
        rows.extend(self.kit_rows());
        rows
      }
    };
    rows.resize(usize::from(self.size.1).saturating_sub(self.input_rows()), Row::default());
    rows.extend(self.input());
    rows
  }

  /// How many rows the feed takes: the window less the head, the rows of an open, the dock and the
  /// input.
  pub(crate) fn feed_room(&self) -> usize {
    let docked = self.feed().map_or(0, |feed| feed.dock().len() + self.kit_rows().len());
    let taken = 1 + self.open_rows().len() + self.time_rows().len() + docked + self.input_rows();
    usize::from(self.size.1).saturating_sub(taken)
  }

  /// The card of the thread that the window started, while its machine told none: the daemon
  /// tells it only once the life stands and made its prompt, so until then the window shows the
  /// thread as it knows it from his first message, at work, with its machine.
  pub(crate) fn untold(&self) -> Option<(usize, Card)> {
    let key = self.client.thread.as_ref().filter(|one| one.thread.started)?.thread.key.clone();
    if self.client.card(&key).is_some() {
      return None;
    }
    let told = &self.client.machines[key.0];
    let session =
      told.cards.iter().find(|card| card.life == key.1).and_then(|card| card.session.clone());
    let thread = view::Card {
      prompt: key.2.clone(),
      title: self.title()?,
      standing: Standing::Thinks,
      model: None,
      agents: 0,
      readers: vec![format!("{OPERATOR}/{}", told.login)],
    };
    let (life, project, since) = (key.1.clone(), self.client.project(key.0, &key.1), self.second());
    let card = Card {
      life,
      session,
      project,
      refused: None,
      held: true,
      viewers: Vec::new(),
      pins: Vec::new(),
      since,
      thread,
    };
    Some((key.0, card))
  }

  /// The hue of a thread, from its record and its title, which the window knows from the send of
  /// its first message, so the hue stays when the daemon names the prompt of a new thread.
  pub(crate) fn hue(&self, key: &Key) -> Color {
    let open = || {
      self.client.thread.as_ref().filter(|one| one.thread.key == *key).and_then(|_| self.title())
    };
    let title = self.client.card(key).map(|card| card.thread.title.clone()).or_else(open);
    self.tokens.hues[hued(&key.1, &title.unwrap_or_default()) % self.tokens.hues.len()]
  }
}

/// The number of a thread that picks its hue, from its record and its title.
pub(crate) fn hued(life: &str, title: &str) -> usize {
  hash(&format!("{life}{title}"))
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
