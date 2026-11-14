//! The input: where it sends, what waits to be sent, and its rows, with the line of the launcher
//! under it.
//!
//! The input sends to its target: the open thread, a new thread of a session, or a new session in
//! the project of the folder of the window. Each message waits in one queue, and only one request
//! of the queue is asked at a time, so each message goes after the one before it, and a message
//! that goes to the thread that the message before it made knows that thread when it goes. His
//! message shows in the thread at once, and a message that the daemon refuses comes back to the
//! input with the message after it.

use std::mem;

use furb::view::Maker;
use furb_host::protocol::{Answer, Verb};
use jiff::Timestamp;
use opentui::style::{Color, Span, Style};
use unicode_width::UnicodeWidthStr;

use crate::{
  keys::Does,
  line::{Line, boxed, run, spread},
  list::project,
  model::{Key, Model},
  opens::{Ask, Hold, Life},
  thread::{Click, Opened, Row, columns, width},
  tokens::{Loader, Role},
};

/// How many rows of text the input grows to before its text scrolls.
const GROWS: usize = 6;

/// The fewest columns of the session in the launcher before the model gives way to it.
const SESSION: usize = 16;

/// The place in the log of a life from which the window hears its events: past each event.
const NO_EVENT: u64 = u64::MAX;

/// Where the input sends: the open thread, a new thread of a session on a machine, or a new
/// session on a machine in the project of the folder of the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Target {
  Thread(Key),
  Session(usize, String),
  New(usize),
}

/// A message that waits to be sent: its machine, its record, its thread, which is none for a new
/// thread and empty for the thread that the message before it makes, its text, and whether it is
/// a word of python that runs. It goes once the daemon answered the open of its life.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Outgoing {
  pub(crate) at: usize,
  pub(crate) life: String,
  pub(crate) thread: Option<String>,
  pub(crate) text: String,
  pub(crate) python: bool,
}

/// The text of the input, as its editor holds it, and whether it is a word of python that runs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Draft {
  pub(crate) text: String,
  pub(crate) python: bool,
}

/// What a request of the window asked of a daemon: the open of a life, a message, a stop, or the
/// answer to a question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Asked {
  Open,
  Say,
  Stop,
  Answer,
}

/// A message of the operator that the thread shows before the daemon tells it: its text, and the
/// act that it made once the daemon said it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Pending {
  pub(crate) act: Option<String>,
  pub(crate) text: String,
}

/// The list of the sessions that the input can send to, while it is open: the text that filters
/// it, and the place of the chosen entry among those that pass the filter.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Picker {
  pub(crate) filter: String,
  pub(crate) at: usize,
}

/// The box of the text of the input, which the editor of the window covers: its column, its row,
/// its columns and its rows, the text that it shows while it is empty, and whether it shows: a
/// form shows it only on the row of the words of the operator, or for a line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Area {
  pub(crate) x: u16,
  pub(crate) y: u16,
  pub(crate) width: u16,
  pub(crate) height: u16,
  pub(crate) placeholder: String,
  pub(crate) shown: bool,
}

impl Model {
  /// Where the input sends now: where the operator turned it, else the open thread, else a new
  /// session on this machine.
  pub(crate) fn target(&self) -> Target {
    match (&self.aim, &self.thread) {
      (Some(target), _) => target.clone(),
      (None, Some(opened)) => Target::Thread(opened.key.clone()),
      (None, None) => Target::New(0),
    }
  }

  /// The input turned to a new thread of the session of the open thread, and back.
  pub(crate) fn turn(&mut self) {
    self.aim = match (&self.aim, &self.thread) {
      (None, Some(opened)) => Some(Target::Session(opened.key.0, opened.key.1.clone())),
      _ => None,
    };
  }

  /// The text of the input sent to its target: it waits in the queue, and the thread that it goes
  /// to shows it at once. A new thread or a new session opens in the window, so the next message
  /// goes to it.
  pub(crate) fn send(&mut self, text: String) {
    self.fault = None;
    let python = self.draft.python;
    let (at, life, thread) = match self.target() {
      Target::Thread(key) => (key.0, key.1, Some(key.2)),
      Target::Session(at, life) => (at, life, None),
      Target::New(at) => (at, self.record(), None),
    };
    self.hold(at, &life, self.cwd(at, &life));
    let made = thread.is_none() && !python;
    if made {
      let key = (at, life.clone(), String::new());
      self.thread = Some(Opened { key, above: Some(0), ..Opened::default() });
      self.aim = None;
    }
    if !python && let Some(opened) = self.thread.as_mut() {
      opened.pending.push(Pending { act: None, text: text.clone() });
    }
    self.outgoing.push_back(Outgoing { at, life, thread, text, python });
  }

  /// The folder that the life of a record opens in when no life holds the record: the folder of
  /// the window for a new record, else the project of its cards.
  pub(crate) fn cwd(&self, at: usize, life: &str) -> Option<String> {
    let cards = self.machines.get(at).map(|told| &told.cards[..]).unwrap_or_default();
    match cards.iter().find(|card| card.life == life) {
      None => Some(self.folder.display().to_string()),
      Some(card) => card.project.clone(),
    }
  }

  /// The record of a new session, in the sessions of the project of the folder of the window,
  /// named by the second and the moment of the window, so no two sessions share it.
  fn record(&self) -> String {
    let second = Timestamp::from_second(i64::try_from(self.now).unwrap_or_default());
    let stamp = second.unwrap_or_default().strftime("%Y-%m-%dT%H-%M-%S");
    let name = format!("{stamp}-{:04x}.jsonl", self.moved.as_nanos() % 0x1_0000);
    self.folder.join(".furb").join("sessions").join(name).display().to_string()
  }

  /// The first message of the queue of the input, once no message waits for its answer and the
  /// daemon answered the open of its life. The daemon holds it until that life stands.
  pub(crate) fn due(&self) -> Option<(usize, Verb)> {
    if self.asked.values().any(|asked| *asked == Asked::Say) {
      return None;
    }
    let next = self.outgoing.front().filter(|next| self.stands(next.at, &next.life))?;
    let life = next.life.clone();
    let message = next.text.clone();
    let verb = match &next.thread {
      _ if next.python => Verb::Run { life, word: message, on: None },
      None => Verb::Prompt { life, shape: "str".to_owned(), message, to: None, on: None },
      Some(thread) => {
        let card = self.told_card(&(next.at, life.clone(), thread.clone()));
        let shape = card.map_or("None", |card| card.thread.reply()).to_owned();
        Verb::Prompt { life, shape, message, to: None, on: Some(thread.clone()) }
      }
    };
    Some((next.at, verb))
  }

  /// The open of each life that the window holds and did not ask yet, with its machine: the daemon
  /// opens it, in its folder when no life holds its record, and attaches the window past each
  /// event, since the window reads the cards and the pages of the threads, and no event.
  pub(crate) fn opens_due(&self) -> Vec<(Life, Verb)> {
    let due = self.holds.iter().filter(|(_, hold)| hold.ask == Ask::Due);
    let open = |(key, hold): (&Life, &Hold)| {
      let open = Verb::Open {
        life: key.1.clone(),
        cwd: hold.cwd.clone(),
        actor: None,
        roster: None,
        skills: None,
        config: None,
        claude: None,
        answers: false,
        attach: Some(NO_EVENT),
      };
      (key.clone(), open)
    };
    due.map(open).collect()
  }

  /// The stop of the work of the open thread: the cancel of its prompt.
  pub(crate) fn stop(&self) -> Option<(usize, Verb)> {
    let opened = self.thread.as_ref().filter(|one| !one.key.2.is_empty())?;
    let (at, life, act) = opened.key.clone();
    Some((at, Verb::Cancel { life, act, why: None }))
  }

  /// The answer to the first message of the queue, with the act that it made: a new thread opens
  /// in the window under its prompt, and each message that waits for it goes to it.
  pub(crate) fn replied(&mut self, answer: Answer) {
    let Answer::Act(act) = answer else { return };
    let Some(sent) = self.outgoing.pop_front() else { return };
    if sent.python {
      return;
    }
    let thread = sent.thread.clone().filter(|one| !one.is_empty()).unwrap_or_else(|| act.clone());
    for next in self.outgoing.iter_mut().filter(|one| one.life == sent.life) {
      if next.thread.as_deref() == Some("") {
        next.thread = Some(thread.clone());
      }
    }
    self.made(&sent, &thread, act);
  }

  /// The thread that a message went to, with the act of the message: the open thread takes the
  /// prompt of a thread that it made, and the message stays until the thread shows it.
  fn made(&mut self, sent: &Outgoing, thread: &str, act: String) {
    let Some(opened) = self.thread.as_mut() else { return };
    if opened.key.0 != sent.at || opened.key.1 != sent.life {
      return;
    }
    if opened.key.2.is_empty() {
      thread.clone_into(&mut opened.key.2);
      opened.above = None;
      self.selected = Some(opened.key.clone());
    }
    if opened.key.2 == thread
      && let Some(pending) = opened.pending.iter_mut().find(|one| one.act.is_none())
    {
      pending.act = Some(act);
      opened.shown();
    }
  }

  /// A request of the input that the daemon refused, and why: each message that waits comes back
  /// to the input before what the operator wrote since, and the thread shows none of them. The
  /// text that the input holds now.
  pub(crate) fn refused(&mut self, asked: Asked, why: String) -> Option<String> {
    self.fault = Some(why);
    if asked == Asked::Stop {
      return None;
    }
    let back: Vec<String> = mem::take(&mut self.outgoing).into_iter().map(|one| one.text).collect();
    if let Some(opened) = self.thread.as_mut() {
      opened.pending.retain(|one| one.act.is_some());
      if opened.key.2.is_empty() {
        self.thread = None;
      }
    }
    let back = back.into_iter().chain([self.draft.text.clone()]).filter(|one| !one.is_empty());
    let text = back.collect::<Vec<_>>().join("\n\n");
    self.draft.text.clone_from(&text);
    Some(text)
  }

  /// How many rows the text of the input takes, as the editor wraps it at its columns.
  pub(crate) fn grown(&self) -> usize {
    let room = self.wide().saturating_sub(4).max(1);
    let rows = self.draft.text.split('\n').map(|line| line.width().div_ceil(room).max(1)).sum();
    usize::clamp(rows, 1, GROWS)
  }

  /// How many rows the input, the launcher and the hints take at the bottom of the left side.
  pub(crate) fn input_rows(&self) -> usize {
    self.input().len()
  }

  /// The box of the text of the input: over its bands, or over the row of a form where the
  /// operator writes.
  pub(crate) fn area(&self) -> Area {
    let opens = self.thread.as_ref().is_some_and(|one| self.opens(one));
    let top = usize::from(self.size.1).saturating_sub(self.input_rows());
    let (x, row, height, placeholder) = match (self.form(), self.form_rows(self.grown()).1) {
      (Some(_), None) => (2, 0, 1, String::new()),
      (Some((_, question)), Some(at)) if question.offers_line() => {
        let hint = question.hint();
        let says = if hint.is_empty() { "Your answer".to_owned() } else { hint };
        (2, at, self.grown(), format!("Write {says}"))
      }
      (Some(_), Some(at)) => (4, at, 1, "type another answer".to_owned()),
      (None, _) => {
        let says = match self.target() {
          _ if self.draft.python => "A word of Python to run",
          Target::Thread(_) if opens => "Write while it opens. Your message goes when it is open.",
          Target::Thread(_) => "Reply to this thread",
          Target::Session(..) => "Ask in a new thread",
          Target::New(_) => "Ask for a change, a review or an answer",
        };
        let waits = usize::from(self.waiting_row().is_some());
        (2, waits, self.grown(), says.to_owned())
      }
    };
    let shown = self.form().is_none() || self.form_rows(self.grown()).1.is_some();
    let number = |one: usize| u16::try_from(one).unwrap_or_default();
    Area {
      x: number(x),
      y: number(top + row),
      width: number(self.wide().saturating_sub(x + 2)),
      height: number(height),
      placeholder,
      shown,
    }
  }

  /// The rows of the input: the form of a question of the open thread, or a band for each row of
  /// its text, which the editor covers, with a bar in the color of its target, under the row that
  /// says that a question waits; then the launcher and the hints.
  pub(crate) fn input(&self) -> Vec<Row> {
    let t = &self.tokens;
    let bar = match self.target() {
      Target::Thread(key) => self.hue(&key),
      Target::Session(..) | Target::New(_) => t.accent,
    };
    let band = |_| {
      let raised = Style::new().bg(t.raised);
      let fill = run(" ".repeat(self.wide().saturating_sub(1)), raised);
      Row::of(vec![run("┃", raised.fg(bar)), fill])
    };
    let mut rows = match self.form() {
      Some(_) => self.form_rows(self.grown()).0,
      None => self.waiting_row().into_iter().chain((0..self.grown()).map(band)).collect(),
    };
    rows.push(self.launcher());
    rows.push(self.hints());
    rows
  }

  /// The line of the launcher: at its left the machine and the session that the input sends to,
  /// which picks another, and at its right the model with its effort and the switch of python.
  fn launcher(&self) -> Row {
    let t = &self.tokens;
    let (muted, prose) = (t.style(Role::Muted), t.style(Role::Prose));
    let at = match self.target() {
      Target::Thread(key) => key.0,
      Target::Session(at, _) | Target::New(at) => at,
    };
    let machine = if at == 0 { "This computer".to_owned() } else { self.machines[at].name.clone() };
    let session = format!("{} ▾", self.session());
    let left = vec![run(format!("  {machine}   "), muted), run(session, prose)];
    let switch = if self.draft.python {
      run("● Python", Style::new().fg(t.accent))
    } else {
      run("○ Python", prose)
    };
    // The session gives way to the model down to a few words, then the model gives way.
    let room = self.wide().saturating_sub(2);
    let session = width(&left[1..]).min(SESSION);
    let model = self.maker().map(|model| run(format!("{model}   "), muted));
    let fits = |one: &Span<'_>| width(&left[..1]) + session + one.content.width() + 2 + 8 <= room;
    let model = model.filter(fits);
    let right: Line = model.into_iter().chain([switch]).collect();
    let line = spread(left, right, room, Style::new());
    // The runs of the line: the machine, the session, the fill, the model when it shows, and the
    // switch.
    let at = columns(&line);
    let hot = vec![
      (at[1].clone(), Some(Click::Key(Does::Sessions))),
      (at[at.len() - 1].clone(), Some(Click::Key(Does::Python))),
    ];
    Row { hot, ..Row::of(line) }
  }

  /// The project and the session that the input sends to.
  fn session(&self) -> String {
    let (at, life, open) = match self.target() {
      Target::Thread(key) => (key.0, key.1, true),
      Target::Session(at, life) => (at, life, false),
      Target::New(_) => return format!("{} · new session", self.project()),
    };
    let cards = self.machines.get(at).map(|told| &told.cards[..]).unwrap_or_default();
    let card = cards.iter().find(|card| card.life == life);
    let project = card.map_or_else(|| self.project(), project);
    let named = card.and_then(|card| card.session.clone()).filter(|name| !name.is_empty());
    // The head of the open thread names it, so the launcher names its session alone; a new thread
    // names its session by its first thread when the session has no name.
    let first = || card.filter(|_| !open).map(|card| card.thread.title.clone());
    named.or_else(first).map_or_else(|| project.clone(), |name| format!("{project} · {name}"))
  }

  /// The title of the open thread: the one of its card, else the first line of its first message.
  pub(crate) fn title(&self) -> Option<String> {
    let opened = self.thread.as_ref()?;
    let card = self.told_card(&opened.key).map(|card| card.thread.title.clone());
    let first = || opened.exchanges.get(&0).map(|one| one.text.clone());
    let pending = || opened.pending.first().map(|one| one.text.clone());
    let title = card.or_else(first).or_else(pending)?;
    title.lines().next().map(str::to_owned)
  }

  /// The model that the target of the input speaks to: the one of the last block of the open
  /// thread with its effort, else the one that its card or the last card of its session names. A
  /// new session names none until its life stands.
  fn maker(&self) -> Option<String> {
    let short = |model: &str| model.rsplit(':').next().unwrap_or(model).to_owned();
    let (at, life) = match self.target() {
      Target::Thread(key) => {
        let opened = self.thread.as_ref()?;
        let block = opened.exchanges.values().rev().find_map(|one| one.block.as_ref());
        if let Some(Maker::Model { model, effort }) = block.map(|one| &one.maker) {
          let name = short(model);
          return Some(effort.as_ref().map_or(name.clone(), |one| format!("{name}/{one}")));
        }
        (key.0, key.1)
      }
      Target::Session(at, life) => (at, life),
      Target::New(_) => return None,
    };
    let cards = self.machines.get(at).map(|told| &told.cards[..]).unwrap_or_default();
    let mut ours = cards.iter().rev().filter(|card| card.life == life);
    ours.find_map(|card| card.thread.model.as_deref()).map(short)
  }

  /// The row of hints: why the daemon refused the last message, else the keys of a form, else the
  /// state of the open thread while it works, with the key that stops it, else the folder of the
  /// window; and at its right how many other threads ask the operator, and the key that turns the
  /// input, or that refuses the question of a form.
  fn hints(&self) -> Row {
    let t = &self.tokens;
    let muted = t.style(Role::Muted);
    let card = self.thread.as_ref().and_then(|one| self.told_card(&one.key));
    let works = card.is_some_and(|card| card.thread.works());
    let mut hot = Vec::new();
    // The thread says why it has no page, so the row says it once more only for another fault.
    let shown = self.thread.as_ref().and_then(|one| one.fault.as_ref());
    let (left, loader) = match self.fault.as_ref().filter(|one| Some(*one) != shown) {
      Some(fault) => (vec![run(format!("  {fault}"), Style::new().fg(t.danger))], None),
      None if self.form().is_some() => (self.form_hints().unwrap_or_default(), None),
      None if works => {
        let age = card.and_then(|card| self.age(card.since)).map(|age| format!(" {age}"));
        let working = format!("    Working{}", age.unwrap_or_default());
        let stop = format!(" · {} stop", self.key(Does::Stop));
        let working = vec![run(working, Style::new().fg(t.accent)), run(stop, muted)];
        let at = columns(&working);
        hot.push((at[1].start + 3..at[1].end, Some(Click::Key(Does::Stop))));
        (working, Some((2, Loader::Model)))
      }
      None => (vec![run(format!("  {}", self.folder_shown()), muted)], None),
    };
    let (does, says) = match (&self.aim, &self.thread) {
      _ if self.form().is_some() => (Does::Empty, "refuse"),
      (None, Some(_)) if self.leaves() => (Does::Close, "back"),
      (None, Some(_)) => (Does::NewThread, "new thread"),
      (Some(_), Some(_)) => (Does::NewThread, "back to the thread"),
      (_, None) if self.listed() => (Does::Commands, "commands"),
      (_, None) => (Does::List, "the list"),
    };
    let key = vec![run(format!("{} {says}", self.key(does)), muted)];
    let room = self.wide().saturating_sub(2).saturating_sub(width(&left) + width(&key) + 2);
    let others = self.others_ask(room);
    let right: Line = others.iter().chain(&key).cloned().collect();
    let line = spread(left, right.clone(), self.wide().saturating_sub(2), Style::new());
    let end = width(&line);
    let start = end - width(&right);
    if !others.is_empty() {
      hot.push((start..start + width(&others) - 3, Some(Click::Key(Does::Needed))));
    }
    hot.push((end - width(&key)..end, Some(Click::Key(does))));
    Row { hot, loader, ..Row::of(line) }
  }

  /// The sessions that the picker offers, which pass its filter: a new session on this machine,
  /// then each session of the cards once, by its machine and its record, with its name and its
  /// project.
  pub(crate) fn sessions(&self) -> Vec<(Target, String, String)> {
    let mut all = vec![(Target::New(0), "+ new session".to_owned(), self.project())];
    for (_, key, card) in &self.settled {
      if all.iter().any(|one| one.0 == Target::Session(key.0, key.1.clone())) {
        continue;
      }
      let named = card.session.clone().filter(|name| !name.is_empty());
      let name = named.unwrap_or_else(|| card.thread.title.clone());
      all.push((Target::Session(key.0, key.1.clone()), name, project(card)));
    }
    let filter = self.picker.as_ref().map(|one| one.filter.to_lowercase()).unwrap_or_default();
    let passes = |one: &(Target, String, String)| {
      format!("{} {}", one.1, one.2).to_lowercase().contains(&filter)
    };
    all.into_iter().filter(passes).collect()
  }

  /// The picker of the sessions opened, or closed when it is open.
  pub(crate) fn pick_session(&mut self) {
    let now = match self.target() {
      Target::Thread(key) => Target::Session(key.0, key.1),
      other => other,
    };
    let at = self.sessions().iter().position(|one| one.0 == now).unwrap_or_default();
    self.picker = match self.picker {
      Some(_) => None,
      None => Some(Picker { filter: String::new(), at }),
    };
  }

  /// The entry of the picker at a place chosen: the input sends to its session from now on, and the
  /// picker closes.
  pub(crate) fn picked(&mut self, at: usize) {
    let Some((target, ..)) = self.sessions().into_iter().nth(at) else { return };
    self.picker = None;
    let open = self.thread.as_ref().map(|one| (one.key.0, one.key.1.clone()));
    self.aim = match target {
      Target::Session(at, life) if open == Some((at, life.clone())) => None,
      Target::New(_) if self.thread.is_none() => None,
      other => Some(other),
    };
  }

  /// The rows of the picker, which stand just above the input: its filter, then each session that
  /// passes it, the chosen one lifted.
  pub(crate) fn picker_rows(&self) -> Vec<Row> {
    let Some(picker) = &self.picker else { return Vec::new() };
    let t = &self.tokens;
    let room = self.wide().saturating_sub(4);
    let framed = |line: Line, ground: Color| {
      let mut row = vec![run("  ", Style::new())];
      row.extend(boxed(line, room, ground));
      row
    };
    let filter = if picker.filter.is_empty() {
      run("Filter the sessions", t.style(Role::Muted))
    } else {
      run(picker.filter.clone(), t.style(Role::Prose))
    };
    let mut rows = vec![Row::of(framed(vec![run("▌", t.style(Role::Prose)), filter], t.raised))];
    for (at, (_, name, project)) in self.sessions().into_iter().enumerate() {
      let ground = if at == picker.at { t.lifted } else { t.raised };
      let line = spread(
        vec![run(name, t.style(Role::Prose))],
        vec![run(project, t.style(Role::Muted))],
        room.saturating_sub(2),
        Style::new(),
      );
      let hot = vec![(2..2 + room, Some(Click::Pick(at)))];
      rows.push(Row { hot, ..Row::of(framed(line, ground)) });
    }
    rows.push(Row::default());
    rows
  }
}
