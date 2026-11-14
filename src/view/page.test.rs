use serde_json::{Value, json};

use super::{Answered, Block, Mark, Reach, Reply, Said, Saying, Streamed, Thought, Work};
use crate::view::{Fold, Held, Maker, Standing, declined};

/// The acts of a list of facts as a host holds them, with what the host heard at the second 100.
fn held(facts: impl IntoIterator<Item = Value>, streams: &mut Streamed) -> Held {
  let mut fold = Fold::default();
  let mut held = Held::default();
  for fact in facts {
    for act in fold.recorded(&fact) {
      streams.heard(&act, 100);
      held.heard(&act);
    }
  }
  held
}

/// A thread of the operator on the root: his prompt, a word of two lines that answers it and
/// closes it with a text, then his message on it, which a word answers with a command that runs.
fn thread() -> Vec<Value> {
  vec![
    json!(["chain", "chain1", "operator", "", "root", ""]),
    json!(["prompt", "prompt1", "operator", "chain1", "str", "where does the time go", ""]),
    json!(["rung", "rung1", "prompt1", "chain1", "", "", "claude-cli:opus/high"]),
    json!(["run", "run1", "chain1", "chain1", "rung1", "x = 1\nclose('In the boot.')", ""]),
    json!(["prompt", "prompt2", "rung1", "chain1", "None", "It is 14 lines.", "operator"]),
    json!(["done", "prompt1", "rung1", "In the boot."]),
    json!(["done", "run1", "chain1", null]),
    json!(["done", "rung1", "chain1", null]),
    json!(["prompt", "prompt3", "operator", "prompt1", "str", "and the seed?", ""]),
    json!(["rung", "rung2", "prompt3", "chain1", "", "", "claude-cli:opus/high"]),
    json!(["run", "run2", "chain1", "chain1", "rung2", "a = 2\nb = await bash('ls')\nc = 3", ""]),
    json!(["bash", "bash1", "rung2", "chain1", "ls", "", 120.0, null]),
  ]
}

fn texts(block: &Block) -> Vec<(usize, &str, &Mark)> {
  block.lines.iter().map(|line| (line.number, line.text.as_str(), &line.mark)).collect()
}

#[test]
fn a_page_gives_each_message_with_its_block_numbered_through_the_thread_and_what_it_said() {
  let mut streams = Streamed::default();
  let held = held(thread(), &mut streams);
  let page = held.page(&streams, "prompt1", Reach { before: None, limit: 10 });
  assert_eq!(page.above, 0);
  let [first, second] = page.exchanges.as_slice() else { panic!("{page:?}") };
  assert_eq!(
    (first.at, first.text.as_str(), &first.by),
    (0, "where does the time go", &Maker::Operator)
  );
  let block = first.block.as_ref().unwrap();
  let opus = Maker::Model { model: "claude-cli:opus".to_owned(), effort: Some("high".to_owned()) };
  assert_eq!(block.maker, opus);
  assert!(!block.more);
  assert_eq!(texts(block), [(1, "x = 1", &Mark::Ran), (2, "close('In the boot.')", &Mark::Ran)]);
  let said =
    |act: &str, kind, text: &str| Said { act: act.to_owned(), kind, text: text.to_owned() };
  assert_eq!(
    first.said,
    [
      said("prompt2", Saying::Notice, "It is 14 lines."),
      said("prompt1", Saying::Answer, "In the boot.")
    ]
  );
  // The block of the second message goes on from the lines of the first, and its line that awaits
  // the command runs since the host saw the command begin.
  let block = second.block.as_ref().unwrap();
  let runs = Mark::Runs { work: Work::Command, since: Some(100) };
  assert_eq!(
    texts(block),
    [(3, "a = 2", &Mark::Ran), (4, "b = await bash('ls')", &runs), (5, "c = 3", &Mark::Ran)]
  );
  assert!(block.more);
}

#[test]
fn an_older_page_holds_the_exchanges_before_the_one_that_it_names_with_their_numbers() {
  let mut streams = Streamed::default();
  let held = held(thread(), &mut streams);
  let tip = held.page(&streams, "prompt1", Reach { before: None, limit: 1 });
  assert_eq!((tip.above, tip.exchanges[0].at), (1, 1));
  assert_eq!(tip.exchanges[0].block.as_ref().unwrap().lines[0].number, 3);
  let older = held.page(&streams, "prompt1", Reach { before: Some(1), limit: 1 });
  assert_eq!((older.above, older.exchanges.len(), older.exchanges[0].at), (0, 1, 0));
  assert_eq!(
    held.page(&streams, "reply9", Reach { before: None, limit: 1 }),
    super::Page::default()
  );
}

#[test]
fn a_model_that_writes_shows_the_lines_that_came_the_line_that_comes_and_its_thought() {
  let mut streams = Streamed::default();
  let mut facts = thread();
  facts.push(json!(["rung", "rung3", "prompt3", "chain1", "", "", "claude-cli:opus/high"]));
  facts.push(json!(["reply", "reply1", "rung3", "chain1", "claude-cli:opus/high"]));
  let held = held(facts, &mut streams);
  streams.wrote("rung3", ["", "The seed is read once. Then"], 101);
  let thought = |took| Some(Thought { text: "The seed is read once. Then".to_owned(), took });
  let block = |streams: &Streamed| {
    held.page(streams, "prompt1", Reach { before: None, limit: 1 }).exchanges[0]
      .block
      .clone()
      .unwrap()
  };
  assert_eq!(block(&streams).thought, thought(None));
  streams.wrote("rung3", ["d = 4\ne = rea", ""], 104);
  let block = block(&streams);
  assert_eq!(block.thought, thought(Some(3)));
  assert_eq!(block.thought.as_ref().unwrap().first(), "The seed is read once.");
  let writes = Mark::Runs { work: Work::Writes, since: Some(101) };
  assert_eq!(&texts(&block)[3..], [(6, "d = 4", &Mark::Ran), (7, "e = rea", &writes)]);
}

#[test]
fn a_line_that_failed_carries_its_reason() {
  let mut streams = Streamed::default();
  let mut facts = thread();
  let fault = json!({"is": "NameError", "args": ["name 'd' is not defined at line 3"]});
  facts.push(json!(["done", "run2", "chain1", fault]));
  facts.push(json!(["done", "rung2", "chain1", fault]));
  let held = held(facts, &mut streams);
  let page = held.page(&streams, "prompt1", Reach { before: None, limit: 1 });
  let block = page.exchanges[0].block.as_ref().unwrap();
  let why = "NameError: name 'd' is not defined at line 3".to_owned();
  assert_eq!(block.lines[2].mark, Mark::Failed { why });
  assert_eq!(block.lines[1].mark, Mark::Ran);
}

#[test]
fn the_agents_of_a_thread_stand_as_cards() {
  let mut streams = Streamed::default();
  let mut facts = thread();
  facts.push(json!(["chain", "chain2", "rung2", "chain1", "worker", ""]));
  facts.push(json!(["prompt", "prompt4", "rung2", "chain2", "str", "study the boot\nwhole", ""]));
  facts.push(json!(["rung", "rung4", "prompt4", "chain2", "", "", "claude-cli:sonnet"]));
  facts.push(json!(["reply", "reply2", "rung4", "chain2", "claude-cli:sonnet"]));
  let held = held(facts, &mut streams);
  let page = held.page(&streams, "prompt1", Reach { before: None, limit: 1 });
  let [agent] = page.agents.as_slice() else { panic!("{page:?}") };
  assert_eq!(agent.title, "study the boot");
  assert_eq!(agent.standing, Standing::Thinks);
  assert_eq!(agent.model.as_deref(), Some("claude-cli:sonnet"));
}

#[test]
fn a_quote_stands_as_its_binding_and_a_string_of_many_lines_as_one_line() {
  let word = "<s:plan>\nread it\n</s:plan>\nx = \"\"\"a\nb\"\"\"\ny = 1";
  let shown = super::shown(word);
  let lines: Vec<(usize, &str)> = shown.iter().map(|(at, text)| (*at, text.as_str())).collect();
  assert_eq!(lines, [(0, "plan = 'read it\\n'"), (3, "x = \"\"\"a\\nb\"\"\""), (5, "y = 1")]);
}

#[test]
fn a_question_holds_its_shape_and_once_closed_who_closed_it_and_what_it_came_to() {
  let word = "side = await prompt(Literal['right', 'left'], 'Where?', to=OPERATOR)\nok = await prompt(bool, 'Sure?', to=OPERATOR)";
  let facts = vec![
    json!(["chain", "chain1", "operator", "", "root", ""]),
    json!(["prompt", "prompt1", "operator", "chain1", "str", "where does the list stand", ""]),
    json!(["rung", "rung1", "prompt1", "chain1", "", "", "claude-cli:opus"]),
    json!(["run", "run1", "chain1", "chain1", "rung1", word, ""]),
    json!([
      "prompt",
      "prompt2",
      "rung1",
      "chain1",
      "Literal['right', 'left']",
      "Where?",
      "operator"
    ]),
    json!(["prompt", "prompt3", "rung1", "chain1", "bool", "Sure?", "operator"]),
    json!(["prompt", "prompt4", "rung1", "chain1", "int", "How many?", "operator"]),
    json!(["close", "prompt2", "operator/ada", "left"]),
    json!(["done", "prompt2", "chain1", "left"]),
    json!(["close", "prompt3", "rung1", true]),
    json!(["done", "prompt3", "chain1", true]),
  ];
  let mut refused = facts.clone();
  let refusal: Value = serde_json::from_str(&declined("the top")).unwrap();
  refused.extend([
    json!(["close", "prompt4", "operator/ada", refusal]),
    json!(["done", "prompt4", "chain1", refusal]),
  ]);
  let question = |shape: &str, answered| Saying::Question { shape: shape.to_owned(), answered };
  let kinds = |facts: Vec<Value>| {
    let mut streams = Streamed::default();
    let page =
      held(facts, &mut streams).page(&streams, "prompt1", Reach { before: None, limit: 10 });
    page.exchanges[0].said.iter().map(|one| one.kind.clone()).collect::<Vec<_>>()
  };
  let opus = Maker::Model { model: "claude-cli:opus".to_owned(), effort: None };
  let left = Answered { by: Maker::Operator, reply: Reply::Value("left".to_owned()) };
  let yes = Answered { by: opus, reply: Reply::Value("yes".to_owned()) };
  assert_eq!(
    kinds(facts),
    [
      question("Literal['right', 'left']", Some(left.clone())),
      question("bool", Some(yes.clone())),
      question("int", None)
    ]
  );
  let the_top = Answered { by: Maker::Operator, reply: Reply::Refused("the top".to_owned()) };
  assert_eq!(
    kinds(refused),
    [
      question("Literal['right', 'left']", Some(left)),
      question("bool", Some(yes)),
      question("int", Some(the_top))
    ]
  );
}

#[test]
fn each_line_that_asks_a_question_that_waits_carries_the_mark_of_a_question() {
  let mut streams = Streamed::default();
  let word =
    "one = prompt(bool, 'A?', to=OPERATOR)\ntwo = prompt(str, 'B?', to=OPERATOR)\nclose(await one)";
  let facts = vec![
    json!(["chain", "chain1", "operator", "", "root", ""]),
    json!(["prompt", "prompt1", "operator", "chain1", "str", "ask twice", ""]),
    json!(["rung", "rung1", "prompt1", "chain1", "", "", "claude-cli:opus"]),
    json!(["run", "run1", "chain1", "chain1", "rung1", word, ""]),
    json!(["prompt", "prompt2", "rung1", "chain1", "bool", "A?", "operator"]),
    json!(["prompt", "prompt3", "rung1", "chain1", "str", "B?", "operator"]),
  ];
  let held = held(facts, &mut streams);
  let page = held.page(&streams, "prompt1", Reach { before: None, limit: 10 });
  let block = page.exchanges[0].block.as_ref().unwrap();
  let marks: Vec<&Mark> = block.lines.iter().map(|line| &line.mark).collect();
  assert_eq!(marks, [&Mark::Asks, &Mark::Asks, &Mark::Ran]);
}
