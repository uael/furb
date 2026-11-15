//! The input as the window draws it: its rows and the line of the launcher under it, and what a
//! key of the operator does to it. Where it sends and what it sends stand in the input of
//! `furb-client`.

use std::{mem, ops::Range, path::Path};

use furb::view::{Act, Maker, RunState, Standing, timer};
use furb_host::protocol::{Answer, Card, Model as Offered, Verb};
use opentui::style::{Color, Span, Style};
use unicode_width::UnicodeWidthStr;

use furb_client::{Asked, Chosen, Outgoing, Target};

use crate::{
  keys::Does,
  line::{Line, clipped, cut, run, spread, wrapped},
  list::project,
  model::{Key, Model},
  thread::{Click, Hot, Row, columns, width},
  tokens::{APART, Loader, Role},
};

/// How many rows of text the input grows to before its text scrolls.
const GROWS: usize = 6;

/// The fewest columns of the target in the launcher before the effort gives way to it: the head
/// names the open thread whole, and the effort is a control that a click turns.
const SESSION: usize = 10;

/// The fewest columns of the name of a project in the launcher of a new session before it gives
/// way to its badge alone.
const SHORTEST: usize = 4;

/// A word of python that the window ran on the root of a life, until the next word goes or an
/// escape forgets it: its machine, its record, its text, its rung, whether the window asked the end
/// of that rung since it last dialed the machine, and how its run ended.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Word {
  pub(crate) at: usize,
  pub(crate) life: String,
  pub(crate) text: String,
  pub(crate) rung: String,
  pub(crate) asked: bool,
  pub(crate) ran: Option<RunState>,
}

/// What a first press of a key named, which a second press of that key does: the resolve of the
/// open thread while it works, which stops its work, the stop of that work, the take back of his
/// last message that still runs, or the leave of the window. Any other key leaves it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Armed {
  Resolve(Key),
  Stop,
  Back(Back),
  Leave,
}

/// His last message that still runs, with its text: one that waits in the queue of the input, by
/// its place there, or one of the open thread, by its act.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Back {
  Queued(usize, String),
  Said(String, String),
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
  /// The text of the input sent to its target, as the client sends it: whether it went, and else
  /// why the window says.
  pub(crate) fn send(&mut self, text: String) -> bool {
    match self.client.send(text) {
      Ok(()) => self.fault = None,
      Err(why) => self.fault = Some(why),
    }
    self.fault.is_none()
  }

  /// The stop of the work of the open thread: the cancel of its prompt.
  pub(crate) fn stop(&self) -> Option<(usize, Verb)> {
    let opened = self.client.thread.as_ref().filter(|one| !one.thread.key.2.is_empty())?;
    let (at, life, act) = opened.thread.key.clone();
    Some((at, Verb::Cancel { life, act, why: None }))
  }

  /// The resolve of the open thread, once the daemon made it, with its machine. A thread that works
  /// asks a confirm first, since a resolve stops its work. A thread that ended takes no resolve: a
  /// reply opens it again, so the input turns to it, as a message, at the end of its feed.
  pub(crate) fn resolve(&mut self) -> Option<(usize, Verb)> {
    let opened = self.client.thread.as_ref().filter(|one| !one.thread.key.2.is_empty())?;
    let card =
      self.client.card(&opened.thread.key).map(|one| (one.thread.ended(), one.thread.works()));
    let (ended, works) = card.unwrap_or_default();
    let opened = self.client.thread.as_mut()?;
    if ended {
      (self.client.aim, self.client.draft.python, opened.top) = (None, false, None);
      return None;
    }
    let armed = Armed::Resolve(opened.thread.key.clone());
    if works && self.armed.replace(armed.clone()) != Some(armed) {
      return None;
    }
    self.armed = None;
    let (at, life, thread) = opened.thread.key.clone();
    Some((at, Verb::Resolve { life, thread }))
  }

  /// A key that he pressed: what a first press of another key named goes, and so does what the key
  /// before did not do. Whether it forgot a first press.
  pub(crate) fn pressed(&mut self, does: Does) -> bool {
    let again = matches!(
      (&self.armed, does),
      (Some(Armed::Resolve(_)), Does::Resolve)
        | (Some(Armed::Stop), Does::Close)
        | (Some(Armed::Back(_)), Does::Empty)
        | (Some(Armed::Leave), Does::Leave)
    );
    self.client.notice = None;
    if again {
      return false;
    }
    let armed = self.armed.take().is_some();
    let forgot = does == Does::Close && self.word.take().is_some();
    armed || forgot
  }

  /// A first escape that nothing else takes, while the open thread works: the next escape stops its
  /// work.
  pub(crate) fn arm_stop(&mut self) {
    let card = self.client.thread.as_ref().and_then(|one| self.client.card(&one.thread.key));
    if card.is_some_and(|card| card.thread.works()) {
      self.armed = Some(Armed::Stop);
    }
  }

  /// A press of the key that empties the input, on an empty input: the first names his last
  /// message that still runs, and a second takes it back. A message that waits in the queue
  /// leaves it, and its text comes back to the input at once; one of the open thread is cancelled
  /// on its machine, and its text comes back once the machine did it. The cancel to ask, with that
  /// text.
  pub(crate) fn take_back(&mut self) -> Option<((usize, Verb), String)> {
    let Some(last) = self.last_own() else {
      self.client.notice = Some("Nothing that you sent still runs here.".to_owned());
      return None;
    };
    let armed = Armed::Back(last.clone());
    if self.armed.replace(armed.clone()) != Some(armed) {
      return None;
    }
    self.armed = None;
    match last {
      Back::Queued(at, text) => {
        self.unqueued(at, &text);
        self.client.restore = Some(self.before_draft(text));
        None
      }
      Back::Said(act, text) => {
        let (at, life, _) = self.client.thread.as_ref()?.thread.key.clone();
        Some(((at, Verb::Cancel { life, act, why: None }), text))
      }
    }
  }

  /// His last message that still runs: the last one of the queue that is not on its way, else the
  /// last one of his on the open thread whose act runs.
  fn last_own(&self) -> Option<Back> {
    let flying = self.client.asked.values().any(|one| *one == Asked::Say);
    let queued = self.client.outgoing.iter().enumerate().skip(usize::from(flying)).next_back();
    if let Some((at, one)) = queued {
      return Some(Back::Queued(at, one.text.clone()));
    }
    let opened = self.client.thread.as_ref()?;
    let login = &self.client.machines[opened.thread.key.0].login;
    let his = |by: &Maker| match by {
      Maker::Operator => true,
      Maker::Member { operator: one } => one == login,
      _ => false,
    };
    let said = opened.thread.exchanges.values().rev().find(|one| one.runs && his(&one.by))?;
    Some(Back::Said(said.message.clone(), said.text.clone()))
  }

  /// A message out of the queue, by its place and its text, and out of the thread that shows it. A
  /// new thread that it alone made goes with it.
  fn unqueued(&mut self, at: usize, text: &str) {
    self.client.outgoing.remove(at);
    let Some(opened) = self.client.thread.as_mut() else { return };
    if let Some(shown) =
      opened.thread.pending.iter().rposition(|one| one.act.is_none() && one.text == text)
    {
      opened.thread.pending.remove(shown);
    }
    if opened.thread.key.2.is_empty() && opened.thread.pending.is_empty() {
      self.client.thread = None;
    }
  }

  /// A text that comes back to the input, before what the operator wrote since.
  pub(crate) fn before_draft(&mut self, text: String) -> String {
    let back = [text, self.client.draft.text.clone()].into_iter().filter(|one| !one.is_empty());
    let text = back.collect::<Vec<_>>().join("\n\n");
    self.client.draft.text.clone_from(&text);
    text
  }

  /// What a second press of the key that a first press armed does, as the row of hints says it.
  /// The button of the head says the resolve of a thread that works.
  fn armed_says(&self) -> Option<String> {
    match self.armed.as_ref()? {
      Armed::Back(back) => {
        let text = match back {
          Back::Queued(_, text) | Back::Said(_, text) => text,
        };
        let first = text.lines().next().unwrap_or_default();
        Some(format!("{} again takes back “{first}”", self.key(Does::Empty)))
      }
      Armed::Stop => Some(format!("{} again stops the work", self.key(Does::Close))),
      Armed::Leave => Some(format!("{} again leaves the window", self.key(Does::Leave))),
      Armed::Resolve(_) => None,
    }
  }

  /// The answer to the first message of the queue, with the act that it made: a new thread opens
  /// in the window under its prompt, and each message that waits for it goes to it.
  pub(crate) fn replied(&mut self, answer: Answer) {
    let Answer::Act(act) = answer else { return };
    let Some(sent) = self.client.outgoing.pop_front() else { return };
    self.client.ask_history();
    if sent.python {
      let (at, life, text) = (sent.at, sent.life, sent.text);
      self.word = Some(Word { at, life, text, rung: act, asked: false, ran: None });
      return;
    }
    let thread = sent.thread.clone().filter(|one| !one.is_empty()).unwrap_or_else(|| act.clone());
    for next in self.client.outgoing.iter_mut().filter(|one| one.life == sent.life) {
      if next.thread.as_deref() == Some("") {
        next.thread = Some(thread.clone());
      }
    }
    self.made(&sent, &thread, act);
  }

  /// The end of the rung of the word that the window ran, when the window did not ask it of its
  /// machine since it last dialed it, once the daemon answered the open of its life.
  pub(crate) fn ended_due(&self) -> Option<(usize, Verb)> {
    let word = self.word.as_ref().filter(|one| !one.asked && one.ran.is_none())?;
    let verb = Verb::Ended { life: word.life.clone(), act: word.rung.clone() };
    self.client.stands(word.at, &word.life).then_some((word.at, verb))
  }

  /// What the rung of the word that the window ran came to, as its life folds it. A rung of another
  /// birth of the life may hold its name and another word, so the life holds no end of the word.
  pub(crate) fn ran(&mut self, acts: Vec<Act>) {
    let Some(word) = self.word.as_mut() else { return };
    let Some(act) = acts.into_iter().find(|one| one.id == word.rung) else { return };
    if let Some(run) = act.run.filter(|_| act.word.as_deref() == Some(&word.text)) {
      word.ran = Some(run);
    } else {
      self.fault = Some(format!("The life holds no end of the word that {} ran.", word.rung));
      self.word = None;
    }
  }

  /// The thread that a message went to, with the act of the message: the open thread takes the
  /// prompt of a thread that it made, and the message stays until the thread shows it.
  fn made(&mut self, sent: &Outgoing, thread: &str, act: String) {
    let Some(opened) = self.client.thread.as_mut() else { return };
    if opened.thread.key.0 != sent.at || opened.thread.key.1 != sent.life {
      return;
    }
    if opened.thread.key.2.is_empty() {
      thread.clone_into(&mut opened.thread.key.2);
      opened.thread.above = None;
      self.client.selected = Some(opened.thread.key.clone());
    }
    if opened.thread.key.2 == thread
      && let Some(pending) = opened.thread.pending.iter_mut().find(|one| one.act.is_none())
    {
      pending.act = Some(act);
      opened.thread.shown();
    }
  }

  /// A request of the input that the daemon refused, and why: each message that waits comes back
  /// to the input before what the operator wrote since, and the thread shows none of them. The
  /// text that the input holds now.
  pub(crate) fn refused(&mut self, why: String) -> String {
    self.fault = Some(why);
    let back: Vec<String> =
      mem::take(&mut self.client.outgoing).into_iter().map(|one| one.text).collect();
    if let Some(opened) = self.client.thread.as_mut() {
      opened.thread.pending.retain(|one| one.act.is_some());
      if opened.thread.key.2.is_empty() {
        self.client.thread = None;
      }
    }
    self.before_draft(back.join("\n\n"))
  }

  /// How many rows the text of the input takes, as the editor wraps it at its columns.
  pub(crate) fn grown(&self) -> usize {
    let room = self.wide().saturating_sub(6).max(1);
    // The editor wraps at a word boundary, as the box of a message does.
    let rows =
      self.client.draft.text.split('\n').map(|line| wrapped(line, room).len().max(1)).sum();
    usize::clamp(rows, 1, GROWS)
  }

  /// How many rows the input, the launcher and the hints take at the bottom of the left side.
  pub(crate) fn input_rows(&self) -> usize {
    self.input().len()
  }

  /// The box of the text of the input: over its bands, or over the row of a form where the
  /// operator writes.
  pub(crate) fn area(&self) -> Area {
    let opens = self.client.thread.as_ref().is_some_and(|one| self.says_opens(one));
    let reads = self
      .client
      .thread
      .as_ref()
      .is_some_and(|one| self.client.fault_of(one.thread.key.0, &one.thread.key.1).is_some());
    let ended = self.client.thread.as_ref().and_then(|one| self.client.card(&one.thread.key));
    let ended = ended.is_some_and(|card| card.thread.ended());
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
        let says = match self.client.target() {
          _ if self.client.draft.python => "A word of Python to run",
          Target::Thread(_) if reads => "The window sends nothing to this session",
          Target::Thread(_) if opens => "Your message goes once the session is open",
          Target::Thread(_) if ended => "A reply opens this thread again",
          Target::Thread(_)
            if self.client.thread.as_ref().is_some_and(|one| one.from.is_some()) =>
          {
            "Write to this agent"
          }
          Target::Thread(_) => "Reply to this thread",
          Target::Session(..) => "Ask in a new thread",
          Target::New(..) => "Ask for a change, a review or an answer",
        };
        // The text stands under the blank row that opens the box, as in the box of a message.
        let waits = usize::from(self.waiting_row().is_some());
        (2, waits + 1, self.grown(), says.to_owned())
      }
    };
    let shown = self.form().is_none() || self.form_rows(self.grown()).1.is_some();
    let number = |one: usize| u16::try_from(one).unwrap_or_default();
    Area {
      x: number(x),
      y: number(top + row),
      width: number(self.wide().saturating_sub(x + 4)),
      height: number(height),
      placeholder,
      shown,
    }
  }

  /// The rows of the input: the form of a question of the open thread, or its box under the row
  /// that says that a question waits; then the row of hints. The box is the box of a message of the
  /// feed, with a bar in the color of its target: a blank row, a band for each row of its text,
  /// which the editor covers, a blank row, the row of its controls, and a blank row.
  pub(crate) fn input(&self) -> Vec<Row> {
    let mut rows = if self.form().is_some() {
      let mut rows = self.form_rows(self.grown()).0;
      rows.push(self.launcher());
      rows
    } else {
      // The row above the text says where a walk through what he sent stands.
      let walking = self.client.walking().map(|(at, of)| {
        vec![run(
          format!("history {at} of {of}"),
          self.tokens.style(Role::Muted).bg(self.tokens.raised),
        )]
      });
      let band = |at| self.band(Vec::new(), if at == 0 { walking.clone() } else { None });
      let bands = (0..self.grown() + 2).map(band);
      let mut rows: Vec<Row> = self.waiting_row().into_iter().chain(bands).collect();
      rows.extend([self.launcher(), self.band(Vec::new(), None)]);
      rows
    };
    rows.push(self.hints());
    rows
  }

  /// The bar of the input: the color of the thread that it replies to, or the accent for a new
  /// thread.
  fn bar(&self) -> Color {
    match self.client.target() {
      Target::Thread(key) => self.hue(&key),
      Target::Session(..) | Target::New(..) => self.tokens.accent,
    }
  }

  /// A row of the box of the input: its bar, then a line of the columns of the box with one blank
  /// column on each side, as wide as the box of a message of the feed, with what its right says.
  fn band(&self, line: Line, right: Option<Line>) -> Row {
    let raised = Style::new().bg(self.tokens.raised);
    let room = self.wide().saturating_sub(6);
    let mut row = vec![run("┃", raised.fg(self.bar())), run(" ", raised)];
    row.extend(spread(line, right.unwrap_or_default(), room, raised));
    row.push(run("  ", raised));
    Row::of(row)
  }

  /// The row of the controls of the box of the input: at its left the switch between a message and
  /// a word of Python, and what the input sends to, which picks another; at its right the model
  /// with its effort, and the key that turns the input to a new thread and back.
  fn launcher(&self) -> Row {
    let t = &self.tokens;
    let raised = Style::new().bg(t.raised);
    let muted = t.style(Role::Muted).bg(t.raised);
    let on = t.style(Role::Button);
    let (message, python) = if self.client.draft.python { (muted, on) } else { (on, muted) };
    let switch = [run(" Message ", message), run(" Python ", python)];
    let gap = || run(" ".repeat(APART), raised);
    let (mark, target, color) = match self.client.target() {
      Target::Thread(key) => ("→ ", self.title().unwrap_or_default(), self.hue(&key)),
      Target::Session(..) => ("+ ", format!("new thread in {}", self.session()), t.accent),
      Target::New(at, folder) => return self.launch(at, &folder, switch),
    };
    let mut left: Line = switch.into();
    left.extend([gap(), run(mark, muted), run(target, raised.fg(color))]);
    let room = self.wide().saturating_sub(6);
    let wanted = left.last().map_or(0, |one| one.content.width());
    // The target says where the message goes, so it keeps its words first: the turn gives way to
    // its mark and its key, then the effort, then the target to a few words.
    let right = |short| self.controls(short);
    // The target leaves room for the mark before it and the arrow of its picker after it.
    let fits =
      |right: &(Line, _)| room.saturating_sub(width(&right.0) + 2 + width(&left[..3]) + 2 + 2);
    let (right, clicks) = [right(0), right(1)]
      .into_iter()
      .find(|one| fits(one) >= wanted)
      .or_else(|| [right(1), right(2)].into_iter().find(|one| fits(one) >= SESSION))
      .unwrap_or_else(|| right(2));
    let fits = room.saturating_sub(width(&right) + 2 + width(&left[..3]) + 2 + 2);
    let target = left.pop().map(|one| run(cut(&one.content, fits), one.style));
    left.extend(target.into_iter().chain([run(" ▾", raised.fg(color))]));
    let line = spread(left, right, room, raised);
    let at = columns(&line);
    let mut hot = vec![
      (at[0].start + 2..at[1].end + 2, Some(Click::Key(Does::Python))),
      (at[3].start + 2..at[5].end + 2, Some(Click::Key(Does::Sessions))),
    ];
    hot.extend(clicked(&at, clicks));
    let Row { line, .. } = self.band(line, None);
    Row { hot, ..Row::of(line) }
  }

  /// The launcher of a new session: after the switch, where it will run, the machine and then the
  /// project with its badge, each with the arrow of the part that chooses it, and at the right the
  /// model and the effort that it will take, as the row under the prompt of a desktop app says.
  fn launch(&self, at: usize, folder: &str, switch: [Span<'static>; 2]) -> Row {
    let t = &self.tokens;
    let raised = Style::new().bg(t.raised);
    let (muted, prose) = (t.style(Role::Muted).bg(t.raised), t.style(Role::Prose).bg(t.raised));
    let machine = if at == 0 { "This computer" } else { &self.client.machines[at].name };
    let project = named(folder);
    let badge = self.badge(&project);
    let gap = || run(" ".repeat(APART), raised);
    let mut left: Line = switch.into();
    let fixed = width(&left) + APART + 2 + APART + badge.content.width() + 2;
    // The names give way before the effort: the turn gives way to its mark, then the name of the
    // project to its badge alone, then the name of the machine to a few letters.
    let room = self.wide().saturating_sub(6);
    let spare = |right: &(Line, _)| room.checked_sub(width(&right.0) + 2 + fixed);
    let wanted = machine.width() + 1 + project.width();
    let (right, clicks) = [self.controls(0), self.controls(1)]
      .into_iter()
      .find(|one| spare(one).is_some_and(|spare| spare >= wanted))
      .or_else(|| {
        let some = |one: &(Line, _)| spare(one).is_some_and(|spare| spare >= SHORTEST);
        [self.controls(1), self.controls(2)].into_iter().find(some)
      })
      .unwrap_or_else(|| self.controls(2));
    let spare = room.saturating_sub(width(&right) + 2 + fixed);
    let machine = cut(machine, spare);
    // A name cut to less than a few letters says nothing that the badge does not.
    let name = match spare.saturating_sub(machine.width()) {
      rest if rest > SHORTEST => format!(" {}", cut(&project, rest - 1)),
      _ => String::new(),
    };
    left.extend([
      gap(),
      run(machine, prose),
      run(" ▾", muted),
      gap(),
      run(badge.content.to_string(), badge.style.bg(t.raised)),
      run(name, prose),
      run(" ▾", muted),
    ]);
    let line = spread(left, right, room, raised);
    let at = columns(&line);
    let mut hot = vec![
      (at[0].start + 2..at[1].end + 2, Some(Click::Key(Does::Python))),
      (at[3].start + 2..at[4].end + 2, Some(Click::Key(Does::Machines))),
      (at[6].start + 2..at[8].end + 2, Some(Click::Key(Does::Projects))),
    ];
    hot.extend(clicked(&at, clicks));
    let Row { line, .. } = self.band(line, None);
    Row { hot, ..Row::of(line) }
  }

  /// The words of the control that turns the input to a new thread and back, and its mark alone.
  fn turned(&self) -> Option<(&'static str, &'static str)> {
    match (&self.client.aim, &self.client.thread) {
      (None, Some(_)) => Some(("+ New thread", "+")),
      (Some(_), Some(_)) => Some(("↩ Back to the thread", "↩")),
      (_, None) => None,
    }
  }

  /// The controls at the right of the launcher: the model and its effort, the turn to a new thread
  /// and its key, and the ring of the context while the list hides. A short form gives the turn its
  /// mark alone, and a shorter one drops the effort too.
  fn controls(&self, short: usize) -> (Line, Vec<Option<Click>>) {
    let t = &self.tokens;
    let raised = Style::new().bg(t.raised);
    let (muted, prose) = (t.style(Role::Muted).bg(t.raised), t.style(Role::Prose).bg(t.raised));
    let gap = || (run(" ".repeat(APART), raised), None);
    let key = |does| Some(Click::Key(does));
    let mut right = Vec::new();
    if let Some((model, effort)) = self.maker() {
      right.push((run(model, prose), key(Does::Model)));
      if let Some(effort) = effort.filter(|_| short < 2) {
        right.extend([(run("  ", raised), None), (run(effort, muted), key(Does::Effort))]);
      }
    }
    if let Some((words, mark)) = self.turned() {
      let turn = run(if short > 0 { mark } else { words }, raised.fg(t.accent));
      let named = run(format!(" {}", self.key(Does::NewThread)), muted);
      right.extend([gap(), (turn, key(Does::NewThread)), (named, key(Does::NewThread))]);
    }
    if let Some(ring) = self.ring() {
      right.extend([gap(), (run(ring, prose), Some(Click::Context))]);
    }
    right.into_iter().unzip()
  }

  /// The project and the session that the input sends to.
  fn session(&self) -> String {
    let (at, life, open) = match self.client.target() {
      Target::Thread(key) => (key.0, key.1, true),
      Target::Session(at, life) => (at, life, false),
      Target::New(_, folder) => return format!("{} · new session", named(&folder)),
    };
    let cards = self.client.machines.get(at).map(|told| &told.cards[..]).unwrap_or_default();
    let card = cards.iter().find(|card| card.life == life);
    let project = card.map_or_else(|| self.project(), project);
    let named = card.and_then(|card| card.session.clone()).filter(|name| !name.is_empty());
    // The head of the open thread names it, so the launcher names its session alone; a new thread
    // names its session by its first thread when the session has no name.
    let first = || card.filter(|_| !open).map(|card| card.thread.title.clone());
    named.or_else(first).map_or_else(|| project.clone(), |name| format!("{project} · {name}"))
  }

  /// The title of the open thread: the one of its card, else the first line of its first message
  /// that says something, as the daemon titles a card.
  pub(crate) fn title(&self) -> Option<String> {
    let opened = self.client.thread.as_ref()?;
    let card = self.client.card(&opened.thread.key).map(|card| card.thread.title.clone());
    let first = || opened.thread.exchanges.get(&0).map(|one| one.text.clone());
    let pending = || opened.thread.pending.first().map(|one| one.text.clone());
    let title = card.or_else(first).or_else(pending)?;
    title.lines().find(|line| !line.trim().is_empty()).map(str::to_owned)
  }

  /// The model that the target of the input speaks to, by its name alone, and its effort.
  pub(crate) fn maker(&self) -> Option<(String, Option<String>)> {
    let (model, effort) = self.actor()?;
    Some((model.rsplit(':').next().unwrap_or(&model).to_owned(), effort))
  }

  /// The model that the target of the input speaks to, by its whole name, and its effort: the one
  /// that the operator chose for it, else the one of the last block of the open thread, else the
  /// one that its card or the last card of its session names. A new session stands on the model
  /// of the last session of its project, as it reads its skills, and a project with no session on
  /// the default actor of the catalog of its machine.
  pub(crate) fn actor(&self) -> Option<(String, Option<String>)> {
    // An actor names its effort after its last slash.
    let leveled = |actor: &str| {
      let (name, effort) =
        actor.rsplit_once('/').map_or((actor, None), |(one, two)| (one, Some(two)));
      (name.to_owned(), effort.map(str::to_owned))
    };
    let target = self.client.target();
    if let Some(chosen) = self.client.chosen.as_ref().filter(|one| one.aims(&target)) {
      return Some(leveled(&chosen.actor));
    }
    let (at, life) = match target {
      Target::Thread(key) => {
        let opened = self.client.thread.as_ref()?;
        let block = opened.thread.exchanges.values().rev().find_map(|one| one.block.as_ref());
        if let Some(Maker::Model { model, effort }) = block.map(|one| &one.maker) {
          return Some((model.clone(), effort.clone()));
        }
        (key.0, key.1)
      }
      Target::Session(at, life) => (at, life),
      Target::New(at, folder) => {
        let told = self.client.machines.get(at)?;
        let here = named(&folder);
        let mut ours = told.cards.iter().rev().filter(|card| project(card) == here);
        let offered = told.models.as_deref().unwrap_or_default();
        // A card names the model of its thread with no effort, and a new session on that model
        // stands on its default effort, which the catalog of the machine gives.
        let effort = |model: &str| {
          let same =
            |one: &&Offered| one.name == model || one.name.rsplit(':').next() == Some(model);
          offered.iter().find(same).and_then(|one| one.effort.clone())
        };
        let last = ours.find_map(|card| card.thread.model.as_deref()).map(leveled);
        let last = last.map(|(model, chosen)| {
          let effort = chosen.or_else(|| effort(&model));
          (model, effort)
        });
        return last.or_else(|| offered.first().map(|one| (one.name.clone(), one.effort.clone())));
      }
    };
    let cards = self.client.machines.get(at).map(|told| &told.cards[..]).unwrap_or_default();
    let mut ours = cards.iter().rev().filter(|card| card.life == life);
    ours.find_map(|card| card.thread.model.as_deref()).map(leveled)
  }

  /// The model of the target of the input at its next effort, from the least again after the
  /// most, as the catalog of its machine gives its efforts.
  pub(crate) fn effort(&mut self) {
    let at = self.client.target().machine();
    let Some((model, effort)) = self.actor() else { return };
    let offered = self.client.machines.get(at).and_then(|told| told.models.as_ref());
    let efforts = offered.and_then(|all| all.iter().find(|one| one.name == model));
    let Some(efforts) = efforts.map(|one| &one.efforts).filter(|all| !all.is_empty()) else {
      self.client.notice = Some("This model takes no effort.".to_owned());
      return;
    };
    let now = effort.and_then(|effort| efforts.iter().position(|one| *one == effort));
    let next = &efforts[now.map_or(0, |now| (now + 1) % efforts.len())];
    self.took_actor(format!("{model}/{next}"));
  }

  /// An actor that the operator chose for the target of the input: a new session opens on it, and
  /// a life that stands takes it on its root, as a word of the operator.
  pub(crate) fn took_actor(&mut self, actor: String) {
    self.client.chosen = Some(Chosen::of(&self.client.target(), actor));
  }

  /// The row of hints: why the daemon refused the last message, else the keys of a form, else the
  /// word of python that the window ran, as it runs and what it came to, else the state of the open
  /// thread while it works, with the key that stops it, else the folder of the
  /// window; and at its right how many other threads ask the operator, and the key that turns the
  /// input, or that refuses the question of a form.
  fn hints(&self) -> Row {
    let t = &self.tokens;
    let muted = t.style(Role::Muted);
    let card = self.client.thread.as_ref().and_then(|one| self.client.card(&one.thread.key));
    let ended = self
      .client
      .thread
      .as_ref()
      .is_some_and(|one| self.client.fault_of(one.thread.key.0, &one.thread.key.1).is_some());
    let works = card.is_some_and(|card| card.thread.works()) && !ended;
    let mut hot = Vec::new();
    // The thread says why it has no page, so the row says it once more only for another fault.
    let shown = self.client.thread.as_ref().and_then(|one| self.why(one));
    let notice = self.client.notice.clone().map(|one| (one, t.muted));
    let says = self.armed_says().map(|one| (one, t.attention)).or(notice);
    let (does, name) = match (&self.client.aim, &self.client.thread) {
      _ if self.form().is_some() => (Does::Empty, "refuse"),
      (None, Some(_)) if self.leaves() => (Does::Close, "back"),
      (None, Some(opened)) if !opened.docked().1.is_empty() => (Does::Agent, "agents"),
      (_, Some(_)) => (Does::Commands, "commands"),
      (_, None) if self.listed() => (Does::Commands, "commands"),
      (_, None) => (Does::List, "the list"),
    };
    let key = vec![run(format!("{} {name}", self.key(does)), muted)];
    let fault = self.fault.as_ref().filter(|one| Some(one.as_str()) != shown);
    let place = fault.is_none() && says.is_none() && self.form().is_none() && self.word.is_none();
    let place = place && !works;
    let (left, loader) = match (fault, says) {
      (Some(fault), _) => (vec![run(format!("  {fault}"), Style::new().fg(t.danger))], None),
      (None, Some((says, color))) => (vec![run(format!("  {says}"), Style::new().fg(color))], None),
      (None, None) if self.form().is_some() => (self.form_hints().unwrap_or_default(), None),
      (None, None) if self.word.is_some() => {
        self.word_hints(self.wide().saturating_sub(2).saturating_sub(width(&key) + 2))
      }
      (None, None) if works => {
        let (working, loader) = self.working(card);
        let at = columns(&working);
        hot.push((at[1].start + 3..at[1].end, Some(Click::Key(Does::Stop))));
        (working, Some((2, loader)))
      }
      // The launcher of a new session names its machine, so the row says the folder alone.
      (None, None) => (self.place(!matches!(self.client.target(), Target::New(..))), None),
    };
    // The machine and the folder of a thread that rests give way to the threads that ask him, which
    // the row never hides.
    let kept = if place { 2 } else { width(&left) };
    let room = self.wide().saturating_sub(2).saturating_sub(kept + width(&key) + 2);
    let others = self.others_ask(room);
    let right: Line = others.iter().chain(&key).cloned().collect();
    // A place that does not fit says its folder alone, which tells more than the machine.
    let fits = width(&left) + 2 + width(&right) <= self.wide().saturating_sub(2);
    let left = if place && !fits { self.place(false) } else { left };
    let line = spread(left, right.clone(), self.wide().saturating_sub(2), Style::new());
    let end = width(&line);
    let start = end - width(&right);
    if !others.is_empty() {
      hot.push((start..start + width(&others) - 3, Some(Click::Key(Does::Needed))));
    }
    hot.push((end - width(&key)..end, Some(Click::Key(does))));
    Row { hot, loader, ..Row::of(line) }
  }

  /// The hint of the work of the open thread: its time, which counts each second as the time of a
  /// line that runs, where a card says its age in minutes, the key that stops it, and the mark of
  /// that work: a model, a command or agents.
  fn working(&self, card: Option<&Card>) -> (Line, Loader) {
    let t = &self.tokens;
    let worked = card.map(|card| self.second().saturating_sub(card.since)).filter(|one| *one > 0);
    let age = worked.map(|seconds| format!(" {}", timer(seconds)));
    let working = format!("    Working{}", age.unwrap_or_default());
    let stop = format!(" · {} stop", self.key(Does::Stop));
    let loader = match card.map(|card| &card.thread.standing) {
      Some(Standing::Runs) => Loader::Command,
      Some(Standing::Delegates) => Loader::Agent,
      _ => Loader::Model,
    };
    (vec![run(working, Style::new().fg(t.accent)), run(stop, t.style(Role::Muted))], loader)
  }

  /// Where the input runs: the machine and the folder, or the folder alone.
  fn place(&self, machine: bool) -> Line {
    let at = match self.client.target() {
      Target::Thread(key) => key.0,
      Target::Session(at, _) | Target::New(at, _) => at,
    };
    let name = if at == 0 { "This computer" } else { &self.client.machines[at].name };
    let text = if machine {
      format!("  {name} · {}", self.folder_shown())
    } else {
      format!("  {}", self.folder_shown())
    };
    vec![run(text, self.tokens.style(Role::Muted))]
  }

  /// The hints of the word that the window ran, in a room of columns: a mark that moves while it
  /// runs, then a check when it ran, or its fault, and its text on one line, cut with the count of
  /// the characters that the cut hides. A word that the open thread shows has its trace there.
  fn word_hints(&self, room: usize) -> (Line, Option<(usize, Loader)>) {
    let t = &self.tokens;
    let muted = t.style(Role::Muted);
    let mut opened = self.client.thread.iter().flat_map(|one| one.thread.exchanges.values());
    let Some(word) = self.word.as_ref().filter(|word| !opened.any(|one| one.message == word.rung))
    else {
      return (Vec::new(), None);
    };
    let text = word.text.trim().replace('\n', "\\n");
    let (head, loader) = match &word.ran {
      None => (vec![run("    ", muted)], Some((2, Loader::Command))),
      Some(RunState::Running | RunState::Done) => (vec![run("  ✓ ", muted)], None),
      Some(RunState::Cancelled(_)) => (vec![run("  ✗ Cancelled · ", muted)], None),
      Some(RunState::Failed(why)) => {
        (vec![run(format!("  ✗ {why}"), Style::new().fg(t.danger)), run(" · ", muted)], None)
      }
    };
    (clipped([head, vec![run(text, muted)]].concat(), room, muted), loader)
  }
}

/// The parts of a row of the box of the input that take a click: the runs at its end, by their
/// columns in the line, each with its click. The box draws the line two columns in.
fn clicked(at: &[Range<usize>], clicks: Vec<Option<Click>>) -> Vec<Hot> {
  let first = at.len() - clicks.len();
  let each = clicks.into_iter().enumerate();
  each
    .filter_map(|(n, click)| Some((at[first + n].start + 2..at[first + n].end + 2, Some(click?))))
    .collect()
}

/// The name of a project, from its folder.
pub(crate) fn named(folder: &str) -> String {
  Path::new(folder).file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default()
}
