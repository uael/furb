//! A page of a thread: some exchanges of it, each message with the block of code that answers it
//! and what the answer says to the operator, with the agents of the thread and the context of its
//! chain.

use std::ops::Range;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
  act::{Act, Maker, Read, fault},
  card::Card,
  words::timer,
};

/// Some exchanges of a thread, the oldest first, with how many exchanges stand before them, the
/// agents of the whole thread, in the order the thread made them, and the context of its chain.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Page {
  pub exchanges: Vec<Exchange>,
  pub above: usize,
  pub agents: Vec<Agent>,
  #[serde(default)]
  pub context: Option<Context>,
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

/// An agent of a thread: the thread that a word of the thread started on another chain, as a card,
/// with the millisecond of the epoch at which the host saw it begin, the seconds that it took once
/// it ended, counted from milliseconds as the time of a thought, and the line of its last word
/// where it stands: the line that runs or asks, else the last one, and the number in the thread
/// of the line of the word that made it, none when no line of a word of the thread made it. A host
/// that opened the life after the agent began saw it begin at that open.
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
}

/// One message of a thread with all that answers it: its place in the thread, 0 for the prompt of
/// the thread, the act of the message, the moment that it was said, who said it and its text, whether it is a word of Python
/// that an operator ran on the thread, whose block holds its code and which has no text, whether
/// the message still runs, the block of code that answers it once a model answers, and what the
/// answer says to the operator, in the order it was said. A message that the operator said on a
/// thread that works is done at once, since its model reads it at its next step, so only one that
/// asks still runs. A message is the prompt of the thread, what an operator said on it, or a word
/// that he ran on it, and what answers it is what came after it and before the next message.
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
/// the last command that it awaited wrote, and what each act that it made came to but a chain, in
/// the order it made them: one for each pass of a loop, the last of which is the value that it
/// bound.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Line {
  pub number: usize,
  pub text: String,
  pub mark: Mark,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub output: Option<Output>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub made: Vec<Made>,
}

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
}

/// Whether a fact says what a command wrote while it runs, which a page shows under its line.
pub fn wrote(fact: &Value) -> bool {
  fact.get(0).and_then(Value::as_str) == Some("out")
}

/// How a line stands: it ran, or will not run; it runs, with the work that it waits for and the
/// millisecond of the epoch at which the host saw it begin, so a time of a second shows after a
/// whole second; it asks the operator; its model is asked again after a call that failed; it
/// failed, with the reason that a person reads; or a stop ended it, at the millisecond of the epoch
/// at which the host saw the stop, which a host that opened the life after it does not know.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "is", rename_all = "snake_case")]
pub enum Mark {
  Ran,
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
    self.lines.iter().map(Line::rows).sum::<usize>() + usize::from(self.more) > ROLLS
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
    let (mut first, mut end, mut used) = (at, at + 1, taken[at]);
    loop {
      if first > 0 && used + taken[first - 1] + edges(first - 1, end) <= ROLLS {
        first -= 1;
        used += taken[first];
      } else if end < all && used + taken[end] + edges(first, end + 1) <= ROLLS {
        used += taken[end];
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
  lines.iter().rposition(|line| line.mark != Mark::Ran).or(lines.len().checked_sub(1))
}

impl Line {
  /// How many rows a line of a block takes: its own, and those of what its command wrote.
  pub fn rows(&self) -> usize {
    1 + self.output.as_ref().map_or(0, |one| one.tail.len() + usize::from(one.more > 0))
  }
}

impl Thought {
  /// The first sentence of the thought, which shows once it ended.
  pub fn first(&self) -> &str {
    let text = self.text.trim_start();
    let line = text.split('\n').next().unwrap_or_default();
    line.find(". ").map_or(line, |at| &line[..=at]).trim_end()
  }
}

/// What an answer says to the operator: the value of the close of its message, a notice, or a
/// question, with its act and its text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Said {
  pub act: String,
  pub kind: Saying,
  pub text: String,
}

/// What a said is. A question holds the shape that its answer takes, which `choices`, `ticks` and
/// `hint` of the view read, and what answered it once it is closed: it is open until then.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Saying {
  Answer,
  Notice,
  Question { shape: String, answered: Option<Answered> },
}

/// What closed a question: who closed it, and what it came to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Answered {
  pub by: Maker,
  pub reply: Reply,
}

/// What a question came to: a value, as a person reads it, or a refusal with its reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "is", content = "text", rename_all = "snake_case")]
pub enum Reply {
  Value(String),
  Refused(String),
}

impl Answered {
  /// What closed a question to the operator, once it is done: a text as its text, a truth as the
  /// choice that answers it, None as nothing, a fault as its first argument, else its value as
  /// JSON.
  pub fn of(asked: &Act) -> Option<Self> {
    let reply = match asked.value()? {
      Value::String(text) => Reply::Value(text.clone()),
      Value::Bool(yes) => Reply::Value(if *yes { "yes" } else { "no" }.to_owned()),
      Value::Null => Reply::Value("nothing".to_owned()),
      value => match fault(value) {
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
