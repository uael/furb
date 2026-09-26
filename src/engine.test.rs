//! One life of the real engine, driven by the crate from end to end, on ears alone.
//!
//! What the engine does is the suite's to prove, on both engines. What this proves is the crate: an engine boots on
//! the ears of the World that the crate writes, and on a provider that the test writes as a coroutine, as a host
//! does. The files read and write, a command runs on this machine and speaks from its own thread, a wait ends at its
//! due, a function of the host is called back, a fault crosses as itself, and the record the store kept opens a
//! second life.

use std::{
  cell::RefCell,
  collections::VecDeque,
  fs,
  future::Future,
  path::PathBuf,
  pin::pin,
  rc::Rc,
  sync::Arc,
  task::{Context, Poll, Wake, Waker},
  thread,
  time::Duration,
};

use crate::{
  Act, Ear, Engine, Exit, Fact, Fault, Object, Text,
  ear::{call, ear, hear, say},
  verbs, world,
};

/// A waker that unparks the thread of the test, so a voice spoken from another thread wakes the poll.
struct Parked(thread::Thread);

impl Wake for Parked {
  fn wake(self: Arc<Self>) {
    self.0.unpark();
  }
}

/// One future driven to its end on this thread.
fn block_on<T>(fut: impl Future<Output = T>) -> T {
  let waker = Waker::from(Arc::new(Parked(thread::current())));
  let mut cx = Context::from_waker(&waker);
  let mut fut = pin!(fut);
  loop {
    if let Poll::Ready(got) = fut.as_mut().poll(&mut cx) {
      return got;
    }
    thread::park_timeout(Duration::from_millis(20));
  }
}

/// A fact of a done of an act with a value.
fn done(about: &str, value: Object) -> Fact {
  Fact::says("done", about, [value])
}

/// The provider of the test: it answers what the chains stand on, and each reply with the next word of its script.
fn provider(
  at: PathBuf,
  words: Rc<RefCell<VecDeque<String>>>,
  read: Rc<RefCell<Vec<String>>>,
) -> Box<dyn Ear> {
  ear(move |co, _| async move {
    loop {
      let a = hear(&co).await;
      if !a.question() {
        continue;
      }
      match a.kind() {
        "stand" => {
          let roster = Object::list([
            Object::list([Object::string("operator"), Object::list([]), Object::int(200_000)]),
            Object::list([
              Object::string("m"),
              Object::list([Object::string("low")]),
              Object::int(200_000),
            ]),
          ]);
          let standing = Object::list([
            roster,
            Object::string(at.display().to_string()),
            Object::string("m/low"),
          ]);
          say(&co, done(a.about(), standing)).await;
        }
        "reply" => {
          say(&co, Fact::says("started", a.about(), [])).await;
          let turns = call(&co, "turns", vec![], vec![("on", Object::string(a.on()))]).await?;
          read.borrow_mut().push(turns.py_repr());
          let word = words.borrow_mut().pop_front().unwrap_or_else(|| "close(None)".to_owned());
          let turn = Object::tuple([
            Object::string("assistant"),
            Object::string(word),
            Object::none(),
            Object::list([]),
          ]);
          say(&co, done(a.about(), turn)).await;
        }
        _ => {}
      }
    }
  })
}

/// One life of the real engine, in a yard of the disk, with the model answering these words.
struct Lived {
  engine: Engine,
  at: PathBuf,
  read: Rc<RefCell<Vec<String>>>,
}

impl Lived {
  /// A life in a yard of its own, fresh, or opened on what the store kept there.
  fn new(yard: &str, words: &[&str], fresh: bool) -> Result<Self, Fault> {
    let at = std::env::temp_dir().join(format!("furb-engine-{yard}"));
    if fresh {
      let _ = fs::remove_dir_all(&at);
    }
    fs::create_dir_all(&at).expect("a yard of the test");
    let (record, store) = world::store(at.join("record.jsonl"))?;
    let read = Rc::default();
    let words = Rc::new(RefCell::new(words.iter().map(|one| (*one).to_owned()).collect()));
    let ears = [
      ("provider", provider(at.clone(), words, Rc::clone(&read))),
      ("files", world::files()),
      ("bash", world::bash()),
      ("time", world::time()),
      ("store", store),
    ];
    let engine = Engine::boot(record, ears)?;
    Ok(Lived { engine, at, read })
  }

  fn root(&self) -> String {
    self.engine.root().to_owned()
  }

  /// What an act came to, once the ears have said it, or the fault it completed with.
  fn settled(&mut self, id: &str) -> Result<Object, Fault> {
    block_on(Act::<Object>::of(&mut self.engine, id))
  }
}

fn on(root: &str) -> Option<String> {
  Some(root.to_owned())
}

#[test]
fn the_files_serve_a_read_and_a_write_of_the_real_engine() {
  let mut lived = Lived::new("reads", &[], true).unwrap();
  let root = lived.root();
  let got =
    lived.engine.write(&Text::new("a.txt", "one\ntwo\n"), verbs::Write { on: on(&root) }).unwrap();
  assert_eq!(got, Text::new(lived.at.join("a.txt").display().to_string(), "one\ntwo\n"));
  let got =
    lived.engine.read("a.txt", verbs::Read { on: on(&root), ..Default::default() }).unwrap();
  assert_eq!(got.content, "one\ntwo\n");
  let no =
    lived.engine.read("none.txt", verbs::Read { on: on(&root), ..Default::default() }).unwrap_err();
  assert_eq!(no.name, "Refused");
  assert!(no.message().contains("There is no file at"), "{no}");
  // A byte order mark is a character of the text, which a read keeps and a write writes again.
  fs::write(lived.at.join("table.csv"), "\u{feff}name,value\n").unwrap();
  let got =
    lived.engine.read("table.csv", verbs::Read { on: on(&root), ..Default::default() }).unwrap();
  assert_eq!(got.content, "\u{feff}name,value\n");
  let changed = Text::new("table.csv", got.content.replace("value", "amount"));
  lived.engine.write(&changed, verbs::Write { on: on(&root) }).unwrap();
  assert_eq!(fs::read(lived.at.join("table.csv")).unwrap()[..3], [0xef, 0xbb, 0xbf]);
}

#[test]
fn a_model_answers_a_prompt_and_the_kernel_gates_and_runs_the_word_it_wrote() {
  let mut lived = Lived::new("prompts", &["close(len(read('a.txt').lines))"], true).unwrap();
  let root = lived.root();
  lived
    .engine
    .write(&Text::new("a.txt", "one\ntwo\nthree\n"), verbs::Write { on: on(&root) })
    .unwrap();
  let with = verbs::Prompt {
    message: Some("count the lines".to_owned()),
    on: on(&root),
    ..Default::default()
  };
  let got = block_on(lived.engine.prompt(Object::string("int"), with).unwrap()).unwrap();
  assert_eq!(got.as_ref().as_int(), Some(3));
  assert_eq!(lived.read.borrow().len(), 1);
  assert!(lived.read.borrow()[0].contains("count the lines"));
}

#[test]
fn a_command_runs_on_this_machine_and_speaks_its_exit_from_its_own_thread() {
  let mut lived = Lived::new("commands", &[], true).unwrap();
  let apart = lived.engine.span(-250, -1).unwrap();
  let with = verbs::Bash { show_err: Some(apart), on: on(&lived.root()), ..Default::default() };
  let exit: Exit = block_on(lived.engine.bash("echo hi; echo no >&2", with).unwrap()).unwrap();
  assert_eq!(exit.code, Some(0));
  assert_eq!(exit.stdout.content, "hi\n");
  assert_eq!(exit.stderr.content, "no\n");
}

#[test]
fn a_command_that_the_engine_merges_says_its_stderr_in_its_stdout() {
  let mut lived = Lived::new("merges", &[], true).unwrap();
  let with = verbs::Bash { on: on(&lived.root()), ..Default::default() };
  let exit: Exit = block_on(lived.engine.bash("echo hi; echo no >&2", with).unwrap()).unwrap();
  assert_eq!((exit.stdout.content.as_str(), exit.stderr.content.as_str()), ("hi\nno\n", ""));
}

#[test]
fn a_command_is_fed_what_the_operator_writes_into_its_stdin() {
  let mut lived = Lived::new("feeds", &[], true).unwrap();
  let root = lived.root();
  let act = lived
    .engine
    .bash("cat", verbs::Bash { fed: Some(true), on: on(&root), ..Default::default() })
    .unwrap();
  let id = act.id().to_owned();
  let stdin = Text::new(format!("{id}/stdin"), "fed\n");
  lived.engine.write(&stdin, verbs::Write { on: on(&root) }).unwrap();
  lived
    .engine
    .write(&Text::new(format!("{id}/stdin"), ""), verbs::Write { on: on(&root) })
    .unwrap();
  let exit = Exit::of(lived.settled(&id).unwrap().as_ref()).expect("the command came to its exit");
  assert_eq!(exit.stdout.content, "fed\n");
}

#[test]
fn a_timeout_or_a_wait_past_the_longest_timer_runs_its_full_time() {
  let mut lived = Lived::new("long", &[], true).unwrap();
  let root = lived.root();
  let wait = lived.engine.wait(verbs::Wait { seconds: Some(1e300), on: on(&root) }).unwrap();
  let wait = wait.id().to_owned();
  let with = verbs::Bash { timeout: Some(1e300), on: on(&root), ..Default::default() };
  let exit: Exit = block_on(lived.engine.bash("echo finished", with).unwrap()).unwrap();
  assert_eq!((exit.code, exit.stdout.content.as_str()), (Some(0), "finished\n"));
  assert!(lived.engine.outcome(&wait).unwrap().is_none(), "the wait runs on");
}

#[test]
fn a_command_that_outlives_its_timeout_ends_with_no_code() {
  let mut lived = Lived::new("late", &[], true).unwrap();
  let with = verbs::Bash { timeout: Some(0.2), on: on(&lived.root()), ..Default::default() };
  let exit: Exit = block_on(lived.engine.bash("sleep 5 & sleep 5", with).unwrap()).unwrap();
  assert_eq!(exit.code, None);
}

#[test]
fn a_command_that_does_not_start_is_closed_with_why_and_the_chain_is_told() {
  let mut lived = Lived::new("unstarted", &[], true).unwrap();
  let root = lived.root();
  lived.engine.cd("nowhere", verbs::Cd { on: on(&root) }).unwrap();
  let with = verbs::Bash { on: on(&root), ..Default::default() };
  let no = block_on(lived.engine.bash("echo hi", with).unwrap()).unwrap_err();
  assert_eq!(no.name, "Refused");
  assert!(no.message().contains("\"echo hi\" did not start"), "{no}");
  let told = lived.engine.turns(verbs::Turns { on: on(&root) }).unwrap();
  let told = told.iter().map(|one| one.as_ref().py_repr()).collect::<Vec<_>>().join("\n");
  assert!(told.contains("#bash1 closed"), "{told}");
}

#[test]
fn the_world_ends_a_command_at_a_cancel_of_its_prompt_and_never_at_a_close_of_it() {
  let words = ["await bash('sleep 5')", "c = bash('sleep 0.2; echo late')\nclose(1)"];
  let mut lived = Lived::new("controls", &words, true).unwrap();
  let root = lived.root();
  let cancelled = lived
    .engine
    .prompt(Object::string("int"), verbs::Prompt { on: on(&root), ..Default::default() })
    .unwrap()
    .id()
    .to_owned();
  while lived.engine.get("bash1").unwrap().is_none() {
    thread::sleep(Duration::from_millis(5));
    let _ = lived.engine.peek("bash1", verbs::Peek::default());
  }
  lived.engine.cancel(&cancelled).unwrap();
  assert_eq!(lived.settled("bash1").unwrap_err().name, "CancelledError");
  let got = block_on(
    lived
      .engine
      .prompt(Object::string("int"), verbs::Prompt { on: on(&root), ..Default::default() })
      .unwrap(),
  )
  .unwrap();
  assert_eq!(got.as_ref().as_int(), Some(1));
  let exit =
    Exit::of(lived.settled("bash2").unwrap().as_ref()).expect("the command came to its exit");
  assert_eq!((exit.code, exit.stdout.content.as_str()), (Some(0), "late\n"));
  let dones = lived.engine.transcript(verbs::Transcript { on: on(&root) }).unwrap();
  let dones = dones.iter().filter(|one| one.kind() == "done" && one.about() == "bash1").count();
  assert_eq!(dones, 1, "the World said no second done of the command a control ended");
}

#[test]
fn a_wait_is_done_when_its_time_is_up_and_it_says_when_it_is_due() {
  let mut lived = Lived::new("waits", &[], true).unwrap();
  let root = lived.root();
  let act = lived.engine.wait(verbs::Wait { seconds: Some(0.1), on: on(&root) }).unwrap();
  let id = act.id().to_owned();
  block_on(act).unwrap();
  let facts = lived.engine.transcript(verbs::Transcript { on: on(&root) }).unwrap();
  let kinds: Vec<(String, String)> = facts
    .iter()
    .filter(|one| one.about() == id)
    .map(|one| (one.kind().to_owned(), one.by().to_owned()))
    .collect();
  let expected = [("wait", "operator"), ("started", "time"), ("due", "time"), ("done", "time")];
  assert_eq!(kinds, expected.map(|(a, b)| (a.to_owned(), b.to_owned())));
}

#[test]
fn a_wait_that_a_wake_starts_again_in_a_later_life_ends_when_it_was_due() {
  let root = {
    let mut first = Lived::new("due", &[], true).unwrap();
    let root = first.root();
    let act = first.engine.wait(verbs::Wait { seconds: Some(0.4), on: on(&root) }).unwrap();
    assert_eq!(act.id(), "wait1");
    root
  };
  let mut second = Lived::new("due", &[], false).unwrap();
  assert!(second.engine.raised().is_none(), "{:?}", second.engine.raised());
  let due = second.engine.transcript(verbs::Transcript { on: on(&root) }).unwrap();
  let due = due
    .iter()
    .find(|one| one.kind() == "due")
    .and_then(|one| one.word(0).and_then(|one| one.as_float()));
  assert!(due.is_some(), "the record said the due again");
  second.engine.wake(&root).unwrap();
  assert_eq!(second.settled("wait1").unwrap().as_ref().type_name(), "NoneType");
  let facts = second.engine.transcript(verbs::Transcript { on: on(&root) }).unwrap();
  assert_eq!(
    facts.iter().filter(|one| one.kind() == "due").count(),
    1,
    "the time ear said no second due"
  );
}

#[test]
fn a_function_of_the_host_is_called_back_by_the_sandbox_with_what_the_word_gave_it() {
  let mut lived = Lived::new("callables", &[], true).unwrap();
  let seen = Rc::new(RefCell::new(0usize));
  let held = Rc::clone(&seen);
  let show = lived.engine.callable(move |args| {
    *held.borrow_mut() =
      args.first().and_then(|one| one.as_ref().items()).map_or(0, |lines| lines.len());
    Ok(Object::list([Object::int(2)]))
  });
  let got = lived.engine.word("show(['one', 'two', 'three'])", vec![("show", show)]).unwrap();
  assert_eq!(*seen.borrow(), 3, "the function was given the lines");
  assert_eq!(got.py_repr(), "[2]", "the word got what the function gave");
  let bad = lived.engine.callable(|_| Err(Fault::refused("not lines")));
  let no = lived.engine.word("show(1)", vec![("show", bad)]);
  assert_eq!(
    no.unwrap_err().name,
    "RuntimeError",
    "what the function raised is raised in the word"
  );
}

#[test]
fn what_the_engine_raised_reaches_the_host_as_the_fault_it_is() {
  let mut lived = Lived::new("faults", &[], true).unwrap();
  assert!(lived.engine.get("bash9").unwrap().is_none(), "a name of no act gives nothing");
  let no = lived.engine.word("{}['bash9']", vec![]).unwrap_err();
  assert_eq!(no.name, "KeyError");
  let no = lived.engine.word("nowhere", vec![]).unwrap_err();
  assert_eq!(no.name, "NameError");
}

#[test]
fn what_is_fed_to_a_command_before_a_wake_starts_it_in_a_later_life_reaches_its_process() {
  let id = {
    let mut first = Lived::new("held", &[], true).unwrap();
    let root = first.root();
    let with = verbs::Bash { fed: Some(true), on: on(&root), ..Default::default() };
    first.engine.bash("cat", with).unwrap().id().to_owned()
  };
  let mut second = Lived::new("held", &[], false).unwrap();
  let root = second.root();
  let fed = Text::new(format!("{id}/stdin"), "before start\n");
  second.engine.write(&fed, verbs::Write { on: on(&root) }).unwrap();
  second
    .engine
    .write(&Text::new(format!("{id}/stdin"), ""), verbs::Write { on: on(&root) })
    .unwrap();
  second.engine.wake(&root).unwrap();
  let exit = Exit::of(second.settled(&id).unwrap().as_ref()).expect("the command came to its exit");
  assert_eq!(exit.stdout.content, "before start\n");
}

#[test]
fn a_wait_that_a_wake_starts_again_says_nothing_after_a_cancel() {
  {
    let mut first = Lived::new("hushed", &[], true).unwrap();
    let root = first.root();
    first.engine.wait(verbs::Wait { seconds: Some(0.2), on: on(&root) }).unwrap();
  }
  let mut second = Lived::new("hushed", &[], false).unwrap();
  let root = second.root();
  second.engine.wake(&root).unwrap();
  second.engine.cancel("wait1").unwrap();
  assert_eq!(second.settled("wait1").unwrap_err().name, "CancelledError");
  // A later wait outlives the deadline of the first.
  let later = second.engine.wait(verbs::Wait { seconds: Some(0.4), on: on(&root) }).unwrap();
  block_on(later).unwrap();
  let facts = second.engine.transcript(verbs::Transcript { on: on(&root) }).unwrap();
  let dones = facts.iter().filter(|one| one.kind() == "done" && one.about() == "wait1");
  assert_eq!(dones.map(|one| one.by().to_owned()).collect::<Vec<_>>(), ["wait1"]);
}

#[test]
fn a_second_life_on_the_record_the_store_kept_makes_the_same_acts_again() {
  let (root, id) = {
    let mut first = Lived::new("again", &["close(len(read('a.txt').lines))"], true).unwrap();
    let root = first.root();
    fs::write(first.at.join("a.txt"), "one\ntwo\n").unwrap();
    let act = first
      .engine
      .prompt(Object::string("int"), verbs::Prompt { on: on(&root), ..Default::default() })
      .unwrap();
    let id = act.id().to_owned();
    assert_eq!(block_on(act).unwrap().as_ref().as_int(), Some(2));
    (root, id)
  };
  let kept = world::kept(std::env::temp_dir().join("furb-engine-again/record.jsonl")).unwrap();
  assert!(
    kept.len() >= 4,
    "the record holds the chain, the prompt, its rung and its answer: {}",
    kept.len()
  );
  let mut second = Lived::new("again", &[], false).unwrap();
  assert!(second.engine.raised().is_none(), "{:?}", second.engine.raised());
  assert_eq!(second.root(), root);
  assert_eq!(
    second.engine.get(&id).unwrap().map(|one| one.kind().to_owned()).as_deref(),
    Some("prompt")
  );
  assert_eq!(second.read.borrow().len(), 0, "a later life asks no model for what the record holds");
  assert_eq!(second.settled(&id).unwrap().as_ref().as_int(), Some(2));
}
