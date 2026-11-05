//! The threads of a chain: each prompt to a model that is no message of another thread, with its
//! messages and what it asks the operator, nested under the thread whose ladder made it.

use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
  act::{Act, Kind, operator},
  state::State,
};

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
    match maker.kind {
      Kind::Prompt => return Some(root_of(maker, rows)),
      Kind::Rung => maker = rows.get(maker.by.as_str())?,
      _ => return None,
    }
  }
  None
}

/// What a prompt of a chain is that is no conversation, which the feed of its chain draws as what
/// it is and the list of its threads leaves out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "aside", content = "from", rename_all = "snake_case")]
pub enum Aside {
  /// A notice of the round: a prompt of None that a rung or a chain made, which no one answers, as
  /// the acknowledgment of an act that stands in no thread of the chain.
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
/// that the operator made himself.
fn verdict<'a>(act: &'a Act, rows: &Rows<'a>) -> Verdict<'a> {
  if act.thread().is_some() {
    return Verdict::Message(root_of(act, rows));
  }
  let maker = rows.get(act.by.as_str()).copied();
  let none = act.words.first().is_some_and(|shape| shape == "None");
  match maker {
    Some(maker) if none && maker.kind == Kind::Chain => acknowledged(act, rows),
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
  (out, asides)
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
    resolved: true,
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

/// Whether a thread stands open in the list of the threads: it is not resolved, or a question
/// waits in it or in an agent under it.
pub fn open(node: &ThreadNode) -> bool {
  !node.thread.resolved || node.state == State::Asks
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
mod tests {
  use std::collections::HashMap;

  use serde_json::{Value, json};

  use super::{Aside, Rows, chain_asides, chain_threads};
  use crate::view::{Act, Fold};

  /// The acts of the record of a conductor that the tests of the TUI read, with after it what a
  /// record of the owner holds and it lacks, scrubbed: a wake of the round, and a word of the
  /// operator that tells the chain, with the acknowledgment of his rung.
  fn conductor() -> Vec<Act> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../tui/fixtures/conductor.jsonl");
    let Ok(record) = std::fs::read_to_string(path) else { panic!("no record at {path}") };
    let wake = "The ask A27 of the owner is missing, and no item carries it now.";
    let more = [
      json!(["prompt", "prompt9001", "quiet2602", "chain1", "None", wake, ""]),
      json!(["rung", "rung9001", "operator", "chain1", "prompt(None, 'Look at #295.')", "", ""]),
      json!(["prompt", "prompt9002", "rung9001", "chain1", "None", "Look at #295.", ""]),
      json!(["done", "rung9001", "chain1", null]),
      json!(["prompt", "prompt9003", "chain1", "chain1", "None", "rung9001 done", ""]),
    ];
    let facts = record.lines().filter_map(|line| serde_json::from_str::<Value>(line).ok());
    let mut fold = Fold::default();
    let mut acts: HashMap<String, Act> = HashMap::new();
    for fact in facts.filter(Value::is_array).chain(more) {
      for act in fold.hear(&fact, &[], None) {
        acts.insert(act.id.clone(), act);
      }
    }
    let mut all: Vec<Act> = acts.into_values().collect();
    all.sort_by_key(|act| act.began);
    all
  }

  /// The threads of a chain, and its asides with what each is.
  fn listed(acts: &[Act], chain: &str) -> (Vec<String>, Vec<(String, Aside)>) {
    let rows: Rows<'_> = acts.iter().map(|act| (act.id.as_str(), act)).collect();
    let on: Vec<&Act> = acts.iter().filter(|act| act.chain == chain).collect();
    let threads = chain_threads(&on, &rows).into_iter().map(|one| one.prompt).collect();
    let asides = chain_asides(&on, &rows).into_iter().map(|one| (one.prompt, one.aside)).collect();
    (threads, asides)
  }

  fn names(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|id| (*id).to_owned()).collect()
  }

  #[test]
  fn a_notice_a_word_of_the_operator_a_wake_and_a_question_of_another_chain_are_no_threads() {
    let acts = conductor();
    // Before, main listed as the threads of chain1 each prompt of these, and of the asides below.
    let (threads, asides) = listed(&acts, "chain1");
    assert_eq!(threads, names(&["prompt1", "prompt29", "prompt34", "prompt95", "prompt97"]));
    let question = |from: &str| Aside::Question(from.to_owned());
    let expected = [
      ("prompt43", Aside::Notice),
      ("prompt46", question("chain8")),
      ("prompt52", question("chain8")),
      ("prompt63", Aside::Notice),
      ("prompt83", Aside::Notice),
      ("prompt87", Aside::Notice),
      ("prompt92", question("chain9")),
      ("prompt93", question("chain10")),
      ("prompt9001", Aside::Wake),
      ("prompt9002", Aside::Rung),
      ("prompt9003", Aside::Rung),
    ];
    let expected: Vec<(String, Aside)> =
      expected.into_iter().map(|(id, aside)| (id.to_owned(), aside)).collect();
    assert_eq!(asides, expected);
    // Main listed prompt36 as a thread of chain7, and prompt47 and prompt64 as threads of chain2.
    assert_eq!(
      listed(&acts, "chain7"),
      (names(&["prompt31"]), vec![("prompt36".to_owned(), Aside::Notice)])
    );
    let (threads, asides) = listed(&acts, "chain2");
    assert_eq!(threads, names(&["prompt3"]));
    let notices = [("prompt47", Aside::Notice), ("prompt64", Aside::Notice)];
    assert_eq!(asides, notices.map(|(id, aside)| (id.to_owned(), aside)).to_vec());
  }
}
