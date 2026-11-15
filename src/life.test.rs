//! What no door of a life proves, or proves fast. The engine bounds a turn at 30 s, and no door
//! shortens it, so only a test that sets the bound of its thread proves the bound in a life that a
//! host opens. The carry that a crossing gives the next engine stands inside the open alone.

#![expect(clippy::disallowed_methods, reason = "a test blocks on its runtime and fails by a panic")]

#[path = "../tests/common/opened.rs"]
mod opened;

use opened::{closed, crossing, opening, prompted, scripted, segment, yard};

/// Each answer of the script of a life of these tests moves the bound of the turn of the words
/// after it to the bound of a loaded machine.
fn answered(nth: usize) {
  testing::answering(nth);
}

use std::{
  fs, future,
  path::Path,
  sync::{Arc, Mutex},
  time::{Duration, Instant},
};

use futures::{channel::mpsc::unbounded, executor, future::select};

use super::{Answer, Opening};
use crate::{
  Act, Engine, Fact, Object, sand,
  sand::BOUND,
  testing,
  testing::{LOADED, Yard, cut_first},
  value::{Fault, entry},
  verbs, wire, world,
};

#[test]
fn a_life_that_opens_on_a_record_that_a_stop_cut_asks_the_reply_again_and_ends_the_command_with_no_wake()
 {
  let at = yard("cut");
  let (asked, heard) = unbounded::<()>();
  let asked = Mutex::new(asked);
  // The first model writes a part of its turn, and the life stops while it writes the rest.
  let streams = Answer(Arc::new(move |_, told| {
    told("x = ", "");
    drop(asked.lock().unwrap().unbounded_send(()));
    Box::pin(future::pending())
  }));
  let first = Opening {
    roster: Some(vec!["claude-cli:opus".to_owned()].into()),
    answer: Some(streams),
    skills: Some(false),
    ..opening(&at, true)
  };
  let mut first = first.boot::<String>([]).unwrap().0;
  let on = Some("chain1".to_owned());
  drop(first.bash("sleep 30", verbs::Bash { on: on.clone(), ..Default::default() }).unwrap());
  let message = Some("go".to_owned());
  let prompt = verbs::Prompt { message, on, ..Default::default() };
  let prompt = first.prompt(Object::string("str"), prompt).unwrap().id().to_owned();
  {
    let mut heard = heard;
    let streamed = futures::StreamExt::next(&mut heard);
    let act = Act::<Object>::of(&mut first, &prompt);
    executor::block_on(select(act, streamed));
  }
  drop(first);
  let mut later = Opening { skills: Some(false), ..scripted(&at, &["close('again')"]) }
    .boot::<String>([])
    .unwrap()
    .0;
  assert!(
    later.pending().unwrap().is_empty(),
    "the open put the work that the stop cut to the outside"
  );
  let got = executor::block_on(Act::<Object>::of(&mut later, &prompt)).unwrap();
  assert_eq!(got.as_ref().as_str(), Some("again"), "the reply that the stop cut is asked again");
  let command = later.word("repr(peek('bash1'))", vec![]).unwrap();
  let command = command.as_ref().as_str().unwrap_or_default().to_owned();
  assert!(
    command.contains("interrupted"),
    "the command that the stop cut ends as interrupted: {command}"
  );
  let running =
    "[x[1] for x in transcript(ROOT) if x[0] == 'started' and x[1] != ROOT and not over(x[1])]";
  let running = later.word(running, vec![]).unwrap().as_ref().py_repr();
  assert_eq!(running, "[]", "no act stays running with no process");
}

#[test]
fn a_word_that_polls_the_clock_ends_at_the_bound_of_a_turn_and_two_readings_in_a_row_still_run() {
  // A bound of one second cuts the word as a longer one would, and the replay of a shorter run
  // takes less time.
  cut_first(Duration::from_secs(1));
  let at = yard("clock-spin");
  let poll = "t0 = clock()\nwhile clock() - t0 < 60:\n  pass\nclose('polled')";
  let twice = "a = clock()\nb = clock()\nclose(b >= a)";
  let mut life =
    Opening { skills: Some(false), ..scripted(&at, &[poll, twice]) }.boot::<String>([]).unwrap().0;
  let asked = verbs::Prompt {
    message: Some("go".to_owned()),
    on: Some("chain1".to_owned()),
    ..Default::default()
  };
  let started = Instant::now();
  let prompt = life.prompt(Object::string("object"), asked).unwrap().id().to_owned();
  let got = executor::block_on(Act::<Object>::of(&mut life, &prompt)).unwrap();
  let took = started.elapsed();
  assert!(life.stopped().is_none(), "the cut of the poll leaves the life whole");
  assert_eq!(got.as_ref().as_bool(), Some(true), "two readings in a row run");
  let raised = life
    .word("repr([x[3] for x in transcript('chain1', 'tell') if 'raised' in x[3][0]][:1])", vec![])
    .unwrap();
  let raised = raised.as_ref().as_str().unwrap_or_default().to_owned();
  assert!(
    raised.contains("past the bound of a turn"),
    "the poll ends at the bound of a turn: {raised}"
  );
  let clocks = life.word("len(transcript('chain1', 'clock'))", vec![]).unwrap();
  eprintln!("the poll made {:?} acts of clock in {took:?}", clocks.as_ref().as_int());
  // The readings of the clock count what the word did, which no load changes: a poll that its
  // own loop ended would span 60 s of them.
  let readings = life.word("[peek(x[1]) for x in transcript('chain1', 'clock')]", vec![]).unwrap();
  let readings: Vec<f64> = readings
    .as_ref()
    .items()
    .unwrap_or_default()
    .iter()
    .filter_map(monty_types::ObjectRef::as_float)
    .collect();
  assert!(readings.len() >= 3, "the poll and the word after it read the clock: {readings:?}");
  // The readings of the poll, before the two of the word after it, count what the poll did, which
  // no load changes: a poll that its own loop ended would span 60 s of them.
  let poll = &readings[..readings.len() - 2];
  let span =
    poll.iter().copied().fold(f64::MIN, f64::max) - poll.iter().copied().fold(f64::MAX, f64::min);
  assert!(span < 30.0, "the poll was cut at the bound, not by its own loop: {span} s");
  drop(life);
  let again = scripted(&at, &[]).boot::<String>([]).unwrap().0;
  assert!(again.raised().is_none(), "a later life replays the poll and its refusal with no drift");
}

#[test]
fn a_word_that_loops_on_an_answer_of_the_host_ends_at_the_bound_of_a_turn_with_a_bounded_record() {
  // A bound of one second cuts the word as a longer one would, and the replay of a shorter run
  // takes less time.
  cut_first(Duration::from_secs(1));
  let at = yard("cd-spin");
  let spin = "while True:\n  cd('.')";
  let mut life = Opening { skills: Some(false), ..scripted(&at, &[spin, "close(1)"]) }
    .boot::<String>([])
    .unwrap()
    .0;
  let started = Instant::now();
  let got = asked_of(&mut life, "go").unwrap();
  let took = started.elapsed();
  assert!(life.stopped().is_none(), "the refusal raises in the word and leaves the life whole");
  assert_eq!(got.as_ref().as_int(), Some(1), "the prompt asks its model again");
  let raised = life
    .word("repr([x[3] for x in transcript('chain1', 'tell') if 'raised' in x[3][0]][:1])", vec![])
    .unwrap();
  let raised = raised.as_ref().as_str().unwrap_or_default().to_owned();
  assert!(raised.contains("past the bound of a turn"), "the loop ends at the bound: {raised}");
  let cds = life.word("len(transcript('chain1', 'cd'))", vec![]).unwrap();
  let cds = cds.as_ref().as_int().unwrap_or_default();
  eprintln!("the loop made {cds} acts of cd in {took:?}");
  assert!(cds < 200_000, "the record holds the acts of the bound alone: {cds}");
  drop(life);
  let again = scripted(&at, &[]).boot::<String>([]).unwrap().0;
  assert!(again.raised().is_none(), "a later life replays the loop and its refusal with no drift");
}

#[test]
fn a_word_that_catches_the_refusal_of_the_bound_and_loops_ends_its_step_and_leaves_no_deadline() {
  BOUND.set(Duration::from_secs(2));
  let at = yard("clock-caught");
  let caught = "while True:\n  try:\n    clock()\n  except Refused:\n    pass";
  let mut first =
    Opening { skills: Some(false), ..scripted(&at, &[caught]) }.boot::<String>([]).unwrap().0;
  drop(asked_of(&mut first, "go"));
  assert!(first.stopped().is_some(), "a word that asks past the refusal ends its step");
  assert!(sand::past().is_none(), "the stop leaves no deadline on the thread");
  drop(first);
  BOUND.set(LOADED);
  let mut later =
    Opening { skills: Some(false), ..scripted(&at, &["close(clock() > 0)", "close(clock() > 0)"]) }
      .boot::<String>([])
      .expect("a life that the bound stopped opens again")
      .0;
  assert!(later.stopped().is_none() && later.raised().is_none(), "{:?}", later.raised());
  let got = asked_of(&mut later, "go on").unwrap();
  assert_eq!(got.as_ref().as_bool(), Some(true), "the next step reads the clock");
}

#[test]
fn a_life_that_a_fault_of_the_sandbox_stopped_opens_again_and_goes_on() {
  // A bound of 100 ms may pass before the loop enters its `try` on a loaded machine, and the
  // refusal then raises outside it, so no `finally` runs and no second excess stops the life.
  BOUND.set(Duration::from_secs(1));
  let at = yard("faulted");
  let cutter = "try:\n  while True:\n    pass\nfinally:\n  while True:\n    pass";
  let mut first =
    Opening { skills: Some(false), ..scripted(&at, &[cutter]) }.boot::<String>([]).unwrap().0;
  let on = Some("chain1".to_owned());
  let message = Some("go".to_owned());
  let asked = verbs::Prompt { message, on: on.clone(), ..Default::default() };
  let prompt = first.prompt(Object::string("str"), asked).unwrap().id().to_owned();
  drop(executor::block_on(Act::<Object>::of(&mut first, &prompt)));
  assert!(first.stopped().is_some(), "the second excess of the turn stops the life");
  drop(first);
  BOUND.set(LOADED);
  let mut later =
    Opening { skills: Some(false), ..scripted(&at, &["close('again')", "close('on')"]) }
      .boot::<String>([])
      .expect("a life that a fault stopped opens again")
      .0;
  assert!(later.stopped().is_none(), "the open does not run the word into the same stop");
  let message = Some("go on".to_owned());
  let asked = verbs::Prompt { message, on, ..Default::default() };
  let next = later.prompt(Object::string("str"), asked).unwrap().id().to_owned();
  let got = executor::block_on(Act::<Object>::of(&mut later, &next)).unwrap();
  assert_eq!(got.as_ref().as_str(), Some("on"), "the life goes on");
  let state = format!(
    "repr([peek('{prompt}'), peek([x[1] for x in transcript('chain1', 'rung') if x[2] == '{prompt}'][0])])"
  );
  let ended = later.word(&state, vec![]).unwrap();
  let ended = ended.as_ref().as_str().unwrap_or_default().to_owned();
  assert!(
    ended.starts_with("['again', Refused(") && ended.contains("was interrupted"),
    "the rung that stopped the life ends as interrupted, and its prompt asks its model again: {ended}"
  );
  drop(later);
  let mut again =
    Opening { skills: Some(false), ..scripted(&at, &[]) }.boot::<String>([]).unwrap().0;
  assert!(again.stopped().is_none(), "a second open replays the end of the rung");
  let replayed = again.word(&state, vec![]).unwrap();
  assert_eq!(
    replayed.as_ref().as_str().unwrap_or_default(),
    ended,
    "the replay holds the same state"
  );
}

/// A life born on a carry holds the teach and the enables of its root in the turns that the carry
/// holds, which no journal says again, so the ear of the extensions teaches nothing again at its
/// first stand, and enables nothing again.
#[test]
fn a_life_born_on_a_carry_teaches_and_enables_nothing_again() {
  let at = yard("taught");
  let open = || opening(&at, true).boot::<String>([]).unwrap().0;
  let mut first = open();
  let said = "close([len(transcript('chain1', 'teach')), len(transcript('chain1', 'enable'))])";
  let was = closed(&mut first, said).as_ref().py_repr();
  drop(first);
  crossing(&at);
  assert_eq!(closed(&mut open(), said).as_ref().py_repr(), was);
}

/// A prompt of the root, in a life that a test opened, and what it came to: an error when the life
/// stopped before it closed.
fn asked_of(life: &mut Engine, message: &str) -> Result<Object, Fault> {
  let asked = verbs::Prompt {
    message: Some(message.to_owned()),
    on: Some("chain1".to_owned()),
    ..Default::default()
  };
  let prompt = life.prompt(Object::string("object"), asked).unwrap().id().to_owned();
  executor::block_on(Act::<Object>::of(life, &prompt))
}

#[test]
fn a_life_that_a_fault_stopped_inside_a_cut_run_opens_at_the_exact_state_of_its_tip() {
  let at = yard("stopped-tip");
  stopped_inside_a_cut_run(&at);
  let record = at.join("record.jsonl");
  let text = fs::read_to_string(&record).unwrap();
  let unplaced =
    text.lines().filter(|line| line.starts_with("[[\"done\"") && line.contains("],null,"));
  assert_eq!(unplaced.count(), 1, "the record keeps the end of the cut run with no place");
  let kept: Vec<String> = world::kept(&record)
    .unwrap()
    .iter()
    .filter_map(|one| entry(&one.as_ref(), 0).and_then(Fact::of))
    .map(|one| format!("('{}', '{}')", one.kind(), one.about()))
    // The root stands on no chain, so no transcript holds the fact of its birth.
    .filter(|one| one != "('chain', 'chain1')")
    .collect();
  let state = "repr([[c, turns(on=c), sorted(k for k in module(c) if not k.startswith('__')), \
               [x[:2] for x in transcript(c)]] for c in [f'chain{n}' for n in range(1, 9) if get(f'chain{n}')]] \
               + [x[:2] for x in transcript('')])";
  let read = |inspecting: bool| {
    let opening = Opening { inspecting: Some(inspecting), ..scripted(&at, &[]) };
    let mut life = Opening { skills: Some(false), ..opening }.boot::<String>([]).unwrap().0;
    assert!(life.stopped().is_none() && life.raised().is_none(), "the open goes to its tip");
    life.word(state, vec![]).unwrap().as_ref().as_str().unwrap().to_owned()
  };
  let inspected = read(true);
  for fact in &kept {
    assert!(inspected.contains(fact.as_str()), "the open holds {fact} of the record");
  }
  assert!(inspected.contains("'kept'"), "the module holds what the first word bound");
  let order = "[x[1] for c in ['chain1'] for t in [transcript(c)] for i, x in enumerate(t) \
               if x[0] == 'started' and ('done', x[1]) in [y[:2] for y in t[:i]]]";
  let mut again = Opening { inspecting: Some(true), skills: Some(false), ..scripted(&at, &[]) }
    .boot::<String>([])
    .unwrap()
    .0;
  let ended = again.word(order, vec![]).unwrap().as_ref().py_repr();
  assert_eq!(ended, "[]", "each act that the open ends was started before, as the life held it");
  assert_eq!(read(true), inspected, "each open that inspects reaches the same state");
  let mut live =
    Opening { skills: Some(false), ..scripted(&at, &["close('again')", "close('on')"]) }
      .boot::<String>([])
      .unwrap()
      .0;
  let shown = live.word(state, vec![]).unwrap().as_ref().as_str().unwrap().to_owned();
  for fact in &kept {
    assert!(shown.contains(fact.as_str()), "the life that keeps the record holds {fact} of it");
  }
  let next = live.prompt(Object::string("str"), prompted("go on")).unwrap().id().to_owned();
  assert_eq!(executor::block_on(Act::<Object>::of(&mut live, &next)).unwrap().py_repr(), "'on'");
}

/// A birth makes each chain again and keeps its turns, so the turns of a chain hold its start once
/// at most, however many births come.
#[test]
fn each_chain_holds_its_start_once_at_most_through_births_in_a_row() {
  let at = yard("reborn-twice");
  let given = || Opening { skills: Some(false), ..opening(&at, true) };
  let mut first = given().boot::<String>([]).unwrap().0;
  closed(&mut first, "x = 1\nwork = chain('work')\nclose(x)");
  drop(first);
  for _ in 0..4 {
    crossing(&at);
    drop(given().boot::<String>([]).unwrap().0);
  }
  let kept = world::kept(at.join("record.jsonl")).unwrap();
  assert_eq!(world::crossings(at.join("record.jsonl")).unwrap().len(), 4);
  let carry = given().carried(&kept).unwrap().expect("the record crossed");
  {
    let data =
      entry(&carry.as_ref(), 0).and_then(Fact::of).and_then(|a| a.word(0).map(wire::record));
    let chains = data.unwrap()["chains"].as_array().cloned().unwrap_or_default();
    for chain in &chains {
      let id = &chain["id"];
      let turns = chain["turns"].as_array().cloned().unwrap_or_default();
      // Each turn is a fact, which the wire writes as a tuple of its words.
      let facts = turns.iter().map(|turn| &turn["args"][0]);
      let starts = facts.filter(|fact| fact[0] == "started" && fact[1] == *id).count();
      assert!(starts <= 1, "the turns of {id} hold its start {starts} times");
    }
  }
}

#[test]
fn a_life_on_a_record_that_crossed_goes_on_with_the_standing_of_its_last_stand() {
  let at = yard("stood-crossing");
  let both = "close((standing()[2], sorted(a[0] for a in standing()[0])))";
  let given = Opening {
    skills: Some(false),
    roster: Some(vec!["claude-cli:opus".to_owned(), "claude-cli:sonnet".to_owned()].into()),
    actor: Some("claude-cli:sonnet/low".to_owned()),
    ..scripted(&at, &[])
  };
  let (mut first, _) = given.boot::<String>([]).unwrap();
  let had = closed(&mut first, both).py_repr();
  drop(first);
  // No stand follows the crossing, and the last stand before it gave the standing.
  crossing(&at);
  let unsaid = Opening { skills: Some(false), roster: None, actor: None, ..scripted(&at, &[]) };
  let (mut later, _) = unsaid.boot::<String>([]).unwrap();
  assert_eq!(closed(&mut later, both).py_repr(), had);
}

/// A life on the record in a directory that a fault stops inside a run that its bound cut: its
/// first word binds a name and closes, and its second runs past the bound twice.
fn stopped_inside_a_cut_run(at: &Path) {
  BOUND.set(Duration::from_secs(1));
  let cutter = "try:\n  while True:\n    pass\nfinally:\n  while True:\n    pass";
  let mut first =
    Opening { skills: Some(false), ..scripted(at, &["kept = 7\nclose('one')", cutter]) }
      .boot::<String>([])
      .unwrap()
      .0;
  let one = first.prompt(Object::string("str"), prompted("one")).unwrap().id().to_owned();
  assert_eq!(executor::block_on(Act::<Object>::of(&mut first, &one)).unwrap().py_repr(), "'one'");
  let cut = first.prompt(Object::string("str"), prompted("go")).unwrap().id().to_owned();
  drop(executor::block_on(Act::<Object>::of(&mut first, &cut)));
  assert!(first.stopped().is_some(), "the second excess of the turn stops the life");
  drop(first);
  BOUND.set(LOADED);
}

#[test]
fn a_carry_holds_data_alone_and_a_later_engine_seeds_a_life_from_it_with_no_act_run_again() {
  let at = yard("carry-data");
  let ran = "rounds = globals().get('rounds', 0) + 1\nawait wait(3600 if rounds == 1 else 0, __name__)\nclose('late')";
  let mut first = scripted(&at, &[ran]).boot::<String>([]).unwrap().0;
  let word = "work = chain('work')\nshell = bash('echo one')\nawait shell\nqueue = [shell]\nSIZE = 6\n\
              def f():\n  return SIZE\nwaiting = prompt(str, 'Wait.', on=work)\nawait wait(0.3, __name__)\nclose(str(waiting))";
  let waiting = closed(&mut first, word).as_ref().as_str().unwrap().to_owned();
  drop(first);
  crossing(&at);
  // The carry that the next open makes, at the fact made of this build that the crossing writes.
  let mut kept = world::kept(at.join("record.jsonl")).unwrap();
  kept.push(world::running(&[]));
  let carry = scripted(&at, &[]).carried(&kept).unwrap().expect("the record crosses");
  let fact = entry(&carry.as_ref(), 0).and_then(Fact::of);
  let data = wire::outward(fact.unwrap().word(0).unwrap());
  assert_eq!(data["schema"], serde_json::json!(world::CHECKPOINT_SCHEMA.to_string()));
  assert!(data.get("version").is_none(), "the carry names its schema and no version of the replay");
  assert!(data.get("entries").is_none(), "the carry holds data and no entry");
  let chains: Vec<&str> =
    data["chains"].as_array().unwrap().iter().filter_map(|one| one["id"].as_str()).collect();
  assert_eq!(chains, ["chain1", "chain2"]);
  let root = &data["chains"][0];
  assert_eq!(root["values"]["queue"], serde_json::json!("[Act('bash1')]"));
  assert_eq!(
    root["definitions"].as_array().and_then(|each| each.last()),
    Some(&serde_json::json!("SIZE = 6\n\ndef f():\n  return SIZE")),
    "the definitions hold one string for each word, the last word last"
  );
  let acts: Vec<(&str, bool)> = data["acts"]
    .as_array()
    .unwrap()
    .iter()
    .map(|one| (one["id"].as_str().unwrap(), one.get("value").is_some()))
    .collect();
  assert!(acts.contains(&("bash1", true)) && acts.contains(&(waiting.as_str(), false)), "{acts:?}");
  assert!(!data.to_string().contains("say('cut'"), "the carry holds no word that the crate crafts");
  let mut later = scripted(&at, &["close('late')"]).boot::<String>([]).unwrap().0;
  let word = format!(
    "wake('chain2')\nclose([await queue[0], f(), get('{waiting}')[:2], await Act('{waiting}'), str(bash('echo two'))])"
  );
  let exit = "Exit(code=0, stdout=Text(path='bash1/stdout', content='one\\n'), stderr=Text(path='bash1/stderr', content=''))";
  assert_eq!(
    closed(&mut later, &word).as_ref().py_repr(),
    format!("[{exit}, 6, ('prompt', '{waiting}'), 'late', 'bash2']"),
    "each value, definition, open prompt and act that a value names stands, and a new act takes the next name"
  );
  drop(later);
  let segment = segment(&at);
  let again = segment
    .iter()
    .filter_map(|one| entry(&one.as_ref(), 0).and_then(Fact::of))
    .filter(|a| a.kind() == "started" && a.about() == "bash1");
  assert_eq!(again.count(), 0, "no carried act runs again");
}
