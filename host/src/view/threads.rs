//! The threads of a chain: each prompt to a model that is no message of another thread, with its
//! messages and what it asks the operator, nested under the thread whose ladder made it.

use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{act::Act, state::State};

/// The acts of a life by their names.
pub type Rows<'a> = HashMap<&'a str, &'a Act>;

/// How deep a walk up the threads or the makers goes at most.
const DEEP: usize = 64;

/// One thread of a chain: its prompt, the first line of its message, its messages, the prompts to
/// the operator that it made and the questions of them that wait, and its state. A thread is
/// resolved once its prompt and each of its messages is done, and a message to a resolved thread
/// opens it again until its model closes the message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ChainThread {
  pub prompt: String,
  pub title: String,
  pub messages: Vec<String>,
  pub asked: Vec<String>,
  pub asks: Vec<String>,
  pub state: State,
  pub resolved: bool,
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

/// The thread that an act stands in: the first prompt of the thread whose rung is over it, walking
/// up the makers. A prompt that the operator, an ear or a chain made stands in none.
pub fn thread_of<'a>(act: &Act, rows: &Rows<'a>) -> Option<&'a str> {
  let mut maker = rows.get(act.by.as_str())?;
  for _ in 0..DEEP {
    match maker.kind.as_str() {
      "prompt" => return Some(root_of(maker, rows)),
      "rung" => maker = rows.get(maker.by.as_str())?,
      _ => return None,
    }
  }
  None
}

/// What a prompt is to the threads.
enum Verdict<'a> {
  /// A thread of its own.
  Own,
  /// A message of a thread.
  Message(&'a str),
  /// The acknowledgment of an act that stands in no thread.
  Stray,
}

/// Whether a prompt is a message of a thread: a prompt on a thread, or a prompt of None that a
/// chain made as the acknowledgment of an act, which belongs to the thread of that act.
fn verdict<'a>(act: &'a Act, rows: &Rows<'a>) -> Verdict<'a> {
  if act.thread().is_some() {
    return Verdict::Message(root_of(act, rows));
  }
  let by_chain = rows.get(act.by.as_str()).is_some_and(|maker| maker.kind == "chain");
  if act.words.first().is_none_or(|shape| shape != "None") || !by_chain {
    return Verdict::Own;
  }
  let message = act.words.get(1).and_then(|one| one.as_str()).unwrap_or_default();
  let about = message.split_once(' ').map(|(about, _)| about).filter(|about| {
    about.len() > 1
      && about.ends_with(|one: char| one.is_ascii_digit())
      && about.chars().all(|one| one.is_ascii_alphanumeric() || one == '_')
  });
  match about.and_then(|about| rows.get(about)) {
    None => Verdict::Stray,
    Some(subject) if subject.kind == "prompt" => {
      Verdict::Message(thread_of(subject, rows).unwrap_or(&subject.id))
    }
    Some(subject) => thread_of(subject, rows).map_or(Verdict::Own, Verdict::Message),
  }
}

/// Whether an act is the operator's own: made by him, or by one of his sessions.
fn by_operator(by: &str) -> bool {
  by == "operator" || by.starts_with("operator/")
}

/// The threads of one chain, from its acts in the order they began: each prompt to a model on the
/// chain that is no message of another thread. It asks while a question waits in it.
pub fn chain_threads(on: &[&Act], rows: &Rows<'_>) -> Vec<ChainThread> {
  let mut out: Vec<ChainThread> = Vec::new();
  let mut later: Vec<(&str, &Act)> = Vec::new();
  for act in on.iter().copied().filter(|act| act.kind == "prompt") {
    if act.words.get(2).is_some_and(|to| to == "operator") {
      later.extend(thread_of(act, rows).map(|thread| (thread, act)));
      continue;
    }
    match verdict(act, rows) {
      Verdict::Message(thread) => later.push((thread, act)),
      Verdict::Stray => {}
      Verdict::Own => out.push(opened(act, rows)),
    }
  }
  for (thread, act) in later {
    let Some(at) = out.iter_mut().find(|one| one.prompt == thread) else { continue };
    if act.words.get(2).is_some_and(|to| to == "operator") {
      at.asked.push(act.id.clone());
      at.asks.extend(act.owed().then(|| act.id.clone()));
    } else {
      at.messages.push(act.id.clone());
    }
  }
  for thread in &mut out {
    let acts = std::iter::once(&thread.prompt).chain(&thread.messages);
    let open = acts.filter_map(|id| rows.get(id.as_str())).rfind(|act| !act.done());
    thread.resolved = open.is_none();
    thread.state = match open.or_else(|| rows.get(thread.prompt.as_str())) {
      _ if !thread.asks.is_empty() => State::Asks,
      Some(act) => State::of(act),
      None => State::Idle,
    };
  }
  out
}

/// A thread of its own, from its prompt, before its messages and its questions join it.
fn opened(act: &Act, rows: &Rows<'_>) -> ChainThread {
  let maker = rows.get(act.by.as_str());
  let mine = by_operator(&act.by);
  let origin = match maker {
    _ if mine => Origin::Operator,
    Some(maker) if maker.kind == "rung" => Origin::Rung,
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
    resolved: true,
    origin,
    yours: mine || (origin == Origin::Rung && maker.is_some_and(|one| by_operator(&one.by))),
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
