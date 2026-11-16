use furb_wire::anchor::{Anchor, Where};
use serde_json::{Value, json};

use furb::view::{
  Answered, Block, Context, Fold, Held, Line, Made, Maker, Mark, Output, Page, Portion, Reach,
  Read, Reply, Retrying, Said, Saying, Standing, Streamed, Thought, Work, Written, declined,
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
  // the command runs since the host saw the command begin, while the line under it waits to run.
  let block = second.block.as_ref().unwrap();
  let runs = Mark::Runs { work: Work::Command, began: Some(100_000) };
  assert_eq!(
    texts(block),
    [(3, "a = 2", &Mark::Ran), (4, "b = await bash('ls')", &runs), (5, "c = 3", &Mark::Waits)]
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
  assert_eq!(held.page(&streams, "reply9", Reach { before: None, limit: 1 }), Page::default());
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
  // The engine runs a word once its model wrote it whole, so the lines that came wait to run.
  assert_eq!(&texts(&block)[3..], [(6, "d = 4", &Mark::Waits), (7, "e = rea", &writes)]);
}

#[test]
fn a_word_that_came_whole_waits_from_its_first_line_and_no_line_is_written() {
  let mut streams = Streamed::default();
  let mut facts = thread();
  facts.push(json!(["rung", "rung3", "prompt3", "chain1", "", "", "claude-cli:opus/high"]));
  facts.push(json!(["reply", "reply1", "rung3", "chain1", "claude-cli:opus/high"]));
  facts.push(json!(["done", "reply1", "chain1", null]));
  let held = held(facts, &mut streams);
  streams.wrote("rung3", ["d = 4\ne = 5", ""], 101_000);
  let page = held.page(&streams, "prompt1", Reach { before: None, limit: 1 });
  let block = page.exchanges[0].block.clone().unwrap();
  let runs = Mark::Runs { work: Work::Command, began: Some(100_000) };
  assert_eq!(&texts(&block)[3..], [(6, "d = 4", &runs), (7, "e = 5", &Mark::Waits)]);
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
fn a_word_that_an_operator_ran_on_a_thread_is_a_message_of_his_as_code_on_that_thread_alone() {
  let mut streams = Streamed::default();
  let mut facts = thread();
  let fault = json!({"is": "ValueError", "args": ["no at line 2"]});
  facts.extend([
    json!(["prompt", "prompt4", "operator", "chain1", "str", "another thread", ""]),
    json!([
      "rung",
      "rung3",
      "operator/uael",
      "prompt1",
      "answer = 41 + 1\nraise ValueError('no')",
      "",
      ""
    ]),
    json!([
      "run",
      "run3",
      "chain1",
      "chain1",
      "rung3",
      "answer = 41 + 1\nraise ValueError('no')",
      ""
    ]),
    json!(["done", "run3", "chain1", fault]),
    json!(["done", "rung3", "chain1", fault]),
  ]);
  let held = held(facts, &mut streams);
  let page = held.page(&streams, "prompt1", Reach { before: None, limit: 10 });
  let word = &page.exchanges[2];
  assert_eq!(
    (word.at, word.message.as_str(), word.python, word.text.as_str(), word.runs),
    (2, "rung3", true, "", false)
  );
  let block = word.block.as_ref().unwrap();
  let why = "ValueError: no at line 2".to_owned();
  assert_eq!(
    texts(block),
    [(6, "answer = 41 + 1", &Mark::Ran), (7, "raise ValueError('no')", &Mark::Failed { why })]
  );
  let other = held.page(&streams, "prompt4", Reach { before: None, limit: 10 });
  assert_eq!(other.exchanges.iter().map(|one| one.python).collect::<Vec<_>>(), [false]);
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
fn a_question_of_a_model_about_a_line_is_a_sub_thread_that_asks_the_operator() {
  let mut streams = Streamed::default();
  let mut facts = thread();
  facts.extend([
    json!([
      "prompt",
      "prompt8",
      "rung2",
      "prompt1",
      "str",
      "On `c.py`, line 3 after the change:\n> z\n\nkeep z?",
      "operator"
    ]),
    json!(["started", "prompt8", "console"]),
  ]);
  let held = held(facts, &mut streams);
  let page = held.page(&streams, "prompt1", Reach { before: None, limit: 10 });
  let [asks] = page.agents.as_slice() else { panic!("{page:?}") };
  assert_eq!(
    (asks.card.title.as_str(), &asks.card.standing),
    ("keep z?", &Standing::Asks { of: None })
  );
  let questions = |page: &Page| -> Vec<String> {
    let said = page.exchanges.iter().flat_map(|one| &one.said);
    said
      .filter(|one| matches!(one.kind, Saying::Question { .. }))
      .map(|one| one.act.clone())
      .collect()
  };
  assert!(questions(&page).is_empty(), "{page:?}");
  let own = held.page(&streams, "prompt8", Reach { before: None, limit: 10 });
  assert_eq!(questions(&own), ["prompt8"]);
  // The card of the thread that it is on says that a question waits under it.
  let cards: Vec<Standing> = held.cards().into_iter().map(|one| one.standing).collect();
  assert_eq!(cards, [Standing::Asks { of: None }]);
}

#[test]
fn a_prompt_on_a_thread_about_a_line_of_a_file_is_a_sub_thread_that_holds_its_answers() {
  let mut streams = Streamed::default();
  let mut facts = thread();
  let about = "On `a.py`, line 2 after the change:\n> x = 2\n\nwhy two?";
  facts.extend([
    json!(["prompt", "prompt5", "operator", "prompt1", "str", about, ""]),
    json!(["rung", "rung5", "prompt5", "chain1", "", "", "claude-cli:opus/high"]),
    json!(["done", "prompt5", "rung5", "Because."]),
    json!(["done", "rung5", "chain1", null]),
    json!(["prompt", "prompt6", "operator", "prompt5", "str", "and three?", ""]),
    // A model opens one with the same prompt.
    json!([
      "prompt",
      "prompt7",
      "rung2",
      "prompt1",
      "str",
      "On `b.py`, line 1 before the change:\n> y\n\nwhy y?",
      ""
    ]),
  ]);
  let held = held(facts, &mut streams);
  // The thread holds no message of it, and no card stands for it.
  let page = held.page(&streams, "prompt1", Reach { before: None, limit: 10 });
  let messages: Vec<&str> = page.exchanges.iter().map(|one| one.message.as_str()).collect();
  assert_eq!(messages, ["prompt1", "prompt3"]);
  let cards: Vec<String> = held.cards().into_iter().map(|one| one.prompt).collect();
  assert_eq!(cards, ["prompt1"]);
  // It stands under the thread with its anchor, and its reply opens it again.
  let [sub, model] = page.agents.as_slice() else { panic!("{page:?}") };
  let line = |path: &str, line, before, quote: &str| {
    let at = Where::Line { line, before, quote: quote.to_owned() };
    Some(Anchor { path: path.to_owned(), at })
  };
  assert_eq!(
    (model.card.title.as_str(), model.anchor.clone()),
    ("why y?", line("b.py", 1, true, "y"))
  );
  assert_eq!(
    (sub.card.title.as_str(), sub.anchor.clone()),
    ("why two?", line("a.py", 2, false, "x = 2"))
  );
  assert!(sub.card.works(), "{sub:?}");
  let own = held.page(&streams, "prompt5", Reach { before: None, limit: 10 });
  let messages: Vec<&str> = own.exchanges.iter().map(|one| one.message.as_str()).collect();
  assert_eq!(messages, ["prompt5", "prompt6"]);
  // A resolve of it closes what stands open in it alone.
  let open: Vec<String> = held.open_in("prompt5").into_iter().map(|one| one.0).collect();
  assert_eq!(open, ["prompt6"]);
  let open: Vec<String> = held.open_in("prompt1").into_iter().map(|one| one.0).collect();
  assert_eq!(open, ["prompt3"]);
}

#[test]
fn an_agent_whose_model_only_thinks_peeks_at_the_last_line_of_its_thought() {
  let peek = |more: Vec<Value>| {
    let mut streams = Streamed::default();
    let mut facts = thread();
    facts.push(json!(["chain", "chain2", "rung2", "chain1", "worker", ""]));
    facts.push(json!(["prompt", "prompt4", "rung2", "chain2", "str", "study the boot", ""]));
    facts.push(json!(["rung", "rung4", "prompt4", "chain2", "", "", "claude-cli:sonnet"]));
    facts.extend(more);
    let held = held(facts, &mut streams);
    streams.wrote("rung4", ["", "The boot reads the record.\nThen it stands \n\n"], 101_000);
    let page = held.page(&streams, "prompt1", Reach { before: None, limit: 1 });
    page.agents.first().map(|one| one.thought.clone())
  };
  assert_eq!(peek(Vec::new()), Some(Some("Then it stands".to_owned())));
  // Once the rung has its word, the line of the word says where the agent stands.
  let word = json!(["run", "run4", "chain2", "chain2", "rung4", "x = 1", ""]);
  assert_eq!(peek(vec![word]), Some(None));
}

#[test]
fn a_note_to_a_chain_where_no_agent_works_is_an_agent_and_a_note_to_the_operator_is_none() {
  let mut streams = Streamed::default();
  let mut facts = thread();
  facts.extend([
    json!(["chain", "chain2", "rung2", "chain1", "fix", ""]),
    json!(["prompt", "prompt4", "rung2", "chain2", "None", "Fix this failure", ""]),
    json!(["rung", "rung4", "prompt4", "chain2", "", "", "claude-cli:sonnet"]),
    json!(["prompt", "prompt5", "rung2", "chain2", "None", "It fails.", "operator"]),
  ]);
  let held = held(facts, &mut streams);
  let page = held.page(&streams, "prompt1", Reach { before: None, limit: 1 });
  let titles: Vec<&str> = page.agents.iter().map(|one| one.card.title.as_str()).collect();
  assert_eq!(titles, ["Fix this failure"]);
  let counts: Vec<(String, usize)> =
    held.cards().into_iter().map(|one| (one.title, one.agents)).collect();
  assert_eq!(counts, [("where does the time go".to_owned(), 1)]);
}

#[test]
fn an_agent_says_when_it_began_when_it_ended_and_the_line_where_it_stands() {
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
  let facts = vec![
    json!(["chain", "chain1", "operator", "", "root", ""]),
    json!(["prompt", "prompt1", "operator", "chain1", "str", "plan it", ""]),
    json!(["rung", "rung1", "prompt1", "chain1", "", "", "claude-cli:opus/high"]),
    json!(["run", "run1", "chain1", "chain1", "rung1", word, ""]),
    json!(["done", "run1", "chain1", null]),
    json!(["done", "rung1", "chain1", null]),
  ];
  let mut streams = Streamed::default();
  let page = held(facts, &mut streams).page(&streams, "prompt1", Reach { before: None, limit: 1 });
  let block = page.exchanges[0].block.clone().unwrap_or_else(|| panic!("{page:?}"));
  let lines: Vec<(usize, &str)> =
    texts(&block).into_iter().map(|(at, text, _)| (at, text)).collect();
  assert_eq!(lines, [(1, "plan = 'read it\\n'"), (4, "x = \"\"\"a\\nb\"\"\""), (6, "y = 1")]);
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
  assert_eq!(marks, [&Mark::Asks, &Mark::Asks, &Mark::Waits]);
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
fn a_message_that_a_person_closes_is_resolved_by_him_and_answered_by_no_model() {
  let mut streams = Streamed::default();
  let facts = thread().into_iter().chain([
    json!(["close", "prompt3", "operator", "resolved"]),
    json!(["done", "prompt3", "operator", "resolved"]),
  ]);
  let held = held(facts, &mut streams);
  let page = held.page(&streams, "prompt1", Reach { before: None, limit: 10 });
  let kinds: Vec<Vec<&Saying>> =
    page.exchanges.iter().map(|one| one.said.iter().map(|said| &said.kind).collect()).collect();
  let resolved = Saying::Resolved { by: Maker::Operator };
  assert_eq!(kinds, [vec![&Saying::Notice, &Saying::Answer], vec![&resolved]]);
}

#[test]
fn a_message_that_the_operator_took_back_is_no_message_of_the_thread() {
  let mut streams = Streamed::default();
  let taken = json!({"is": "CancelledError", "args": []});
  let facts = thread().into_iter().chain([
    json!(["cancel", "prompt3", "operator", null]),
    json!(["done", "prompt3", "operator", taken]),
  ]);
  let held = held(facts, &mut streams);
  let page = held.page(&streams, "prompt1", Reach { before: None, limit: 10 });
  let messages: Vec<&str> = page.exchanges.iter().map(|one| one.message.as_str()).collect();
  assert_eq!(messages, ["prompt1"]);
}

#[test]
fn a_message_that_a_stop_of_its_thread_cancelled_stays_a_message_of_the_thread() {
  let mut streams = Streamed::default();
  let stopped = json!({"is": "CancelledError", "args": []});
  let facts = thread().into_iter().chain([
    json!(["cancel", "prompt1", "operator", null]),
    json!(["done", "prompt3", "operator", stopped]),
  ]);
  let held = held(facts, &mut streams);
  let page = held.page(&streams, "prompt1", Reach { before: None, limit: 10 });
  let messages: Vec<&str> = page.exchanges.iter().map(|one| one.message.as_str()).collect();
  assert_eq!(messages, ["prompt1", "prompt3"]);
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
fn a_word_with_no_act_open_runs_the_line_after_its_last_act_and_its_first_line_before_any() {
  let mut facts = thread();
  // The command of the second word is done, and its word runs on.
  facts.push(json!(["done", "bash1", "chain1", {"is": "Exit", "code": 0}]));
  let mut streams = Streamed::default();
  let ran = held(facts.clone(), &mut streams);
  let page = ran.page(&streams, "prompt1", Reach { before: None, limit: 10 });
  let block = page.exchanges[1].block.as_ref().unwrap();
  let runs = Mark::Runs { work: Work::Command, began: Some(100_000) };
  let marks: Vec<&Mark> = block.lines.iter().map(|line| &line.mark).collect();
  assert_eq!(marks, [&Mark::Ran, &Mark::Ran, &runs]);
  // A word that made no act yet runs its first line, never its last, and the lines under it wait.
  facts.truncate(facts.len() - 2);
  let mut streams = Streamed::default();
  let none = held(facts, &mut streams);
  let page = none.page(&streams, "prompt1", Reach { before: None, limit: 10 });
  let block = page.exchanges[1].block.as_ref().unwrap();
  let marks: Vec<&Mark> = block.lines.iter().map(|line| &line.mark).collect();
  assert_eq!(marks, [&runs, &Mark::Waits, &Mark::Waits]);
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
      (3, "print('done')", &Mark::Waits)
    ]
  );
}

#[test]
fn a_line_says_what_each_act_that_it_made_came_to_one_for_each_pass_of_a_loop() {
  let mut streams = Streamed::default();
  let done = json!(["done", "bash2", "chain1", {"is": "Bash", "code": 0, "out": "b"}]);
  let held = held(looped([done]), &mut streams);
  let page = held.page(&streams, "prompt1", Reach { before: None, limit: 1 });
  let block = page.exchanges[0].block.as_ref().unwrap();
  let made: Vec<&[Made]> = block.lines.iter().map(|line| line.made.as_slice()).collect();
  let passes = [Made::Done("''".to_owned()), Made::Done("Bash(code=0, out='b')".to_owned())];
  assert_eq!(made, [&[][..], &passes[..], &[][..]]);
}

#[test]
fn the_chain_that_a_line_makes_for_its_prompt_is_no_outcome_of_the_line() {
  let facts = vec![
    json!(["chain", "chain1", "operator", "", "root", ""]),
    json!(["prompt", "prompt1", "operator", "chain1", "str", "ask", ""]),
    json!(["rung", "rung1", "prompt1", "chain1", "", "", "claude-cli:opus"]),
    json!([
      "run",
      "run1",
      "chain1",
      "chain1",
      "rung1",
      "found = await prompt(str, 'a', on=chain('a'))",
      ""
    ]),
    json!(["chain", "chain2", "rung1", "", "a", ""]),
    json!(["prompt", "prompt2", "rung1", "chain2", "str", "a", ""]),
    json!(["done", "prompt2", "chain2", "x"]),
  ];
  let mut streams = Streamed::default();
  let held = held(facts, &mut streams);
  let page = held.page(&streams, "prompt1", Reach { before: None, limit: 1 });
  let block = page.exchanges[0].block.as_ref().unwrap();
  assert_eq!(block.lines[0].made, [Made::Done("'x'".to_owned())]);
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
fn a_word_that_raises_at_a_command_that_failed_marks_the_line_of_that_command_with_its_reason() {
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
  assert_eq!(block.lines[0].mark, Mark::Failed { why: "exited 3".to_owned() });
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
  // The host keeps the streams in the image of the life, whose form names the rung of each thought.
  let image = serde_json::to_value(&streams).unwrap_or_default();
  let mut kept: Vec<&str> =
    image["wrote"].as_object().into_iter().flatten().map(|(rung, _)| rung.as_str()).collect();
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
fn a_stop_that_cut_no_act_marks_the_line_where_the_word_stood() {
  let mut facts = thread();
  let word = "a = 2\nb = await bash('ls')\nc = 3\nd = 4";
  facts[10] = json!(["run", "run2", "chain1", "chain1", "rung2", word, ""]);
  let stop = json!({"is": "CancelledError", "args": ["stopped by the operator"]});
  facts.push(json!(["done", "bash1", "chain1", {"exit": 0}]));
  for act in ["run2", "rung2"] {
    facts.push(json!(["done", act, "chain1", stop]));
  }
  let mut streams = Streamed::default();
  let held = held(facts, &mut streams);
  let page = held.page(&streams, "prompt1", Reach { before: None, limit: 1 });
  let block = page.exchanges[0].block.as_ref().unwrap();
  let stopped = Mark::Stopped { at: Some(100_000) };
  assert_eq!(
    texts(block),
    [
      (3, "a = 2", &Mark::Ran),
      (4, "b = await bash('ls')", &Mark::Ran),
      (5, "c = 3", &stopped),
      (6, "d = 4", &Mark::Ran)
    ]
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

#[test]
fn a_line_that_awaited_two_commands_carries_what_each_wrote_and_the_time_of_the_slowest() {
  let mut facts = thread();
  let word = "a, b = bash('echo a'), bash('echo b')\nawait b";
  facts[10] = json!(["run", "run2", "chain1", "chain1", "rung2", word, ""]);
  facts[11] = json!(["bash", "bash1", "rung2", "chain1", "echo a", "", 120.0, null]);
  facts.push(json!(["bash", "bash2", "rung2", "chain1", "echo b", "", 120.0, null]));
  let text = |content: &str| json!({"is": "Text", "path": "bash/out", "content": content});
  let exit = |out: &str| json!({"is": "Exit", "code": 0, "stdout": text(out), "stderr": text("")});
  facts.push(json!(["done", "bash1", "bash", exit("a\n")]));
  facts.push(json!(["done", "bash2", "bash", exit("b\n")]));
  let mut streams = Streamed::default();
  let held = held(facts, &mut streams);
  let page = held.page(&streams, "prompt1", Reach { before: None, limit: 1 });
  let block = page.exchanges[0].block.as_ref().unwrap();
  let tail = ["a", "b"].map(str::to_owned).to_vec();
  assert_eq!(block.lines[0].output, Some(Output { tail, more: 0, took: Some(0) }));
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
    made: Vec::new(),
    writes: Vec::new(),
    awaits: Vec::new(),
    pictures: Vec::new(),
  };
  let block = |lines| Block { maker: Maker::Operator, lines, more: false, thought: None };
  let small = block((1..=7).map(|at| line(at, 0)).collect());
  assert_eq!((small.rolls(), small.shown(false)), (false, 0..7));
  // Six rows roll beside the row that counts the lines above: the last line and its four rows of
  // output, and one line before it.
  let tall = block((1..=9).map(|at| line(at, if at == 9 { 3 } else { 0 })).collect());
  assert_eq!((tall.rolls(), tall.shown(false), tall.shown(true)), (true, 7..9, 0..9));
}

#[test]
fn a_block_whose_word_came_whole_rolls_to_the_line_that_runs_and_says_the_lines_that_follow() {
  let runs = Mark::Runs { work: Work::Command, began: None };
  let line = |number| Line {
    number,
    text: String::new(),
    mark: if number == 9 { runs.clone() } else { Mark::Ran },
    output: None,
    made: Vec::new(),
    writes: Vec::new(),
    awaits: Vec::new(),
    pictures: Vec::new(),
  };
  let block = Block {
    maker: Maker::Operator,
    lines: (1..=20).map(line).collect(),
    more: false,
    thought: None,
  };
  // Five lines stand between the row that counts the lines above and the row that counts those
  // below, the line that runs the last of them.
  assert_eq!(block.shown(false), 4..9);
  // Near the top, the lines below fill the rows that the lines above leave.
  let early = Block { lines: block.lines[4..].to_vec(), ..block.clone() };
  assert_eq!(early.shown(false), 0..6);
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

#[test]
fn an_act_made_inside_a_call_on_its_line_marks_that_line_and_not_a_later_one() {
  // Each prompt names its chain in its arguments, so the word makes the chain before the prompt
  // of the same line.
  let word = "bench = prompt(str, 'a', on=chain('bench'))\nboot = prompt(str, 'b', on=chain('boot'))\nawait wait(5, bench)\nlines = await prompt(str, 'c', on=chain('count'))\nawait wait(600, bench)";
  let facts = vec![
    json!(["chain", "chain1", "operator", "", "root", ""]),
    json!(["prompt", "prompt1", "operator", "chain1", "str", "measure", ""]),
    json!(["rung", "rung1", "prompt1", "chain1", "", "", "claude-cli:opus"]),
    json!(["run", "run1", "chain1", "chain1", "rung1", word, ""]),
    json!(["chain", "chain2", "rung1", "", "bench", ""]),
    json!(["prompt", "prompt2", "rung1", "chain2", "str", "a", ""]),
    json!(["chain", "chain3", "rung1", "", "boot", ""]),
    json!(["prompt", "prompt3", "rung1", "chain3", "str", "b", ""]),
    json!(["wait", "wait1", "rung1", "chain1", 5.0, "prompt2"]),
  ];
  let mut streams = Streamed::default();
  let held = held(facts, &mut streams);
  let page = held.page(&streams, "prompt1", Reach { before: None, limit: 1 });
  let block = page.exchanges[0].block.as_ref().unwrap();
  let runs: Vec<usize> = block
    .lines
    .iter()
    .filter(|line| matches!(line.mark, Mark::Runs { .. }))
    .map(|line| line.number)
    .collect();
  assert_eq!(runs, [3]);
}

#[test]
fn each_act_of_a_comprehension_marks_its_line_and_a_later_call_its_own() {
  let word = "rs = [prompt(str, p, on=chain(p)) for p in ['a', 'b']]\nfound = [await r for r in rs]\nlast = await prompt(str, 'c', on=chain('c'))";
  let facts = vec![
    json!(["chain", "chain1", "operator", "", "root", ""]),
    json!(["prompt", "prompt1", "operator", "chain1", "str", "review", ""]),
    json!(["rung", "rung1", "prompt1", "chain1", "", "", "claude-cli:opus"]),
    json!(["run", "run1", "chain1", "chain1", "rung1", word, ""]),
    json!(["chain", "chain2", "rung1", "", "a", ""]),
    json!(["prompt", "prompt2", "rung1", "chain2", "str", "a", ""]),
    json!(["chain", "chain3", "rung1", "", "b", ""]),
    json!(["prompt", "prompt3", "rung1", "chain3", "str", "b", ""]),
    json!(["done", "prompt2", "chain2", "x"]),
    json!(["done", "prompt3", "chain3", "y"]),
    json!(["chain", "chain4", "rung1", "", "c", ""]),
    json!(["prompt", "prompt4", "rung1", "chain4", "str", "c", ""]),
  ];
  let mut streams = Streamed::default();
  let held = held(facts, &mut streams);
  let page = held.page(&streams, "prompt1", Reach { before: None, limit: 1 });
  let block = page.exchanges[0].block.as_ref().unwrap();
  let made: Vec<usize> = block.lines.iter().map(|line| line.made.len()).collect();
  // A chain is no outcome of its line: two prompts on the first line, one on the last.
  assert_eq!(made, [2, 0, 1]);
  let at: Vec<Option<usize>> = page.agents.iter().map(|agent| agent.at).collect();
  assert_eq!(at, [Some(1), Some(1), Some(3)]);
}

#[test]
fn a_thread_shows_at_each_moment_of_its_acts_what_it_was_just_after_it() {
  let mut streams = Streamed::default();
  let held = held(thread(), &mut streams);
  let moments = held.moments(Some("prompt1"));
  let began = |id: &str| held.shared(id).unwrap().began;
  // The moments are those of the acts of the thread: its messages, its rungs and what they made,
  // and no act of another thread or chain.
  assert_eq!(moments.first(), Some(&began("prompt1")));
  assert!(moments.contains(&began("bash1")));
  let page = |at: u64| held.past(at).page(&streams, "prompt1", Reach { before: None, limit: 10 });
  assert_eq!(
    page(*moments.last().unwrap()),
    held.page(&streams, "prompt1", Reach { before: None, limit: 10 })
  );
  // Just before his second message, the thread holds the first, answered and closed.
  let before = moments[moments.iter().position(|one| *one == began("prompt3")).unwrap() - 1];
  let then = page(before);
  let [first] = then.exchanges.as_slice() else { panic!("{then:?}") };
  assert_eq!(
    texts(first.block.as_ref().unwrap()),
    [(1, "x = 1", &Mark::Ran), (2, "close('In the boot.')", &Mark::Ran)]
  );
  // At its first moment, the thread is his message alone.
  let first = page(moments[0]);
  let [first] = first.exchanges.as_slice() else { panic!("{first:?}") };
  assert_eq!((first.text.as_str(), &first.block), ("where does the time go", &None));
}

#[test]
fn a_line_that_wrote_a_file_says_what_it_changed_against_the_text_that_the_life_knew_there() {
  let text = |path: &str, content: &str| json!({"is": "Text", "path": path, "content": content});
  let word = "m = read('a.py')\nwrite(m.replace('two', '2'))\nwrite(Text('b.py', 'one\\n'))";
  let facts = vec![
    json!(["chain", "chain1", "operator", "", "root", ""]),
    json!(["prompt", "prompt1", "operator", "chain1", "str", "fix a.py, add b.py", ""]),
    json!(["rung", "rung1", "prompt1", "chain1", "", "", "claude-cli:opus/high"]),
    json!(["run", "run1", "chain1", "chain1", "rung1", word, ""]),
    json!(["read", "read1", "rung1", "chain1", "a.py"]),
    json!(["done", "read1", "files", text("~/w/a.py", "one\ntwo\nthree\n")]),
    json!(["write", "write1", "rung1", "chain1", text("~/w/a.py", "one\n2\nthree\n")]),
    json!(["done", "write1", "files", text("~/w/a.py", "one\n2\nthree\n")]),
    json!(["write", "write2", "rung1", "chain1", text("b.py", "one\n")]),
    json!(["done", "write2", "files", text("/w/b.py", "one\n")]),
  ];
  let mut streams = Streamed::default();
  let held = held(facts, &mut streams);
  let page = held.page(&streams, "prompt1", Reach { before: None, limit: 1 });
  let block = page.exchanges[0].block.as_ref().unwrap();
  let writes: Vec<&[Written]> = block.lines.iter().map(|line| line.writes.as_slice()).collect();
  let diff = |lines: &[&str]| lines.iter().map(|one| (*one).to_owned()).collect();
  let changed = Written {
    act: "write1".to_owned(),
    path: "~/w/a.py".to_owned(),
    added: 1,
    removed: 1,
    known: true,
    diff: diff(&[" one", "-two", "+2", " three"]),
    more: 0,
  };
  // The life never read b.py, so it does not know what stood there: each line counts as added.
  let added = Written {
    act: "write2".to_owned(),
    path: "/w/b.py".to_owned(),
    added: 1,
    removed: 0,
    known: false,
    diff: diff(&["+one"]),
    more: 0,
  };
  assert_eq!(writes, [&[][..], &[changed][..], &[added][..]]);
}
