//! One life of the real engine, driven by the crate from end to end, on ears alone.
//!
//! Every other test of the crate holds one piece. This holds the whole of it against the engine itself: an engine
//! boots on the ears of the World that the crate writes, and on a provider and a console that the test writes as
//! coroutines, as a host does. An operator says verbs, a model answers a prompt, the Kernel gates the word it wrote
//! and runs it, a command runs on this machine and speaks from its own thread, a wait ends, the console answers a
//! prompt, and the record the store kept opens a second life.

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

/// The console of the test: it takes each prompt to the operator, closes one of an int with 3, and refuses the rest.
fn console() -> Box<dyn Ear> {
  ear(|co, _| async move {
    loop {
      let a = hear(&co).await;
      if a.kind() != "prompt" || !a.question() {
        continue;
      }
      say(&co, Fact::says("started", a.about(), [])).await;
      let shape = a.word(1).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default();
      let value = match shape.as_str() {
        "int" => Object::int(3),
        _ => Fault::refused(format!("the console answers no {shape}")).object(),
      };
      call(&co, "close", vec![value], vec![("id", Object::string(a.about()))]).await?;
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
    Lived::on(yard, words, fresh, vec![])
  }

  /// A life whose ears of the host come before the ears of the World.
  fn on(
    yard: &str,
    words: &[&str],
    fresh: bool,
    first: Vec<(&str, Box<dyn Ear>)>,
  ) -> Result<Self, Fault> {
    let at = std::env::temp_dir().join(format!("furb-engine-{yard}"));
    if fresh {
      let _ = fs::remove_dir_all(&at);
    }
    fs::create_dir_all(&at).expect("a yard of the test");
    let (record, store) = world::store(at.join("record.jsonl"))?;
    let read = Rc::default();
    let words = Rc::new(RefCell::new(words.iter().map(|one| (*one).to_owned()).collect()));
    let mut ears = first;
    ears.extend([
      ("provider", provider(at.clone(), words, Rc::clone(&read))),
      ("console", console()),
      ("files", world::files()),
      ("bash", world::bash()),
      ("time", world::time()),
      ("store", store),
    ]);
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
fn an_engine_opens_on_its_root_and_the_provider_answers_what_the_root_stands_on() {
  let mut lived = Lived::new("opens", &[], true).unwrap();
  assert_eq!(lived.root(), "chain1");
  assert!(lived.engine.raised().is_none(), "{:?}", lived.engine.raised());
  let cwd = lived.engine.cwd(verbs::Cwd { on: on(&lived.root()) }).unwrap();
  assert_eq!(cwd, lived.at.display().to_string());
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
fn the_gate_refuses_a_word_and_the_engine_asks_the_model_again() {
  let mut lived = Lived::new("gates", &["close(nowhere)", "close(7)"], true).unwrap();
  let with = verbs::Prompt { on: on(&lived.root()), ..Default::default() };
  let got = block_on(lived.engine.prompt(Object::string("int"), with).unwrap()).unwrap();
  assert_eq!(got.as_ref().as_int(), Some(7));
  assert_eq!(lived.read.borrow().len(), 2, "the model was asked again after the refusal");
  assert!(lived.read.borrow()[1].contains("refused"), "{}", lived.read.borrow()[1]);
}

#[test]
fn a_word_that_raises_is_asked_again_and_the_model_reads_what_it_raised() {
  let mut lived = Lived::new("raises", &["close(1 / 0)", "close(9)"], true).unwrap();
  let with = verbs::Prompt { on: on(&lived.root()), ..Default::default() };
  let got = block_on(lived.engine.prompt(Object::string("int"), with).unwrap()).unwrap();
  assert_eq!(got.as_ref().as_int(), Some(9));
  assert!(lived.read.borrow()[1].contains("ZeroDivisionError"), "{}", lived.read.borrow()[1]);
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
fn a_command_that_outlives_its_timeout_ends_with_no_code() {
  let mut lived = Lived::new("late", &[], true).unwrap();
  let with = verbs::Bash { timeout: Some(0.2), on: on(&lived.root()), ..Default::default() };
  let exit: Exit = block_on(lived.engine.bash("sleep 5 & sleep 5", with).unwrap()).unwrap();
  assert_eq!(exit.code, None);
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
fn the_console_closes_a_prompt_to_the_operator_with_what_it_answered() {
  let mut lived = Lived::new("operator", &[], true).unwrap();
  let root = lived.root();
  let with = verbs::Prompt { to: Some("operator".to_owned()), on: on(&root), ..Default::default() };
  let got = block_on(lived.engine.prompt(Object::string("int"), with.clone()).unwrap()).unwrap();
  assert_eq!(got.as_ref().as_int(), Some(3));
  let no = block_on(lived.engine.prompt(Object::string("str"), with).unwrap()).unwrap_err();
  assert_eq!(no.name, "Refused");
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
fn an_ear_that_comes_before_the_files_takes_a_read_in_their_place() {
  let mine = ear(|co, _| async move {
    loop {
      let a = hear(&co).await;
      if a.kind() == "read" && a.question() {
        say(&co, done(a.about(), Text::new("mine", "mine\n").object())).await;
      }
    }
  });
  let mut lived = Lived::on("replaces", &[], true, vec![("mine", mine)]).unwrap();
  let got = lived
    .engine
    .read("a.txt", verbs::Read { on: on(&lived.root()), ..Default::default() })
    .unwrap();
  assert_eq!(got, Text::new("mine", "mine\n"));
}

#[test]
fn an_ear_wraps_the_files_by_taking_a_write_and_asking_it_again() {
  let wraps = ear(|co, _| async move {
    loop {
      let a = hear(&co).await;
      if a.kind() != "write" || !a.question() || a.by() == "loud" {
        continue;
      }
      let Some(text) = a.word(1).and_then(Text::of) else { continue };
      let loud = Text::new(text.path, text.content.to_uppercase()).object();
      let asked = vec![Object::string("write"), Object::string(a.on()), loud];
      let got = call(&co, "ask", asked, vec![]).await;
      say(&co, done(a.about(), got.unwrap_or_else(|fault| fault.object()))).await;
    }
  });
  let mut lived = Lived::on("wraps", &[], true, vec![("loud", wraps)]).unwrap();
  let root = lived.root();
  let got =
    lived.engine.write(&Text::new("a.txt", "quiet\n"), verbs::Write { on: on(&root) }).unwrap();
  assert_eq!(got.content, "QUIET\n");
  assert_eq!(fs::read_to_string(lived.at.join("a.txt")).unwrap(), "QUIET\n");
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

#[test]
fn a_second_store_on_a_record_that_another_holds_is_refused() {
  let lived = Lived::new("leased", &[], true).unwrap();
  let no = world::store(lived.at.join("record.jsonl")).map(|_| ()).unwrap_err();
  assert!(no.message().contains("Another process owns"), "{no}");
  drop(lived);
}
