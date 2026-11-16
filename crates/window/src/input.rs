//! The input as the window draws it: its rows and the line of the launcher under it, and what a
//! key of the operator does to it. Where it sends and what it sends stand in the input of
//! `furb-client`.

use std::{mem, ops::Range, path::Path};

use crate::cells::Cells;
use furb::view::{
  Act, Exchange, Maker, RunState, Standing, Written, operator, reason, said, timer,
};
use furb_host::protocol::{Answer, Card, Frame, Model as Offered, Verb};
use opentui::style::{Color, Span, Style};
use serde_json::Value;

use furb_client::{Anchor, Asked, Chosen, FOLDED, Outgoing, Pending, RENAME, Target, Where};

use crate::{
  keys::Does,
  line::{BARRED, Line, barred, clipped, cut, run, spread, wrapped},
  link::Heard,
  list::{NO_PROJECT, project},
  model::{Key, Model},
  picture,
  review::shown_at,
  thread::{Click, Hot, Row, columns, width},
  tokens::{APART, Loader, Role, WORKS},
};

/// The fewest rows of text that the input grows to before its text scrolls: it grows to a third of
/// the rows of the window when they are more, as a box of a rival grows to most of a screen.
/// How many levels of the microphone the row of hints shows while his voice records.
pub(crate) const LEVELS: usize = 12;

/// The bars of a level of the microphone, from silence to the loudest.
const BARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

const GROWS: usize = 6;

/// The fewest columns of the target in the launcher before the effort gives way to it: the head
/// names the open thread whole, and the effort is a control that a click turns.
const SESSION: usize = 10;

/// The fewest columns of the name of a project in the launcher of a new session before it gives
/// way to its badge alone.
const SHORTEST: usize = 4;

/// The mark of the branch of the place of the input, after a dot.
const BRANCH: &str = "⑂ ";

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
/// last message that still runs, which the hint names as it stands at each frame, or the leave of
/// the window. Any other key leaves it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Armed {
  Resolve(Key),
  Stop,
  Back,
  Leave,
}

/// His last message that still runs, with its text: one that waits in the queue of the input, by
/// its place there, one on its way to its machine, or one of the open thread, by its act.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Back {
  Queued(usize, String),
  Flying(String),
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

/// The columns of a prompt of the interpreter before a line of a word of Python: `>>> ` or `... `.
const PROMPT_WIDTH: usize = 4;

/// Each file that the lines of an answer wrote.
pub(crate) fn writes(exchange: &Exchange) -> impl Iterator<Item = &Written> {
  exchange.block.iter().flat_map(|block| &block.lines).flat_map(|line| &line.writes)
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

  /// The text of the input queued on the open thread: whether it went, and else why the window
  /// says.
  pub(crate) fn queue(&mut self, text: &str) -> bool {
    match self.client.queue(text) {
      Ok(()) => self.fault = None,
      Err(why) => self.fault = Some(why),
    }
    self.fault.is_none()
  }

  /// The control of the open thread that a key asks: the stop of its work, its resolve, or its
  /// pause or the wake of his pause.
  pub(crate) fn control(&mut self, does: Does) -> Option<(usize, Verb)> {
    match does {
      Does::Stop => self.stop(),
      Does::Resolve => self.resolve(),
      Does::Revert => self.revert(),
      Does::Pause => self.pause(),
      Does::Reload => self.reload(),
      _ => None,
    }
  }

  /// What the input runs on, as a notice: the machine, the project, the model with its effort, the
  /// share of the window of the open thread, and the version of furb.
  pub(crate) fn status(&mut self) {
    let at = self.client.target().machine();
    let machine = self.client.machines.get(at).map(|one| one.name.clone());
    let actor = self.maker().map(|(model, effort)| match effort {
      Some(effort) => format!("{model} at {effort}"),
      None => model,
    });
    let build = Some(format!("furb {}", env!("CARGO_PKG_VERSION")));
    let parts = [machine, Some(self.project()), actor, self.ring(), build];
    self.client.notice = Some(parts.into_iter().flatten().collect::<Vec<_>>().join(" · "));
  }

  /// The messages of the open thread that the window holds, as Markdown, and how many they are:
  /// each message under who said it, the code that answered it, and what the answer said.
  pub(crate) fn exported(&self) -> Option<(String, usize)> {
    let thread = &self.client.thread.as_ref()?.thread;
    let mut parts: Vec<String> = Vec::new();
    for one in thread.exchanges.values() {
      parts.push(match &one.by {
        Maker::Operator | Maker::Member { .. } => "## You".to_owned(),
        Maker::Model { model, .. } => format!("## {model}"),
        Maker::Skill => "## A skill".to_owned(),
        Maker::Engine => "## furb".to_owned(),
      });
      parts.extend((!one.text.trim().is_empty()).then(|| one.text.trim_end().to_owned()));
      let code: Vec<&str> =
        one.block.iter().flat_map(|block| &block.lines).map(|line| line.text.as_str()).collect();
      parts.extend((!code.is_empty()).then(|| format!("```python\n{}\n```", code.join("\n"))));
      let said = one.said.iter().filter(|said| !said.text.trim().is_empty());
      parts.extend(said.map(|said| said.text.trim_end().to_owned()));
    }
    (!parts.is_empty()).then(|| (parts.join("\n\n") + "\n", thread.exchanges.len()))
  }

  /// The reload of the session of the open thread: the word `await reload()` on the root of its
  /// life, which loads its skills again, reads its servers of MCP again and moves furb to its
  /// newest build, as a model that runs the word does.
  pub(crate) fn reload(&mut self) -> Option<(usize, Verb)> {
    let opened = self.client.thread.as_ref().filter(|one| !one.thread.key.1.is_empty());
    let Some(opened) = opened else {
      self.client.notice = Some("Open a thread to reload its session.".to_owned());
      return None;
    };
    let (at, life, _) = opened.thread.key.clone();
    Some((at, Verb::Run { life, word: "await reload()".to_owned(), on: None }))
  }

  /// The stop of the work of the open thread: the cancel of its prompt.
  pub(crate) fn stop(&self) -> Option<(usize, Verb)> {
    let opened = self.client.thread.as_ref().filter(|one| !one.thread.key.2.is_empty())?;
    let (at, life, act) = opened.thread.key.clone();
    Some((at, Verb::Cancel { life, act, why: None }))
  }

  /// The pause of the work of the open thread, or the wake of its pause: the work that a pause of
  /// his held goes on, and a pause that furb said after a failure asks the model again.
  pub(crate) fn pause(&mut self) -> Option<(usize, Verb)> {
    let opened = self.client.thread.as_ref().filter(|one| !one.thread.key.2.is_empty())?;
    let key = opened.thread.key.clone();
    let card = self.client.card(&key);
    let control = match card.map(|card| &card.thread.standing) {
      Some(Standing::Paused { .. }) => self.client.wake(&key),
      _ if card.is_some_and(|card| card.thread.works()) => {
        Some((key.0, Verb::Pause { life: key.1.clone(), act: key.2.clone(), why: None }))
      }
      _ => None,
    };
    if control.is_none() {
      self.client.notice = Some("Nothing works in this thread.".to_owned());
    }
    control
  }

  /// The resolve of the open thread, once the daemon made it, with its machine. A thread that works
  /// asks a confirm first, since a resolve stops its work. A thread that ended takes no resolve: a
  /// reply opens it again, so the input turns to it, as a message, at the end of its feed.
  pub(crate) fn resolve(&mut self) -> Option<(usize, Verb)> {
    let opened = self.client.thread.as_ref().filter(|one| !one.thread.key.2.is_empty())?;
    let card = self.client.card(&opened.thread.key).map(|one| &one.thread);
    let card = card.or_else(|| opened.sub().map(|one| &one.card)).or_else(|| opened.agent());
    let card = card.map(|one| (one.ended(), one.works()));
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

  /// The last answer of the open thread that wrote a file that the life knew before it: what a
  /// revert writes back.
  pub(crate) fn reverting(&self) -> Option<&Exchange> {
    let opened = self.client.thread.as_ref()?;
    opened.thread.exchanges.values().rev().find(|one| writes(one).any(|one| one.known))
  }

  /// The revert of the files that the last answer of the open thread wrote, as a word of his on the
  /// thread. A file that furb knew before none of its writes stays, and the window says so.
  pub(crate) fn revert(&mut self) -> Option<(usize, Verb)> {
    let Some(writes) = self.reverting().map(|one| writes(one).collect::<Vec<_>>()) else {
      self.client.notice = Some("No answer here wrote a file that furb knew before.".to_owned());
      return None;
    };
    let known = |path: &str| writes.iter().any(|one| one.known && one.path == path);
    let mut kept: Vec<String> =
      writes.iter().filter(|one| !known(&one.path)).map(|one| one.path.clone()).collect();
    kept.dedup();
    let writes = writes.iter().map(|one| one.act.clone()).collect();
    let (at, life, thread) = self.client.thread.as_ref()?.thread.key.clone();
    self.client.notice = (!kept.is_empty()).then(|| {
      let kept: Vec<String> = kept.iter().map(|one| named(one)).collect();
      format!("{} stays as it is: furb did not know it before.", kept.join(", "))
    });
    Some((at, Verb::Revert { life, writes, on: Some(thread) }))
  }

  /// A key that he pressed: what a first press of another key named goes, and so does what the key
  /// before did not do. Whether it forgot a first press.
  pub(crate) fn pressed(&mut self, does: Does) -> bool {
    let again = matches!(
      (&self.armed, does),
      (Some(Armed::Resolve(_)), Does::Resolve)
        | (Some(Armed::Stop), Does::Close)
        | (Some(Armed::Back), Does::Empty)
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
  /// message that still runs, and a second takes back the one that the hint names then. A message
  /// that waits in the queue leaves it, and its text comes back to the input at once; one of the
  /// open thread is cancelled on its machine, and its text comes back once the machine did it. A
  /// message on its way stays named, and the next press takes it once its machine took it. The
  /// cancel to ask, with that text.
  pub(crate) fn take_back(&mut self) -> Option<((usize, Verb), String)> {
    let Some(last) = self.last_own() else {
      self.client.notice = Some("Nothing that you sent still runs here.".to_owned());
      return None;
    };
    let again = self.armed.replace(Armed::Back) == Some(Armed::Back);
    match last {
      // A message on its way stays named, and the next press takes it once its machine took it.
      _ if !again => None,
      Back::Flying(_) => None,
      Back::Queued(at, text) => {
        self.armed = None;
        self.unqueued(at, &text);
        self.client.restore = Some(self.before_draft(text));
        None
      }
      Back::Said(act, text) => {
        self.armed = None;
        let (at, life, _) = self.client.thread.as_ref()?.thread.key.clone();
        Some(((at, Verb::Cancel { life, act, why: None }), text))
      }
    }
  }

  /// His last message that still runs: the last one of the queue that is not on its way, else the
  /// one on its way, else the last one that the daemon took and the page does not show yet, else
  /// his last one on the open thread while its act runs, so a take back never reaches past a later
  /// message to an earlier one, as the message that began the thread.
  fn last_own(&self) -> Option<Back> {
    let flying = self.client.asked.values().any(|one| *one == Asked::Say);
    let queued = self.client.outgoing.iter().enumerate().skip(usize::from(flying)).next_back();
    if let Some((at, one)) = queued {
      return Some(Back::Queued(at, one.text.clone()));
    }
    if let Some(one) = self.client.outgoing.front().filter(|_| flying) {
      return Some(Back::Flying(one.text.clone()));
    }
    let opened = self.client.thread.as_ref()?;
    let mut taken = opened.thread.pending.iter().rev();
    if let Some((act, text)) = taken.find_map(|one| Some((one.act.clone()?, one.text.clone()))) {
      return Some(Back::Said(act, text));
    }
    let login = self.client.machines.get(opened.thread.key.0).map_or("", |one| one.login.as_str());
    let his = |by: &Maker| match by {
      Maker::Operator => true,
      Maker::Member { operator: one } => one == login,
      _ => false,
    };
    let said =
      opened.thread.exchanges.values().rev().find(|one| his(&one.by)).filter(|one| one.runs)?;
    Some(Back::Said(said.message.clone(), said.text.clone()))
  }

  /// A message out of the queue, by its place and its text, and out of the thread that shows it. A
  /// new thread that it alone made goes with it.
  fn unqueued(&mut self, at: usize, text: &str) {
    if let Some(taken) = self.client.outgoing.remove(at) {
      self.client.draft.images.extend(taken.images);
    }
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

  /// What a second press of the key that a first press armed does, as the row of hints says it in
  /// a count of columns: a message that it takes back is cut inside its quotes. The button of the
  /// head says the resolve of a thread that works.
  fn armed_says(&self, room: usize) -> Option<String> {
    match self.armed.as_ref()? {
      Armed::Back => {
        let text = match self.last_own()? {
          Back::Queued(_, text) | Back::Flying(text) | Back::Said(_, text) => text,
        };
        let said = format!("{} again takes back “", self.key(Does::Empty));
        let first =
          cut(text.lines().next().unwrap_or_default(), room.saturating_sub(said.width() + 1));
        Some(format!("{said}{first}”"))
      }
      Armed::Stop => Some(format!("{} again stops the work", self.key(Does::Close))),
      Armed::Leave => Some(format!("{} again leaves the window", self.key(Does::Leave))),
      Armed::Resolve(_) => {
        Some(format!("{} again stops and resolves the thread", self.key(Does::Resolve)))
      }
    }
  }

  /// The answer to the first message of the queue, with the act that it made: a new thread opens
  /// in the window under its prompt, and each message that waits for it goes to it. An answer to
  /// an image of the message is the text by which the message names it.
  pub(crate) fn replied(&mut self, answer: Answer) {
    if let Answer::Value(Value::String(named)) = answer {
      return self.client.imaged(named);
    }
    let Answer::Act(act) = answer else { return };
    let Some(sent) = self.client.outgoing.pop_front() else { return };
    self.client.ask_history();
    // A word on a thread stands in the feed of that thread, as a rewind does, so the row of hints
    // keeps only a word that no thread shows.
    if sent.python {
      let on = sent.thread.filter(|one| !one.is_empty());
      let (at, life, text) = (sent.at, sent.life, sent.text);
      self.word =
        on.is_none().then_some(Word { at, life, text, rung: act, asked: false, ran: None });
      return;
    }
    // A message makes the hints of the last word that ran go, so they never cover its work.
    self.word = None;
    // A sub thread opens from the thread that it is on, which the back key returns to.
    let on = self.client.thread.as_ref().map(|one| &one.thread.key);
    let sub =
      sent.thread.as_ref().filter(|one| !one.is_empty() && Anchor::of(&sent.text).is_some());
    if let Some(thread) = sub
      && on.is_some_and(|key| key.0 == sent.at && key.1 == sent.life && key.2 == *thread)
    {
      // The sub thread shows the message that made it until its page comes, as a new thread does.
      self.open_agent(act.clone());
      if let Some(opened) = self.client.thread.as_mut() {
        opened.thread.pending.push(Pending { act: Some(act), text: sent.text });
        opened.thread.started = true;
      }
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

  /// The answer to the ask of the end of a rung of a life of a machine, which reaches the word
  /// that the window runs only when it is that rung of that life: each life names its rungs alike.
  pub(crate) fn ended(&mut self, at: usize, (life, rung): (&str, &str), heard: Heard) {
    let ours = |one: &Word| one.at == at && one.life == life && one.rung == rung;
    if !self.word.as_ref().is_some_and(ours) {
      return;
    }
    match heard {
      Ok(Frame::Reply { answer: Answer::Acts(acts), .. }) => self.ran(acts),
      Ok(Frame::Refused { why, .. }) => (self.word, self.fault) = (None, Some(why)),
      _ => {}
    }
  }

  /// What the rung of the word that the window ran came to, as its life folds it. A rung of another
  /// birth of the life may hold its name and another word, so the life holds no end of the word.
  fn ran(&mut self, acts: Vec<Act>) {
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
    let mut back = Vec::new();
    for one in mem::take(&mut self.client.outgoing) {
      self.client.draft.images.extend(one.images);
      back.push(one.text);
    }
    if let Some(opened) = self.client.thread.as_mut() {
      opened.thread.pending.retain(|one| one.act.is_some());
      if opened.thread.key.2.is_empty() {
        self.client.thread = None;
      }
    }
    self.before_draft(back.join("\n\n"))
  }

  /// How many rows the text of the input takes in its box, which grows up to a count of rows.
  pub(crate) fn grown(&self) -> usize {
    usize::clamp(self.lines(), 1, self.grows())
  }

  /// How many rows of text the input grows to before its text scrolls.
  fn grows(&self) -> usize {
    GROWS.max(usize::from(self.size.1) / 3)
  }

  /// How many lines the text of the input holds, as the editor wraps it at its columns.
  fn lines(&self) -> usize {
    let prompt = if self.client.draft.python { PROMPT_WIDTH } else { 0 };
    let room = self.wide().saturating_sub(BARRED + 4 + prompt).max(1);
    // The editor wraps at a word boundary, as the box of a message does.
    self.client.draft.text.split('\n').map(|line| wrapped(line, room).len().max(1)).sum()
  }

  /// How many rows the input, the launcher, the hints and the bar of the time take at the bottom of
  /// the left side, with no row of them made.
  pub(crate) fn input_rows(&self) -> usize {
    self.input_box().len() + 1 + self.time_count()
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
      (Some(_), None) => (BARRED, 0, 1, String::new()),
      (Some((_, question)), Some(at)) if question.offers_line() => {
        let hint = question.hint();
        let says = if hint.is_empty() { "Your answer".to_owned() } else { hint };
        (BARRED, at, self.grown(), format!("Write {says}"))
      }
      (Some(_), Some(at)) => (BARRED + 2, at, 1, "type another answer".to_owned()),
      (None, _) => {
        let anchored = self.client.draft.anchor.as_ref().map(|one| match &one.at {
          Where::Line { line, before, .. } => {
            let side = if *before { ", before the change" } else { "" };
            format!("Say what to do with {}, line {line}{side}", named(&one.path))
          }
          Where::Folder => format!("Say what to do with the folder {}/", one.path),
        });
        let says = match self.client.target() {
          _ if anchored.is_some() => anchored.as_deref().unwrap_or_default(),
          _ if self.client.draft.python => "A word of Python to run",
          Target::Thread(_) if reads => "The window sends nothing to this session",
          Target::Thread(_) if opens => "Your message goes once the session is open",
          Target::Thread(_) if ended => "A reply opens this thread again",
          Target::Thread(_)
            if self
              .client
              .thread
              .as_ref()
              .is_some_and(|one| one.from.is_some() && one.sub().is_none()) =>
          {
            "Write to this agent"
          }
          Target::Thread(_) => "Reply to this thread",
          Target::Session(..) => "Ask in a new thread",
          Target::New(..) => "Ask for a change, a review or an answer",
        };
        // The text stands under the blank row that opens the box, as in the box of a message, and
        // after the prompt of the interpreter while it is a word of Python.
        let waits = usize::from(self.waiting_row().is_some());
        let x = if self.client.draft.python { BARRED + PROMPT_WIDTH } else { BARRED };
        (x, waits + 1, self.grown(), says.to_owned())
      }
    };
    // A window that only reads has no field to write in.
    let shown = (self.form().is_none() || self.form_rows(self.grown()).1.is_some())
      && self.client.only_reads(self.client.target().machine()).is_none();
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
  /// that says that a question waits; then the row of hints, and the bar of the time of the open
  /// thread. The box is the box of a message of the
  /// feed, with a bar in the color of its target: a blank row, a band for each row of its text,
  /// which the editor covers, a blank row, the row of its controls, and a blank row.
  pub(crate) fn input(&self) -> Vec<Row> {
    let mut rows = self.input_box();
    rows.push(self.hints());
    // The bar of the time stands under the status row, always: calm at now, or the bar he holds.
    rows.extend(self.time_rows());
    rows
  }

  /// The rows of the input above its row of hints: the form of a question, or its box.
  fn input_box(&self) -> Vec<Row> {
    if self.client.only_reads(self.client.target().machine()).is_some() {
      vec![self.reader()]
    } else if self.form().is_some() {
      let mut rows = self.form_rows(self.grown()).0;
      let blank = || self.band(Vec::new(), None);
      rows.extend([blank(), self.launcher(), blank()]);
      rows
    } else {
      // A word of Python takes the prompts of the interpreter: `>>>` before its first line and
      // `...` before each other line, as the REPL of Python writes them, in the accent of the mode.
      let prompt = |at: usize| {
        let mark = if at == 1 { ">>> " } else { "... " };
        let python = self.client.draft.python && (1..=self.grown()).contains(&at);
        python
          .then(|| run(mark, Style::new().fg(self.tokens.accent).bg(self.tokens.raised)))
          .into_iter()
      };
      let band = |at| self.band(prompt(at).collect(), None);
      let bands = (0..self.grown() + 2).map(band);
      let mut rows: Vec<Row> = self.waiting_row().into_iter().chain(bands).collect();
      rows.extend([self.launcher(), self.band(Vec::new(), None)]);
      rows
    }
  }

  /// The keys of what stands over the input, in the order that they hear a key: the completion, the
  /// bar of the time while he holds it, or a form; else where a walk through what he sent stands,
  /// in the form of the place in a list, or how many lines of a text longer than its box show.
  fn over_input(&self) -> Option<Line> {
    let muted = self.tokens.style(Role::Muted);
    let walked = || {
      let (at, of) = self.client.walking()?;
      Some(vec![run(format!("  ↑↓ history {at} / {of}"), muted)])
    };
    let lines = self.lines();
    let grows = self.grows();
    let long = || (lines > grows).then(|| vec![run(format!("  {grows} of {lines} lines"), muted)]);
    self
      .listening_hints()
      .or_else(|| self.completion_hints())
      .or_else(|| self.time_hints())
      .or_else(|| self.form_hints())
      .or_else(walked)
      .or_else(long)
  }

  /// While his voice records: the word that says so, then the last levels of the microphone as
  /// bars that rise and fall with his voice, then the keys that end the record.
  fn listening_hints(&self) -> Option<Line> {
    let levels = self.listening.as_ref()?;
    let (accent, muted) = (Style::new().fg(self.tokens.accent), self.tokens.style(Role::Muted));
    let bars: String = (0..LEVELS)
      .map(|at| {
        let level = (at + levels.len()).checked_sub(LEVELS).and_then(|at| levels.get(at));
        let step = usize::from(level.copied().unwrap_or_default()) * (BARS.len() - 1) / 100;
        BARS.get(step).or(BARS.last()).copied().unwrap_or(' ')
      })
      .collect();
    let keys = format!("  let go to write · {} drop", self.key(Does::Close));
    Some(vec![run("  ● Listening ", accent), run(bars, accent), run(keys, muted)])
  }

  /// The bar of the input: the color of his turn while it holds the form of a question, else the
  /// color of the thread that it replies to, or the accent for a new thread.
  pub(crate) fn bar(&self) -> Color {
    match self.client.target() {
      _ if self.form().is_some() => self.tokens.attention,
      Target::Thread(key) => self.hue(&key),
      Target::Session(..) | Target::New(..) => self.tokens.accent,
    }
  }

  /// A row of the box of the input: its bar, then a line of the columns of the box with one blank
  /// column on each side, as wide as the box of a message of the feed, with what its right says.
  fn band(&self, line: Line, right: Option<Line>) -> Row {
    let raised = Style::new().bg(self.tokens.raised);
    let room = self.wide().saturating_sub(BARRED + 4);
    let mut row = barred(self.bar(), self.tokens.raised);
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
    // The mode of Python takes the accent, as its prompts do, so the input says that a word runs.
    let (message, python) =
      if self.client.draft.python { (muted, on.fg(t.accent)) } else { (on, muted) };
    let switch = [run(" Message ", message), run(" Python ", python)];
    let gap = || run(" ".repeat(APART), raised);
    let (mark, target, color) = match self.client.target() {
      Target::Thread(key) => ("→ ", self.title().unwrap_or_default(), self.hue(&key)),
      Target::Session(..) => ("+ ", format!("new thread in {}", self.session()), t.accent),
      Target::New(at, folder) => return self.launch(at, &folder, switch),
    };
    let mut left: Line = switch.into();
    left.extend([gap(), run(mark, muted), run(target, raised.fg(color))]);
    let room = self.wide().saturating_sub(BARRED + 4);
    let wanted = left.last().map_or(0, |one| one.content.width());
    // The turn keeps its label, as on each other launcher: the target gives way first, down to a
    // few words, then the turn to its mark and its key, then the effort.
    let right = |short| self.controls(short);
    // The target leaves room for the mark before it and the arrow of its picker after it.
    let head = width(left.get(..3).unwrap_or(&left));
    let fits = |right: &(Line, _)| room.saturating_sub(width(&right.0) + 2 + head + 2 + 2);
    let (right, clicks) = [right(0), right(1), right(2)]
      .into_iter()
      .find(|one| fits(one) >= wanted.min(SESSION))
      .unwrap_or_else(|| right(2));
    let fits = room.saturating_sub(width(&right) + 2 + head + 2 + 2);
    // The mark and the accent of a new thread say what the words say, so a target too long for
    // its room drops its words before the name of its session.
    let short = |text: &str| match self.client.target() {
      Target::Session(..) if text.width() > fits => self.session(),
      _ => text.to_owned(),
    };
    let target = left.pop().map(|one| run(cut(&short(&one.content), fits), one.style));
    left.extend(target.into_iter().chain([run(" ▾", raised.fg(color))]));
    let line = spread(left, right, room, raised);
    let at = columns(&line);
    let spans = [((0, 1), Does::Python), ((3, 5), Does::Sessions)];
    let mut hot: Vec<Hot> =
      spans.into_iter().filter_map(|(runs, does)| spanned(&at, runs, Click::Key(does))).collect();
    hot.extend(clicked(&at, clicks));
    let Row { line, .. } = self.band(line, None);
    Row { hot, ..Row::of(line) }
  }

  /// The one row of the input of a window that only reads: no field to write in and no control
  /// that writes, a muted line that says so, and the ring of the context, which only reads.
  fn reader(&self) -> Row {
    let t = &self.tokens;
    let watched = if self.client.thread.is_some() { "this session" } else { "each session live" };
    let left = vec![run(format!("Read only · you watch {watched}"), t.style(Role::Muted))];
    let ring = self.ring().map(|ring| (run(ring, t.style(Role::Prose)), Some(Click::Context)));
    let (right, clicks): (Line, Vec<_>) = ring.into_iter().unzip();
    // The line stands where the text of a box of the input stands.
    let line = spread(left, right, self.wide().saturating_sub(BARRED + 4), Style::new());
    let hot = clicked(&columns(&line), clicks);
    let mut row = vec![run(" ".repeat(BARRED), Style::new())];
    row.extend(line);
    Row { hot, ..Row::of(row) }
  }

  /// The launcher of a new session: after the switch, where it will run, the machine and then the
  /// project with its badge, each with the arrow of the part that chooses it, and at the right the
  /// model and the effort that it will take, as the row under the prompt of a desktop app says.
  fn launch(&self, at: usize, folder: &str, switch: [Span<'static>; 2]) -> Row {
    let t = &self.tokens;
    let raised = Style::new().bg(t.raised);
    let (muted, prose) = (t.style(Role::Muted).bg(t.raised), t.style(Role::Prose).bg(t.raised));
    let name = self.client.machines.get(at).map_or("", |one| one.name.as_str());
    let machine = if at == 0 { "This computer" } else { name };
    let project = self.folder_named(at, folder);
    // A session with no project has no badge to give way to, so its name stays, cut if it must.
    let alone = project == NO_PROJECT;
    let badge = if alone { run("", raised) } else { self.badge(&project) };
    let gap = || run(" ".repeat(APART), raised);
    let mut left: Line = switch.into();
    let fixed = width(&left) + APART + 2 + APART + badge.content.width() + 2;
    // The names give way before the effort, the one control whose label is its value, which a
    // click changes in place: the turn gives way to its mark, then the name of the machine to a
    // few letters, then the name of the project to its badge, and the effort goes last.
    let room = self.wide().saturating_sub(BARRED + 4);
    let spare = |right: &(Line, _)| room.checked_sub(width(&right.0) + 2 + fixed);
    let (whole, least) = (machine.width() + 1 + project.width(), SHORTEST + 2 + project.width());
    let tries = [(0, whole), (1, whole), (1, least), (1, SHORTEST)];
    let (short, need) = tries
      .into_iter()
      .find(|(short, need)| spare(&self.controls(*short)).is_some_and(|spare| spare >= *need))
      .unwrap_or((2, SHORTEST));
    let (right, clicks) = self.controls(short);
    let spare = room.saturating_sub(width(&right) + 2 + fixed);
    // The machine leaves the name of the project its room while the name stays.
    let kept = if need > SHORTEST { project.width() + 1 } else { 0 };
    let machine = cut(machine, spare.saturating_sub(kept));
    // A name cut to less than a few letters says nothing that the badge does not.
    let name = match spare.saturating_sub(machine.width()) {
      rest if alone => cut(&project, rest.max(SHORTEST)),
      rest if rest > SHORTEST => format!(" {}", cut(&project, rest - 1)),
      _ => String::new(),
    };
    left.extend([
      gap(),
      run(machine, prose),
      run(" ▾", muted),
      gap(),
      badge,
      run(name, prose),
      run(" ▾", muted),
    ]);
    let line = spread(left, right, room, raised);
    let at = columns(&line);
    let spans = [((0, 1), Does::Python), ((3, 4), Does::Machines), ((6, 8), Does::Projects)];
    let mut hot: Vec<Hot> =
      spans.into_iter().filter_map(|(runs, does)| spanned(&at, runs, Click::Key(does))).collect();
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
      let turn = run(if short > 0 { mark } else { words }, prose);
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
      Target::New(at, folder) => {
        return format!("{} · new session", self.folder_named(at, &folder));
      }
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
  /// that says something with each image that it names as its mark, as the daemon titles a card.
  pub(crate) fn title(&self) -> Option<String> {
    let opened = self.client.thread.as_ref()?;
    let card = self.client.card(&opened.thread.key).map(|card| &card.thread);
    let card = card.or_else(|| opened.sub().map(|one| &one.card));
    let card = card.or_else(|| opened.agent()).map(|card| card.title.clone());
    let first = || opened.thread.exchanges.get(&0).map(|one| one.text.clone());
    // A message about a line is titled by what it says of the line, as the daemon titles its card.
    let pending = || {
      let text = &opened.thread.pending.first()?.text;
      Some(Anchor::of(text).map_or(text.as_str(), |(_, said)| said).to_owned())
    };
    let title = card.or_else(first).or_else(pending)?;
    title.lines().find(|line| !line.trim().is_empty()).map(picture::shown)
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
    let target = self.client.target();
    if let Some(chosen) = self.client.chosen.as_ref().filter(|one| one.aims(&target)) {
      return Some(leveled(&chosen.actor));
    }
    let (at, life) = match target {
      Target::Thread(key) => {
        // A word of Python that the operator ran on the thread asked no model, so the model of the
        // thread is the one of the last block that a model wrote. A thread whose page has not come
        // yet stands on the model of its session.
        let opened = self.client.thread.iter().filter(|one| one.thread.key == key);
        let exchanges = opened.flat_map(|one| one.thread.exchanges.values().rev());
        let blocks = exchanges.filter_map(|one| one.block.as_ref());
        let model = blocks.map(|one| &one.maker).find_map(|maker| match maker {
          Maker::Model { model, effort } => Some((model.clone(), effort.clone())),
          _ => None,
        });
        if model.is_some() {
          return model;
        }
        (key.0, key.1)
      }
      Target::Session(at, life) => (at, life),
      Target::New(at, folder) => return self.opens_on(at, &folder),
    };
    let cards = self.client.machines.get(at).map(|told| &told.cards[..]).unwrap_or_default();
    let mut ours = cards.iter().rev().filter(|card| card.life == life);
    let named = ours.find_map(|card| card.thread.model.as_deref()).map(leveled);
    // A life that the window opens names no model before its first answer: it stands on the actor
    // that he chose for it, else on the one that a new session of its folder takes.
    let hold = self.client.holds.get(&(at, life));
    let opened_on = || {
      let hold = hold?;
      hold.actor.as_deref().map(leveled).or_else(|| self.opens_on(at, hold.cwd.as_deref()?))
    };
    // A card names the model of its thread with no effort: the life stands on the effort that it
    // opened on, else on the default effort of that model.
    let effort = |model: &str| {
      let opened = opened_on().filter(|one| one.0 == model).and_then(|one| one.1);
      opened.or_else(|| self.default_effort(at, model))
    };
    match named {
      Some((model, None)) => {
        let effort = effort(&model);
        Some((model, effort))
      }
      Some(named) => Some(named),
      None => opened_on(),
    }
  }

  /// The name of the project of a folder of a machine where a new session runs: the last part of
  /// the folder, or that it has none for a folder in that of the sessions with no project.
  pub(crate) fn folder_named(&self, at: usize, folder: &str) -> String {
    if self.client.alone(at, folder) { NO_PROJECT.to_owned() } else { named(folder) }
  }

  /// The effort that a model of the catalog of a machine takes when none is named.
  fn default_effort(&self, at: usize, model: &str) -> Option<String> {
    let offered = self.client.machines.get(at)?.models.as_deref().unwrap_or_default();
    let same = |one: &&Offered| one.name == model || one.name.rsplit(':').next() == Some(model);
    offered.iter().find(same).and_then(|one| one.effort.clone())
  }

  /// The actor that a new session of a folder of a machine opens on: the model of the last
  /// session of its project, else the default actor of the catalog of the machine.
  fn opens_on(&self, at: usize, folder: &str) -> Option<(String, Option<String>)> {
    let told = self.client.machines.get(at)?;
    let here = self.folder_named(at, folder);
    let mut ours = told.cards.iter().rev().filter(|card| project(card) == here);
    let offered = told.models.as_deref().unwrap_or_default();
    // A card names the model of its thread with no effort, and a new session on that model
    // stands on its default effort, which the catalog of the machine gives.
    let last = ours.find_map(|card| card.thread.model.as_deref()).map(leveled);
    let last = last.map(|(model, chosen)| {
      let effort = chosen.or_else(|| self.default_effort(at, &model));
      (model, effort)
    });
    last.or_else(|| offered.first().map(|one| (one.name.clone(), one.effort.clone())))
  }

  /// An actor that the operator chose for the target of the input: a new session opens on it, and
  /// a life that stands takes it on its root, as a word of the operator.
  pub(crate) fn took_actor(&mut self, actor: String) {
    self.client.chosen = Some(Chosen::of(&self.client.target(), actor));
  }

  /// The place that the next message is about, while the operator writes it, with the key that
  /// turns it back to a plain message.
  fn anchored(&self) -> Option<Line> {
    let t = &self.tokens;
    let line = shown_at(self.client.draft.anchor.as_ref()?);
    let drop = format!("  ·  {} a plain message", self.key(Does::Close));
    let (accent, prose, muted) =
      (Style::new().fg(t.accent), t.style(Role::Prose), t.style(Role::Muted));
    Some(vec![run("  ◆ ", accent), run(line, prose), run(drop, muted)])
  }

  /// The hint of a draft that holds a long text that a paste folded: the same paste shows it whole.
  fn folded(&self) -> Option<Line> {
    let draft = &self.client.draft;
    draft.folds.iter().find(|one| draft.text.contains(&one.mark()))?;
    let t = &self.tokens;
    let (accent, muted) = (Style::new().fg(t.accent), t.style(Role::Muted));
    Some(vec![run(format!("  {FOLDED}"), accent), run("paste it again to show it whole", muted)])
  }

  /// The key that the right of the row of hints names, and what it does.
  fn hint_key(&self) -> (Does, &'static str) {
    match (&self.client.aim, &self.client.thread) {
      _ if self.form().is_some() => (Does::Empty, "refuse"),
      (None, Some(_)) if self.leaves() => (Does::Close, "back"),
      (None, Some(opened)) if !opened.docked().1.is_empty() => (Does::Agent, "agents"),
      (_, Some(_)) => (Does::GoTo, "find"),
      (_, None) if self.listed() => (Does::GoTo, "find"),
      (_, None) => (Does::List, "the list"),
    }
  }

  /// The row of hints: why the daemon refused the last message, else the keys of a form, of the
  /// completion or of the bar of the time, else the word of python that the window ran, as it runs
  /// and what it came to, else the state of the open thread while it works, with the key that stops
  /// it, or while a pause holds it, with what resumes it, else the keys of the mode of Python while
  /// it is on, else the folder of the window; and at its right how many other threads ask the
  /// operator, and the key that turns the input, or that refuses the question of a form.
  pub(crate) fn hints(&self) -> Row {
    let t = &self.tokens;
    // The row keeps two blank columns at each side, as the text of the box of the input above it.
    let wide = self.wide().saturating_sub(4);
    let muted = t.style(Role::Muted);
    let card = self.client.thread.as_ref().and_then(|one| self.client.card(&one.thread.key));
    let ended = self
      .client
      .thread
      .as_ref()
      .is_some_and(|one| self.client.fault_of(one.thread.key.0, &one.thread.key.1).is_some());
    let works = card.is_some_and(|card| card.thread.works()) && !ended;
    let stands = card.and_then(|card| self.stands(card)).filter(|_| !ended);
    let mut hot = Vec::new();
    // The thread says why it has no page, so the row says it once more only for another fault.
    let shown = self.client.thread.as_ref().and_then(|one| self.why(one));
    let notice = self.client.notice.clone().map(|one| (one, t.muted));
    let (does, name) = self.hint_key();
    let key = vec![run(format!("{} {name}", self.key(does)), muted)];
    // A second press says what it does in the room that the key at the right leaves.
    let armed = self.armed_says(wide.saturating_sub(2 + width(&key) + 2));
    // While his voice records, the record takes the row, over any notice.
    let notice = notice.filter(|_| self.listening.is_none());
    // The moment under the pointer on the bar of the time, or the one he walks to, takes the place
    // of a notice while he is on the bar.
    let moment = self.time_said().map(|one| (one, t.text));
    let says = armed.map(|one| (one, t.attention)).or(moment).or(notice);
    let fault = self.fault.as_ref().filter(|one| Some(one.as_str()) != shown);
    // The keys of the input go first, then the line of a file that the next message is about.
    let keyed = self.over_input().or_else(|| self.anchored()).or_else(|| self.folded());
    let room = wide.saturating_sub(width(&key) + 2);
    let word = Some(self.word_hints(room)).filter(|(hints, _)| !hints.is_empty());
    let python = self.python_hints().filter(|_| !works);
    let place = fault.is_none() && says.is_none() && keyed.is_none() && word.is_none();
    let place = place && python.is_none() && stands.is_none();
    let (left, loaders) = match (fault, says) {
      (Some(fault), _) => (vec![run(format!("  {fault}"), Style::new().fg(t.danger))], vec![]),
      (None, Some((says, color))) => {
        (vec![run(format!("  {says}"), Style::new().fg(color))], vec![])
      }
      (None, None) if keyed.is_some() => (keyed.unwrap_or_default(), vec![]),
      (None, None) if let Some((hints, loader)) = word => (hints, loader.into_iter().collect()),
      (None, None) if let Some(stands) = stands => {
        hot.extend(stands.hot);
        (stands.line, stands.loaders)
      }
      (None, None) if let Some(python) = python => (python, vec![]),
      // The place, which takes the room that the right of the row leaves, below.
      (None, None) => (Vec::new(), vec![]),
    };
    // The machine and the folder of a thread that rests give way to the threads that ask him, which
    // the row never hides.
    let kept = if place { 2 } else { width(&left) };
    let room = wide.saturating_sub(kept + width(&key) + 2);
    let others = self.others_ask(room);
    let right: Line = others.iter().chain(&key).cloned().collect();
    let left = if place { self.place(wide.saturating_sub(2 + width(&right))) } else { left };
    let line = spread(left, right.clone(), wide, Style::new());
    hot.extend(branched(&line).filter(|_| place));
    let (end, start) = (width(&line), width(&line) - width(&right));
    if !others.is_empty() {
      hot.push((start..start + width(&others) - 3, Some(Click::Key(Does::Needed))));
    }
    hot.push((end - width(&key)..end, Some(Click::Key(does))));
    Row { hot, loaders, ..Row::of(line) }
  }

  /// The hint of the open thread while it works, with the key that stops it, or while a pause holds
  /// it, with what resumes it, in the color that its card gives it: the key of the pause that he
  /// said, or a message, for one that furb said. None while it rests.
  fn stands(&self, card: &Card) -> Option<Row> {
    let muted = self.tokens.style(Role::Muted);
    if card.thread.works() {
      let (line, loader) = self.working(card);
      let at = columns(&line);
      let hot = at
        .get(1)
        .map(|one| (one.start + 3..one.end, Some(Click::Key(Does::Stop))))
        .into_iter()
        .collect();
      return Some(Row {
        hot,
        loaders: vec![(2, loader), (4, Loader::Light(WORKS))],
        ..Row::of(line)
      });
    }
    let Standing::Paused { by, .. } = &card.thread.standing else { return None };
    let key = self.key(Does::Pause);
    let color = if operator(by) { self.tokens.muted } else { self.tokens.attention };
    let resumes =
      if self.only_reads() { String::new() } else { format!(" · {key} or a message resumes it") };
    let line = vec![run("  ‖ Paused", Style::new().fg(color)), run(resumes, muted)];
    let at = width(line.get(..1).unwrap_or_default()) + 3;
    Some(Row { hot: vec![(at..at + key.width(), Some(Click::Key(Does::Pause)))], ..Row::of(line) })
  }

  /// Whether the open thread stands on a machine where the window only reads.
  fn only_reads(&self) -> bool {
    let at = self.client.thread.as_ref().map(|one| one.thread.key.0);
    at.is_some_and(|at| self.client.only_reads(at).is_some())
  }

  /// The hint of the work of the open thread: its time, which counts each second as the time of a
  /// line that runs, where a card says its age in minutes, the key that starts to stop it, whose
  /// second press the row then asks, and the mark of that work: a model, a command or agents.
  fn working(&self, card: &Card) -> (Line, Loader) {
    let t = &self.tokens;
    let worked = Some(self.second().saturating_sub(card.since)).filter(|one| *one > 0);
    let age = worked.map(|seconds| format!(" {}", timer(seconds)));
    let working = format!("    {WORKS}{}", age.unwrap_or_default());
    // A window that only reads stops nothing.
    let stop =
      if self.only_reads() { String::new() } else { format!(" · {} stop", self.key(Does::Close)) };
    let loader = match &card.thread.standing {
      Standing::Runs => Loader::Command,
      Standing::Delegates => Loader::Agent,
      _ => Loader::Model,
    };
    (vec![run(working, Style::new().fg(t.accent)), run(stop, t.style(Role::Muted))], loader)
  }

  /// Where the input runs in a room of columns: the machine and the folder, or the folder alone,
  /// and the branch of its checkout with the count of the files that differ from its last commit.
  /// On a thread the folder is that of its chain, where the next word runs, which a cd of his or
  /// of a model moves. A place that does not fit says its folder alone, which tells more than the
  /// machine, and a folder that still does not fit keeps its end, which names it, as `…/docs`.
  /// The launcher of a new session names its machine, so the row says the folder alone.
  fn place(&self, room: usize) -> Line {
    let muted = self.tokens.style(Role::Muted);
    if let Some(filled) = &self.client.filled {
      let skill = self.client.views.iter().find(|one| one.viewer == filled.viewer);
      let skill = skill.map(|one| one.skill.as_str()).unwrap_or_default();
      return vec![run(format!("  The input fills {} of the view {skill}", filled.node), muted)];
    }
    let at = match self.client.target() {
      Target::Thread(key) => key.0,
      Target::Session(at, _) | Target::New(at, _) => at,
    };
    let named = self.client.machines.get(at).map_or("", |one| one.name.as_str());
    let name = if at == 0 { "This computer" } else { named };
    // A session on another machine, or with no project, runs in a folder of its own, not in the
    // folder of the window.
    let elsewhere = self
      .client
      .target_folder()
      .filter(|(at, one)| *at != 0 || self.folder_named(*at, one) == NO_PROJECT);
    let folder = elsewhere.map_or_else(|| self.client.folder.clone(), |(_, one)| one);
    // On a thread, the next word runs in the folder of its chain, which a cd moves.
    let opened =
      self.client.thread.as_ref().filter(|_| matches!(self.client.target(), Target::Thread(_)));
    let folder = opened.and_then(|one| one.thread.folder.clone()).unwrap_or(folder);
    let branch = self.client.target_folder().and_then(|(at, folder)| {
      let changes = self.client.changes(at, &folder)?;
      let count = (!changes.files.is_empty()).then(|| format!(" ±{}", changes.files.len()));
      Some(format!(" · {BRANCH}{}{}", changes.branch.as_deref()?, count.unwrap_or_default()))
    });
    let folder = self.folder_shown(at, &folder);
    let room = room.saturating_sub(2 + branch.as_deref().map_or(0, Cells::width));
    let new = matches!(self.client.target(), Target::New(..));
    let machine = !new && name.width() + 3 + folder.width() <= room;
    let folder = tail(&folder, room);
    let text = if machine { format!("  {name} · {folder}") } else { format!("  {folder}") };
    let mut line = vec![run(text, muted)];
    line.extend(branch.map(|one| run(one, muted)));
    line
  }

  /// The keys of the mode of Python while it is on: the key that runs the word, and the key that
  /// turns the input back to a message.
  fn python_hints(&self) -> Option<Line> {
    let keys = format!("  {} run · {} a message", self.key(Does::Send), self.key(Does::Python));
    self.client.draft.python.then(|| vec![run(keys, self.tokens.style(Role::Muted))])
  }

  /// The hints of the word that the window ran, in a room of columns: a mark that moves while it
  /// runs, then a check when it ran, or its fault, and its text on one line, cut with the count of
  /// the characters that the cut hides. A word that the open thread shows has its trace there.
  fn word_hints(&self, room: usize) -> (Line, Option<(usize, Loader)>) {
    let t = &self.tokens;
    let muted = t.style(Role::Muted);
    // Each life names its rungs alike, so the open thread shows the word only on its own life.
    let shows = |word: &&Word| {
      self.client.thread.as_ref().is_some_and(|one| {
        let key = &one.thread.key;
        let mut exchanges = one.thread.exchanges.values();
        (key.0, &key.1) == (word.at, &word.life) && exchanges.any(|one| one.message == word.rung)
      })
    };
    let Some(word) = self.word.as_ref().filter(|word| !shows(word)) else {
      return (Vec::new(), None);
    };
    // The word that names the session reads as the command that he wrote.
    let text = said(&word.text)
      .map_or_else(|| word.text.trim().replace('\n', "\\n"), |name| format!("{RENAME} {name}"));
    let (head, loader) = match &word.ran {
      None => (vec![run("    ", muted)], Some((2, Loader::Command))),
      Some(RunState::Running | RunState::Done) => (vec![run("  ✓ ", muted)], None),
      Some(RunState::Cancelled(_)) => (vec![run("  ✗ Cancelled · ", muted)], None),
      Some(RunState::Failed(why)) => {
        let why = reason(why);
        (vec![run(format!("  ✗ {why}"), Style::new().fg(t.danger)), run(" · ", muted)], None)
      }
    };
    (clipped([head, vec![run(text, muted)]].concat(), room, muted), loader)
  }
}

/// The parts of a row of the box of the input that take a click: the runs at its end, by their
/// columns in the line, each with its click. The box draws the line two columns in.
fn clicked(at: &[Range<usize>], clicks: Vec<Option<Click>>) -> Vec<Hot> {
  let first = at.len().saturating_sub(clicks.len());
  let each = clicks.into_iter().enumerate();
  each.filter_map(|(n, click)| spanned(at, (first + n, first + n), click?)).collect()
}

/// The columns of a row of the box of the input from one run of its line to another, with its
/// click; none when the line holds fewer runs. The box draws the line two columns in.
fn spanned(at: &[Range<usize>], (first, last): (usize, usize), click: Click) -> Option<Hot> {
  Some((at.get(first)?.start + BARRED..at.get(last)?.end + BARRED, Some(click)))
}

/// The name of a project, from its folder.
pub(crate) fn named(folder: &str) -> String {
  Path::new(folder).file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default()
}

/// The columns of the branch and the count of the changes in the place of the input, which open
/// the review.
fn branched(line: &Line) -> Option<Hot> {
  let at = line.iter().position(|one| one.content.contains(BRANCH))?;
  let at = columns(line).into_iter().nth(at)?;
  Some((at.start + " · ".width()..at.end, Some(Click::Key(Does::Changes))))
}

/// An actor as its model and its effort, which it names after its last slash.
fn leveled(actor: &str) -> (String, Option<String>) {
  let (name, effort) = actor.rsplit_once('/').map_or((actor, None), |(one, two)| (one, Some(two)));
  (name.to_owned(), effort.map(str::to_owned))
}

/// A path in a room of columns: whole when it fits, else its end after `…` from a separator,
/// which names its last folders, else its start, cut as a text is.
fn tail(path: &str, room: usize) -> String {
  if path.width() <= room {
    return path.to_owned();
  }
  let ends = path.match_indices(['/', '\\']).filter_map(|(at, _)| path.get(at..));
  let end = ends.into_iter().find(|end| end.width() < room);
  end.map_or_else(|| cut(path, room), |end| format!("…{end}"))
}

#[cfg(test)]
#[path = "input.test.rs"]
mod tests;
