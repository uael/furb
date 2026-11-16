//! What the window knows: the cards that each machine told, and where the hand of the operator
//! is.
//!
//! A card shows its state at once. Its place is the one of the last time that the list settled,
//! which the window does only while the hand is away from the list, so no card moves under a
//! click. The rules of a card, its part and its place among the others, come from the daemon and
//! the crate: the window only draws them.

#[cfg(test)]
use std::cell::Cell;
use std::{
  cell::RefCell,
  collections::HashMap,
  io,
  path::{Path, PathBuf},
  rc::Rc,
  time::Duration,
};

use furb::view::{Part, age, homed, since};
use furb_client::{Ask, Asked, Client, ENDS, HOLD, Hold, Listing, Step, Time};
pub(crate) use furb_client::{Key, Link, Told};
use furb_host::{
  Event, ProtocolError,
  protocol::{Answer, Card, Frame},
};
use jiff::{Timestamp, tz::TimeZone};
use opentui::native::CapabilityFlags;

use crate::{
  Options,
  code::Colors,
  feed::Read,
  input::{Armed, Word},
  kept,
  keys::{self, Does, System},
  link::Heard,
  list::Row,
  names::Names,
  pick::Picker,
  thread,
  tokens::{BESIDE, CALL, LIST, LISTS, MEASURE, Tokens},
};

/// The rows at the top of the list that never scroll.
const PINNED: usize = 2;

/// The rows of the left side, with the count of changes, the clock and the place of the pointer
/// that they show.
pub(crate) type Laid = ((u64, Duration, Option<(u16, u16)>), Vec<thread::Row>);

/// The rows of each part of the feed, with the count of changes and the clock that they show.
pub(crate) type Parts = (Option<(u64, Duration)>, HashMap<thread::Item, Vec<thread::Row>>);

pub(crate) struct Model {
  /// What each machine told, and the cards of all of them in the order of the list.
  pub(crate) client: Client,
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
  pub(crate) size: (u16, u16),
  /// The home folder of the operator, which a path shows as `~`.
  pub(crate) home: Option<PathBuf>,
  pub(crate) zone: TimeZone,
  /// The system of the client, which says how a text writes a key.
  pub(crate) system: System,
  /// What the terminal told that it speaks, as the keyboard of Kitty, which tells the keys that
  /// hold super, so a key with command reaches the window.
  pub(crate) speaks: CapabilityFlags,
  /// The opener of an address in the browser of the client.
  pub(crate) browse: fn(&str) -> io::Result<()>,
  /// The editor of the operator, which writes a file and ends.
  pub(crate) edit: fn(&Path) -> io::Result<()>,
  /// How long a form takes no key after it shows: [`HOLD`], which keeps a hand from answering what
  /// it did not read, and none while the clocks stand still, as in a capture, where a script
  /// presses each key and the clock that the hold reads never moves.
  pub(crate) hold: Duration,
  /// How long the number of a line that ended settles: [`ENDS`], and none while the clocks stand
  /// still, so a capture shows each line as it rests.
  pub(crate) settles: Duration,
  pub(crate) tokens: Tokens,
  /// Why the window could not keep its cards.
  pub(crate) fault: Option<String>,
  /// The last levels of the microphone while his voice records, from 0 to 100, the newest last.
  pub(crate) listening: Option<Vec<u8>>,
  /// The colors of each text of python that the window drew.
  pub(crate) colors: Colors,
  /// The names of each text of python that the window read.
  pub(crate) names: Names,
  /// The code of each block that the window read, by the message of its answer.
  pub(crate) read: RefCell<HashMap<String, Rc<Read>>>,
  /// How many changes of what the left side shows the model heard: each change but a step of the
  /// clocks and a move of the pointer, which change no row of it but through the clock.
  pub(crate) changes: u64,
  /// The rows of the left side, with what they show: a layout costs the most of what the window
  /// does, and a step of the loaders or a move of the pointer takes them again.
  pub(crate) laid: RefCell<Option<Laid>>,
  /// How many changes of what the parts of the feed show the model heard: each change of the left
  /// side but one that only scrolls the feed, which moves it and changes no part of it.
  pub(crate) shows: u64,
  /// Whether the change that the model hears only scrolled the feed.
  pub(crate) scrolled: bool,
  /// The rows of each part of the feed that the window laid out, with what they show, so a scroll
  /// lays out only the parts that come into sight.
  pub(crate) parts: RefCell<Parts>,
  /// The count of changes for which the window last told the images and the whole outputs of the
  /// open thread.
  pub(crate) pictured: u64,
  /// How many times the left side was laid out, which a test of speed counts.
  #[cfg(test)]
  pub(crate) layouts: Cell<u64>,
  /// What a first press of a key named, which a second press does.
  pub(crate) armed: Option<Armed>,
  /// The list part, while it is open.
  pub(crate) picker: Option<Picker>,
  /// The word of python that the window ran last, until the next one goes or an escape.
  pub(crate) word: Option<Word>,
  /// What the choices of sessions act on, while the list part shows them or a leave waits.
  pub(crate) acts: Option<Acts>,
}

/// What the choices of sessions act on: one session, by its machine and its record, or the leave
/// of the window, which leaves once each choice went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Acts {
  On(usize, String),
  Leaving,
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
      client: {
        let mut client = Client::new(machines, options.folder.display().to_string());
        client.system = furb::SYSTEM;
        client.stash = kept::kept(&kept::stash_of(&options.kept));
        client
      },
      open: false,
      toggled: false,
      slow: false,
      pointer: None,
      scroll: 0,
      size,
      home: options.home.clone(),
      zone: options.zone.clone(),
      system: options.system,
      speaks: CapabilityFlags::default(),
      browse: options.browse,
      edit: options.edit,
      hold: if options.still { Duration::ZERO } else { HOLD },
      settles: if options.still { Duration::ZERO } else { ENDS },
      tokens,
      fault: None,
      listening: None,
      colors: Colors::new(),
      names: Names::default(),
      read: RefCell::default(),
      changes: 0,
      laid: RefCell::default(),
      shows: 0,
      scrolled: false,
      parts: RefCell::default(),
      pictured: 0,
      #[cfg(test)]
      layouts: Cell::default(),
      armed: None,
      picker: None,
      word: None,
      acts: None,
    }
  }

  /// One frame of the daemon of a machine, or the end of its connection: whether what the machine
  /// told changed. The first cause of a loss stays.
  pub(crate) fn hear(&mut self, at: usize, heard: Heard) -> bool {
    // A daemon that stops refuses each request that waits there, then its stream ends: the window
    // reads the two as one loss of the machine, which gives each message on its way back once.
    if let Ok(Frame::Refused { error: ProtocolError::Stopped, .. }) = &heard {
      return false;
    }
    if let Ok(Frame::Reply { id, .. } | Frame::Refused { id: Some(id), .. }) = &heard
      && let Some(asked) = self.client.asked.remove(&(at, *id))
    {
      self.answered_input(at, (*id, asked), heard);
      return true;
    }
    match heard {
      Ok(Frame::Reply { id, answer: Answer::Page(page) }) => {
        return self.answered(at, id, Ok(page));
      }
      Ok(Frame::Refused { id: Some(id), why, .. }) if self.waits(at, id) => {
        return self.answered(at, id, Err(why));
      }
      Ok(Frame::Thread { life, thread, page }) => return self.changed(at, (&life, &thread), page),
      Ok(Frame::Opening { life, stage, done, of }) => {
        return self.client.opening(at, life, Step { stage, done, of, at: self.client.moved });
      }
      Ok(Frame::SignIn { state, url, code, .. }) => {
        return self.told_sign_in(at, state, (url, code));
      }
      Ok(Frame::Cards { ref life, ref changed, .. }) if changed.iter().any(|card| !card.held) => {
        self.client.let_go(at, life);
      }
      Ok(Frame::Event { life, event: Event::Ended(Some(fault)), .. }) => {
        return self.failed(at, &life, fault);
      }
      _ => {}
    }
    match self.client.listed(at, heard.map_err(|fault| fault.to_string()), self.second()) {
      Listing::Changed => true,
      Listing::Same => false,
      Listing::Lost(_) => self.lost(at),
    }
  }

  /// The connection to a machine ended, while the window dials it again: each request
  /// that waits for its answer there goes nowhere. Each message on its way there comes back to the
  /// input with each message after it, since the window cannot know whether the daemon took it,
  /// and each life of the machine opens again, with the page of the open thread, once it answers.
  fn lost(&mut self, at: usize) -> bool {
    let name = self.client.machines[at].name.clone();
    let gone: Vec<Asked> =
      self.client.asked.extract_if(|key, _| key.0 == at).map(|one| one.1).collect();
    // A take back that the machine may not have done gives its text back all the same: the input
    // never loses a word, and the thread shows whether the message still runs.
    for one in &gone {
      if let Asked::Back(text) = one {
        self.client.restore = Some(self.before_draft(text.clone()));
      }
    }
    if gone.contains(&Asked::SignIn) {
      self.client.signed_in(None);
    }
    if gone.contains(&Asked::Answer) {
      self.unanswered(format!("{name} went away before it took the answer. Send it again."));
    }
    if self.client.outgoing.iter().any(|one| one.at == at) {
      let why =
        format!("{name} went away before it took your message. If the thread shows it, it went.");
      self.client.restore = Some(self.refused(why));
    }
    for (_, hold) in self.client.holds.iter_mut().filter(|one| one.0.0 == at) {
      hold.ask = Ask::Due;
    }
    if let Some(opened) = self.client.thread.as_mut().filter(|one| one.thread.key.0 == at) {
      opened.thread.asked.clear();
      opened.thread.again = true;
    }
    // The end of the word comes from the record of its life, so the window asks it again.
    if let Some(word) = self.word.as_mut().filter(|one| one.at == at) {
      word.asked = false;
    }
    true
  }

  /// A state of the sign-in of Codex that a machine told: the page of the sign-in opens in the
  /// browser once, when the machine gives its address.
  fn told_sign_in(
    &mut self,
    at: usize,
    state: String,
    told: (Option<String>, Option<String>),
  ) -> bool {
    if let Some(address) = self.client.told_sign_in(at, state, told)
      && let Err(no) = (self.browse)(&address)
    {
      self.client.notice = Some(format!("The browser did not open the sign-in of Codex: {no}"));
    }
    true
  }

  /// The answer of a daemon to a request of the input, with its id and what it asked.
  fn answered_input(&mut self, at: usize, (id, asked): (u64, Asked), heard: Heard) {
    if asked.read_here() {
      return self.client.answered(at, asked, heard.ok());
    }
    match (asked, heard) {
      (Asked::Open, Ok(Frame::Reply { .. })) => self.client.answered_open(at, id),
      (Asked::Open, Ok(Frame::Refused { why, .. })) => {
        self.client.restore = self.unopened(at, id, why);
      }
      (Asked::Say, Ok(Frame::Reply { answer, .. })) => self.replied(answer),
      (Asked::Parts(key, read), heard) => self.client.parted(&key, read, replied(heard)),
      (asked @ (Asked::Settings | Asked::Setting | Asked::Plays | Asked::SignIn), heard) => {
        self.client.configured(&asked, heard.ok());
      }
      (Asked::Back(text), Ok(Frame::Reply { .. })) => {
        self.client.restore = Some(self.before_draft(text));
      }
      (Asked::Answer, Ok(Frame::Refused { why, .. })) => self.unanswered(why),
      (Asked::Ended(life, rung), heard) => self.ended(at, (&life, &rung), heard),
      (
        Asked::Seen
        | Asked::List
        | Asked::Projects
        | Asked::Control
        | Asked::Back(_)
        | Asked::History(_)
        | Asked::Skills(_)
        | Asked::Files(_)
        | Asked::Changes(..)
        | Asked::View,
        Ok(Frame::Refused { why, .. }),
      ) => self.fault = Some(why),
      // A choice for a session that its machine refused keeps the window, which says why.
      (Asked::Running, Ok(Frame::Refused { why, .. })) => {
        (self.acts, self.fault) = (None, Some(why));
      }
      (Asked::List, Ok(Frame::Reply { answer: Answer::Lives(lives), .. })) => {
        self.client.machines[at].lives = lives;
      }
      (Asked::Projects, Ok(Frame::Reply { answer: Answer::Projects(projects), .. })) => {
        self.client.machines[at].projects = projects;
      }
      (Asked::Models, Ok(Frame::Reply { answer: Answer::Models(models), .. })) => {
        self.client.machines[at].models = Some(models);
      }
      // A catalog that gave no models leaves the launcher with none, and says why once.
      (Asked::Models, Ok(Frame::Refused { why, .. })) => {
        (self.client.machines[at].models, self.fault) = (Some(Vec::new()), Some(why));
      }
      (Asked::History(target), Ok(Frame::Reply { answer: Answer::History(sent), .. })) => {
        self.client.historied(&target, sent);
      }
      (
        asked @ (Asked::Skills(_) | Asked::Files(_) | Asked::Changes(..)),
        Ok(Frame::Reply { answer, .. }),
      ) => {
        self.client.listed_for(at, asked, answer);
      }
      // Where he writes is a hint to the other windows: a reader, who writes nothing to the life,
      // is refused it, and nothing changes.
      (Asked::Answer | Asked::Writes, _) => {}
      (_, Ok(Frame::Refused { why, .. })) => self.client.restore = Some(self.refused(why)),
      _ => {}
    }
  }

  /// The list settled: each card of each machine at its place.
  pub(crate) fn settle(&mut self) {
    // Once its machine told the card of a thread that the window started, the machine says how it
    // stands, also when the card goes.
    let told =
      self.client.thread.as_ref().is_some_and(|one| self.client.card(&one.thread.key).is_some());
    if let Some(opened) = self.client.thread.as_mut().filter(|_| told) {
      opened.thread.started = false;
    }
    let untold = self.untold();
    self.client.settle(untold.as_ref());
    // The thread that the window opened stays selected while its card is not told yet.
    let open = self.client.thread.as_ref().map(|one| &one.thread.key);
    let stands =
      |key: &Key| open == Some(key) || self.client.settled.iter().any(|one| one.1 == *key);
    self.client.selected = self.client.selected.take().filter(stands);
    self.scrolled(0);
  }

  /// The card that stands a count of cards after the selected one, selected. The resolved threads
  /// take part while they show, and a step down from the last card above them shows them.
  pub(crate) fn step(&mut self, by: isize) {
    // A step past the last card above the folded resolved threads shows them, as a click on their
    // head does, so the keys reach each row that the pointer reaches.
    let above = self.client.settled.iter().filter(|one| one.0 != Part::Resolved).count();
    let at = self
      .client
      .selected
      .as_ref()
      .and_then(|key| self.client.settled.iter().position(|one| one.1 == *key));
    let last = at.map_or(above == 0, |at| at + 1 == above);
    if by > 0 && !self.open && last && self.client.settled.len() > above {
      self.open = true;
    }
    let shown = |(part, ..): &&(Part, Key, Card)| self.open || *part != Part::Resolved;
    let keys: Vec<&Key> = self.client.settled.iter().filter(shown).map(|one| &one.1).collect();
    let at = self.client.selected.as_ref().and_then(|key| keys.iter().position(|one| *one == key));
    let next = match at {
      Some(at) => at.saturating_add_signed(by).min(keys.len().saturating_sub(1)),
      None if by > 0 => 0,
      None => keys.len().saturating_sub(1),
    };
    self.client.selected = keys.get(next).map(|key| (*key).clone());
    self.reveal();
  }

  /// The next card that needs the operator after the selected one, selected, from the first one
  /// again after the last.
  pub(crate) fn needed(&mut self) {
    let needs: Vec<&Key> = self
      .client
      .settled
      .iter()
      .filter(|one| self.needs(&one.1, &one.2))
      .map(|one| &one.1)
      .collect();
    let at = self.client.selected.as_ref().and_then(|key| needs.iter().position(|one| *one == key));
    let next = at.map_or(0, |at| (at + 1) % needs.len().max(1));
    if let Some(key) = needs.get(next) {
      self.client.selected = Some((*key).clone());
      self.reveal();
    }
  }

  /// A click on a row of the window, in the list: a card is selected and its thread opens, the head
  /// of the resolved threads shows or folds their rows, and the scope and a key do what their key
  /// does, which this gives.
  pub(crate) fn clicked(&mut self, y: usize) -> Option<Does> {
    match self.row(y) {
      Some(Row::Scope) => return Some(Does::GoTo),
      Some(Row::Key(does, _)) => return Some(does),
      Some(Row::Card(at, _) | Row::Resolved(at)) => {
        self.client.selected = self.client.settled.get(at).map(|one| one.1.clone());
        self.open_thread();
        // A click on a thread shows its end, also when it is open already.
        self.jump();
      }
      // The bar of an open box shows its other split, and each other row folds it.
      Some(Row::Context(at)) => self.context_clicked(at),
      Some(Row::Changes(at)) => return self.changes_clicked(at),
      Some(Row::Head(Part::Resolved)) => {
        self.open = !self.open;
        // The resolved threads end the list, so the list moves to its end to show them.
        self.scrolled(isize::MAX);
      }
      _ => {}
    }
    None
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

  /// Whether the terminal tells the keys that hold super.
  pub(crate) fn supers(&self) -> bool {
    self.speaks.contains(CapabilityFlags::KITTY_KEYBOARD)
  }

  /// The key that does a thing, as the system of the client writes it.
  pub(crate) fn key(&self, does: Does) -> String {
    keys::drawn(does, self.system)
  }

  /// Whether a column of the window stands on the list.
  pub(crate) fn on_list(&self, x: u16) -> bool {
    self.listed() && x >= self.size.0.saturating_sub(self.list_width())
  }

  /// The columns of the list of cards: its least width, and half of the columns that a wider
  /// window leaves past the measure of the left side, up to its most, so a long title stands whole
  /// where the window has the room, and the left side takes the rest.
  pub(crate) fn list_width(&self) -> u16 {
    let spare = self.size.0.saturating_sub(LIST + 1 + MEASURE) / 2;
    LIST + spare.min(LISTS - LIST)
  }

  /// The columns of the list and of the column of its panel at its left.
  pub(crate) fn panel_width(&self) -> u16 {
    self.list_width() + 1
  }

  /// How many columns the left side draws: those that the list leaves it. It draws up to two
  /// columns before its last, apart from the list.
  pub(crate) fn wide(&self) -> usize {
    usize::from(self.side())
  }

  /// The pointer on the left side, at a column of its rows.
  pub(crate) fn pointed(&self) -> Option<(u16, u16)> {
    self.pointer.filter(|(x, _)| !self.on_list(*x))
  }

  /// The columns that the list leaves to the left side while the list shows.
  fn side(&self) -> u16 {
    self.size.0.saturating_sub(if self.listed() { self.panel_width() } else { 0 })
  }

  /// Whether each machine answered or was lost, so the window knows what stands.
  pub(crate) fn known(&self) -> bool {
    self.client.machines.iter().all(|one| one.link != Link::Dialing)
  }

  /// Whether a mark of the window moves, or an open runs, whose time counts.
  pub(crate) fn moves(&self) -> bool {
    let works = |one: &(Part, Key, Card)| {
      one.2.thread.works()
        && self.confirmed(&one.1)
        && self.client.fault_of(one.1.0, &one.1.1).is_none()
    };
    let pulses =
      self.client.called.values().any(|since| self.client.moved.saturating_sub(*since) < CALL);
    let views = || self.kit_rows().iter().any(|row| !row.loaders.is_empty());
    pulses || self.steps() || self.client.settled.iter().any(works) || views() || self.checks_wait()
  }

  /// Whether the rows of the left side change with each moment, past the cells of its loaders: a
  /// machine is slow to answer, a life opens, the bar of the time plays or reads, or a line that
  /// ended settles into its grey.
  pub(crate) fn steps(&self) -> bool {
    let opens = self.client.holds.values().any(Hold::runs);
    let thread = self.client.thread.as_ref().map(|one| &one.thread);
    let reads = thread.and_then(|one| one.time.as_ref()).is_some_and(Time::reads);
    let settles = thread.is_some_and(|one| {
      one.ended.values().any(|at| self.client.moved.saturating_sub(*at) < self.settles)
    });
    self.slow && !self.known() || opens || self.client.plays() || reads || settles
  }

  pub(crate) fn needs(&self, key: &Key, card: &Card) -> bool {
    card.needs(&self.client.machines[key.0].login)
  }

  /// Whether the machine of a card told the card in this connection.
  pub(crate) fn confirmed(&self, key: &Key) -> bool {
    self.client.machines[key.0].link == Link::Listed
  }

  /// A text with the home folder of the operator as `~`.
  pub(crate) fn homed(&self, text: &str) -> String {
    self
      .home
      .as_ref()
      .map_or_else(|| text.to_owned(), |home| homed(text, &home.display().to_string()))
  }

  /// The hour and the minute of a second, in the zone of the window.
  pub(crate) fn hour(&self, second: u64) -> String {
    let at = Timestamp::from_second(i64::try_from(second).unwrap_or_default()).unwrap_or_default();
    at.to_zoned(self.zone.clone()).strftime("%H:%M").to_string()
  }

  /// The second of the clock of the machine.
  pub(crate) fn second(&self) -> u64 {
    self.client.now / 1000
  }

  /// How many whole seconds passed since a millisecond of the epoch, from one second on: a time of
  /// a work shows only after a whole second of it.
  pub(crate) fn since(&self, began: u64) -> Option<u64> {
    since(began, self.client.now)
  }

  /// How long ago a second was, from one minute on.
  pub(crate) fn age(&self, since: u64) -> Option<String> {
    age(since, self.second())
  }

  /// The rows of the list that scroll, and how many of them show.
  fn room(&self) -> usize {
    usize::from(self.size.1).saturating_sub(PINNED + self.boxed())
  }

  /// The rows at the foot of the list that the part of the changes and the box of the context take,
  /// with the blank row above them, as the input has one above it.
  fn boxed(&self) -> usize {
    let rows = self.changes_rows().len() + self.context_rows().len();
    if rows == 0 { 0 } else { rows + 1 }
  }

  /// The row of the list at a row of the window: a row of the box of the context at its foot.
  pub(crate) fn row(&self, y: usize) -> Option<Row> {
    let foot = usize::from(self.size.1).saturating_sub(self.boxed());
    if y >= foot {
      let (at, changes) = ((y - foot).saturating_sub(1), self.changes_rows().len());
      return Some(match () {
        () if y == foot => Row::Blank,
        () if at < changes => Row::Changes(at),
        () => Row::Context(at - changes),
      });
    }
    let rows = self.rows();
    rows.get(if y < PINNED { y } else { y + self.scroll }).copied()
  }

  /// The rows of the list that show, from the top of the window, and the box of the context at
  /// its foot.
  pub(crate) fn shown(&self) -> impl Iterator<Item = Row> {
    let rows = self.rows();
    let pinned = rows.len().min(PINNED);
    let mut scrolled: Vec<Row> =
      rows[pinned..].iter().skip(self.scroll).take(self.room()).copied().collect();
    let boxed = self.boxed();
    if boxed > 0 {
      scrolled.resize(self.room(), Row::Blank);
      scrolled.push(Row::Blank);
      let changes = self.changes_rows().len();
      scrolled.extend((0..changes).map(Row::Changes));
      scrolled.extend((0..boxed - 1 - changes).map(Row::Context));
    }
    rows.into_iter().take(pinned).chain(scrolled)
  }

  /// The list moved as far as the selected card needs to show, with the row above it and the row
  /// under it.
  fn reveal(&mut self) {
    let Some(key) = &self.client.selected else { return };
    let card = |row: &Row| match row {
      Row::Card(at, _) | Row::Resolved(at) => self.client.settled[*at].1 == *key,
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

/// The answer of a reply, none for a refusal or a lost machine.
fn replied(heard: Heard) -> Option<Answer> {
  match heard {
    Ok(Frame::Reply { answer, .. }) => Some(answer),
    _ => None,
  }
}
