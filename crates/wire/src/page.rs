//! A page of a thread: some exchanges of it, each message with the block of code that answers it
//! and what the answer says to the operator, with the agents of the thread and the context of its
//! chain.

use std::ops::Range;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
  act::{Act, Maker, Read, fault},
  anchor::Anchor,
  card::Card,
  protocol::{Node, Slot},
  words::timer,
};

/// Some exchanges of a thread, the oldest first, with how many exchanges stand before them, the
/// agents of the whole thread, in the order the thread made them, the context of its chain, and,
/// for a sub thread, which no list holds, the thread itself as the thread that it is on holds it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Page {
  pub exchanges: Vec<Exchange>,
  pub above: usize,
  pub agents: Vec<Agent>,
  #[serde(default)]
  pub context: Option<Context>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub sub: Option<Box<Agent>>,
  /// The folder of the chain of the thread, where its next word runs: the last that a cd of the
  /// chain came to, a cd of its setup first.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub folder: Option<String>,
  /// The views of the session that stand open, in the order that their words drew them.
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub views: Vec<Viewed>,
}

/// The version of the vocabulary of a view, which the message of its prompt names.
pub const VIEW: u64 = 1;

/// A view that a word draws for the operator: a prompt to him, open, whose message is a tree of
/// elements in a slot of a window, as `{"view": 1, "slot": "footer", "tree": {...}}`, and whose
/// answer is what a person or a model did to one of its nodes, as
/// `{"node": "add", "value": "milk"}`. So the record keeps the view and each gesture, and a
/// replay draws it again.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Viewed {
  pub prompt: String,
  #[serde(flatten)]
  pub slot: Slot,
  pub tree: Node,
}

/// The message of the prompt of a view.
#[derive(Deserialize)]
struct Drawn {
  view: u64,
  #[serde(flatten)]
  slot: Slot,
  tree: Node,
}

impl Viewed {
  /// The view that the message of a prompt draws, when it is one in a version that this build
  /// reads.
  #[must_use]
  pub fn of(prompt: &str, message: &str) -> Option<Self> {
    let drawn = serde_json::from_str::<Drawn>(message).ok().filter(|one| one.view == VIEW)?;
    Some(Self { prompt: prompt.to_owned(), slot: drawn.slot, tree: drawn.tree })
  }
}

/// The context of a chain at its last answer: what that answer read, the window that the roster of
/// the standing offers its actor, none when the roster holds no such actor, and the dollars of each
/// answer on the chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Context {
  pub read: Read,
  pub window: Option<u64>,
  pub dollars: f64,
}

/// A part of what an answer read, or the room that it left in the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Portion {
  Cached,
  Written,
  New,
  Free,
}

impl Context {
  /// The whole that a share of the context counts from: the window, or what the answer read when
  /// no window is known.
  pub fn whole(&self) -> u64 {
    self.window.unwrap_or(self.read.tokens).max(1)
  }

  /// The parts of what the last answer read, in the order of a bar, each with its tokens: what came
  /// from the cache, what it wrote to the cache, what was new, then the room that it left in the
  /// window when the window is known.
  pub fn portions(&self) -> Vec<(Portion, u64)> {
    let read = &self.read;
    let new = read.tokens.saturating_sub(read.cached + read.written);
    let mut all =
      vec![(Portion::Cached, read.cached), (Portion::Written, read.written), (Portion::New, new)];
    all.extend(self.window.map(|window| (Portion::Free, window.saturating_sub(read.tokens))));
    all
  }
}

/// An agent of a thread: a thread under it, which a word of the thread started on another chain,
/// or a sub thread, a prompt on it whose message carries an anchor. It holds its card, the
/// millisecond of the epoch at which the host saw it begin, the seconds that it took once it
/// ended, counted from milliseconds as the time of a thought, the anchor of a sub thread, and the
/// line of its last word
/// where it stands: the line that runs or asks, else the last one, and the number in the thread
/// of the line of the word that made it, none when no line of a word of the thread made it, and
/// the last line of the thought of its model while it thinks and writes no word yet. A host that
/// opened the life after the agent began saw it begin at that open.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Agent {
  #[serde(flatten)]
  pub card: Card,
  #[serde(default)]
  pub began: Option<u64>,
  #[serde(default)]
  pub took: Option<u64>,
  #[serde(default)]
  pub line: Option<Line>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub at: Option<usize>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub thought: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub anchor: Option<Anchor>,
}

/// One message of a thread with all that answers it: its place in the thread, 0 for the prompt of
/// the thread, the act of the message, the moment that it was said, who said it and its text,
/// whether it is a word of Python that an operator ran on the thread, whose block holds its code
/// and which has no text, whether the message still runs, the block of code that answers it once
/// a model answers, and what the answer says to the operator, in the order it was said. A message
/// that the operator said on a thread that works is done at once, since its model reads it at its
/// next step, so only one that asks still runs. A message is the prompt of the thread, what an
/// operator said on it, or a word that he ran on it, and what answers it is what came after it and
/// before the next message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Exchange {
  pub at: usize,
  pub message: String,
  #[serde(default)]
  pub said_at: u64,
  pub by: Maker,
  pub text: String,
  #[serde(default)]
  pub python: bool,
  #[serde(default)]
  pub runs: bool,
  pub block: Option<Block>,
  pub said: Vec<Said>,
}

/// The code of one answer: the model that wrote its last word, each line of its words, whether more
/// comes since the answer works, and the thought of its model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Block {
  pub maker: Maker,
  pub lines: Vec<Line>,
  pub more: bool,
  pub thought: Option<Thought>,
}

/// One line of a block: its number in the thread, its text, in which each quote stands as the
/// binding that it makes and a string of many lines as one line with its escapes, its mark, what
/// the last command that it awaited wrote, what each act that it made came to but a chain, in
/// the order it made them: one for each pass of a loop, the last of which is the value that it
/// bound, what each write of a file that it made changed, in the same order, the name of each act
/// that the line awaits while it runs: the act that it made last, or each act that its wait names,
/// and the text by which each value that its acts came to names an image, `![name](uri)`, as a
/// read of an image file gives it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Line {
  pub number: usize,
  pub text: String,
  pub mark: Mark,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub output: Option<Output>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub made: Vec<Made>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub writes: Vec<Written>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub awaits: Vec<String>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub pictures: Vec<String>,
}

/// What a write of a file that a line made changed: the name of its act, the path of the file, the
/// lines that it added and removed against the text of the path that the life knew before it, from
/// a read or a write of the path, and the first `DIFF` lines of their diff, as git writes a unified
/// diff with one line around each change but the header of its first hunk, with the count of the
/// lines past them. A write of a path that the life did not know before counts each of its lines as
/// added, and is not `known`, since the life does not know what stood there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Written {
  pub act: String,
  pub path: String,
  pub added: u32,
  pub removed: u32,
  pub known: bool,
  pub diff: Vec<String>,
  pub more: usize,
}

/// The most lines of the diff of a write that a line holds, which a block shows under it.
pub const DIFF: usize = 8;

/// What an act that a line made came to: it works, it failed, or it is done with a value, as a
/// literal of python cut to `LITERAL` characters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "made", content = "value", rename_all = "snake_case")]
pub enum Made {
  Works,
  Failed,
  Done(String),
}

/// The most characters of the value of an act that a line made, which a block shows beside it.
pub const LITERAL: usize = 80;

/// What a command that a line awaited wrote, as it writes it and once it ended: its last lines,
/// those of its stdout then those of its stderr, each as a terminal shows it, how many lines came
/// before them, and the seconds that it took once it ended, counted from milliseconds, which a
/// host that opened the life after the command began does not know.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Output {
  pub tail: Vec<String>,
  pub more: usize,
  pub took: Option<u64>,
  /// The name of each command whose lines these are, in the order that the line made them, from
  /// which a window asks what each wrote whole.
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub acts: Vec<String>,
}

/// Whether a fact says what a command wrote while it runs, which a page shows under its line.
pub fn wrote(fact: &Value) -> bool {
  fact.get(0).and_then(Value::as_str) == Some("out")
}

/// How a line stands: it ran, or will not run; it waits to run, as a line that its model wrote
/// before its word runs, or a line under the line where a running word stands; it runs, with the
/// work that it waits for and the millisecond of the epoch at which the host saw it begin, so a
/// time of a second shows after a whole second; it asks the operator; its model is asked again
/// after a call that failed; it failed, with the reason that a person reads; or a stop ended it, at
/// the millisecond of the epoch at which the host saw the stop, which a host that opened the life
/// after it does not know.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "is", rename_all = "snake_case")]
pub enum Mark {
  Ran,
  Waits,
  Runs { work: Work, began: Option<u64> },
  Asks,
  Retries(Retrying),
  Failed { why: String },
  Stopped { at: Option<u64> },
}

impl Retrying {
  /// What a line whose model is asked again says at a second of the epoch: the attempt that comes
  /// of the most, and the time left before it while it waits. The time left is never more than the
  /// wait, so a clock that stands behind the host shows the wait.
  pub fn said(&self, now: u64) -> String {
    let left = self.at.saturating_sub(now).min(self.wait);
    let wait = if left > 0 { format!(" in {}", timer(left)) } else { String::new() };
    format!("Attempt {} of {}{wait}", self.attempt, self.most)
  }
}

/// A call of a model that failed for a cause that passes, so its provider asks the model again:
/// the attempt that comes, the most attempts of the call, the seconds that the provider waits
/// before that attempt and the second of the epoch at which it comes, and why the attempt before
/// it failed. What the model wrote before is void.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Retrying {
  pub attempt: u32,
  pub most: u32,
  pub wait: u64,
  pub at: u64,
  pub why: String,
}

/// What a line that runs waits for: its model that writes it, a command or a wait, or an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Work {
  Writes,
  Command,
  Agent,
}

/// What a model thought before it wrote: the millisecond of the epoch at which it began to think,
/// as the host saw it begin, and in how many seconds it thought, which is none while it thinks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Thought {
  pub text: String,
  pub began: u64,
  pub took: Option<u64>,
}

/// How many rows of a block roll while it is folded: the window of the lines of a program.
pub const ROLLS: usize = 7;

impl Block {
  /// Whether the block takes more rows than roll, so it folds.
  pub fn rolls(&self) -> bool {
    let mut sums = self.lines.iter().scan(usize::from(self.more), |sum, line| {
      *sum += line.rows();
      Some(*sum)
    });
    sums.any(|sum| sum > ROLLS)
  }

  /// The lines of the block that show: each while it is open or while all fits, else those that
  /// fill the rows that roll around the line where its word stands, which stands as low as the
  /// lines above it let it, beside one row that says how many lines stand above and one row that
  /// says how many follow or that more comes. So the box keeps its height while its word runs. The
  /// line where the word stands shows whatever rows it takes.
  pub fn shown(&self, open: bool) -> Range<usize> {
    let all = self.lines.len();
    let Some(at) = stands(&self.lines).filter(|_| !open && self.rolls()) else {
      return 0..all;
    };
    let taken: Vec<usize> = self.lines.iter().map(Line::rows).collect();
    let edges =
      |first: usize, end: usize| usize::from(first > 0) + usize::from(end < all || self.more);
    // Each place that the loop reads stands in the lines, as `first > 0` and `end < all` hold.
    let rows = |at: usize| taken.get(at).copied().unwrap_or_default();
    let (mut first, mut end, mut used) = (at, at + 1, rows(at));
    loop {
      if first > 0 && used + rows(first - 1) + edges(first - 1, end) <= ROLLS {
        first -= 1;
        used += rows(first);
      } else if end < all && used + rows(end) + edges(first, end + 1) <= ROLLS {
        used += rows(end);
        end += 1;
      } else {
        break first..end;
      }
    }
  }
}

/// Where a word stands among its lines: the last line that runs, asks, failed or stopped, else its
/// last line, none when it has no line.
pub fn stands(lines: &[Line]) -> Option<usize> {
  lines
    .iter()
    .rposition(|line| !matches!(line.mark, Mark::Ran | Mark::Waits))
    .or(lines.len().checked_sub(1))
}

impl Line {
  /// A line of a number and a text, with a mark, which made no act yet.
  pub fn of(number: usize, text: String, mark: Mark) -> Self {
    Self {
      number,
      text,
      mark,
      output: None,
      made: Vec::new(),
      writes: Vec::new(),
      awaits: Vec::new(),
      pictures: Vec::new(),
    }
  }

  /// How many rows a line of a block that rolls takes: its own, those of what its command wrote,
  /// one for each file that it wrote, whose diff shows only in a block that stands open, and one
  /// for each image that it gave, which draws only in a block that stands open.
  pub fn rows(&self) -> usize {
    let output = self.output.as_ref().map_or(0, |one| one.tail.len() + usize::from(one.more > 0));
    1 + output + self.writes.len() + self.pictures.len()
  }
}

impl Thought {
  /// The first sentence of the thought, which shows once it ended.
  pub fn first(&self) -> &str {
    let text = self.text.trim_start();
    let line = text.split('\n').next().unwrap_or_default();
    line.find(". ").and_then(|at| line.get(..=at)).unwrap_or(line).trim_end()
  }
}

/// What an answer says to the operator: the value of the close of its message, a notice, a
/// question, or the resolve of its message by a person, with its act and its text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Said {
  pub act: String,
  pub kind: Saying,
  pub text: String,
}

/// What a said is. A question holds the shape that its answer takes, which `choices`, `ticks` and
/// `hint` of the view read, and what answered it once it is closed: it is open until then. A
/// resolve is the close of a message by a person, who it was, which no model answered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Saying {
  Answer,
  Notice,
  Question { shape: String, answered: Option<Answered> },
  Resolved { by: Maker },
}

/// What closed a question: who closed it, and what it came to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Answered {
  pub by: Maker,
  pub reply: Reply,
}

/// What a question came to: a value, as a person reads it, a refusal with its reason, or its
/// withdrawal, when a cancel ended it before anyone answered, with the reason that the cancel gave.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "is", content = "text", rename_all = "snake_case")]
pub enum Reply {
  Value(String),
  Refused(String),
  Withdrawn(Option<String>),
}

impl Answered {
  /// What closed a question to the operator, once it is done: a text as its text, a truth as the
  /// choice that answers it, None as nothing, a cancel as its withdrawal, another fault as its
  /// first argument, else its value as JSON.
  pub fn of(asked: &Act) -> Option<Self> {
    let reply = match asked.value()? {
      Value::String(text) => Reply::Value(text.clone()),
      Value::Bool(yes) => Reply::Value(if *yes { "yes" } else { "no" }.to_owned()),
      Value::Null => Reply::Value("nothing".to_owned()),
      value => match fault(value) {
        Some(("CancelledError", args)) => {
          Reply::Withdrawn(args.first().and_then(Value::as_str).map(str::to_owned))
        }
        Some((is, args)) => {
          Reply::Refused(args.first().and_then(Value::as_str).unwrap_or(is).to_owned())
        }
        None => Reply::Value(value.to_string()),
      },
    };
    Some(Self { by: asked.closer.clone().unwrap_or_default(), reply })
  }
}

/// Which exchanges of a thread a page holds: those before the one at `before`, or before the end,
/// at most `limit` of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reach {
  pub before: Option<usize>,
  pub limit: usize,
}
