//! The time of the open thread: while the operator holds its bar, he steps through each moment at
//! which an act that its page shows began or ended, and the thread shows what it was just after
//! that moment, page by page, as the daemon folds it. A play steps forward on its own until now.
//! Under the bar, the chains of the life stand on the same moments: each that works, and each
//! that a text that he types finds.

use std::{
  collections::{BTreeMap, HashMap},
  mem,
  time::Duration,
};

use furb_wire::{
  Began, ChainTrack, Exchange, State,
  protocol::{Answer, Frame, Verb},
};
use serde_json::Value;

use crate::{Asked, Client, Key, ranked};

/// How long a play shows each moment.
const STEP: Duration = Duration::from_millis(120);

/// The bar of the time of a thread: the moments that its machine gave, the moment that the thread
/// shows, none for now, whether they changed since the machine gave them, when the play last
/// stepped while it plays, the first line of each message that a page of the thread gave, by the
/// moment that it was said, which the track marks, and the chains of the life while they show.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Time {
  pub moments: Vec<u64>,
  pub at: Option<u64>,
  pub stale: bool,
  pub played: Option<Duration>,
  pub messages: BTreeMap<u64, String>,
  pub chains: Option<Chains>,
}

/// The chains of a life under the bar of the time of a thread: what its machine gave, whether the
/// life changed since, and the text that finds a chain by its label.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Chains {
  pub tracks: Vec<ChainTrack>,
  pub stale: bool,
  pub find: String,
}

impl Chains {
  /// The chains that show, each with how deep it stands: each that works or waits, as a tree
  /// while no text finds, each chain under the chain whose word made it, which shows too, in the
  /// order the chains began; else each whose label the text finds, the best first. A chain that
  /// rests and made no chain that shows shows only when a text finds it.
  pub fn shown(&self) -> Vec<(&ChainTrack, usize)> {
    if !self.find.is_empty() {
      let labels = self.tracks.iter().map(|one| one.label.as_str());
      let found = ranked(&self.find, labels).into_iter();
      return found.map(|one| (&self.tracks[one.at], 0)).collect();
    }
    let count = self.tracks.len();
    let places: HashMap<&str, usize> =
      self.tracks.iter().enumerate().map(|(at, one)| (one.chain.as_str(), at)).collect();
    let parent = |at: usize| {
      let made = self.tracks[at].parent.as_deref().and_then(|one| places.get(one).copied());
      made.filter(|one| *one != at)
    };
    // Each chain that works shows, and each chain whose word made it, up the tree.
    let mut keep = vec![false; count];
    for start in (0..count).filter(|at| !resting(self.tracks[*at].state)) {
      let mut at = Some(start);
      while let Some(one) = at.filter(|one| !keep[*one]) {
        keep[one] = true;
        at = parent(one);
      }
    }
    let mut under = vec![Vec::new(); count];
    let mut stack = Vec::new();
    for at in 0..count {
      match parent(at) {
        Some(made) => under[made].push(at),
        None => stack.push((at, 0)),
      }
    }
    stack.reverse();
    let mut seen = vec![false; count];
    let mut out = Vec::new();
    while let Some((at, depth)) = stack.pop() {
      if mem::replace(&mut seen[at], true) || !keep[at] {
        continue;
      }
      out.push((&self.tracks[at], depth));
      stack.extend(under[at].iter().rev().map(|one| (*one, depth + 1)));
    }
    out
  }
}

/// The label of the chain of a rewind, by which the line of its word says that it branches.
const REWIND: &str = "rewind";

/// Whether the text of a line is the word of a rewind, which branches its thread.
pub(crate) fn rewinds(line: &str) -> bool {
  line.contains(&format!("on=chain('{REWIND}'"))
}

/// Whether a chain rests: its work ended, or never began.
pub fn resting(state: State) -> bool {
  matches!(state, State::Done | State::Idle | State::Cancelled)
}

impl Time {
  /// The place among the moments of the moment that the thread shows: the last for now, and for a
  /// moment between two of them, as a chain opens it, the next one.
  pub fn place(&self) -> usize {
    let last = self.moments.len().saturating_sub(1);
    self.at.map_or(last, |at| self.moments.partition_point(|one| *one < at).min(last))
  }

  /// The messages of exchanges that a page gave: a word of Python by the first line of its code.
  pub fn heard<'a>(&mut self, exchanges: impl IntoIterator<Item = &'a Exchange>) {
    for one in exchanges {
      let code = one.block.as_ref().and_then(|block| block.lines.first()).map(|line| &line.text);
      let text = if one.python { code.map_or("", String::as_str) } else { &one.text };
      self.messages.insert(one.said_at, text.lines().next().unwrap_or_default().to_owned());
    }
  }

  /// The message in force at a place among the moments: the last one said by then.
  pub fn message(&self, at: usize) -> Option<&str> {
    let moment = *self.moments.get(at)?;
    self.messages.range(..=moment).next_back().map(|(_, text)| text.as_str())
  }

  /// The moment at a place among them: none for the last, which is now.
  fn placed(&self, at: usize) -> Option<u64> {
    self.moments.get(at).copied().filter(|_| at + 1 < self.moments.len())
  }

  /// The moment a count of moments later, back for a negative count: never before the first, and
  /// now past the last. A moment between two of them steps to the next one first.
  fn stepped(&self, by: isize) -> Option<u64> {
    let (last, from) = (self.moments.len().saturating_sub(1), self.place());
    let between = self.at.is_some_and(|at| self.moments.get(from) != Some(&at));
    let by = if between && by > 0 { by - 1 } else { by };
    self.placed(from.saturating_add_signed(by).min(last))
  }
}

impl Client {
  /// The bar of the time of the open thread opened at now, or closed back to now.
  pub fn time(&mut self) {
    let Some(opened) = self.thread.as_mut() else { return };
    let thread = &mut opened.thread;
    if thread.time.take().is_some_and(|time| time.at.is_some()) {
      thread.moved();
    } else if !thread.key.2.is_empty() {
      let mut time = Time { stale: true, ..Time::default() };
      time.heard(thread.exchanges.values());
      thread.time = Some(time);
    }
  }

  /// The open thread a count of moments later, back for a negative count. The play stops.
  pub fn step(&mut self, by: isize) {
    self.move_to(|time| time.stepped(by));
  }

  /// The open thread at a moment by its place among them, as a click on its bar gives it: the
  /// last is now. The play stops.
  pub fn reach(&mut self, at: usize) {
    self.move_to(|time| time.placed(at));
  }

  fn move_to(&mut self, to: impl FnOnce(&Time) -> Option<u64>) {
    let Some(thread) = self.thread.as_mut().map(|one| &mut one.thread) else { return };
    let Some(time) = thread.time.as_mut() else { return };
    let at = to(time);
    time.played = None;
    if at != time.at {
      time.at = at;
      thread.moved();
    }
  }

  /// The play of the open thread started from where it shows, from the first moment when it shows
  /// now, or stopped.
  pub fn play(&mut self) {
    let moved = self.moved;
    let Some(thread) = self.thread.as_mut().map(|one| &mut one.thread) else { return };
    let Some(time) = thread.time.as_mut() else { return };
    if time.played.take().is_some() {
      return;
    }
    time.played = Some(moved);
    if time.at.is_none() && time.moments.len() > 1 {
      time.at = time.placed(0);
      thread.moved();
    }
  }

  /// The play of the open thread one moment on when its step passed; at now it stops.
  pub fn played(&mut self) {
    let moved = self.moved;
    let Some(thread) = self.thread.as_mut().map(|one| &mut one.thread) else { return };
    let Some(time) = thread.time.as_mut() else { return };
    let Some(last) = time.played.filter(|last| moved.saturating_sub(*last) >= STEP) else { return };
    time.at = time.stepped(1);
    time.played = time.at.map(|_| last + STEP);
    thread.moved();
  }

  /// Whether the open thread plays its time.
  pub fn plays(&self) -> bool {
    let time = self.thread.as_ref().and_then(|one| one.thread.time.as_ref());
    time.is_some_and(|time| time.played.is_some())
  }

  /// The word that sends a message of the operator from the moment that the open thread shows, as
  /// a rewind: a new chain stands on the chain of the thread with the messages of the thread until
  /// that moment and what they made, and the message asks its model there. The bar goes back to
  /// now, where the thread shows the word on its line and the agent that it made under it. The
  /// source keeps each act, so the thread as it is now stays a branch. None while the thread shows
  /// now; a refusal while the window does not hold each message until that moment, since a rewind
  /// that the window would make then drops the messages that it does not hold.
  pub(crate) fn rewound(&mut self, text: &str) -> Option<Result<String, String>> {
    let thread = &mut self.thread.as_mut().filter(|_| self.aim.is_none())?.thread;
    if !thread.past() {
      return None;
    }
    if thread.above != Some(0) {
      let why = "Scroll up to the first message of the thread, then send from this moment again.";
      return Some(Err(why.to_owned()));
    }
    let kept: Vec<String> =
      thread.exchanges.values().map(|one| Value::String(one.message.clone()).to_string()).collect();
    let (text, kept) = (Value::String(text.to_owned()), kept.join(", "));
    thread.time = None;
    thread.moved();
    Some(Ok(format!(
      "prompt(str, {text}, on=chain('{REWIND}', source=__name__, filter=take({kept})))"
    )))
  }

  /// The ask of the moments of the open thread, while its bar shows and they changed since its
  /// machine gave them.
  pub(crate) fn moments_due(&self) -> Option<(usize, Verb, Asked)> {
    let thread = &self.thread.as_ref()?.thread;
    let asked = Asked::Moments(thread.key.clone());
    let asking = self.asked.values().any(|one| *one == asked);
    let (life, on) = (thread.key.1.clone(), Some(thread.key.2.clone()));
    (thread.time.as_ref()?.stale && !asking)
      .then(|| (thread.key.0, Verb::Moments { life, on }, asked))
  }

  /// What a machine said to the ask of the moments of the bar of a thread. A refusal or a lost
  /// machine gives none, so the bar stays at now.
  pub fn timed(&mut self, key: &Key, heard: Option<Frame>) {
    let Some(opened) = self.thread.as_mut().filter(|one| one.thread.key == *key) else { return };
    let Some(time) = opened.thread.time.as_mut() else { return };
    time.moments = match heard {
      Some(Frame::Reply { answer: Answer::Moments(moments), .. }) => moments,
      _ => Vec::new(),
    };
    time.stale = false;
    let last = time.moments.last().copied();
    // A moment at or past the last of the thread is now, where the page follows each change.
    if time.at.take_if(|at| last.is_none_or(|last| *at >= last)).is_some() {
      opened.thread.moved();
    }
  }

  /// The chains of the life under the bar of the time of the open thread shown, with the bar,
  /// or hidden. The text that finds a chain starts empty.
  pub fn chains(&mut self) {
    if self.thread.as_ref().is_some_and(|one| one.thread.time.is_none()) {
      self.time();
    }
    let Some(time) = self.thread.as_mut().and_then(|one| one.thread.time.as_mut()) else { return };
    time.chains = match time.chains.take() {
      Some(_) => None,
      None => Some(Chains { stale: true, ..Chains::default() }),
    };
  }

  /// The text that finds a chain under the bar, changed, while the chains show.
  pub fn find(&mut self, change: impl FnOnce(&mut String)) {
    let time = self.thread.as_mut().and_then(|one| one.thread.time.as_mut());
    if let Some(chains) = time.and_then(|time| time.chains.as_mut()) {
      change(&mut chains.find);
    }
  }

  /// The thread to open from a row of the chains under the bar at a place among the moments of
  /// the bar: the last thread of that chain that began by that moment, else its first, and that
  /// moment, none for now.
  pub fn chain_thread(&self, row: usize, at: usize) -> Option<(String, Option<u64>)> {
    let time = self.thread.as_ref()?.thread.time.as_ref()?;
    let track = time.chains.as_ref()?.shown().get(row)?.0;
    let moment = time.placed(at);
    let began = |one: &&Began| moment.is_none_or(|moment| one.at <= moment);
    let thread = track.threads.iter().rev().find(began).or(track.threads.first())?;
    Some((thread.prompt.clone(), moment))
  }

  /// The ask of the chains of the life of the open thread, while they show under its bar and the
  /// life changed since its machine gave them.
  pub(crate) fn tracks_due(&self) -> Option<(usize, Verb, Asked)> {
    let thread = &self.thread.as_ref()?.thread;
    let chains = thread.time.as_ref()?.chains.as_ref()?;
    let asked = Asked::Tracks(thread.key.clone());
    let asking = self.asked.values().any(|one| *one == asked);
    let life = thread.key.1.clone();
    (chains.stale && !asking).then(|| (thread.key.0, Verb::Tracks { life }, asked))
  }

  /// The chains under the bar asked again when a frame changed the cards of their life.
  pub(crate) fn retracked(&mut self, at: usize, frame: &Frame) {
    let Frame::Cards { life, .. } = frame else { return };
    let thread = self.thread.as_mut().map(|one| &mut one.thread).filter(|one| one.key.0 == at);
    let time = thread.filter(|one| one.key.1 == *life).and_then(|one| one.time.as_mut());
    if let Some(chains) = time.and_then(|time| time.chains.as_mut()) {
      chains.stale = true;
    }
  }

  /// What a machine said to the ask of the chains of a life. A refusal or a lost machine gives
  /// none, so no chain shows.
  pub fn tracked(&mut self, key: &Key, heard: Option<Frame>) {
    let Some(opened) = self.thread.as_mut().filter(|one| one.thread.key == *key) else { return };
    let time = opened.thread.time.as_mut();
    let Some(chains) = time.and_then(|time| time.chains.as_mut()) else { return };
    chains.tracks = match heard {
      Some(Frame::Reply { answer: Answer::Tracks(tracks), .. }) => tracks,
      _ => Vec::new(),
    };
    chains.stale = false;
  }
}
