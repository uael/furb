//! What no door of the crate proves, or proves fast. The engine bounds a turn at 30 s, and no door
//! shortens it, so only a test that sets the bound of its thread proves the bound. The runs that
//! the sandbox began are counted inside it alone. The template strings of the two doors are made
//! by functions that only those doors call.

#[path = "../tests/common/lived.rs"]
mod lived;

use lived::{Lived, done, on, paused, provider, said};

/// Each answer of the script of a life of these tests moves the bound of the turn of the words
/// after it to the bound of a loaded machine.
fn answered(nth: usize) {
  testing::answering(nth);
}

use std::{
  cell::RefCell,
  fs,
  rc::Rc,
  task::Waker,
  time::{Duration, Instant},
};

use crate::{
  Ear, Engine, Fact, Object,
  ear::{ear, hear, say},
  verbs,
};

use crate::{
  sand::BOUND,
  testing,
  testing::{Yard, block_on},
  value,
};
use std::{env, process};

#[test]
fn a_generator_that_a_builtin_steps_waits_on_an_act() {
  let word = "def h(p):\n  return read(p).content\nclose(len(''.join(h(p) for p in ['a.txt'])))";
  let mut lived = Lived::new("joined", &[word], true).unwrap();
  fs::write(lived.at.join("a.txt"), "one\ntwo\n").unwrap();
  let root = lived.root();
  let with = verbs::Prompt { on: on(&root), ..Default::default() };
  let act = lived.engine.prompt(Object::string("int"), with).unwrap();
  assert_eq!(block_on(act).unwrap().as_ref().as_int(), Some(8));
}

#[test]
fn a_word_that_never_yields_raises_at_the_bound_and_the_chain_asks_again() {
  bounded();
  // The second word polls the clock for a second, past the short bound, and runs whole
  // under the bound of the words that the test does not cut.
  let polls = "t0 = clock()\nwhile clock() - t0 < 1:\n  pass\nclose(8)";
  let mut lived = Lived::new("spun", &["while True:\n  pass", polls, "close(0)"], true).unwrap();
  let root = lived.root();
  let with = verbs::Prompt { on: on(&root), ..Default::default() };
  let act = lived.engine.prompt(Object::string("int"), with).unwrap();
  assert_eq!(block_on(act).unwrap().as_ref().as_int(), Some(8));
  let read = lived.read.borrow();
  assert!(read[1].contains("TimeoutError('turn time limit exceeded: 500ms')"), "{}", read[1]);
}

#[test]
fn a_word_that_never_yields_on_another_chain_raises_and_the_root_goes_on() {
  bounded();
  let word = "c = chain('w')\nr = rung('while True:\\n  pass', on=c)\nawait wait(1, __name__)\nclose(repr(peek(r)))";
  let mut lived = Lived::new("apart", &[word, "pass"], true).unwrap();
  let root = lived.root();
  let with = verbs::Prompt { on: on(&root), ..Default::default() };
  let act = lived.engine.prompt(Object::string("str"), with).unwrap();
  let got = block_on(act).unwrap();
  assert_eq!(got.as_ref().as_str(), Some("TimeoutError('turn time limit exceeded: 500ms')"));
}

#[test]
fn an_ear_that_spins_past_the_bound_is_dropped_as_an_ear_that_raises_and_the_life_goes_on() {
  bounded();
  let word =
    "def spin():\n  yield\n  while True:\n    pass\ndrive(spin(), 'spin')\ndebug(t'driven')";
  let again =
    "def calm():\n  yield\ntry:\n  drive(calm(), 'spin')\n  close(8)\nexcept Refused:\n  close(0)";
  let mut lived = Lived::new("ear", &[word, again], true).unwrap();
  let root = lived.root();
  let with = verbs::Prompt { on: on(&root), ..Default::default() };
  let act = lived.engine.prompt(Object::string("int"), with).unwrap();
  assert_eq!(
    block_on(act).unwrap().as_ref().as_int(),
    Some(8),
    "the name of the dropped ear is free again"
  );
  assert!(lived.engine.stopped().is_none(), "{:?}", lived.engine.stopped());
}

#[test]
fn a_record_whose_word_never_yields_replays_to_the_same_facts() {
  bounded();
  let (root, facts, _yard) = {
    let mut first = Lived::new("replayed", &["while True:\n  pass", "close(8)"], true).unwrap();
    let root = first.root();
    let with = verbs::Prompt { on: on(&root), ..Default::default() };
    let act = first.engine.prompt(Object::string("int"), with).unwrap();
    assert_eq!(block_on(act).unwrap().as_ref().as_int(), Some(8));
    let facts =
      first.engine.transcript(verbs::Transcript { on: on(&root), of: None, at: None }).unwrap();
    (root, facts.iter().filter_map(said).collect::<Vec<_>>(), first.end())
  };
  bounded();
  let mut second = Lived::new("replayed", &[], false).unwrap();
  assert!(second.engine.raised().is_none(), "{:?}", second.engine.raised());
  assert_eq!(second.read.borrow().len(), 0, "a later life asks no model for what the record holds");
  let again =
    second.engine.transcript(verbs::Transcript { on: on(&root), of: None, at: None }).unwrap();
  let again = again.iter().filter_map(said).collect::<Vec<_>>();
  // After the record, the later life stands again at its tip.
  let (made, tip) = again.split_at(facts.len().min(again.len()));
  assert_eq!(made, facts);
  assert!(tip.first().is_some_and(|one| one.starts_with("stand ")), "{tip:?}");
}

#[test]
fn a_fault_that_cuts_a_run_stops_the_life_which_refuses_each_later_entry() {
  // A bound of 100 ms may pass before the loop enters its `try` on a loaded machine, as in
  // life.test.rs.
  BOUND.set(Duration::from_secs(1));
  let cutter = "try:\n  while True:\n    pass\nfinally:\n  while True:\n    pass";
  let mut lived = Lived::new("stopped", &[cutter, "close(8)"], true).unwrap();
  let root = lived.root();
  let with = verbs::Prompt { on: on(&root), ..Default::default() };
  let cut = lived
    .engine
    .prompt(Object::string("int"), with)
    .expect_err("the second excess of the turn cuts the run");
  assert_eq!(cut.name, "TimeoutError");
  assert_eq!(lived.engine.stopped().map(|fault| fault.name.as_str()), Some("TimeoutError"));
  let with = verbs::Prompt { on: on(&root), ..Default::default() };
  let no =
    lived.engine.prompt(Object::string("int"), with).expect_err("a life that stopped makes no act");
  assert_eq!(no.name, "Refused");
  assert!(no.message().starts_with("The life stopped at TimeoutError: "), "{no}");
  let view = lived.engine.word("len(transcript(ROOT))", vec![]).expect_err("nor reads one");
  assert_eq!(view.message(), no.message());
  assert_eq!(lived.read.borrow().len(), 1, "the model was asked once");
}

#[test]
fn an_ear_that_ends_after_it_says_still_has_each_saying_heard() {
  let at = env::temp_dir().join(format!("furb-engine-briefly-{}", process::id()));
  let ears = [("provider", provider(at, Rc::default(), Rc::default())), ("briefly", briefly())];
  let mut engine = Engine::boot(Vec::<Object>::new(), ears).unwrap();
  let root = engine.root().to_owned();
  let act =
    engine.wait(&[root.as_str()], verbs::Wait { seconds: Some(9.0), on: on(&root) }).unwrap();
  let id = act.id().to_owned();
  block_on(act).unwrap();
  let facts = engine.transcript(verbs::Transcript { on: on(&root), of: None, at: None }).unwrap();
  let heard: Vec<&str> = facts.iter().filter(|one| one.about() == id).map(Fact::kind).collect();
  assert_eq!(heard, ["wait", "started", "done"]);
}

#[test]
fn a_life_whose_words_each_ran_within_the_bound_replays_past_it() {
  let word = "for i in range(300):\n  await wait(0, __name__)\nclose(8)";
  let (root, facts, _yard) = {
    let mut first = Lived::new("outlasts", &[word], true).unwrap();
    let root = first.root();
    let with = verbs::Prompt { on: on(&root), ..Default::default() };
    let act = first.engine.prompt(Object::string("int"), with).unwrap();
    assert_eq!(block_on(act).unwrap().as_ref().as_int(), Some(8), "the life runs its words");
    let facts =
      first.engine.transcript(verbs::Transcript { on: on(&root), of: None, at: None }).unwrap();
    (root, facts.iter().filter_map(said).collect::<Vec<_>>(), first.end())
  };
  // A life that opens on the record and keeps nothing leaves the record as it was, so each replay
  // does the same work.
  let whole = least(|| Lived::new("outlasts", &[], false).expect("the replay opens the life"));
  let replay = bounds(whole);
  bounded_at(replay);
  let began = Instant::now();
  let mut second = Lived::new("outlasts", &[], false).expect("the replay opens the life");
  assert!(began.elapsed() > replay, "the replay as a whole outlasts the bound");
  assert!(second.engine.raised().is_none(), "{:?}", second.engine.raised());
  assert!(second.engine.stopped().is_none(), "{:?}", second.engine.stopped());
  let again =
    second.engine.transcript(verbs::Transcript { on: on(&root), of: None, at: None }).unwrap();
  let again = again.iter().filter_map(said).collect::<Vec<_>>();
  // The time and the Kernel say their facts in an order of their own, so the replay holds each
  // fact, in any order.
  let mut missing = facts;
  for one in &again {
    if let Some(at) = missing.iter().position(|was| was == one) {
      missing.swap_remove(at);
    }
  }
  assert!(
    missing.is_empty(),
    "the replay holds each fact of the record: {:?}",
    &missing[..missing.len().min(3)]
  );
  drop(second);
  let third = Lived::new("outlasts", &[], false).expect("the life opens again");
  assert!(third.engine.raised().is_none(), "{:?}", third.engine.raised());
  assert_eq!(third.root(), root);
}

#[test]
fn a_replay_takes_the_answer_of_each_question_that_the_record_holds_and_a_live_word_past_the_bound_is_refused()
 {
  bounded_at(Duration::from_secs(30));
  // A word that asks the clock again and again in one step, well within the bound of its life.
  let word = "n = 0\nfor i in range(3000):\n  n += clock() > 0\nclose(8)";
  let (root, facts, took, _yard) = {
    let mut first = Lived::new("settled", &[word], true).unwrap();
    let root = first.root();
    let with = verbs::Prompt { on: on(&root), ..Default::default() };
    let began = Instant::now();
    let act = first.engine.prompt(Object::string("int"), with).unwrap();
    assert_eq!(block_on(act).unwrap().as_ref().as_int(), Some(8), "the life runs its word");
    let took = began.elapsed();
    let facts =
      first.engine.transcript(verbs::Transcript { on: on(&root), of: None, at: None }).unwrap();
    (root, facts.iter().filter_map(said).collect::<Vec<_>>(), took, first.end())
  };
  assert!(took > Duration::from_millis(40), "the word runs long enough to pass a bound: {took:?}");
  // A machine under load runs the replay slower than the life ran it: a bound of a twentieth of the
  // word stands for it.
  bounded_at(took / 20);
  let spin = "while True:\n  pass";
  let mut second = Lived::new("settled", &[spin, "close(9)"], false).expect("the replay opens");
  assert!(second.engine.raised().is_none(), "{:?}", second.engine.raised());
  assert!(second.engine.stopped().is_none(), "{:?}", second.engine.stopped());
  let again =
    second.engine.transcript(verbs::Transcript { on: on(&root), of: None, at: None }).unwrap();
  let again = again.iter().filter_map(said).collect::<Vec<_>>();
  let ends: Vec<_> = facts.iter().filter(|one| one.starts_with("done")).collect();
  let missing: Vec<_> = ends.iter().filter(|one| !again.contains(one)).collect();
  assert!(missing.is_empty(), "the replay ends each act as the record does: {missing:?}");
  // The live steps that follow run under a bound that a machine under load keeps, and the spin
  // passes any bound.
  bounded_at(Duration::from_secs(2));
  let with = verbs::Prompt { on: on(&root), ..Default::default() };
  let act = second.engine.prompt(Object::string("int"), with).unwrap();
  assert_eq!(block_on(act).unwrap().as_ref().as_int(), Some(9), "the chain asks again");
  let read = second.read.borrow();
  assert!(
    read.iter().any(|one| one.contains("TimeoutError('turn time limit exceeded")),
    "a live word past the bound is refused: {read:?}"
  );
  drop(read);
  drop(second);
  // The record keeps the end that the bound gave the word that spins, so its replay ends it so.
  let third = Lived::new("settled", &[], false).expect("the replay of the spin opens");
  assert!(third.engine.raised().is_none(), "{:?}", third.engine.raised());
  assert!(third.engine.stopped().is_none(), "{:?}", third.engine.stopped());
}

#[test]
fn a_run_of_the_record_that_goes_on_live_after_the_replay_is_bounded() {
  bounded_at(Duration::from_secs(2));
  // The word waits, and the life ends while it waits: the record holds the run but not its end.
  let word = "await wait(1.5, __name__)\nwhile True:\n  pass";
  let (root, _yard) = {
    let mut first = Lived::new("held-live", &[word], true).unwrap();
    let root = first.root();
    let with = verbs::Prompt { on: on(&root), ..Default::default() };
    let _asked = first.engine.prompt(Object::string("int"), with).unwrap();
    let pause = first
      .engine
      .wait(&[root.as_str()], verbs::Wait { seconds: Some(0.3), on: on(&root) })
      .unwrap();
    block_on(pause).unwrap();
    (root, first.end())
  };
  let mut second = Lived::new("held-live", &["pass", "close(9)"], false).expect("the replay opens");
  // The wake covers the thread, so the chain asks it one reply while its word waits, and a pass
  // leaves the thread to rest on that wait.
  second.engine.wake(&root).unwrap();
  // The life runs in short waits until the bound refuses the word, which it does after 2 s.
  let refused =
    |read: &[String]| read.iter().any(|one| one.contains("TimeoutError('turn time limit exceeded"));
  let started = Instant::now();
  while !refused(&second.read.borrow()) && started.elapsed() < Duration::from_secs(8) {
    let pause = second
      .engine
      .wait(&[root.as_str()], verbs::Wait { seconds: Some(0.2), on: on(&root) })
      .unwrap();
    block_on(pause).unwrap();
  }
  let read = second.read.borrow();
  assert!(refused(&read), "the word that spins after its wait is refused by the bound: {read:?}");
}

/// Each sandbox that this thread makes from now on, with a turn bound short enough for a
/// test, which the first word of the script meets; each word after it runs under
/// [`crate::testing::LOADED`],
/// since the test proves what the first word did, not how fast the machine runs the others.
fn bounded() {
  testing::cut_first(Duration::from_millis(500));
}

/// An ear that says a wait started and done, and ends in the same poll.
fn briefly() -> Box<dyn Ear> {
  ear(move |mut co| async move {
    loop {
      let a = hear(&mut co).await;
      if a.kind() == "wait" && a.question() {
        say(&mut co, Fact::says("started", a.about(), []));
        say(&mut co, done(a.about(), Object::none()));
        return Ok(());
      }
    }
  })
}

/// Each sandbox that this thread makes from now on, and each that holds the bound again after
/// the work of the engine, with this bound of the turn.
fn bounded_at(bound: Duration) {
  testing::bound_all(bound);
}

/// The bound of the replay that the test proves: half of `replay`, the least time that a replay of
/// the record as a whole took in this run.
///
/// Each bound is a share of the work that must outlast it, as this machine ran that work, so the
/// work outlasts it on each machine. One run of the same replay takes up to two thirds more than
/// another on a quiet machine, so the share leaves a single run that margin. A step of a word in a
/// replay holds more than the word: it holds the work of each ear that the word wakes, and each run
/// of the collector of cycles, whose time grows with the heap of the life. The word of this life
/// takes 300 steps, so each step stays well under half of the replay as a whole.
fn bounds(replay: Duration) -> Duration {
  replay / 2
}

/// The least time that a piece of work took of three: a pause of the machine only makes the work
/// longer, so the least is the work. What the work made goes after its time is read.
fn least<T>(mut work: impl FnMut() -> T) -> Duration {
  let timed = (0..3).map(|_| {
    let began = Instant::now();
    let made = work();
    let took = began.elapsed();
    drop(made);
    took
  });
  timed.min().unwrap_or_default()
}

#[test]
fn a_template_string_of_the_host_comes_in_whole_and_a_pair_comes_in_as_its_expression_with_an_equals_sign()
 {
  let mut lived = Lived::new("templates", &[], true).unwrap();
  let whole = value::templated([
    Object::string("n is "),
    Object::list([Object::int(5), Object::string("n"), Object::none(), Object::string("03")]),
  ]);
  let got = lived.engine.word("repr(list(t))", vec![("t", whole)]).unwrap();
  assert_eq!(
    got.as_ref().as_str(),
    Some("['n is ', Interpolation(5, 'n', None, '03')]"),
    "each string and each interpolation came in as it was, its conversion and its format spec among it"
  );
  let pairs =
    value::paired([("n".to_owned(), Object::int(5)), ("s".to_owned(), Object::string("a"))]);
  let got = lived.engine.word("repr(list(t))", vec![("t", pairs)]).unwrap();
  assert_eq!(
    got.as_ref().as_str(),
    Some("['n=', Interpolation(5, 'n', 'r', ''), ' s=', Interpolation('a', 's', 'r', '')]"),
    "each pair came in as the f-string {{expression=}} makes it, with a space between two"
  );
}

#[test]
fn what_the_ears_say_of_their_own_accord_in_one_drive_is_one_feed_of_the_sandbox() {
  let (mut engine, gate) = paused("drained");
  let root = engine.root().to_owned();
  let id = engine
    .wait(&[root.as_str()], verbs::Wait { seconds: Some(9.0), on: on(&root) })
    .unwrap()
    .id()
    .to_owned();
  let told = Rc::new(RefCell::new(Vec::new()));
  let held = Rc::clone(&told);
  engine.watch(&id, move |value| held.borrow_mut().push(value.py_repr())).unwrap();
  let before = engine.sand.fed();
  engine.pump(Waker::noop()).unwrap();
  assert_eq!(engine.sand.fed() - before, 0, "a drive where no ear says anything feeds nothing");
  gate.send(()).unwrap();
  engine.pump(Waker::noop()).unwrap();
  assert_eq!(engine.sand.fed() - before, 1, "one drive of three things said is one feed");
  assert_eq!(*told.borrow(), ["None"], "whoever watches the wait is told in that feed");
}

#[test]
fn a_read_of_a_name_runs_no_code_of_the_life_and_says_the_representation_of_plain_values() {
  let word = "seen = []\nclass Loud:\n  def __repr__(self):\n    seen.append(1)\n    return 'loud'\nloud = \
              Loud()\nnumbers = [1, 'two']\nclose(0)";
  let mut lived = Lived::new("inspected", &[word], true).unwrap();
  let root = lived.root();
  let with = verbs::Prompt { on: on(&root), ..Default::default() };
  let act = lived.engine.prompt(Object::string("int"), with).unwrap();
  assert_eq!(block_on(act).unwrap().as_ref().as_int(), Some(0));
  let facts = |engine: &mut Engine| {
    engine.transcript(verbs::Transcript { on: on(&root), of: None, at: None }).unwrap().len()
  };
  let before = facts(&mut lived.engine);
  let (kind, repr, _) = lived.engine.inspect("numbers", None).unwrap();
  assert_eq!((kind.as_str(), repr.as_str()), ("list", "[1, 'two']"));
  // The representation of a value of a class of the life is code of the life, which a read never
  // runs: the read says its type alone.
  let (kind, repr, _) = lived.engine.inspect("loud", None).unwrap();
  assert_eq!((kind.as_str(), repr.as_str()), ("Loud", ""));
  let (_, seen, _) = lived.engine.inspect("seen", None).unwrap();
  assert_eq!(seen, "[]");
  assert_eq!(facts(&mut lived.engine), before);
}
