//! One thread as its feed shows it, a page at a time: each message of the thread with all that
//! answers it, the block of code of each answer as its lines, the thought of its model, and the
//! agents that the thread made. A page reads the acts of its thread through the lists of the
//! acts by maker, so it takes the time of the thread and not of the life.

use std::{
  collections::{HashMap, HashSet},
  mem,
};

use anstream::adapter::strip_str;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use furb_wire::{
  act::{Act, Kind, Maker, RunState, fault, operator},
  card::{Card, Standing},
  page::{
    Agent, Answered, Block, Context, Exchange, Line, Mark, Output, Page, Reach, Retrying, Said,
    Saying, Thought, Work, wrote,
  },
  state::{State, working},
};

use super::{
  fold::followed,
  held::Held,
  ladder::lives,
  quote::{quotes, writing},
  threads::{Lookup, message_of, noted},
};

/// How many lines of what a command wrote a tail shows.
const TAIL: usize = 3;

/// The most bytes of a stream that a tail keeps, so a line that never ends, as a bar of progress,
/// keeps its end, which is what a terminal shows of a line that writes over itself.
const KEPT: usize = 4096;

/// The output of a command from the ends of its two streams, none when it wrote nothing and its
/// time is not known.
fn output_of([stdout, stderr]: &[Kept; 2], took: Option<u64>) -> Option<Output> {
  let lines: Vec<&str> = stdout.end.lines().chain(stderr.end.lines()).collect();
  let skip = lines.len().saturating_sub(TAIL);
  let more = stdout.before + stderr.before + skip;
  let tail: Vec<String> = lines[skip..].iter().map(|line| shown_line(line)).collect();
  (!tail.is_empty() || took.is_some()).then_some(Output { tail, more, took })
}

/// The end of what one stream of a command wrote: its last lines, at most `KEPT` bytes of them, and
/// how many lines came before them.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Kept {
  end: String,
  before: usize,
}

impl Kept {
  /// The end of a whole text. A newline at its end ends its last line and starts no new one.
  fn of(text: &str) -> Self {
    let body = text.strip_suffix('\n').unwrap_or(text);
    let mut start = body.rmatch_indices('\n').nth(TAIL - 1).map_or(0, |(at, _)| at + 1);
    if text.len() - start > KEPT {
      start = text.len() - KEPT;
      while !text.is_char_boundary(start) {
        start += 1;
      }
    }
    Self { end: text[start..].to_owned(), before: text[..start].matches('\n').count() }
  }

  /// The end once the stream wrote a text more.
  fn push(&mut self, text: &str) {
    let mut end = mem::take(&mut self.end);
    end.push_str(text);
    let kept = Self::of(&end);
    self.end = kept.end;
    self.before += kept.before;
  }
}

/// A line that a command wrote as a terminal shows it: what comes after its last return, with no
/// escape of a terminal and no other control, and a tab as a space.
fn shown_line(line: &str) -> String {
  let line = line.rsplit('\r').find(|part| !part.is_empty()).unwrap_or_default();
  let plain = strip_str(line).to_string();
  plain
    .chars()
    .filter_map(|one| if one == '\t' { Some(' ') } else { (!one.is_control()).then_some(one) })
    .collect()
}

/// What a host heard of a life beside its facts, which a page reads: what each model wrote and
/// thought for a rung and the retry of its call that waits, the millisecond of the epoch at which
/// each act that lives began, at which each prompt began and ended, so an agent that ended says
/// how long it took, at which each command began and ended, at which a stop ended each rung that it
/// ended, and the end of what each command that runs wrote, which its exit gives once it ended. The
/// record holds none of the rest: the turn of a reply keeps its thought sealed, and a fact tells no
/// time, so a host keeps it in the image of the life, once [`Held::shown`] let go of each thought
/// that no page shows.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Streamed {
  wrote: HashMap<String, Wrote>,
  began: HashMap<String, u64>,
  #[serde(default)]
  ended: HashMap<String, u64>,
  #[serde(default)]
  outs: HashMap<String, [Kept; 2]>,
}

/// What a model wrote for a rung: its text until the rung has its word, its thought, the
/// milliseconds at which it began and at which its thought ended, so a thought that ends within
/// a second of its start took no second, whatever second its start and its end fall in, and the
/// retry of its call until the model writes again or the reply ends.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Wrote {
  text: String,
  thinking: String,
  began: u64,
  thought: Option<u64>,
  #[serde(default)]
  retry: Option<Retrying>,
}

impl Streamed {
  /// What a model added to the text and to the thought of a rung, at a millisecond of the epoch.
  pub fn wrote(&mut self, rung: &str, [text, thinking]: [&str; 2], now: u64) {
    let wrote =
      self.wrote.entry(rung.to_owned()).or_insert_with(|| Wrote { began: now, ..Wrote::default() });
    wrote.text.push_str(text);
    wrote.thinking.push_str(thinking);
    wrote.retry = None;
    if !wrote.text.is_empty() && !wrote.thinking.is_empty() {
      wrote.thought.get_or_insert(now);
    }
  }

  /// A retry of the call of the model of a rung: what the model wrote and thought for it is void,
  /// and the next attempt begins at the second of the retry.
  pub fn retried(&mut self, rung: &str, retry: Retrying) {
    let began = retry.at.saturating_mul(1000);
    self.wrote.insert(rung.to_owned(), Wrote { began, retry: Some(retry), ..Wrote::default() });
  }

  /// A fact that a command wrote a text to one of its streams while it runs.
  pub fn out(&mut self, fact: &Value) {
    let word = |at: usize| fact.get(at).and_then(Value::as_str);
    let (true, Some(about), Some(text)) = (wrote(fact), word(1), word(3)) else { return };
    let stream = usize::from(word(4) == Some("stderr"));
    self.outs.entry(about.to_owned()).or_default()[stream].push(text);
  }

  /// An act as it stands now, at a millisecond of the epoch: an act that lives keeps when it began,
  /// a prompt and a command keep when they began and ended, a command that ended lets go of what it
  /// wrote, which its exit holds, a rung that has its word keeps only the thought of its model, and
  /// the end of a reply ends the retry of its rung.
  pub fn heard(&mut self, act: &Act, now: u64) {
    if act.kind == Kind::Reply
      && act.done()
      && let Some(wrote) = self.wrote.get_mut(&act.by)
    {
      wrote.retry = None;
    }
    if act.done() && act.kind == Kind::Bash {
      self.outs.remove(&act.id);
    }
    if act.done() && matches!(act.kind, Kind::Prompt | Kind::Bash) {
      if self.began.contains_key(&act.id) {
        self.ended.entry(act.id.clone()).or_insert(now);
      }
    } else if act.kind == Kind::Rung && act.cancelled() {
      self.began.remove(&act.id);
      self.ended.entry(act.id.clone()).or_insert(now);
    } else if act.done() && act.run != Some(RunState::Running) {
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

impl Held {
  /// Lets go of each thought that no page of these acts shows: a page shows the thought of the
  /// last rung of an exchange that thought, so the thought of a rung before it in that exchange,
  /// and of a rung that answers no thread, never shows again. What a model writes for a rung
  /// that has no word yet stays.
  pub fn shown(&self, streams: &mut Streamed) {
    let reader = Reader { held: self, streams };
    let mut threads = HashSet::new();
    for rung in streams.wrote.keys().filter_map(|id| self.act(id)) {
      if let Some(prompt) = self.act(&rung.by) {
        threads.insert(message_of(prompt, &self).unwrap_or(&prompt.id));
      }
    }
    let shown: HashSet<String> = threads
      .into_iter()
      .filter_map(|thread| reader.told(thread))
      .flat_map(|told| {
        told.answers.iter().filter_map(|one| reader.thinker(one)).collect::<Vec<_>>()
      })
      .map(|rung| rung.id.clone())
      .collect();
    let writes = |id: &str| self.act(id).is_none_or(|rung| rung.word.is_none() && !rung.done());
    streams.wrote.retain(|id, _| shown.contains(id) || writes(id));
  }
}

/// One message of a thread with what answers it: each rung that began after it and before the
/// next message, each question and notice to the operator said there, each message whose close
/// came there, and whether more comes.
struct Answers<'a> {
  message: &'a Act,
  rungs: Vec<&'a Act>,
  asked: Vec<&'a Act>,
  closes: Vec<&'a Act>,
  more: bool,
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

/// A thread as a page reads it: its prompts, what answers each message of it, and its rungs, in
/// the order they began.
struct Told<'a> {
  prompts: Vec<&'a Act>,
  answers: Vec<Answers<'a>>,
  rungs: Vec<&'a Act>,
}

impl<'a> Reader<'a> {
  fn page(self, thread: &str, Reach { before, limit }: Reach) -> Page {
    let Some(Told { prompts, mut answers, rungs }) = self.told(thread) else {
      return Page::default();
    };
    let end = before.unwrap_or(answers.len()).min(answers.len());
    let start = end.saturating_sub(limit);
    let mut number: usize = answers[..start]
      .iter()
      .flat_map(|one| &one.rungs)
      .map(|rung| count(rung, self.streams))
      .sum();
    // More comes after the last message, and in the block of a rung that lives.
    let busy = prompts.iter().any(|one| !one.done() && State::of(one).busy());
    let messages = answers.len();
    for (at, one) in answers.iter_mut().enumerate() {
      one.more = busy && (at + 1 == messages || one.rungs.last().is_some_and(|rung| lives(rung)));
    }
    let exchanges = (start..end).map(|at| self.exchange(at, &answers[at], &mut number)).collect();
    let agents = rungs.iter().flat_map(|rung| {
      self
        .held
        .made_by(&rung.id)
        .filter(|act| act.kind == Kind::Prompt && act.chain != rung.chain)
        .filter(|act| !act.asks_operator() && !noted(act, &self.held))
    });
    let agents = agents.map(|agent| self.agent(agent)).collect();
    Page { exchanges, above: start, agents, context: self.context(&prompts[0].chain) }
  }

  /// The context of a chain at its last answer, and none before it.
  fn context(self, chain: &str) -> Option<Context> {
    let spend = self.held.act(chain)?.spend.as_ref()?;
    let window = spend.actor.as_deref().and_then(|actor| self.held.window(actor));
    Some(Context { read: spend.read?, window, dollars: spend.dollars })
  }

  /// A thread, its prompt, as a page reads it: none for a prompt that is no thread.
  fn told(self, thread: &str) -> Option<Told<'a>> {
    let prompt = self.held.act(thread).filter(|act| act.kind == Kind::Prompt)?;
    let on = |act: &&Act| {
      act.kind == Kind::Prompt && act.id != thread && message_of(act, &self.held) == Some(thread)
    };
    // Each prompt of the thread, whose rungs answer the thread, and the messages that the operator
    // said among them: a wake or an acknowledgment that the engine says is no message.
    let prompts: Vec<&Act> =
      [prompt].into_iter().chain(self.held.on_chain(&prompt.chain).filter(on)).collect();
    // Each word that an operator ran on a prompt of the thread is a message of his, as code.
    let word = |act: &&Act| {
      act.kind == Kind::Rung && operator(&act.by) && prompts.iter().any(|one| one.id == act.on)
    };
    let words: Vec<&Act> = self.held.on_chain(&prompt.chain).filter(word).collect();
    let mut messages: Vec<&Act> =
      prompts.iter().copied().filter(|one| one.id == thread || operator(&one.by)).collect();
    messages.extend(&words);
    messages.sort_by_key(|one| one.began);
    // What answers a message is what came after it and before the next one, whichever prompt of
    // the thread a rung advances: a model goes on with the prompt of the thread while the operator
    // says more on it.
    let after = |place: u64| messages.partition_point(|one| one.began <= place).saturating_sub(1);
    let mut answers: Vec<Answers> = messages
      .iter()
      .map(|message| Answers {
        message,
        rungs: Vec::new(),
        asked: Vec::new(),
        closes: Vec::new(),
        more: false,
      })
      .collect();
    let mut rungs: Vec<&Act> = prompts.iter().flat_map(|one| self.rungs(&one.id)).collect();
    rungs.extend(words);
    rungs.sort_by_key(|rung| rung.began);
    for rung in &rungs {
      answers[after(rung.began)].rungs.push(rung);
      for asked in self.held.made_by(&rung.id).filter(|act| act.asks_operator()) {
        answers[after(asked.began)].asked.push(asked);
      }
    }
    for message in messages.iter().filter(|one| one.kind == Kind::Prompt) {
      if let Some(ended) = message.ended.filter(|_| message.value().is_some_and(Value::is_string)) {
        answers[after(ended)].closes.push(message);
      }
    }
    Some(Told { prompts, answers, rungs })
  }

  /// The rung whose thought an exchange shows: the last of its rungs that thought.
  fn thinker(self, answers: &Answers<'a>) -> Option<&'a Act> {
    answers.rungs.iter().rev().copied().find(|rung| self.streams.wrote.contains_key(&rung.id))
  }

  /// The rungs that advance on a prompt, in order.
  fn rungs(self, prompt: &str) -> impl Iterator<Item = &'a Act> + use<'a> {
    self.held.made_by(prompt).filter(|act| act.kind == Kind::Rung)
  }

  /// One message with all that answers it, at its place in the thread. `number` is the count of
  /// the lines of the thread before its block, and after it once it returns.
  fn exchange(self, at: usize, answers: &Answers, number: &mut usize) -> Exchange {
    let mut lines = Vec::new();
    // Each thing said, at the place in the life where it was said.
    let mut said = Vec::new();
    let last = answers.rungs.len().saturating_sub(1);
    for (at, rung) in answers.rungs.iter().enumerate() {
      // A rung that ended with no word failed, and the rung after it asked its model again, so
      // the last rung alone tells how the answer stands, as a retry voids what came before it.
      if at < last && rung.word.is_none() && rung.done() {
        continue;
      }
      lines.extend(self.lines(rung, *number));
      *number += count(rung, self.streams);
    }
    for asked in &answers.asked {
      let shape = text(asked.words.first());
      let kind = if shape == "None" {
        Saying::Notice
      } else {
        Saying::Question { shape, answered: Answered::of(asked) }
      };
      let one = Said { act: asked.id.clone(), kind, text: text(asked.words.get(1)) };
      said.push((asked.began, one));
    }
    for closed in &answers.closes {
      let one = Said { act: closed.id.clone(), kind: Saying::Answer, text: text(closed.value()) };
      said.push((closed.ended.unwrap_or(closed.began), one));
    }
    said.sort_by_key(|(place, _)| *place);
    let streamed = self.thinker(answers).and_then(|rung| self.streams.wrote.get(&rung.id));
    let thought = streamed.and_then(|wrote| {
      let took = wrote.thought.map(|ended| ended.saturating_sub(wrote.began) / 1000);
      (!wrote.thinking.trim().is_empty()).then(|| Thought {
        text: wrote.thinking.clone(),
        began: wrote.began,
        took,
      })
    });
    let block = answers.rungs.last().map(|last| Block {
      maker: last.maker.clone(),
      lines,
      more: answers.more,
      thought,
    });
    let message = answers.message;
    let python = message.kind == Kind::Rung;
    Exchange {
      at,
      message: message.id.clone(),
      by: message.maker.clone(),
      text: if python { String::new() } else { text(message.words.get(1)) },
      python,
      runs: !message.done(),
      block,
      said: said.into_iter().map(|(_, one)| one).collect(),
    }
  }

  /// The lines of the word of a rung, numbered from the count of the lines of the thread before
  /// it, each with what the last command that it awaited wrote. While its model writes the word,
  /// the lines that came ran and the line that comes runs.
  fn lines(self, rung: &Act, before: usize) -> Vec<Line> {
    let Some(word) = &rung.word else { return self.writing(rung, before) };
    let acts: Vec<&Act> = self.held.made_by(&rung.id).filter(|act| waits(act)).collect();
    let made: Vec<(&Act, Option<usize>)> = acts.iter().copied().zip(placed(word, &acts)).collect();
    let mut lines = self.marked(rung, before, &made);
    for (act, place) in &made {
      let output = place.and_then(|source| at_line(&lines, before, source)).zip(self.output(act));
      if let Some((at, output)) = output {
        lines[at].output = Some(output);
      }
    }
    lines
  }

  /// The lines of the word of a rung, each with the mark of how it stands, from the acts that the
  /// word made, each with the line of the word that made it.
  fn marked(self, rung: &Act, before: usize, made: &[(&Act, Option<usize>)]) -> Vec<Line> {
    let word = rung.word.as_deref().unwrap_or_default();
    let began = |id: &str| self.streams.began.get(id).copied();
    let mut lines: Vec<Line> = shown(word)
      .into_iter()
      .map(|(at, text)| Line { number: before + at + 1, text, mark: Mark::Ran, output: None })
      .collect();
    let Some(run @ (RunState::Failed(_) | RunState::Cancelled(_) | RunState::Running)) = &rung.run
    else {
      return lines;
    };
    let line = |lines: &[Line], source: Option<usize>| {
      source.and_then(|source| at_line(lines, before, source)).or(lines.len().checked_sub(1))
    };
    if let RunState::Failed(why) = run {
      // A reason that names no line came from an act that the word awaited, as a refused
      // question or a command that exited with a code that the word refuses: the line that made
      // that act raised it: the act that failed, or else the last act that the word made. The
      // last line of the word is the line where its model stood when it raised, so it would hang
      // on the time of the fault.
      let raised = made.iter().rposition(|(act, _)| act.failed()).or(made.len().checked_sub(1));
      let source = line_of(why)
        .map(|at| at.saturating_sub(1))
        .or_else(|| raised.and_then(|at| made[at].1))
        .unwrap_or_else(|| word.split('\n').count());
      if let Some(at) = line(&lines, Some(source)) {
        lines[at].mark = Mark::Failed { why: readable(why, made) };
      }
    } else if let RunState::Cancelled(_) = run {
      // The stop ended the word on the line of the act that it cut, or else on its last line.
      let cut = made.iter().rposition(|(act, _)| act.cancelled()).and_then(|at| made[at].1);
      if let Some(at) = line(&lines, cut) {
        lines[at].mark = Mark::Stopped { at: self.streams.ended.get(&rung.id).copied() };
      }
    } else {
      let open = made.iter().rposition(|(act, _)| !act.done());
      let mark = match open.map(|at| made[at].0) {
        Some(act) if act.asks_operator() => Mark::Asks,
        Some(act) if act.kind == Kind::Prompt => {
          Mark::Runs { work: Work::Agent, began: began(&act.id) }
        }
        Some(act) => Mark::Runs { work: Work::Command, began: began(&act.id) },
        None => Mark::Runs { work: Work::Command, began: began(&rung.id) },
      };
      // Each line that asks the operator a question that waits says so, the last one or not.
      let asking: Vec<usize> = made
        .iter()
        .filter(|(act, _)| act.asks_operator() && !act.done())
        .filter_map(|(_, place)| place.and_then(|source| at_line(&lines, before, source)))
        .collect();
      if let Some(at) = line(&lines, open.and_then(|at| made[at].1)) {
        lines[at].mark = mark;
      }
      for at in asking {
        lines[at].mark = Mark::Asks;
      }
    }
    lines
  }

  /// What a command wrote: the end of each of its streams while it runs, from its exit once it
  /// ended. Nothing for an act that is no command, or that ended with no exit.
  fn output(self, act: &Act) -> Option<Output> {
    if act.kind != Kind::Bash {
      return None;
    }
    let took = self.streams.began.get(&act.id).zip(self.streams.ended.get(&act.id));
    let took = took.map(|(began, ended)| ended.saturating_sub(*began) / 1000);
    match act.value() {
      Some(exit) => {
        let stream = |name: &str| exit.get(name)?.get("content")?.as_str();
        let (stdout, stderr) = (stream("stdout")?, stream("stderr")?);
        output_of(&[Kept::of(stdout), Kept::of(stderr)], took)
      }
      None => output_of(self.streams.outs.get(&act.id)?, None),
    }
  }

  /// The lines that the model of a rung with no word wrote so far: those that came ran, and the
  /// line that comes runs from the moment its model is asked, waits for the retry of its call,
  /// failed with the last reply of the rung, or a stop ended it.
  fn writing(self, rung: &Act, before: usize) -> Vec<Line> {
    let reply = self.held.made_by(&rung.id).filter(|act| act.kind == Kind::Reply).last();
    let failed = reply.filter(|one| one.failed()).and_then(|one| {
      let (_, args) = fault(one.value()?)?;
      Some(Mark::Failed { why: text(args.first()) })
    });
    let at = self.streams.ended.get(&rung.id).copied();
    let stopped = rung.cancelled().then_some(Mark::Stopped { at });
    let streamed = self.streams.wrote.get(&rung.id);
    let mark = failed.or(stopped).or_else(|| streamed?.retry.clone().map(Mark::Retries));
    // The model of a reply that runs is asked: its line comes from then, before its first part.
    let asked = reply.filter(|one| !one.done()).and_then(|one| self.streams.began.get(&one.id));
    let (text, began) =
      streamed.map_or(("", asked.copied()), |one| (one.text.as_str(), Some(one.began)));
    if text.is_empty() && mark.is_none() && began.is_none() {
      return Vec::new();
    }
    let came = writing(text, "");
    let ran = came.lines.split('\n').zip(&came.numbers).filter(|_| !came.lines.is_empty());
    let mut lines: Vec<Line> = ran
      .map(|(text, at)| Line {
        number: before + at,
        text: text.to_owned(),
        mark: Mark::Ran,
        output: None,
      })
      .collect();
    let mark = mark.unwrap_or(Mark::Runs { work: Work::Writes, began });
    lines.push(Line { number: before + came.count + 1, text: came.tail, mark, output: None });
    lines
  }

  /// An agent of a thread: the thread that a word of the thread started on another chain.
  fn agent(self, prompt: &Act) -> Agent {
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
    let card = Card {
      prompt: prompt.id.clone(),
      title: title.lines().find(|line| !line.trim().is_empty()).unwrap_or_default().to_owned(),
      standing: Standing::of(state, prompt.done(), by),
      model,
      agents: 0,
      readers: Vec::new(),
    };
    let lines = rungs.last().map(|last| self.lines(last, 0)).unwrap_or_default();
    let stands = lines.iter().rposition(|line| line.mark != Mark::Ran);
    let began = self.streams.began.get(&prompt.id);
    let ended = self.streams.ended.get(&prompt.id);
    Agent {
      card,
      began: began.copied(),
      took: began.zip(ended).map(|(began, ended)| ended.saturating_sub(*began) / 1000),
      line: stands.or(lines.len().checked_sub(1)).map(|at| lines[at].clone()),
    }
  }
}

/// Why a word failed, as a person reads it at the line that raised: a refusal of furb says its
/// reason alone, with no name of its class, and with no name of the act that the line made, which
/// the line shows as code.
fn readable(why: &str, made: &[(&Act, Option<usize>)]) -> String {
  let reason = why.strip_prefix("Refused: ").unwrap_or(why);
  let named = made.iter().find_map(|(act, _)| reason.strip_prefix(&format!("{} ", act.id)));
  named.unwrap_or(reason).to_owned()
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

/// The line of a word, numbered from the count of the lines of the thread before it, that shows
/// a line of its source, counted from 0: a quote and a string of many lines show as one line.
fn at_line(lines: &[Line], before: usize, source: usize) -> Option<usize> {
  lines.iter().rposition(|line| line.number <= before + source + 1)
}

/// The line that the reason of a fault names, counted from 1: the last word `line` with a number
/// after it, which the interpreter puts at the end of the reason.
fn line_of(why: &str) -> Option<usize> {
  why.rmatch_indices("line ").find_map(|(at, mark)| {
    let starts = why[..at].chars().next_back().is_none_or(|one| !one.is_alphanumeric());
    let number = why[at + mark.len()..].split(|one: char| !one.is_ascii_digit()).next()?;
    starts.then(|| number.parse().ok()).flatten()
  })
}

/// The line of a word that made each act, in order: each act takes the first call of its kind by
/// name that no act took, from the line of the act before it on, since a call in the arguments of
/// another, as the chain of a prompt, makes its act first. An act whose kind no later line calls
/// takes the line of the act of its kind before it, as a loop makes many acts from one line, and
/// the first of its kind takes none.
fn placed(word: &str, made: &[&Act]) -> Vec<Option<usize>> {
  let mut slots: Vec<(usize, &str)> = Vec::new();
  for (at, line) in word.split('\n').enumerate() {
    let code = line.split('#').next().unwrap_or_default();
    for (start, _) in code.match_indices('(') {
      let name = code[..start].rsplit(|one: char| !(one.is_alphanumeric() || one == '_')).next();
      slots.extend(name.filter(|name| !name.is_empty()).map(|name| (at, name)));
    }
  }
  let mut taken = vec![false; slots.len()];
  let mut from = 0;
  let mut last: HashMap<&str, usize> = HashMap::new();
  made
    .iter()
    .map(|act| {
      let kind = act.kind.as_str();
      let free = |at: &usize| !taken[*at] && slots[*at].1 == kind;
      let Some(found) = (from..slots.len()).find(free) else {
        return last.get(kind).copied();
      };
      let line = slots[found].0;
      taken[found] = true;
      from = slots.iter().position(|slot| slot.0 == line).unwrap_or(found);
      last.insert(kind, line);
      Some(line)
    })
    .collect()
}

#[cfg(test)]
#[path = "page.test.rs"]
mod tests;
