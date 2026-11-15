use serde_json::{Value, json};

use crate::view::{
  Answered, Block, Context, Fold, Held, Line, Maker, Mark, Output, Portion, Reach, Read, Reply,
  Retrying, Said, Saying, Standing, Streamed, Thought, Work, declined,
};

/// The acts of a list of facts as a host holds them, with what the host heard at the second 100.
fn held(facts: impl IntoIterator<Item = Value>, streams: &mut Streamed) -> Held {
  let mut fold = Fold::default();
  let mut held = Held::default();
  for fact in facts {
    for act in fold.recorded(&fact) {
      streams.heard(&act, 100_000);
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
  let runs = Mark::Runs { work: Work::Command, began: Some(100_000) };
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
  streams.wrote("rung3", ["", "The seed is read once. Then"], 101_000);
  let text = "The seed is read once. Then".to_owned();
  let thought = |took| Some(Thought { text: text.clone(), began: 101_000, took });
  let block = |streams: &Streamed| {
    held.page(streams, "prompt1", Reach { before: None, limit: 1 }).exchanges[0]
      .block
      .clone()
      .unwrap()
  };
  assert_eq!(block(&streams).thought, thought(None));
  streams.wrote("rung3", ["d = 4\ne = rea", ""], 104_000);
  let block = block(&streams);
  assert_eq!(block.thought, thought(Some(3)));
  assert_eq!(block.thought.as_ref().unwrap().first(), "The seed is read once.");
  let writes = Mark::Runs { work: Work::Writes, began: Some(101_000) };
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
fn a_fault_whose_reason_names_no_line_marks_the_line_of_the_act_that_failed() {
  let mut streams = Streamed::default();
  let word = "side = await prompt(Literal['right', 'left'], 'Where?', to=OPERATOR)\nclose(side)";
  let refusal: Value = serde_json::from_str(&declined("the top")).unwrap();
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
    json!(["close", "prompt2", "operator/ada", refusal]),
    json!(["done", "prompt2", "chain1", refusal]),
    json!(["done", "run1", "chain1", refusal]),
    json!(["done", "rung1", "chain1", refusal]),
  ];
  let held = held(facts, &mut streams);
  let page = held.page(&streams, "prompt1", Reach { before: None, limit: 1 });
  let block = page.exchanges[0].block.as_ref().unwrap();
  assert!(matches!(block.lines[0].mark, Mark::Failed { .. }), "{block:?}");
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
  assert_eq!(agent.card.title, "study the boot");
  assert_eq!(agent.card.standing, Standing::Thinks);
  assert_eq!(agent.card.model.as_deref(), Some("claude-cli:sonnet"));
}

#[test]
fn an_agent_says_when_it_began_when_it_ended_and_the_line_where_it_stands_and_a_note_is_none() {
  let mut facts = thread();
  facts.extend([
    json!(["chain", "chain2", "rung2", "chain1", "worker", ""]),
    json!(["prompt", "prompt4", "rung2", "chain2", "str", "count the lines", ""]),
    json!(["rung", "rung4", "prompt4", "chain2", "", "", "claude-cli:sonnet"]),
    json!(["run", "run4", "chain2", "chain2", "rung4", "close('3 lines')", ""]),
    json!(["chain", "chain3", "rung2", "chain1", "bench", ""]),
    json!(["prompt", "prompt5", "rung2", "chain3", "str", "run the bench", ""]),
    json!(["rung", "rung5", "prompt5", "chain3", "", "", "claude-cli:sonnet"]),
    json!(["run", "run5", "chain3", "chain3", "rung5", "n = 1\nout = await bash('make')", ""]),
    json!(["bash", "bash5", "rung5", "chain3", "make", "", 120.0, null]),
    json!(["done", "prompt4", "rung4", "3 lines"]),
    json!(["done", "run4", "chain2", null]),
    json!(["done", "rung4", "chain2", null]),
    // A note to a chain that works is no agent: its model reads it at its next step.
    json!(["prompt", "prompt6", "rung2", "chain3", "None", "and the seed", ""]),
  ]);
  // The host hears each fact 600 ms after the one before, from the second 100 on.
  let (mut fold, mut held, mut streams) = (Fold::default(), Held::default(), Streamed::default());
  for (at, fact) in facts.iter().enumerate() {
    for act in fold.recorded(fact) {
      streams.heard(&act, 100_000 + at as u64 * 600);
      held.heard(&act);
    }
  }
  let page = held.page(&streams, "prompt1", Reach { before: None, limit: 1 });
  let [counts, bench] = page.agents.as_slice() else { panic!("{page:?}") };
  // It began at 107.8 s and ended at 112.6 s: it took 4.8 s, so 4, though its seconds differ by 5.
  let times = (counts.card.title.as_str(), counts.began, counts.took);
  assert_eq!(times, ("count the lines", Some(107_800), Some(4)));
  let line = counts.line.as_ref().map(|line| (line.text.as_str(), &line.mark));
  assert_eq!(line, Some(("close('3 lines')", &Mark::Ran)));
  assert_eq!((bench.began, bench.took), (Some(110_200), None));
  let runs = Mark::Runs { work: Work::Command, began: Some(112_000) };
  let line = bench.line.as_ref().map(|line| (line.text.as_str(), &line.mark));
  assert_eq!(line, Some(("out = await bash('make')", &runs)));
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

/// A thread whose prompt stays open while the operator says more on it, as a model of a long work
/// answers each message on the prompt of the thread, and the engine wakes the thread on it.
fn conversation() -> Vec<Value> {
  vec![
    json!(["chain", "chain1", "operator", "", "root", ""]),
    json!(["prompt", "prompt1", "operator", "chain1", "str", "unstick the session", ""]),
    json!(["rung", "rung1", "prompt1", "chain1", "", "", "claude-cli:opus/high"]),
    json!(["run", "run1", "chain1", "chain1", "rung1", "a = 1", ""]),
    json!(["done", "run1", "chain1", null]),
    json!(["done", "rung1", "chain1", null]),
    json!(["prompt", "prompt2", "operator", "prompt1", "None", "merged ?", ""]),
    json!(["prompt", "prompt3", "chain1", "prompt1", "None", "bash1 done", ""]),
    json!(["prompt", "prompt4", "prompt1", "prompt1", "None", "prompt2 unanswered", ""]),
    json!(["rung", "rung2", "prompt1", "chain1", "", "", "claude-cli:opus/high"]),
    json!(["run", "run2", "chain1", "chain1", "rung2", "b = 2", ""]),
    json!(["prompt", "prompt5", "rung2", "chain1", "None", "Yes, it merged.", "operator"]),
    json!(["done", "prompt5", "operator", null]),
    json!(["done", "run2", "chain1", null]),
    json!(["done", "rung2", "chain1", null]),
  ]
}

#[test]
fn each_answer_stands_after_the_message_that_came_before_it_whichever_prompt_its_rung_advances() {
  let mut streams = Streamed::default();
  let held = held(conversation(), &mut streams);
  let page = held.page(&streams, "prompt1", Reach { before: None, limit: 10 });
  let [first, second] = page.exchanges.as_slice() else { panic!("{page:?}") };
  assert_eq!(texts(first.block.as_ref().unwrap()), [(1, "a = 1", &Mark::Ran)]);
  assert!(first.said.is_empty(), "{first:?}");
  assert_eq!(second.text, "merged ?");
  let block = second.block.as_ref().unwrap();
  assert_eq!(texts(block), [(2, "b = 2", &Mark::Ran)]);
  let merged =
    Said { act: "prompt5".to_owned(), kind: Saying::Notice, text: "Yes, it merged.".to_owned() };
  assert_eq!(second.said, [merged]);
}

#[test]
fn a_wake_that_the_engine_says_on_a_thread_is_no_message_of_the_operator() {
  let mut streams = Streamed::default();
  let held = held(conversation(), &mut streams);
  let page = held.page(&streams, "prompt1", Reach { before: None, limit: 10 });
  let messages: Vec<&str> = page.exchanges.iter().map(|one| one.message.as_str()).collect();
  assert_eq!(messages, ["prompt1", "prompt2"]);
}

#[test]
fn a_notice_stands_after_the_message_that_came_before_it_though_its_rung_began_before() {
  let mut streams = Streamed::default();
  let facts = vec![
    json!(["chain", "chain1", "operator", "", "root", ""]),
    json!(["prompt", "prompt1", "operator", "chain1", "str", "set a goal", ""]),
    json!(["rung", "rung1", "prompt1", "chain1", "", "", "claude-cli:opus/high"]),
    json!(["run", "run1", "chain1", "chain1", "rung1", "goal = await read('plan')", ""]),
    json!(["prompt", "prompt2", "operator", "prompt1", "None", "your goal is stable", ""]),
    json!(["prompt", "prompt3", "rung1", "chain1", "None", "Yes: stable.", "operator"]),
  ];
  let held = held(facts, &mut streams);
  let page = held.page(&streams, "prompt1", Reach { before: None, limit: 10 });
  let [first, second] = page.exchanges.as_slice() else { panic!("{page:?}") };
  assert_eq!(first.block.as_ref().unwrap().lines.len(), 1);
  assert!(first.said.is_empty(), "{first:?}");
  let yes =
    Said { act: "prompt3".to_owned(), kind: Saying::Notice, text: "Yes: stable.".to_owned() };
  assert_eq!(second.said, [yes]);
}

/// A thread whose word runs a loop that makes a command at each turn, then a line after the loop.
fn looped(more: impl IntoIterator<Item = Value>) -> Vec<Value> {
  let word = "for c in ['a', 'b']:\n  await bash(c)\nprint('done')";
  let mut facts = vec![
    json!(["chain", "chain1", "operator", "", "root", ""]),
    json!(["prompt", "prompt1", "operator", "chain1", "str", "run both", ""]),
    json!(["rung", "rung1", "prompt1", "chain1", "", "", "claude-cli:opus"]),
    json!(["run", "run1", "chain1", "chain1", "rung1", word, ""]),
    json!(["bash", "bash1", "rung1", "chain1", "a", "", 120.0, null]),
    json!(["done", "bash1", "chain1", ""]),
    json!(["bash", "bash2", "rung1", "chain1", "b", "", 120.0, null]),
  ];
  facts.extend(more);
  facts
}

#[test]
fn each_act_that_a_loop_makes_marks_the_line_that_makes_it() {
  let mut streams = Streamed::default();
  let held = held(looped([]), &mut streams);
  let page = held.page(&streams, "prompt1", Reach { before: None, limit: 1 });
  let block = page.exchanges[0].block.as_ref().unwrap();
  let runs = Mark::Runs { work: Work::Command, began: Some(100_000) };
  assert_eq!(
    texts(block),
    [
      (1, "for c in ['a', 'b']:", &Mark::Ran),
      (2, "  await bash(c)", &runs),
      (3, "print('done')", &Mark::Ran)
    ]
  );
}

#[test]
fn a_reason_that_names_a_line_only_inside_a_word_marks_the_line_of_the_act_that_failed() {
  let mut streams = Streamed::default();
  let fault = json!({"is": "RuntimeError", "args": ["the deadline 3 passed"]});
  let facts = looped([
    json!(["done", "bash2", "chain1", fault]),
    json!(["done", "run1", "chain1", fault]),
    json!(["done", "rung1", "chain1", fault]),
  ]);
  let held = held(facts, &mut streams);
  let page = held.page(&streams, "prompt1", Reach { before: None, limit: 1 });
  let block = page.exchanges[0].block.as_ref().unwrap();
  let marks: Vec<bool> =
    block.lines.iter().map(|line| matches!(line.mark, Mark::Failed { .. })).collect();
  assert_eq!(marks, [false, true, false], "{block:?}");
}

#[test]
fn a_word_that_raises_at_a_command_that_failed_marks_the_line_of_that_command() {
  // The reason of the refusal names no line, and the command is done with its exit, no fault.
  let mut streams = Streamed::default();
  let word = "await bash('exit 3')\nclose('done')";
  let fault = json!({"is": "Refused", "args": ["bash1 exited 3"]});
  let exit = json!({"is": "Exit", "code": 3});
  let facts = vec![
    json!(["chain", "chain1", "operator", "", "root", ""]),
    json!(["prompt", "prompt1", "operator", "chain1", "str", "run it", ""]),
    json!(["rung", "rung1", "prompt1", "chain1", "", "", "claude-cli:opus"]),
    json!(["run", "run1", "chain1", "chain1", "rung1", word, ""]),
    json!(["bash", "bash1", "rung1", "chain1", "exit 3", "", 120.0, null]),
    json!(["done", "bash1", "bash", exit]),
    json!(["done", "run1", "chain1", fault]),
    json!(["done", "rung1", "chain1", fault]),
  ];
  let held = held(facts, &mut streams);
  let page = held.page(&streams, "prompt1", Reach { before: None, limit: 1 });
  let block = page.exchanges[0].block.as_ref().unwrap();
  let marks: Vec<bool> =
    block.lines.iter().map(|line| matches!(line.mark, Mark::Failed { .. })).collect();
  assert_eq!(marks, [true, false], "{block:?}");
}

#[test]
fn a_host_keeps_only_the_thoughts_that_a_page_shows() {
  let mut streams = Streamed::default();
  let mut fold = Fold::default();
  let mut held = Held::default();
  let mut hear = |fact: Value, streams: &mut Streamed| {
    for act in fold.recorded(&fact) {
      streams.heard(&act, 100_000);
      held.heard(&act);
    }
  };
  for fact in [
    json!(["chain", "chain1", "operator", "", "root", ""]),
    json!(["prompt", "prompt1", "operator", "chain1", "str", "where does the time go", ""]),
    json!(["chain", "chain2", "operator", "", "worker", ""]),
    json!(["prompt", "prompt2", "chain1", "chain2", "str", "study the boot", ""]),
  ] {
    hear(fact, &mut streams);
  }
  // Two rungs answer the thread and one answers the prompt of another chain, each with a thought:
  // the page of each prompt shows the thought of its last rung alone.
  for (rung, prompt, chain) in
    [("rung1", "prompt1", "chain1"), ("rung2", "prompt1", "chain1"), ("rung3", "prompt2", "chain2")]
  {
    hear(json!(["rung", rung, prompt, chain, "", "", "claude-cli:opus"]), &mut streams);
    streams.wrote(rung, ["", &format!("The thought of {rung}.")], 100);
    streams.wrote(rung, ["x = 1", ""], 101);
    hear(json!(["run", format!("run{rung}"), chain, chain, rung, "x = 1", ""]), &mut streams);
    hear(json!(["done", format!("run{rung}"), chain, null]), &mut streams);
    hear(json!(["done", rung, chain, null]), &mut streams);
  }
  // A rung that its model still writes keeps what it wrote.
  hear(json!(["rung", "rung4", "prompt1", "chain1", "", "", "claude-cli:opus"]), &mut streams);
  streams.wrote("rung4", ["", "The thought of rung4."], 102);
  let page = |streams: &Streamed| held.page(streams, "prompt1", Reach { before: None, limit: 10 });
  let before = page(&streams);
  held.shown(&mut streams);
  assert_eq!(page(&streams), before, "the page is the same");
  let mut kept: Vec<&str> = streams.wrote.keys().map(String::as_str).collect();
  kept.sort_unstable();
  assert_eq!(kept, ["rung3", "rung4"]);
}

#[test]
fn a_thought_under_a_second_took_no_second_whatever_seconds_it_falls_in() {
  let mut streams = Streamed::default();
  let mut facts = thread();
  facts.push(json!(["rung", "rung3", "prompt3", "chain1", "", "", "claude-cli:opus/high"]));
  facts.push(json!(["reply", "reply1", "rung3", "chain1", "claude-cli:opus/high"]));
  let held = held(facts, &mut streams);
  // The thought begins late in the second 110 and ends early in the second 111.
  streams.wrote("rung3", ["", "A short thought."], 110_900);
  streams.wrote("rung3", ["d = 4", ""], 111_100);
  let page = held.page(&streams, "prompt1", Reach { before: None, limit: 1 });
  let thought = page.exchanges[0].block.as_ref().unwrap().thought.clone().unwrap();
  assert_eq!(thought.took, Some(0));
}

#[test]
fn a_page_gives_the_context_of_its_chain_at_its_last_answer_with_the_window_of_its_actor() {
  let roster = json!([["claude-cli:opus", ["low", "high"], 200_000], ["operator", ["uael"], 0]]);
  let mut facts = vec![
    json!(["stand", "stand1", "operator", "chain1"]),
    json!(["done", "stand1", "journal", [roster, "/tmp", "claude-cli:opus/high"]]),
  ];
  facts.extend(thread());
  let mut streams = Streamed::default();
  let before =
    held(facts.clone(), &mut streams).page(&streams, "prompt1", Reach { before: None, limit: 1 });
  assert_eq!(before.context, None);
  facts.extend([
    json!(["reply", "reply1", "rung2", "chain1", "claude-cli:opus/high"]),
    json!(["done", "reply1", "journal", ["assistant", "x", [1000, 20, 600, 100, 0.25], []]]),
    json!(["reply", "reply2", "rung2", "chain1", "claude-cli:opus/high"]),
    json!(["done", "reply2", "journal", ["assistant", "x", [124_000, 30, 98_000, 8_400, 0.5], []]]),
  ]);
  let held = held(facts, &mut streams);
  let page = held.page(&streams, "prompt1", Reach { before: None, limit: 1 });
  let read = Read { tokens: 124_000, cached: 98_000, written: 8_400 };
  assert_eq!(page.context, Some(Context { read, window: Some(200_000), dollars: 0.75 }));
}

#[test]
fn the_line_that_comes_runs_from_the_ask_of_its_model_and_a_stop_marks_it_stopped() {
  let mut facts = thread();
  facts.push(json!(["rung", "rung3", "prompt3", "chain1", "", "", "claude-cli:opus/high"]));
  facts.push(json!(["reply", "reply1", "rung3", "chain1", "claude-cli:opus/high"]));
  let runs = Mark::Runs { work: Work::Writes, began: Some(100_000) };
  assert_eq!(last(&facts), Some((6, String::new(), runs)));
  let stop = json!({"is": "CancelledError", "args": ["stopped by the operator"]});
  facts.push(json!(["done", "reply1", "chain1", stop]));
  facts.push(json!(["done", "rung3", "chain1", stop]));
  assert_eq!(last(&facts), Some((6, String::new(), Mark::Stopped { at: Some(100_000) })));
}

#[test]
fn a_stop_marks_the_line_of_the_command_that_it_cut() {
  let mut facts = thread();
  let stop = json!({"is": "CancelledError", "args": ["stopped by the operator"]});
  for act in ["bash1", "run2", "rung2"] {
    facts.push(json!(["done", act, "chain1", stop]));
  }
  let mut streams = Streamed::default();
  let held = held(facts, &mut streams);
  let page = held.page(&streams, "prompt1", Reach { before: None, limit: 1 });
  let block = page.exchanges[0].block.as_ref().unwrap();
  let stopped = Mark::Stopped { at: Some(100_000) };
  assert_eq!(
    texts(block),
    [(3, "a = 2", &Mark::Ran), (4, "b = await bash('ls')", &stopped), (5, "c = 3", &Mark::Ran)]
  );
}

#[test]
fn a_line_carries_the_last_lines_that_its_command_wrote_as_it_runs_and_once_it_ended() {
  let output = |facts: &[Value]| {
    let mut streams = Streamed::default();
    let held = held(facts.to_vec(), &mut streams);
    for fact in facts {
      streams.out(fact);
    }
    let page = held.page(&streams, "prompt1", Reach { before: None, limit: 1 });
    let block = page.exchanges[0].block.clone().unwrap();
    (block.lines[0].output.clone(), block.lines[1].output.clone())
  };
  let mut facts = thread();
  facts.push(json!(["out", "bash1", "bash", "1\n2\n3\n", "stdout"]));
  facts.push(json!(["out", "bash1", "bash", "4\n\u{1b}[1m5\u{1b}[0m", "stdout"]));
  facts.push(json!(["out", "bash1", "bash", "50%\r100%\tok\n", "stderr"]));
  let tail = ["4", "5", "100% ok"].map(str::to_owned).to_vec();
  let runs = Output { tail: tail.clone(), more: 3, took: None };
  assert_eq!(output(&facts), (None, Some(runs)));
  let text = |content: &str| json!({"is": "Text", "path": "bash1/out", "content": content});
  let exit = json!({
    "is": "Exit", "code": 0,
    "stdout": text("1\n2\n3\n4\n\u{1b}[1m5\u{1b}[0m"), "stderr": text("50%\r100%\tok\n"),
  });
  facts.push(json!(["done", "bash1", "bash", exit]));
  assert_eq!(output(&facts), (None, Some(Output { tail, more: 3, took: Some(0) })));
}

/// The last line of the last block of the thread of these facts: its number, its text and its mark.
fn last(facts: &[Value]) -> Option<(usize, String, Mark)> {
  let mut streams = Streamed::default();
  let held = held(facts.to_vec(), &mut streams);
  let page = held.page(&streams, "prompt1", Reach { before: None, limit: 1 });
  let block = page.exchanges[0].block.clone()?;
  block.lines.last().map(|one| (one.number, one.text.clone(), one.mark.clone()))
}

#[test]
fn a_block_rolls_by_rows_and_keeps_its_last_line_with_what_its_command_wrote() {
  let line = |number, tail: usize| Line {
    number,
    text: String::new(),
    mark: Mark::Ran,
    output: (tail > 0).then(|| Output { tail: vec![String::new(); tail], more: 9, took: None }),
  };
  let block = |lines| Block { maker: Maker::Operator, lines, more: false, thought: None };
  let small = block((1..=7).map(|at| line(at, 0)).collect());
  assert_eq!((small.rolls(), small.first(false)), (false, 0));
  // Six rows roll beside the row that counts the lines above: the last line and its four rows of
  // output, and one line before it.
  let tall = block((1..=9).map(|at| line(at, if at == 9 { 3 } else { 0 })).collect());
  assert_eq!((tall.rolls(), tall.first(false), tall.first(true)), (true, 7, 0));
}

#[test]
fn a_context_parts_its_read_in_the_order_of_a_bar_and_leaves_the_rest_of_its_window() {
  let read = Read { tokens: 1_000, cached: 600, written: 100 };
  let context = Context { read, window: Some(10_000), dollars: 0.0 };
  let parts =
    [(Portion::Cached, 600), (Portion::Written, 100), (Portion::New, 300), (Portion::Free, 9_000)];
  assert_eq!((context.portions(), context.whole()), (parts.to_vec(), 10_000));
}

#[test]
fn a_retry_says_its_attempt_and_the_time_left_never_more_than_its_wait() {
  let why = String::new();
  let retry = Retrying { attempt: 2, most: 3, wait: 4, at: 100, why };
  let said = [90, 98, 100].map(|now| retry.said(now));
  assert_eq!(said, ["Attempt 2 of 3 in 4 s", "Attempt 2 of 3 in 2 s", "Attempt 2 of 3"]);
  let opus = Maker::Model { model: "claude-cli:opus".to_owned(), effort: Some("high".to_owned()) };
  assert_eq!([opus.shown(), Maker::Engine.shown()], ["opus/high", "furb"]);
}
