//! The meaning of a life: the state of each act and of each chain in one set, the desk, the
//! threads, the spend, and the acts as they stood at a moment, each with the values that the views
//! of TypeScript give.

use std::collections::HashMap;

use furb_host::view::{
  Act, Answer, Attention, ChainThread, Concern, Origin, Pause, Phase, Rows, RunState, Spend, State,
  ThreadNode, Tick, as_of, attention, chain_threads, choices, concerns, desk, hint, moments_of,
  nest_threads, restate, takes_nothing, ticks, wants, working,
};
use serde_json::{Value, json};

/// An act of a life, made at a moment, and done at another when it is done, in the state that this
/// gives it.
fn act(id: &str, kind: &str, on: &str, by: &str, began: u64, ended: Option<u64>) -> Act {
  let mut made = Act {
    kind: kind.to_owned(),
    id: id.to_owned(),
    by: by.to_owned(),
    on: if kind == "chain" { id } else { on }.to_owned(),
    chain: if kind == "chain" { id } else { on }.to_owned(),
    words: Vec::new(),
    phase: if ended.is_some() { Phase::Done(Value::Null) } else { Phase::Asked },
    operator: false,
    pause: None,
    run: (kind == "rung").then_some(if ended.is_some() {
      RunState::Done
    } else {
      RunState::Running
    }),
    word: None,
    began,
    ended,
    spend: None,
    state: State::Idle,
  };
  made.state = State::of(&made);
  made
}

/// An act with its words.
fn worded(act: Act, words: Value) -> Act {
  let Value::Array(words) = words else { panic!("the words of an act are a list") };
  Act { words, ..act }
}

/// An act in the state that its parts give.
fn stated(mut act: Act) -> Act {
  act.state = State::of(&act);
  act
}

/// A question to the operator, which his console took.
fn asked(act: Act) -> Act {
  stated(Act { operator: true, ..act })
}

/// The acts by their names.
fn rows(acts: &[Act]) -> Rows<'_> {
  acts.iter().map(|one| (one.id.as_str(), one)).collect()
}

/// The ids of acts.
fn ids(acts: &[Act]) -> Vec<&str> {
  acts.iter().map(|one| one.id.as_str()).collect()
}

/// The state of a chain, from each act of a life.
fn state_of(acts: &[Act], chain: &str) -> State {
  let mut all = acts.to_vec();
  restate(&mut all);
  match all.into_iter().find(|one| one.id == chain) {
    Some(one) => one.state,
    None => panic!("{chain} is no act"),
  }
}

#[test]
fn each_act_takes_one_state_of_the_one_set() {
  let rung = act("rung1", "rung", "chain1", "prompt1", 1, None);
  let worded = stated(Act { word: Some("x = 1".to_owned()), ..rung.clone() });
  let failed = Act {
    phase: Phase::Done(json!({"is": "ValueError", "args": ["no"]})),
    ..act("bash2", "bash", "chain1", "rung1", 1, Some(2))
  };
  let paused = Act {
    pause: Some(Pause { by: "operator".to_owned(), why: None }),
    ..act("bash1", "bash", "chain1", "rung1", 1, None)
  };
  let table = [
    (asked(act("prompt9", "prompt", "chain1", "operator", 1, None)), State::Asks),
    (stated(paused), State::Paused),
    (stated(failed), State::Failed),
    (rung, State::Thinks),
    (worded, State::Runs),
    (act("bash3", "bash", "chain1", "rung1", 1, None), State::Runs),
    (act("chain2", "chain", "chain1", "rung1", 1, None), State::Delegates),
    (act("read1", "read", "chain1", "rung1", 1, Some(2)), State::Done),
  ];
  for (one, state) in table {
    assert_eq!((one.id.as_str(), one.state), (one.id.as_str(), state));
  }
}

#[test]
fn a_whole_takes_the_first_state_of_its_parts_in_the_order_of_the_one_set() {
  let order = [
    State::Asks,
    State::Unread,
    State::Failed,
    State::Paused,
    State::Thinks,
    State::Runs,
    State::Delegates,
    State::Done,
    State::Cancelled,
    State::Idle,
  ];
  assert_eq!(State::ALL, order);
  for (at, state) in order.iter().enumerate() {
    assert_eq!(State::roll_up(order[at..].iter().rev().copied()), *state);
  }
  assert_eq!(State::roll_up([]), State::Idle);
  let busy: Vec<bool> =
    [State::Thinks, State::Runs, State::Delegates, State::Done].map(State::busy).to_vec();
  assert_eq!(busy, [true, true, true, false]);
}

#[test]
fn the_work_of_a_chain_thinks_runs_or_delegates() {
  let thinking = act("rung1", "rung", "chain1", "prompt1", 1, None);
  let running = Act {
    word: Some("await bash1".to_owned()),
    ..act("rung2", "rung", "chain1", "prompt1", 2, None)
  };
  let command = act("bash1", "bash", "chain1", "rung2", 3, None);
  let prompt = act("prompt5", "prompt", "chain2", "rung2", 4, None);
  assert_eq!(working(&[&thinking, &running, &command], &[&prompt]), State::Thinks);
  assert_eq!(working(&[&running, &command], &[&prompt]), State::Runs);
  assert_eq!(working(&[&running], &[&prompt]), State::Delegates);
  assert_eq!(working(&[&running], &[]), State::Runs);
}

#[test]
fn a_chain_thinks_while_a_model_writes_runs_while_a_command_runs_and_is_done_once_its_prompts_end()
{
  let acts = [
    act("chain1", "chain", "", "operator", 1, None),
    act("rung1", "rung", "chain1", "operator", 2, None),
    act("chain2", "chain", "", "rung1", 3, None),
    act("chain3", "chain", "", "rung1", 4, None),
    act("chain4", "chain", "", "rung1", 5, None),
    act("prompt1", "prompt", "chain2", "rung1", 6, None),
    act("rung2", "rung", "chain2", "prompt1", 7, None),
    act("prompt2", "prompt", "chain3", "rung1", 8, None),
    stated(Act {
      word: Some("await bash(\"make\")".to_owned()),
      ..act("rung3", "rung", "chain3", "prompt2", 9, None)
    }),
    act("bash1", "bash", "chain3", "rung3", 10, None),
    act("prompt3", "prompt", "chain4", "rung1", 11, Some(14)),
    act("rung4", "rung", "chain4", "prompt3", 12, Some(13)),
    act("read1", "read", "chain3", "rung3", 15, Some(15)),
  ];
  let states: Vec<State> = ["chain2", "chain3", "chain4"].map(|one| state_of(&acts, one)).to_vec();
  assert_eq!(states, [State::Thinks, State::Runs, State::Done]);
}

#[test]
fn a_chain_asks_while_the_operator_owes_a_prompt_of_it() {
  let acts = [
    act("chain1", "chain", "", "operator", 1, None),
    act("rung1", "rung", "chain1", "operator", 2, None),
    act("chain2", "chain", "", "rung1", 3, None),
    asked(worded(
      act("prompt3", "prompt", "chain1", "rung1", 4, Some(5)),
      json!(["str", "Which name?", "operator"]),
    )),
    asked(worded(
      act("prompt1", "prompt", "chain2", "rung1", 6, None),
      json!(["str", "Which effort?", "operator"]),
    )),
    asked(worded(
      act("prompt2", "prompt", "chain1", "rung1", 7, None),
      json!(["bool", "Merge?", "operator"]),
    )),
  ];
  assert_eq!([state_of(&acts, "chain1"), state_of(&acts, "chain2")], [State::Asks, State::Asks]);
}

#[test]
fn a_pause_holds_a_chain_over_its_work_and_its_last_act_of_work_says_a_cancel_or_a_failure() {
  let chain = act("chain1", "chain", "", "operator", 1, None);
  let pause = Some(Pause { by: "operator".to_owned(), why: Some(json!("ceiling")) });
  let held = stated(Act { pause, ..act("bash1", "bash", "chain1", "rung1", 3, None) });
  let running = act("bash2", "bash", "chain1", "rung1", 4, None);
  assert_eq!(state_of(&[chain.clone(), held, running], "chain1"), State::Paused);
  let cancel = json!({"is": "CancelledError", "args": []});
  let cancelled =
    Act { phase: Phase::Done(cancel), ..act("bash3", "bash", "chain1", "rung1", 3, Some(4)) };
  let failed = json!({"is": "Refused", "args": ["no"]});
  let read =
    Act { phase: Phase::Done(failed), ..act("read1", "read", "chain1", "rung1", 5, Some(5)) };
  assert_eq!(
    state_of(&[chain.clone(), stated(cancelled), stated(read)], "chain1"),
    State::Cancelled
  );
  assert_eq!(state_of(&[chain], "chain1"), State::Idle);
}

#[test]
fn a_bool_gives_yes_and_no_a_literal_each_of_its_values_and_any_other_shape_none() {
  assert_eq!(choices("bool"), ["yes", "no"]);
  assert_eq!(choices("Literal['fast', 'safe']"), ["fast", "safe"]);
  assert_eq!(
    choices(r#"Literal["a, b", 'it\'s', 3, True, None]"#),
    ["a, b", "it's", "3", "True", "None"]
  );
  // The console refuses a shape that names its module, so the desk offers it no choice.
  for shape in ["str", "int", "float", "list", "dict", "None", "list[str]", "typing.Literal['one']"]
  {
    assert_eq!(choices(shape), Vec::<String>::new(), "{shape}");
  }
}

#[test]
fn what_an_ask_wants_reads_as_its_choices_its_shape_or_nothing_for_a_notice() {
  assert_eq!(wants("bool", false), "yes or no");
  assert_eq!(wants("Literal['a', 'b', 'c']", false), "a, b or c");
  assert_eq!(wants("int", false), "int");
  assert_eq!(wants("None", true), "nothing");
  assert_eq!(wants("list[Literal['a', 'b']]", false), "any of a or b");
}

#[test]
fn the_desk_holds_the_questions_and_the_notices_the_oldest_first_and_drops_a_question_that_is_done()
{
  let acts = [
    act("chain1", "chain", "", "operator", 1, None),
    act("chain2", "chain", "", "rung1", 2, None),
    asked(worded(
      act("prompt1", "prompt", "chain2", "rung2", 3, None),
      json!(["str", "Which base?", "operator"]),
    )),
    worded(
      act("prompt2", "prompt", "chain1", "rung1", 4, Some(4)),
      json!(["None", "The build is green.", "operator"]),
    ),
    asked(worded(
      act("prompt3", "prompt", "chain1", "rung1", 5, None),
      json!(["bool", "Merge it?", "operator"]),
    )),
    asked(worded(
      act("prompt4", "prompt", "chain2", "rung2", 6, Some(7)),
      json!(["int", "How many?", "operator"]),
    )),
  ];
  let got = desk(&acts, &["prompt2"]);
  let asks: Vec<(&str, &str, bool, Vec<String>)> = got
    .asks
    .iter()
    .map(|one| (one.id.as_str(), one.chain.as_str(), one.notice, one.choices.clone()))
    .collect();
  let yes = vec!["yes".to_owned(), "no".to_owned()];
  let want = [
    ("prompt1", "chain2", false, Vec::new()),
    ("prompt2", "chain1", true, Vec::new()),
    ("prompt3", "chain1", false, yes),
  ];
  assert_eq!(asks, want);
  assert_eq!((got.questions, got.notices), (2, 1));
  let literal = asked(worded(
    act("prompt5", "prompt", "chain1", "rung1", 8, None),
    json!(["Literal['squash', 'rebase']", "How should #305 merge?", "operator"]),
  ));
  let [ask] = &desk(&[literal], &[]).asks[..] else { panic!("one ask") };
  assert_eq!(
    (ask.shape.as_str(), &ask.choices[..]),
    ("Literal['squash', 'rebase']", &["squash".to_owned(), "rebase".to_owned()][..])
  );
  assert_eq!(wants(&ask.shape, ask.notice), "squash or rebase");
}

#[test]
fn a_list_of_a_literal_gives_the_values_that_the_operator_ticks() {
  let tick = |shown: &str, json: &str| Tick { shown: shown.to_owned(), json: json.to_owned() };
  assert_eq!(
    ticks("list[Literal['a b', 3, True, None]]"),
    [tick("a b", "\"a b\""), tick("3", "3"), tick("True", "true"), tick("None", "null")]
  );
  for shape in ["Literal['a']", "list[str]", "list[Literal[1.5]]", "tuple[Literal['a']]", "list"] {
    assert_eq!(ticks(shape), [], "{shape}");
  }
  assert_eq!(choices("list[Literal['a']]"), Vec::<String>::new());
}

#[test]
fn a_shape_that_is_more_than_text_says_what_its_line_is_with_an_example() {
  assert_eq!(hint("int"), "a whole number");
  assert_eq!(hint("float"), "a number");
  assert_eq!(hint("list[str]"), r#"a JSON list of text, as ["a", "b"]"#);
  assert_eq!(hint("list"), r#"a JSON list, as ["a", 1]"#);
  assert_eq!(hint("dict[str, int]"), r#"a JSON object of whole numbers, as {"a": 1}"#);
  assert_eq!(
    hint("tuple[int, str]"),
    r#"a JSON list of 2 items: a whole number; a text, as [1, "a"]"#
  );
  assert_eq!(hint("int | None"), "a whole number, or no word for nothing");
  assert_eq!(hint("str | None"), "a text, or no word for nothing");
  for shape in ["str", "bool", "None", "Literal['a']", "set", "list[Option]", "tuple[int, ...]"] {
    assert_eq!(hint(shape), "", "{shape}");
  }
}

#[test]
fn a_union_that_holds_none_takes_a_line_of_no_word_and_is_no_choice() {
  assert_eq!(choices("Literal['a', 'b'] | None"), Vec::<String>::new());
  for shape in ["int | None", "str | None", "list[str] | None"] {
    assert!(takes_nothing(shape), "{shape}");
  }
  for shape in ["int", "None", "str", "list[str]", "int | str", "Literal['a']", "set | None"] {
    assert!(!takes_nothing(shape), "{shape}");
  }
}

/// A prompt on the root from the operator.
fn prompt(id: &str, began: u64) -> Act {
  worded(
    act(id, "prompt", "chain1", "operator", began, None),
    json!(["str", format!("{id} says"), ""]),
  )
}

/// A prompt on a thread.
fn on_thread(act: Act, thread: &str) -> Act {
  Act { on: thread.to_owned(), ..act }
}

/// The threads of a chain, each with the ids of its messages.
fn listed(acts: &[Act]) -> Vec<(String, Vec<String>)> {
  let on: Vec<&Act> = acts.iter().collect();
  let threads = chain_threads(&on, &rows(acts));
  threads.into_iter().map(|one| (one.prompt, one.messages)).collect()
}

#[test]
fn a_prompt_on_a_thread_and_the_acknowledgment_of_an_act_of_a_thread_join_that_thread() {
  let two = [prompt("prompt1", 1), prompt("prompt2", 2)];
  assert_eq!(listed(&two), [("prompt1".to_owned(), vec![]), ("prompt2".to_owned(), vec![])]);
  let joined = [prompt("prompt1", 1), on_thread(prompt("prompt2", 2), "prompt1")];
  assert_eq!(listed(&joined), [("prompt1".to_owned(), vec!["prompt2".to_owned()])]);
  let all = [
    act("chain1", "chain", "", "", 0, None),
    worded(prompt("prompt1", 1), json!(["str", "Hire two workers.", "claude"])),
    on_thread(prompt("prompt2", 2), "prompt1"),
    on_thread(prompt("prompt3", 3), "prompt2"),
    act("rung1", "rung", "chain1", "prompt1", 4, None),
    act("bash5", "bash", "chain1", "rung1", 5, None),
    Act {
      by: "chain1".to_owned(),
      ..worded(prompt("prompt9", 6), json!(["None", "bash5 done", ""]))
    },
  ];
  let messages = ["prompt2", "prompt3", "prompt9"].map(str::to_owned).to_vec();
  assert_eq!(listed(&all), [("prompt1".to_owned(), messages)]);
}

#[test]
fn a_question_and_a_notice_that_a_thread_made_stand_in_that_thread_and_one_of_no_thread_in_none() {
  let acts = [
    prompt("prompt1", 1),
    act("rung1", "rung", "chain1", "prompt1", 2, None),
    asked(worded(
      act("prompt2", "prompt", "chain1", "rung1", 3, None),
      json!(["bool", "Keep it?", "operator"]),
    )),
    worded(
      act("prompt3", "prompt", "chain1", "rung1", 4, None),
      json!(["None", "It is 14 lines.", "operator"]),
    ),
    asked(worded(
      act("prompt4", "prompt", "chain1", "chain1", 5, None),
      json!(["str", "A question of no thread.", "operator"]),
    )),
  ];
  let on: Vec<&Act> = acts.iter().collect();
  let [thread] = &chain_threads(&on, &rows(&acts))[..] else { panic!("one thread") };
  assert_eq!(thread.asked, ["prompt2", "prompt3"]);
  assert_eq!(thread.asks, ["prompt2"]);
  assert_eq!(thread.state, State::Asks);
}

/// A thread of a list, which asks while a question waits in it, and else runs.
fn thread(act: &Act, asks: &[&str]) -> ChainThread {
  ChainThread {
    prompt: act.id.clone(),
    title: act.id.clone(),
    messages: Vec::new(),
    asked: Vec::new(),
    asks: asks.iter().map(|one| (*one).to_owned()).collect(),
    state: if asks.is_empty() { State::Runs } else { State::Asks },
    resolved: false,
    origin: Origin::Rung,
    yours: false,
  }
}

/// The ids of the threads of a level of a tree.
fn level(nodes: &[ThreadNode]) -> Vec<&str> {
  nodes.iter().map(|one| one.thread.prompt.as_str()).collect()
}

#[test]
fn the_threads_that_the_ladder_of_a_thread_made_nest_under_it_and_a_question_in_an_agent_rolls_up()
{
  let lead = prompt("prompt1", 1);
  let agents: Vec<Act> = (0..8)
    .map(|n| act(&format!("prompt{}", n + 2), "prompt", "chain1", "rung1", n + 3, None))
    .collect();
  let deep = act("prompt20", "prompt", "chain1", "rung2", 20, None);
  let other = prompt("prompt30", 30);
  let steps = [
    act("rung1", "rung", "chain1", "prompt1", 2, None),
    act("rung2", "rung", "chain1", "prompt9", 19, None),
  ];
  let all: Vec<Act> = [lead.clone()]
    .into_iter()
    .chain(steps)
    .chain(agents.clone())
    .chain([deep.clone(), other.clone()])
    .collect();
  let list: Vec<ChainThread> = [thread(&lead, &[])]
    .into_iter()
    .chain(agents.iter().map(|one| thread(one, &[])))
    .chain([thread(&deep, &["prompt21"]), thread(&other, &[])])
    .collect();
  let tree = nest_threads(list, &rows(&all));
  assert_eq!(level(&tree), ["prompt1", "prompt30"]);
  assert_eq!(level(&tree[0].agents), ids(&agents));
  let deepest = tree[0].agents.last().unwrap();
  assert_eq!(level(&deepest.agents), ["prompt20"]);
  assert_eq!(
    [tree[0].state, deepest.state, tree[1].state],
    [State::Asks, State::Asks, State::Runs]
  );
  assert!(tree[0].thread.asks.is_empty());
  assert_eq!(tree[0].thread.state, State::Runs);
}

#[test]
fn a_failed_thread_of_the_operator_whose_agent_asks_counts_as_asks_and_keeps_failed() {
  let lead = prompt("prompt1", 1);
  let agent = act("prompt2", "prompt", "chain1", "rung1", 3, None);
  let all = [lead.clone(), act("rung1", "rung", "chain1", "prompt1", 2, None), agent.clone()];
  let rows = rows(&all);
  let answered = nest_threads(vec![thread(&lead, &[]), thread(&agent, &[])], &rows);
  assert_eq!([answered[0].state, answered[0].agents[0].state], [State::Runs, State::Runs]);
  let failed = ChainThread { state: State::Failed, yours: true, ..thread(&lead, &[]) };
  let node = nest_threads(vec![failed.clone(), thread(&agent, &["prompt9"])], &rows);
  assert_eq!([node[0].state, node[0].thread.state], [State::Asks, State::Failed]);
  assert_eq!(concerns(&node[0]), Some(Concern::Asks));
  assert_eq!(attention(&node), Attention { asks: 1, failed: 0, paused: 0 });
  let alone = nest_threads(vec![failed, thread(&agent, &[])], &rows);
  assert_eq!(attention(&alone), Attention { asks: 0, failed: 1, paused: 0 });
}

#[test]
fn a_question_concerns_the_operator_in_any_thread_and_a_failure_or_a_pause_only_in_his_own() {
  let error = Phase::Done(json!({"is": "Refused", "args": ["no"]}));
  let base = [
    prompt("prompt1", 1),
    act("rung1", "rung", "chain1", "operator", 1, None),
    act("rung2", "rung", "chain1", "prompt1", 3, None),
  ];
  let made =
    |by: &str| worded(act("prompt2", "prompt", "chain1", by, 4, None), json!(["str", "says", ""]));
  let read = |more: Vec<Act>| {
    let acts: Vec<Act> = base.iter().cloned().chain(more).collect();
    let on: Vec<&Act> = acts.iter().collect();
    let rows = rows(&acts);
    let found: HashMap<String, (Origin, bool, bool)> =
      nest_threads(chain_threads(&on, &rows), &rows)
        .iter()
        .flat_map(|top| std::iter::once(top).chain(&top.agents))
        .map(|node| {
          (
            node.thread.prompt.clone(),
            (node.thread.origin, node.thread.yours, concerns(node).is_some()),
          )
        })
        .collect();
    found["prompt2"]
  };
  let step = act("rung3", "rung", "chain1", "prompt2", 5, None);
  let ask = asked(worded(
    act("prompt9", "prompt", "chain1", "rung3", 9, None),
    json!(["bool", "Keep it?", "operator"]),
  ));
  assert_eq!(read(vec![made("rung2"), step, ask]), (Origin::Rung, false, true));
  let ended = |by: &str| stated(Act { phase: error.clone(), ended: Some(5), ..made(by) });
  let held = |by: &str| {
    stated(Act { pause: Some(Pause { by: "operator".to_owned(), why: None }), ..made(by) })
  };
  assert_eq!(read(vec![ended("rung2")]), (Origin::Rung, false, false));
  assert_eq!(read(vec![ended("rung1")]), (Origin::Rung, true, true));
  assert_eq!(read(vec![held("rung1")]), (Origin::Rung, true, true));
  assert_eq!(read(vec![held("rung2")]), (Origin::Rung, false, false));
  assert_eq!(read(vec![made("operator")]), (Origin::Operator, true, false));
  assert_eq!(read(vec![ended("ear7")]), (Origin::Ear, false, false));
}

#[test]
fn the_acts_as_they_stood_at_a_moment_hold_what_began_by_then_and_an_act_that_ended_later_runs() {
  let acts = [
    act("chain1", "chain", "", "operator", 1, None),
    Act {
      phase: Phase::Done(json!("this")),
      ..asked(worded(
        act("prompt1", "prompt", "chain1", "rung1", 2, Some(9)),
        json!(["str", "Which?", "operator"]),
      ))
    },
    act("rung2", "rung", "chain1", "operator", 3, Some(5)),
    Act {
      phase: Phase::Done(json!({"stdout": {"content": "all\n"}, "stderr": {"content": ""}})),
      ..act("bash1", "bash", "chain1", "rung2", 4, Some(8))
    },
    act("read1", "read", "chain1", "rung2", 7, Some(7)),
  ];
  assert_eq!(moments_of(&acts), [1, 2, 3, 4, 5, 7, 8, 9]);
  let then = as_of(&acts, 5);
  let done: Vec<(&str, bool)> = then.iter().map(|one| (one.id.as_str(), one.done())).collect();
  assert_eq!(done, [("chain1", false), ("prompt1", false), ("rung2", true), ("bash1", false)]);
  assert_eq!([then[1].value(), then[3].value()], [None, None]);
  let owed = |acts: &[Act]| {
    acts.iter().filter(|one| one.owed()).map(|one| one.id.clone()).collect::<Vec<_>>()
  };
  assert_eq!(owed(&then), ["prompt1"]);
  assert_eq!(then[0].state, State::Asks);
  assert_eq!(owed(&as_of(&acts, 9)), Vec::<String>::new());
}

#[test]
fn the_spend_counts_the_answers_sums_their_dollars_and_names_the_actor_of_the_last() {
  let answers = [
    Answer { actor: "claude-cli:sonnet/high".to_owned(), usage: [100.0, 10.0, 0.0, 0.0, 0.25] },
    Answer { actor: "claude-cli:opus/high".to_owned(), usage: [200.0, 20.0, 0.0, 0.0, 0.5] },
  ];
  let want = Spend { dollars: 0.75, answers: 2, actor: Some("claude-cli:opus/high".to_owned()) };
  assert_eq!(Spend::of(&answers), want);
}
