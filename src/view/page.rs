//! One thread as its feed shows it, a page at a time: each message of the thread with all that
//! answers it, the block of code of each answer as its lines, the thought of its model, and the
//! agents that the thread made. A page reads the acts of its thread through the lists of the
//! acts by maker, so it takes the time of the thread and not of the life.

use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
  act::{Act, Kind, Maker, RunState},
  cards::{Card, Standing},
  fold::followed,
  held::Held,
  quote::{quotes, writing},
  state::{State, working},
  threads::{Lookup, message_of},
};

/// Some exchanges of a thread, the oldest first, with how many exchanges stand before them, and the
/// agents of the whole thread.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Page {
  pub exchanges: Vec<Exchange>,
  pub above: usize,
  pub agents: Vec<Card>,
}

/// One message of a thread with all that answers it: its place in the thread, 0 for the prompt of
/// the thread, the act of the message, who said it and its text, the block of code that answers it
/// once a model answers, and what the answer says to the operator, in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Exchange {
  pub at: usize,
  pub message: String,
  pub by: Maker,
  pub text: String,
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
/// binding that it makes and a string of many lines as one line with its escapes, and its mark.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Line {
  pub number: usize,
  pub text: String,
  pub mark: Mark,
}

/// How a line stands: it ran, or will not run; it runs, with the work that it waits for and the
/// second of the epoch since which it does when the host saw it begin; it asks the operator; or it
/// failed, with the reason that a person reads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "is", rename_all = "snake_case")]
pub enum Mark {
  Ran,
  Runs { work: Work, since: Option<u64> },
  Asks,
  Failed { why: String },
}

/// What a line that runs waits for: its model that writes it, a command or a wait, or an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Work {
  Writes,
  Command,
  Agent,
}

/// What a model thought before it wrote, and in how many seconds, which is none while it thinks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Thought {
  pub text: String,
  pub took: Option<u64>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Saying {
  Answer,
  Notice,
  Question,
}

/// What a host heard of a life beside its facts, which a page reads: what each model wrote and
/// thought for a rung, and the second of the epoch at which each act that lives began. The record
/// holds neither: the turn of a reply keeps its thought sealed, and a fact tells no time.
#[derive(Debug, Default)]
pub struct Streamed {
  wrote: HashMap<String, Wrote>,
  began: HashMap<String, u64>,
}

/// What a model wrote for a rung: its text until the rung has its word, its thought, and the
/// seconds at which it began and at which its thought ended.
#[derive(Debug, Default)]
struct Wrote {
  text: String,
  thinking: String,
  began: u64,
  thought: Option<u64>,
}

impl Streamed {
  /// What a model added to the text and to the thought of a rung, at a second.
  pub fn wrote(&mut self, rung: &str, [text, thinking]: [&str; 2], now: u64) {
    let wrote =
      self.wrote.entry(rung.to_owned()).or_insert_with(|| Wrote { began: now, ..Wrote::default() });
    wrote.text.push_str(text);
    wrote.thinking.push_str(thinking);
    if !wrote.text.is_empty() && !wrote.thinking.is_empty() {
      wrote.thought.get_or_insert(now);
    }
  }

  /// An act as it stands now, at a second: an act that lives keeps the second it began, and a rung
  /// that has its word keeps only the thought of its model.
  pub fn heard(&mut self, act: &Act, now: u64) {
    if act.done() && act.run != Some(RunState::Running) {
      self.began.remove(&act.id);
    } else {
      self.began.entry(act.id.clone()).or_insert(now);
    }
    if (act.word.is_some() || act.done())
      && let Some(wrote) = self.wrote.get_mut(&act.id)
    {
      if wrote.thinking.is_empty() {
        self.wrote.remove(&act.id);
      } else {
        wrote.text = String::new();
        wrote.thought.get_or_insert(now);
      }
    }
  }
}

/// Which exchanges of a thread a page holds: those before the one at `before`, or before the end,
/// at most `limit` of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reach {
  pub before: Option<usize>,
  pub limit: usize,
}

/// The acts that a host holds, with what it heard of them beside their facts.
#[derive(Clone, Copy)]
struct Reader<'a> {
  held: &'a Held,
  streams: &'a Streamed,
}

impl Held {
  /// A page of a thread, its prompt. A prompt that is no thread gives an empty page.
  pub fn page(&self, streams: &Streamed, thread: &str, reach: Reach) -> Page {
    Reader { held: self, streams }.page(thread, reach)
  }
}

impl<'a> Reader<'a> {
  fn page(self, thread: &str, Reach { before, limit }: Reach) -> Page {
    let Some(prompt) = self.held.act(thread).filter(|act| act.kind == Kind::Prompt) else {
      return Page::default();
    };
    let on = |act: &&Act| {
      act.kind == Kind::Prompt && act.id != thread && message_of(act, &self.held) == Some(thread)
    };
    let messages: Vec<&Act> =
      [prompt].into_iter().chain(self.held.on_chain(&prompt.chain).filter(on)).collect();
    let end = before.unwrap_or(messages.len()).min(messages.len());
    let start = end.saturating_sub(limit);
    let mut number: usize = messages[..start]
      .iter()
      .flat_map(|message| self.rungs(&message.id))
      .map(|rung| count(rung, self.streams))
      .sum();
    let exchanges = (start..end).map(|at| self.exchange(messages[at], at, &mut number)).collect();
    let agents = messages.iter().flat_map(|message| self.rungs(&message.id)).flat_map(|rung| {
      self
        .held
        .made_by(&rung.id)
        .filter(|act| act.kind == Kind::Prompt && act.chain != rung.chain && !act.asks_operator())
    });
    Page { exchanges, above: start, agents: agents.map(|agent| self.agent(agent)).collect() }
  }

  /// The rungs that advance on a prompt, in order.
  fn rungs(self, prompt: &str) -> impl Iterator<Item = &'a Act> + use<'a> {
    self.held.made_by(prompt).filter(|act| act.kind == Kind::Rung)
  }

  /// One message with all that answers it. `number` is the count of the lines of the thread before
  /// its block, and after it once it returns.
  fn exchange(self, message: &Act, at: usize, number: &mut usize) -> Exchange {
    let rungs: Vec<&Act> = self.rungs(&message.id).collect();
    let mut lines = Vec::new();
    let mut said = Vec::new();
    for rung in &rungs {
      lines.extend(self.lines(rung, *number));
      *number += count(rung, self.streams);
      for asked in self.held.made_by(&rung.id).filter(|act| act.asks_operator()) {
        let kind = if asked.words.first().is_some_and(|shape| shape == "None") {
          Saying::Notice
        } else {
          Saying::Question
        };
        said.push(Said { act: asked.id.clone(), kind, text: text(asked.words.get(1)) });
      }
    }
    if let Some(Value::String(answer)) = message.value().filter(|value| !value.is_null()) {
      said.push(Said { act: message.id.clone(), kind: Saying::Answer, text: answer.clone() });
    }
    let thought =
      rungs.iter().rev().find_map(|rung| self.streams.wrote.get(&rung.id)).and_then(|wrote| {
        let took = wrote.thought.map(|ended| ended.saturating_sub(wrote.began));
        (!wrote.thinking.trim().is_empty()).then(|| Thought { text: wrote.thinking.clone(), took })
      });
    let block = rungs.last().map(|last| Block {
      maker: last.maker.clone(),
      lines,
      more: !message.done() && State::of(message).busy(),
      thought,
    });
    Exchange {
      at,
      message: message.id.clone(),
      by: message.maker.clone(),
      text: text(message.words.get(1)),
      block,
      said,
    }
  }

  /// The lines of the word of a rung, numbered from the count of the lines of the thread before
  /// it. While its model writes the word, the lines that came ran and the line that comes runs.
  fn lines(self, rung: &Act, before: usize) -> Vec<Line> {
    let began = |id: &str| self.streams.began.get(id).copied();
    let Some(word) = &rung.word else {
      let streamed = self.streams.wrote.get(&rung.id).filter(|one| !one.text.is_empty());
      let Some(streamed) = streamed else { return Vec::new() };
      let came = writing(&streamed.text, "");
      let ran = came.lines.split('\n').zip(&came.numbers).filter(|_| !came.lines.is_empty());
      let mut lines: Vec<Line> = ran
        .map(|(text, at)| Line { number: before + at, text: text.to_owned(), mark: Mark::Ran })
        .collect();
      let writes = Mark::Runs { work: Work::Writes, since: Some(streamed.began) };
      lines.push(Line { number: before + came.count + 1, text: came.tail, mark: writes });
      return lines;
    };
    let mut lines: Vec<Line> = shown(word)
      .into_iter()
      .map(|(at, text)| Line { number: before + at + 1, text, mark: Mark::Ran })
      .collect();
    let at_line = |source: usize| lines.iter().rposition(|line| line.number <= before + source + 1);
    match &rung.run {
      Some(RunState::Failed(why)) => {
        let source = line_of(why).map_or(word.split('\n').count(), |at| at.saturating_sub(1));
        if let Some(at) = at_line(source).or(lines.len().checked_sub(1)) {
          lines[at].mark = Mark::Failed { why: why.clone() };
        }
      }
      Some(RunState::Running) => {
        let made: Vec<&Act> = self.held.made_by(&rung.id).filter(|act| waits(act)).collect();
        let open = made.iter().rposition(|act| !act.done());
        let source = open.and_then(|at| placed(word, &made).get(at).copied().flatten());
        let mark = match open.map(|at| made[at]) {
          Some(act) if act.asks_operator() => Mark::Asks,
          Some(act) if act.kind == Kind::Prompt => {
            Mark::Runs { work: Work::Agent, since: began(&act.id) }
          }
          Some(act) => Mark::Runs { work: Work::Command, since: began(&act.id) },
          None => Mark::Runs { work: Work::Command, since: began(&rung.id) },
        };
        if let Some(at) = source.and_then(at_line).or(lines.len().checked_sub(1)) {
          lines[at].mark = mark;
        }
      }
      _ => {}
    }
    lines
  }

  /// An agent of a thread, as a card: the thread that a word of the thread started on another
  /// chain.
  fn agent(self, prompt: &Act) -> Card {
    let rungs: Vec<&Act> = self.rungs(&prompt.id).collect();
    let work: Vec<&Act> = rungs
      .iter()
      .copied()
      .chain(rungs.iter().flat_map(|rung| self.held.made_by(&rung.id)))
      .collect();
    let asks = work.iter().any(|act| act.asks_operator() && act.owed());
    let state = match State::of(prompt) {
      _ if asks => State::Asks,
      State::Runs if working(&work, &[]) == State::Thinks => State::Thinks,
      state => state,
    };
    let by = prompt.pause.as_ref().map(|pause| pause.by.clone()).unwrap_or_default();
    let model = rungs.iter().rev().find_map(|rung| match &rung.maker {
      Maker::Model { model, .. } => Some(model.clone()),
      _ => None,
    });
    let title = text(prompt.words.get(1));
    Card {
      prompt: prompt.id.clone(),
      title: title.lines().find(|line| !line.trim().is_empty()).unwrap_or_default().to_owned(),
      standing: Standing::of(state, prompt.done(), by),
      model,
      agents: 0,
      readers: Vec::new(),
    }
  }
}

/// The text of a word of an act, or nothing.
fn text(word: Option<&Value>) -> String {
  word.and_then(Value::as_str).unwrap_or_default().to_owned()
}

/// Whether a line of a word waits for an act that it made: each act that a person follows, but the
/// reply that writes the word.
fn waits(act: &Act) -> bool {
  followed(act.kind.as_str()) && act.kind != Kind::Reply
}

/// How many lines of the thread a rung takes: the lines of its word, or of what its model wrote of
/// it.
fn count(rung: &Act, streams: &Streamed) -> usize {
  match (&rung.word, streams.wrote.get(&rung.id)) {
    (Some(word), _) => word.split('\n').count(),
    (None, Some(wrote)) if !wrote.text.is_empty() => wrote.text.split('\n').count(),
    _ => 0,
  }
}

/// The lines of a word that show, each with the line of the word where it starts: a quote as the
/// binding that it makes, and a string of three marks that runs over many lines as one line with
/// its escapes.
fn shown(word: &str) -> Vec<(usize, String)> {
  let source: Vec<&str> = word.split('\n').collect();
  let quoted: HashMap<usize, (usize, String)> = quotes(word, false)
    .into_iter()
    .map(|quote| (quote.start, (quote.end, quote.binding)))
    .collect();
  let mut out = Vec::new();
  let mut at = 0;
  while at < source.len() {
    if let Some((end, binding)) = quoted.get(&at) {
      out.push((at, binding.clone()));
      at = end + 1;
      continue;
    }
    let mut end = at;
    let opens =
      |line: &str| ["\"\"\"", "'''"].iter().any(|mark| line.matches(mark).count() % 2 == 1);
    if opens(source[at]) {
      while end + 1 < source.len() && !opens(source[end + 1]) {
        end += 1;
      }
      end = (end + 1).min(source.len() - 1);
    }
    out.push((at, source[at..=end].join("\\n")));
    at = end + 1;
  }
  out
}

/// The line that the reason of a fault names, counted from 1.
fn line_of(why: &str) -> Option<usize> {
  let (_, after) = why.split_once("line ")?;
  after.split(|one: char| !one.is_ascii_digit()).next()?.parse().ok()
}

/// The line of a word that made each act, in order: each act takes the next line that calls its
/// kind by name, and an act whose kind no later line calls takes none.
fn placed(word: &str, made: &[&Act]) -> Vec<Option<usize>> {
  let mut slots: Vec<(usize, &str)> = Vec::new();
  for (at, line) in word.split('\n').enumerate() {
    let code = line.split('#').next().unwrap_or_default();
    for (start, _) in code.match_indices('(') {
      let name = code[..start].rsplit(|one: char| !(one.is_alphanumeric() || one == '_')).next();
      slots.extend(name.filter(|name| !name.is_empty()).map(|name| (at, name)));
    }
  }
  let mut next = 0;
  made
    .iter()
    .map(|act| {
      let found = slots[next..].iter().position(|slot| slot.1 == act.kind.as_str())?;
      next += found + 1;
      Some(slots[next - 1].0)
    })
    .collect()
}

#[cfg(test)]
#[path = "page.test.rs"]
mod tests;
