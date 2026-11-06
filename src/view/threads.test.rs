//! The threads of a chain, their asides, their runs of notes, their nest and their order, as the
//! crate folds them from the acts of a life.

use std::collections::HashMap;

use serde_json::{Value, json};

use super::{
  Aside, Attention, NoteRun, Rows, ThreadNode, chain_asides, chain_notes, chain_threads,
  chain_tree, open,
};
use crate::view::{Act, Fold, Held, State};

/// The acts of the record of a conductor that the tests of the TUI read, with after it what a
/// record of the owner holds and it lacks, scrubbed: a wake of the round, and a word of the
/// operator that tells the chain, with the acknowledgment of his rung.
fn conductor() -> Vec<Act> {
  let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tui/fixtures/conductor.jsonl");
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

/// The acts that a list of facts makes, in the order that they began.
fn folded(facts: impl IntoIterator<Item = Value>) -> Vec<Act> {
  let mut fold = Fold::default();
  let mut acts: HashMap<String, Act> = HashMap::new();
  for fact in facts {
    for act in fold.hear(&fact, &[], None) {
      acts.insert(act.id.clone(), act);
    }
  }
  let mut all: Vec<Act> = acts.into_values().collect();
  all.sort_by_key(|act| act.began);
  all
}

/// The tree of the threads of a chain, each thread as its depth, its prompt and its name.
fn tree(acts: &[Act], chain: &str) -> Vec<(usize, String, String)> {
  fn walk(nodes: &[ThreadNode], depth: usize, out: &mut Vec<(usize, String, String)>) {
    for node in nodes {
      out.push((depth, node.thread.prompt.clone(), node.thread.title.clone()));
      walk(&node.agents, depth + 1, out);
    }
  }
  let rows: Rows<'_> = acts.iter().map(|act| (act.id.as_str(), act)).collect();
  let all: Vec<&Act> = acts.iter().collect();
  let mut out = Vec::new();
  walk(&chain_tree(chain, &all, &rows), 0, &mut out);
  out
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

/// The runs of notes of a chain, each with its prompts and its unread count.
fn runs(acts: &[Act], chain: &str, read: u64) -> Vec<(Vec<String>, usize)> {
  let rows: Rows<'_> = acts.iter().map(|act| (act.id.as_str(), act)).collect();
  let on: Vec<&Act> = acts.iter().filter(|act| act.chain == chain).collect();
  let notes = chain_notes(&on, &rows, read);
  assert!(notes.iter().all(|run| run.count == run.prompts.len()), "{notes:?}");
  notes.into_iter().map(|NoteRun { prompts, unread, .. }| (prompts, unread)).collect()
}

#[test]
fn the_notes_that_follow_one_another_in_a_chain_stand_as_one_run_with_the_ones_not_yet_read() {
  let acts = conductor();
  // The two notices of chain2 follow one another, since the program of its thread stands between
  // them; the one notice of chain7 stands alone.
  let two = names(&["prompt47", "prompt64"]);
  assert_eq!(runs(&acts, "chain2", 0), [(two.clone(), 2)]);
  assert_eq!(runs(&acts, "chain7", 0), [(names(&["prompt36"]), 1)]);
  // A note that began after the place that was read counts as unread, and none else does.
  let began = |id: &str| acts.iter().find(|act| act.id == id).map(|act| act.began).unwrap();
  assert_eq!(runs(&acts, "chain2", began("prompt47")), [(two.clone(), 1)]);
  assert_eq!(runs(&acts, "chain2", u64::MAX), [(two, 0)]);
  // On chain1 the word of the operator that runs between the wake and the acknowledgment of that
  // word ends the first run, and the notices that its threads made stand in their programs.
  let (wake, rung) = (names(&["prompt9001"]), names(&["prompt9003"]));
  assert_eq!(runs(&acts, "chain1", 0), [(wake, 1), (rung, 1)]);
}

#[test]
fn each_worker_nests_under_the_thread_that_made_it_and_takes_the_label_of_its_chain() {
  let acts = conductor();
  let got = tree(&acts, "chain1");
  // Before, the tree of chain1 held its own threads alone, each at the top, and each was named
  // by the first line of its message: a worker showed the head of its brief.
  let shape: Vec<(usize, &str)> = got.iter().map(|(depth, id, _)| (*depth, id.as_str())).collect();
  let expected = [
    (0, "prompt1"),
    (1, "prompt3"),
    (1, "prompt4"),
    (1, "prompt5"),
    (1, "prompt6"),
    (1, "prompt12"),
    (1, "prompt31"),
    (1, "prompt59"),
    (2, "prompt94"),
    (1, "prompt79"),
    (1, "prompt86"),
    (0, "prompt29"),
    (0, "prompt34"),
    (1, "prompt35"),
    (0, "prompt95"),
    (0, "prompt97"),
  ];
  assert_eq!(shape, expected);
  let agents: Vec<(&str, &str)> = got
    .iter()
    .filter(|(depth, ..)| *depth > 0)
    .map(|(_, id, name)| (id.as_str(), name.as_str()))
    .collect();
  let labels = [
    ("prompt3", "life3/thought"),
    ("prompt4", "life3/fabricated"),
    ("prompt5", "life3/qwen"),
    ("prompt6", "life3/citime"),
    ("prompt12", "w/fix295"),
    ("prompt31", "life3/feed"),
    ("prompt59", "w/literal"),
    ("prompt94", "literal/monty"),
    ("prompt79", "w/wheels"),
    ("prompt86", "w/bench-replay"),
    ("prompt35", "untitled"),
  ];
  assert_eq!(agents, labels);
  // The record names no session, so the first thread of the operator keeps its first line.
  let message = acts.iter().find(|act| act.id == "prompt1").and_then(|act| act.words[1].as_str());
  assert_eq!(message.and_then(|one| one.lines().next()), Some(got[0].2.as_str()));
}

#[test]
fn a_worker_that_a_round_made_nests_under_the_thread_that_started_the_round() {
  // A slice of a real record, its messages scrubbed: the operator asks on prompt24, whose word
  // rung234 runs rung236, which conducts a round, conduct1, on the chain chain4 that it made,
  // and the round makes a chain and a prompt for each worker. Before, the walk up the makers
  // stopped at conduct1 and each worker stood at the top.
  let facts = [
    json!(["chain", "chain1", "operator", "", "root", ""]),
    json!(["prompt", "prompt1", "operator", "chain1", "str", "Open the life that died.", ""]),
    json!(["name_session", "name_session1", "rung14", "chain1"]),
    json!(["done", "name_session1", "name_session1", "Resume the crashed Nov 5 session"]),
    json!(["prompt", "prompt24", "operator", "chain1", "str", "\nFix it for each life.", ""]),
    json!(["rung", "rung234", "prompt24", "chain1", "", "", "claude-cli:opus/high"]),
    json!(["rung", "rung236", "rung234", "chain1", "conduct()", "", ""]),
    json!(["chain", "chain4", "rung236", "chain1", "round", ""]),
    json!(["conduct", "conduct1", "rung236", "chain4"]),
    json!(["chain", "chain5", "conduct1", "chain4", "w/fix1296", ""]),
    json!(["prompt", "prompt98", "conduct1", "chain5", "tuple[int, str]", "You fix #1296.", ""]),
    json!(["chain", "chain6", "conduct1", "chain4", "promise/grant-cancelled-reply", ""]),
    json!(["prompt", "prompt100", "conduct1", "chain6", "tuple[int, str]", "An item.", ""]),
  ];
  let acts = folded(facts);
  let got: Vec<(usize, String, String)> = tree(&acts, "chain1");
  let expected = [
    (0, "prompt1", "Resume the crashed Nov 5 session"),
    (0, "prompt24", "Fix it for each life."),
    (1, "prompt98", "w/fix1296"),
    (1, "prompt100", "promise/grant-cancelled-reply"),
  ];
  let expected: Vec<(usize, String, String)> =
    expected.into_iter().map(|(depth, id, name)| (depth, id.to_owned(), name.to_owned())).collect();
  assert_eq!(got, expected);
  // The chain of a worker answers its own thread, nested in none, under the label of its chain.
  assert_eq!(tree(&acts, "chain5"), vec![(0, "prompt98".to_owned(), "w/fix1296".to_owned())]);
}

#[test]
fn the_name_that_the_operator_gave_the_session_wins_over_the_name_that_naming_gave() {
  let named = |more: Vec<Value>| {
    let mut facts = vec![
      json!(["chain", "chain1", "operator", "", "root", ""]),
      json!(["name_session", "name_session1", "rung1", "chain1"]),
      json!(["done", "name_session1", "name_session1", "A given name"]),
    ];
    facts.extend(more);
    let acts = folded(facts);
    let on: Vec<&Act> = acts.iter().filter(|act| act.chain == "chain1").collect();
    crate::view::Named::of_acts(&on).name().map(|one| one.trim().to_owned())
  };
  assert_eq!(named(vec![]).as_deref(), Some("A given name"));
  let said = |id: &str, name: &str| {
    [json!(["rename_session", id, "rung2", "chain1"]), json!(["done", id, id, name])]
  };
  assert_eq!(named(said("rename_session1", "Mine").to_vec()).as_deref(), Some("Mine"));
  let both = [said("rename_session1", "Mine"), said("rename_session2", "")].concat();
  assert_eq!(named(both).as_deref(), Some(""));
}

/// The meaning that a window reads of the facts of a record: each act heard as the fold of the
/// record gives it, pauses and wakes found as the engine finds them.
fn held(facts: impl IntoIterator<Item = Value>) -> Held {
  let mut fold = Fold::default();
  let mut held = Held::default();
  for fact in facts {
    for act in fold.recorded(&fact) {
      held.heard(&act);
    }
  }
  held
}

/// A prompt of the operator on the root.
fn asked(id: &str) -> Value {
  json!(["prompt", id, "operator", "chain1", "str", format!("{id} says"), ""])
}

/// The prompts of a tree, each with its agents.
fn shape(nodes: &[ThreadNode]) -> Vec<(String, Vec<String>)> {
  nodes
    .iter()
    .map(|node| {
      let agents = node.agents.iter().map(|one| one.thread.prompt.clone()).collect();
      (node.thread.prompt.clone(), agents)
    })
    .collect()
}

#[test]
fn a_question_and_a_notice_that_a_thread_made_for_the_operator_stand_in_it_with_the_questions_that_wait()
 {
  let got = held([
    json!(["chain", "chain1", "operator", "", "root", ""]),
    asked("prompt1"),
    json!(["rung", "rung1", "prompt1", "chain1", "x = 1", "", ""]),
    json!(["prompt", "prompt2", "rung1", "chain1", "bool", "Keep it?", "operator"]),
    json!(["started", "prompt2", "console"]),
    json!(["prompt", "prompt3", "rung1", "chain1", "None", "It is 14 lines.", "operator"]),
  ])
  .threads("chain1", None, 0);
  let [node] = got.threads.as_slice() else { panic!("{got:?}") };
  assert_eq!(node.thread.asked, names(&["prompt2", "prompt3"]));
  assert_eq!(node.thread.asks, names(&["prompt2"]));
  assert_eq!(node.state, State::Asks);
  assert_eq!(got.attention, Attention { asks: 1, failed: 0, paused: 0 });
}

#[test]
fn an_acknowledgment_that_a_chain_makes_is_a_message_of_the_thread_of_the_act_that_it_names() {
  let got = held([
    json!(["chain", "chain1", "operator", "", "root", ""]),
    asked("prompt1"),
    json!(["rung", "rung1", "prompt1", "chain1", "x = 1", "", ""]),
    json!(["prompt", "prompt2", "rung1", "chain1", "str", "Work.", ""]),
    json!(["done", "prompt2", "rung1", "worked"]),
    json!(["prompt", "prompt3", "chain1", "chain1", "None", "prompt2 done", ""]),
  ])
  .threads("chain1", None, 0);
  let messages: Vec<(String, Vec<String>)> = std::iter::once(&got.threads[0])
    .chain(&got.threads[0].agents)
    .map(|node| (node.thread.prompt.clone(), node.thread.messages.clone()))
    .collect();
  assert_eq!(
    messages,
    vec![("prompt1".to_owned(), names(&["prompt3"])), ("prompt2".to_owned(), Vec::new())]
  );
}

#[test]
fn the_agents_of_a_thread_nest_under_it_and_a_question_in_an_agent_makes_each_thread_over_it_ask() {
  let mut facts = vec![
    json!(["chain", "chain1", "operator", "", "root", ""]),
    asked("prompt1"),
    json!(["rung", "rung1", "prompt1", "chain1", "hire()", "", ""]),
  ];
  let agents: Vec<String> = (2..10).map(|n| format!("prompt{n}")).collect();
  for id in &agents {
    facts.push(json!(["prompt", id, "rung1", "chain1", "str", "Work.", ""]));
  }
  facts.extend([
    json!(["rung", "rung2", "prompt9", "chain1", "hire()", "", ""]),
    json!(["prompt", "prompt20", "rung2", "chain1", "str", "Work.", ""]),
    json!(["rung", "rung3", "prompt20", "chain1", "ask()", "", ""]),
    json!(["prompt", "prompt21", "rung3", "chain1", "bool", "Keep it?", "operator"]),
    json!(["started", "prompt21", "console"]),
    asked("prompt30"),
  ]);
  let got = held(facts).threads("chain1", None, 0);
  let tops = &got.threads;
  assert_eq!(
    shape(tops),
    vec![("prompt1".to_owned(), agents), ("prompt30".to_owned(), Vec::new())]
  );
  let last = &tops[0].agents[7];
  assert_eq!(shape(std::slice::from_ref(last)), vec![("prompt9".to_owned(), names(&["prompt20"]))]);
  assert_eq!([tops[0].state, last.state, tops[1].state], [State::Asks, State::Asks, State::Runs]);
  // The question rolls into the node over it, and the thread over it keeps its own state and asks.
  assert_eq!((tops[0].thread.asks.len(), tops[0].thread.state), (0, State::Runs));
  // A question counts once, in the thread where it waits.
  assert_eq!(got.attention, Attention { asks: 1, failed: 0, paused: 0 });
}

#[test]
fn a_failure_or_a_pause_concerns_the_operator_only_in_a_thread_that_he_started() {
  let error = json!({"is": "Refused", "args": ["no"]});
  let attention = |by: &str, last: Value| {
    held([
      json!(["chain", "chain1", "operator", "", "root", ""]),
      asked("prompt1"),
      json!(["rung", "rung1", "operator", "chain1", "x = 1", "", ""]),
      json!(["rung", "rung2", "prompt1", "chain1", "hire()", "", ""]),
      json!(["prompt", "prompt2", by, "chain1", "str", "Work.", ""]),
      last,
    ])
    .threads("chain1", None, 0)
    .attention
  };
  let failed = json!(["done", "prompt2", "", error]);
  let paused = json!(["pause", "prompt2", "operator", "held"]);
  // A thread that a word of the operator made concerns him when it fails or a pause holds it.
  assert_eq!(attention("rung1", failed.clone()), Attention { asks: 0, failed: 1, paused: 0 });
  assert_eq!(attention("rung1", paused.clone()), Attention { asks: 0, failed: 0, paused: 1 });
  // An agent that a ladder hired does not: its failure rolls into the thread that hired it.
  assert_eq!(attention("rung2", failed), Attention::default());
  assert_eq!(attention("rung2", paused), Attention::default());
}

#[test]
fn the_rail_holds_the_closed_threads_first_and_the_open_ones_last_each_by_its_last_message() {
  let facts = vec![
    json!(["chain", "chain1", "operator", "", "root", ""]),
    asked("prompt1"),
    asked("prompt2"),
    asked("prompt3"),
    asked("prompt4"),
    asked("prompt6"),
    json!(["rung", "rung1", "prompt6", "chain1", "hire()", "", ""]),
    json!(["prompt", "prompt7", "rung1", "chain1", "str", "Work.", ""]),
    json!(["prompt", "prompt8", "rung1", "chain1", "str", "Work.", ""]),
    json!(["rung", "rung2", "prompt7", "chain1", "ask()", "", ""]),
    json!(["prompt", "prompt10", "rung2", "chain1", "bool", "Keep it?", "operator"]),
    json!(["started", "prompt10", "console"]),
    json!(["prompt", "prompt9", "operator", "prompt3", "str", "More.", ""]),
    json!(["prompt", "prompt5", "operator", "prompt1", "str", "More.", ""]),
    json!(["done", "prompt5", "", "ok"]),
    json!(["done", "prompt1", "", "ok"]),
    json!(["done", "prompt2", "", "ok"]),
    json!(["done", "prompt6", "", "ok"]),
    json!(["done", "prompt8", "", "ok"]),
  ];
  let life = held(facts);
  let rail = life.rail(&[], None);
  // prompt1 had its last message after prompt2; prompt6 is over, and stands open while a question
  // waits in its agent prompt7; prompt3 had its last message after prompt4.
  let tops: Vec<&str> = rail.iter().map(|node| node.thread.prompt.as_str()).collect();
  assert_eq!(tops, ["prompt2", "prompt1", "prompt4", "prompt6", "prompt3"]);
  assert_eq!(rail.iter().map(open).collect::<Vec<_>>(), [false, false, true, true, true]);
  assert_eq!(shape(&rail[3..4]), vec![("prompt6".to_owned(), names(&["prompt8", "prompt7"]))]);
  // The rail leaves out each chain that it names, and stands as it stood at a moment.
  assert!(life.rail(&names(&["chain1"]), None).is_empty());
  let moments = life.moments();
  let early = life.rail(&[], moments.get(2).copied());
  assert_eq!(
    early.iter().map(|node| node.thread.prompt.as_str()).collect::<Vec<_>>(),
    ["prompt1", "prompt2"]
  );
}
