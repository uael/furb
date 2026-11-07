//! The threads of a chain: each prompt to a model that is no message of another thread, with its
//! messages and what it asks the operator, nested under the thread whose ladder made it.

use std::collections::{HashMap, HashSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
  act::{Act, Kind, WATCH, operator},
  fold::followed,
  named::NAME,
  state::{State, working},
};

/// The acts of a life by their names.
pub type Rows<'a> = HashMap<&'a str, &'a Act>;

/// How deep a walk up the threads or the makers goes at most.
const DEEP: usize = 64;

/// One thread of a chain: its prompt, its name, its messages, the prompts to the operator that it
/// made and the questions of them that wait, and its state. A thread is over once its prompt
/// and each of its messages is done, and a message to a thread that is over opens it again until
/// its model closes the message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ChainThread {
  pub prompt: String,
  /// The name of the thread: the label of its chain for the first thread that no operator made on
  /// a chain under the root, the name of the session for the first thread of the operator on the
  /// root, and else the first line of its message that is not empty.
  pub title: String,
  pub messages: Vec<String>,
  pub asked: Vec<String>,
  pub asks: Vec<String>,
  pub state: State,
  pub over: bool,
  pub origin: Origin,
  /// Whether the operator started it, himself or by a word of his own.
  pub yours: bool,
}

/// Who made the prompt of a thread: the operator, a rung of a word, or an ear.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
  Operator,
  Rung,
  Ear,
}

/// A thread with the threads that its ladder made, its agents. Its state is its own, or asks while
/// a question waits in an agent under it: a failure or an unread in an agent does not roll up.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ThreadNode {
  pub thread: ChainThread,
  pub agents: Vec<ThreadNode>,
  pub state: State,
}

/// How a thread concerns the operator: a question waits for him in it, or he started it and it
/// failed or a pause holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Concern {
  Asks,
  Failed,
  Paused,
}

/// What a tree of threads says to the operator, counted by thread.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Attention {
  pub asks: usize,
  pub failed: usize,
  pub paused: usize,
}

/// The first prompt of the thread that a prompt stands in: the prompt itself, or the prompt that
/// its thread starts at.
pub fn root_of<'a>(prompt: &'a Act, rows: &Rows<'a>) -> &'a str {
  let mut at = prompt;
  for _ in 0..DEEP {
    match at.thread().and_then(|up| rows.get(up)) {
      Some(up) => at = up,
      None => break,
    }
  }
  &at.id
}

/// The thread that an act stands in: the first prompt of the thread of the nearest prompt above it,
/// walking up the maker of each act whatever its kind, a rung, a conduct, a chain or any other, so
/// that a worker that a round made stands in the thread whose word started the round. An act whose
/// makers end before a prompt, as one that the operator made, stands in none.
pub fn thread_of<'a>(act: &Act, rows: &Rows<'a>) -> Option<&'a str> {
  let mut maker = rows.get(act.by.as_str())?;
  for _ in 0..DEEP {
    if maker.kind == Kind::Prompt {
      return Some(root_of(maker, rows));
    }
    maker = rows.get(maker.by.as_str())?;
  }
  None
}

/// What a prompt of a chain is that is no conversation, which the feed of its chain draws as what
/// it is and the list of its threads leaves out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "aside", content = "from", rename_all = "snake_case")]
pub enum Aside {
  /// A notice of the round: a prompt of None that a rung or a chain made, which no one answers, as
  /// the acknowledgment of an act that stands in no thread of the chain, a stall that the watch of
  /// the daemon tells, or the question of the act that names the session.
  Notice,
  /// What a word of the operator told the chain: a prompt of None that a rung of his made, or the
  /// acknowledgment of a rung that stands in no thread.
  Rung,
  /// A wake of the chain: a prompt of None that an ear or a verb of the round made.
  Wake,
  /// A question of another chain to this one, a worker that asks its coordinator, with the chain
  /// that asks.
  Question(String),
}

/// A prompt of a chain that is no thread, with what it is and the prompts on it, in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ChainAside {
  pub prompt: String,
  pub aside: Aside,
  pub messages: Vec<String>,
}

/// What a prompt is to the threads.
enum Verdict<'a> {
  /// A thread of its own.
  Own,
  /// A message of a thread.
  Message(&'a str),
  /// No conversation.
  Aside(Aside),
}

/// What a prompt to a model is: a message of a thread when it stands on one, the acknowledgment of
/// an act that a chain made, which belongs to the thread of that act, a notice, a word of the
/// operator, a wake or a question of another chain, and else a thread of its own, as each prompt
/// that the operator made himself. What the watch of the daemon tells of a stall, and what the act
/// that names the session asks, are notices: no one of the operator made them.
fn verdict<'a>(act: &'a Act, rows: &Rows<'a>) -> Verdict<'a> {
  if act.thread().is_some() {
    return Verdict::Message(root_of(act, rows));
  }
  let maker = rows.get(act.by.as_str()).copied();
  let none = act.words.first().is_some_and(|shape| shape == "None");
  match maker {
    Some(maker) if none && maker.kind == Kind::Chain => acknowledged(act, rows),
    _ if act.by == WATCH => Verdict::Aside(Aside::Notice),
    Some(maker) if maker.kind.as_str() == NAME => Verdict::Aside(Aside::Notice),
    _ if operator(&act.by) => Verdict::Own,
    Some(maker) if none && maker.kind == Kind::Rung && operator(&maker.by) => {
      Verdict::Aside(Aside::Rung)
    }
    Some(maker) if none && matches!(maker.kind, Kind::Rung | Kind::Prompt) => {
      Verdict::Aside(Aside::Notice)
    }
    _ if none => Verdict::Aside(Aside::Wake),
    Some(maker) if maker.chain != act.chain && !within(&act.chain, &maker.chain, rows) => {
      Verdict::Aside(Aside::Question(maker.chain.clone()))
    }
    _ => Verdict::Own,
  }
}

/// What the acknowledgment of an act that a chain made is: a message of the thread of that act, or
/// an aside when the act stands in no thread.
fn acknowledged<'a>(act: &'a Act, rows: &Rows<'a>) -> Verdict<'a> {
  let message = act.words.get(1).and_then(|one| one.as_str()).unwrap_or_default();
  let about = message.split_once(' ').map(|(about, _)| about).filter(|about| {
    about.len() > 1
      && about.ends_with(|one: char| one.is_ascii_digit())
      && about.chars().all(|one| one.is_ascii_alphanumeric() || one == '_')
  });
  let Some(subject) = about.and_then(|about| rows.get(about)) else {
    return Verdict::Aside(Aside::Notice);
  };
  if subject.kind == Kind::Prompt {
    return Verdict::Message(thread_of(subject, rows).unwrap_or(&subject.id));
  }
  match thread_of(subject, rows) {
    Some(thread) => Verdict::Message(thread),
    None if subject.kind == Kind::Rung => Verdict::Aside(Aside::Rung),
    None => Verdict::Aside(Aside::Notice),
  }
}

/// Whether a chain stands under another, or is it, walking up the makers of the chains.
fn within<'a>(chain: &'a str, of: &str, rows: &Rows<'a>) -> bool {
  let mut at = chain;
  for _ in 0..DEEP {
    if at == of {
      return true;
    }
    match rows.get(at).and_then(|one| rows.get(one.by.as_str())) {
      Some(maker) if maker.chain != at => at = &maker.chain,
      _ => return false,
    }
  }
  false
}

/// The threads of one chain, from its acts in the order they began: each prompt to a model on the
/// chain that is no message of another thread and no aside. It asks while a question waits in it.
pub fn chain_threads(on: &[&Act], rows: &Rows<'_>) -> Vec<ChainThread> {
  sorted(on, rows).0
}

/// The asides of one chain, from its acts in the order they began: each prompt to a model on the
/// chain that is no conversation, with the prompts on it, and each message of a thread that is no
/// thread of the chain, as a notice.
pub fn chain_asides(on: &[&Act], rows: &Rows<'_>) -> Vec<ChainAside> {
  sorted(on, rows).1
}

/// A run of notes: the asides of a chain that are no question and follow one another in its feed
/// with no card of another kind between them, which a window draws as one block. `unread` counts
/// the notes that began or ended after the place up to which the operator read the chain.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct NoteRun {
  pub prompts: Vec<String>,
  pub count: usize,
  pub unread: usize,
}

/// The runs of notes of one chain, from its acts in the order they began. Only a step of the top of
/// the chain draws a card: an act that a rung of the chain made stands in the program of that rung,
/// a message stands in its thread or its note, and a reply, a grant, a chain and a rung that the
/// chain wrote to retell its source draw none. Each other act ends the run that it follows, as a
/// thread, a question, a command and a word of the operator do. `read` is the place up to which
/// the operator read the chain, which only a window knows.
pub fn chain_notes(on: &[&Act], rows: &Rows<'_>, read: u64) -> Vec<NoteRun> {
  let (threads, asides) = sorted(on, rows);
  let notes: HashSet<&str> = asides
    .iter()
    .filter(|one| !matches!(one.aside, Aside::Question(_)))
    .map(|one| one.prompt.as_str())
    .collect();
  let inside: HashSet<&str> = threads
    .iter()
    .flat_map(|one| one.messages.iter().chain(&one.asked))
    .chain(asides.iter().flat_map(|one| &one.messages))
    .map(String::as_str)
    .collect();
  let mut runs: Vec<NoteRun> = Vec::new();
  let mut open = false;
  for act in on {
    let maker = rows.get(act.by.as_str());
    let below = maker.is_some_and(|one| one.chain == act.chain && one.kind != Kind::Chain);
    let retold = act.kind == Kind::Rung && maker.is_some_and(|one| one.kind == Kind::Chain);
    let silent = act.kind == Kind::Chain
      || !followed(act.kind.as_str())
      || matches!(&act.kind, Kind::Other(kind) if kind == "grant");
    if below || retold || silent || inside.contains(act.id.as_str()) {
      continue;
    }
    if !notes.contains(act.id.as_str()) {
      open = false;
      continue;
    }
    if !open {
      runs.push(NoteRun::default());
      open = true;
    }
    let Some(run) = runs.last_mut() else { continue };
    run.prompts.push(act.id.clone());
    run.count += 1;
    run.unread += usize::from(act.ended.unwrap_or(act.began) > read);
  }
  runs
}

/// The threads and the asides of one chain.
fn sorted(on: &[&Act], rows: &Rows<'_>) -> (Vec<ChainThread>, Vec<ChainAside>) {
  let mut out: Vec<ChainThread> = Vec::new();
  let mut asides: Vec<(u64, ChainAside)> = Vec::new();
  let mut later: Vec<(&str, &Act)> = Vec::new();
  for act in on.iter().copied().filter(|act| act.kind == Kind::Prompt) {
    if act.asks_operator() {
      later.extend(thread_of(act, rows).map(|thread| (thread, act)));
      continue;
    }
    match verdict(act, rows) {
      Verdict::Message(thread) => later.push((thread, act)),
      Verdict::Aside(aside) => asides.push((act.began, aside_of(act, aside))),
      Verdict::Own => out.push(opened(act, rows)),
    }
  }
  for (thread, act) in later {
    let Some(at) = out.iter_mut().find(|one| one.prompt == thread) else {
      match asides.iter_mut().find(|one| one.1.prompt == thread) {
        Some(aside) if !act.asks_operator() => aside.1.messages.push(act.id.clone()),
        None if !act.asks_operator() => asides.push((act.began, aside_of(act, Aside::Notice))),
        _ => {}
      }
      continue;
    };
    if act.asks_operator() {
      at.asked.push(act.id.clone());
      at.asks.extend(act.owed().then(|| act.id.clone()));
    } else {
      at.messages.push(act.id.clone());
    }
  }
  asides.sort_by_key(|one| one.0);
  let asides = asides.into_iter().map(|one| one.1).collect();
  named(&mut out, rows);
  for thread in &mut out {
    let acts = std::iter::once(&thread.prompt).chain(&thread.messages);
    let open = acts.filter_map(|id| rows.get(id.as_str())).rfind(|act| !act.done());
    thread.over = open.is_none();
    thread.state = match open.or_else(|| rows.get(thread.prompt.as_str())) {
      _ if !thread.asks.is_empty() => State::Asks,
      Some(act) => match State::of(act) {
        // A model that writes a word for the thread thinks, as the chain and the session say.
        State::Runs
          if act.kind == Kind::Prompt && working(&work_of(&act.id, on), &[]) == State::Thinks =>
        {
          State::Thinks
        }
        state => state,
      },
      None => State::Idle,
    };
  }
  (out, asides)
}

/// The acts that work for a prompt: the rungs that advance on it, and the acts that those rungs
/// made, as the commands and the waits of their words.
fn work_of<'a>(prompt: &str, on: &[&'a Act]) -> Vec<&'a Act> {
  let rungs: HashSet<&str> = on
    .iter()
    .filter(|act| act.kind == Kind::Rung && act.by == prompt)
    .map(|act| act.id.as_str())
    .collect();
  on.iter()
    .copied()
    .filter(|act| rungs.contains(act.id.as_str()) || rungs.contains(act.by.as_str()))
    .collect()
}

/// The first thread that no operator made on a chain under the root takes the label of that chain.
/// Any other thread keeps the first line of its own message, also each thread of the root: the
/// name of the session stands on the row of the session alone, and a thread never takes it.
fn named(out: &mut [ChainThread], rows: &Rows<'_>) {
  let Some(chain) = out.first().and_then(|one| rows.get(one.prompt.as_str())).map(|act| &act.chain)
  else {
    return;
  };
  let Some(chain) = rows.get(chain.as_str()).filter(|act| act.kind == Kind::Chain) else { return };
  if chain.on.is_empty() {
    return;
  }
  let Some(name) =
    chain.words.first().and_then(Value::as_str).map(str::trim).filter(|one| !one.is_empty())
  else {
    return;
  };
  if let Some(first) = out.iter_mut().find(|one| one.origin != Origin::Operator) {
    name.clone_into(&mut first.title);
  }
}

/// The threads of a chain as a tree, with the threads of each chain under it that nest under one
/// of them: the workers that a round made under a thread, at any depth, so a window draws them
/// under the thread whose word started the round. A thread of a chain under it that nests in none
/// stays the answer of its own chain.
pub fn chain_tree<'a>(chain: &str, acts: &[&'a Act], rows: &Rows<'a>) -> Vec<ThreadNode> {
  let mut by_chain: HashMap<&str, Vec<&Act>> = HashMap::new();
  for act in acts.iter().copied().filter(|act| act.id != act.chain) {
    by_chain.entry(act.chain.as_str()).or_default().push(act);
  }
  let mut list = by_chain.get(chain).map(|on| chain_threads(on, rows)).unwrap_or_default();
  let mut under: Vec<&Act> = acts
    .iter()
    .copied()
    .filter(|act| act.kind == Kind::Chain && act.id != chain && within(&act.id, chain, rows))
    .collect();
  under.sort_by_key(|act| act.began);
  for inner in under {
    let Some(on) = by_chain.get(inner.id.as_str()) else { continue };
    for thread in chain_threads(on, rows) {
      let parent = rows.get(thread.prompt.as_str()).and_then(|act| thread_of(act, rows));
      if parent.is_some_and(|parent| list.iter().any(|one| one.prompt == parent)) {
        list.push(thread);
      }
    }
  }
  nest_threads(list, rows)
}

/// An aside of its own, from its prompt, before the prompts on it join it.
fn aside_of(act: &Act, aside: Aside) -> ChainAside {
  ChainAside { prompt: act.id.clone(), aside, messages: Vec::new() }
}

/// A thread of its own, from its prompt, before its messages and its questions join it.
fn opened(act: &Act, rows: &Rows<'_>) -> ChainThread {
  let maker = rows.get(act.by.as_str());
  let mine = operator(&act.by);
  let origin = match maker {
    _ if mine => Origin::Operator,
    Some(maker) if maker.kind == Kind::Rung => Origin::Rung,
    _ => Origin::Ear,
  };
  let message = act.words.get(1).and_then(|one| one.as_str()).unwrap_or_default();
  ChainThread {
    prompt: act.id.clone(),
    title: message.lines().find(|line| !line.trim().is_empty()).unwrap_or_default().to_owned(),
    messages: Vec::new(),
    asked: Vec::new(),
    asks: Vec::new(),
    state: State::Done,
    over: true,
    origin,
    yours: mine || (origin == Origin::Rung && maker.is_some_and(|one| operator(&one.by))),
  }
}

/// The threads of a list as a tree: each under the thread whose ladder made its prompt, and a
/// thread whose maker is no thread of the list at the top, each list in the order that it was
/// given.
pub fn nest_threads(list: Vec<ChainThread>, rows: &Rows<'_>) -> Vec<ThreadNode> {
  let at: HashMap<&str, usize> =
    list.iter().enumerate().map(|(n, one)| (one.prompt.as_str(), n)).collect();
  let parents: Vec<Option<usize>> = list
    .iter()
    .enumerate()
    .map(|(n, one)| {
      let act = rows.get(one.prompt.as_str())?;
      at.get(thread_of(act, rows)?).copied().filter(|parent| *parent != n)
    })
    .collect();
  let mut threads: Vec<Option<ChainThread>> = list.into_iter().map(Some).collect();
  let tops: Vec<usize> = (0..threads.len()).filter(|n| parents[*n].is_none()).collect();
  tops.into_iter().filter_map(|top| node(top, &parents, &mut threads)).collect()
}

/// The node of a thread of the list, with its agents.
fn node(
  n: usize,
  parents: &[Option<usize>],
  threads: &mut [Option<ChainThread>],
) -> Option<ThreadNode> {
  let thread = threads[n].take()?;
  let agents: Vec<ThreadNode> = (0..parents.len())
    .filter(|one| parents[*one] == Some(n))
    .filter_map(|one| node(one, parents, threads))
    .collect();
  let asks = agents.iter().any(|one| one.state == State::Asks);
  let state = if asks { State::Asks } else { thread.state };
  Some(ThreadNode { thread, agents, state })
}

/// The threads of a tree in the order that the list of the threads shows them: the closed ones
/// first and the open ones last, so that the open ones stand nearest the input, and in each group
/// by the time of their last message, the oldest first. The agents of each thread take the same
/// order.
pub fn order_threads(mut nodes: Vec<ThreadNode>, rows: &Rows<'_>) -> Vec<ThreadNode> {
  for node in &mut nodes {
    node.agents = order_threads(std::mem::take(&mut node.agents), rows);
  }
  nodes.sort_by_key(|node| (open(node), last_message(&node.thread, rows)));
  nodes
}

/// Whether a thread stands open in the list of the threads: it is not over, or a question
/// waits in it or in an agent under it.
pub fn open(node: &ThreadNode) -> bool {
  !node.thread.over || node.state == State::Asks
}

/// When the last message of a thread began: its prompt, or the last prompt on it.
fn last_message(thread: &ChainThread, rows: &Rows<'_>) -> u64 {
  let acts = std::iter::once(&thread.prompt).chain(&thread.messages);
  acts.filter_map(|id| rows.get(id.as_str())).map(|act| act.began).max().unwrap_or_default()
}

/// How a thread concerns the operator, or none, from the state of its node. The failure of an agent
/// does not concern him: it rolls into the thread that made it, and only a question escalates.
pub fn concerns(node: &ThreadNode) -> Option<Concern> {
  match node.state {
    State::Asks => Some(Concern::Asks),
    State::Failed if node.thread.yours => Some(Concern::Failed),
    State::Paused if node.thread.yours => Some(Concern::Paused),
    _ => None,
  }
}

/// What a tree of threads says to the operator, counted by thread: a question counts once, in the
/// thread where it waits, and the failure or the pause of a thread that he started counts in that
/// thread.
pub fn attention(tops: &[ThreadNode]) -> Attention {
  let mut count = Attention::default();
  let mut stack: Vec<&ThreadNode> = tops.iter().collect();
  while let Some(node) = stack.pop() {
    match concerns(node) {
      Some(Concern::Asks) if node.agents.iter().any(|one| one.state == State::Asks) => {}
      Some(Concern::Asks) => count.asks += 1,
      Some(Concern::Failed) => count.failed += 1,
      Some(Concern::Paused) => count.paused += 1,
      None => {}
    }
    stack.extend(&node.agents);
  }
  count
}

#[cfg(test)]
#[path = "threads.test.rs"]
mod tests;
