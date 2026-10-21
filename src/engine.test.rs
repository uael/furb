//! One life of the real engine, driven by the crate from end to end, on ears alone.
//!
//! What the engine does is the suite's to prove, on both engines. What this proves is the crate: an engine boots on
//! the ears of the World that the crate writes, and on a provider that the test writes as a coroutine, as a host
//! does. The files read and write, a command runs on this machine and speaks as it writes, a wait ends at its
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
  time::{Duration, Instant},
};

use futures::{FutureExt, stream};
use unsync::oneshot;

use crate::{
  Act, Ear, Engine, Exit, Fact, Fault, Object, Text,
  ear::{Co, Next, ear, hear, say},
  engine::{CRASHES, callable},
  verbs, world,
};

/// A waker that unparks the thread of the test, so what an ear waits for wakes the poll when it is ready.
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
  ear(move |mut co| async move {
    loop {
      let a = hear(&mut co).await;
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
          say(&mut co, done(a.about(), standing));
        }
        "reply" => {
          say(&mut co, Fact::says("started", a.about(), []));
          let turns = co.call("turns", vec![], vec![("on", Object::string(a.on()))]).await?;
          read.borrow_mut().push(turns.py_repr());
          let word = words.borrow_mut().pop_front().unwrap_or_else(|| "close(None)".to_owned());
          let turn = Object::tuple([
            Object::string("assistant"),
            Object::string(word),
            Object::none(),
            Object::list([]),
          ]);
          say(&mut co, done(a.about(), turn));
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
    let at = std::env::temp_dir().join(format!("furb-engine-{yard}-{}", std::process::id()));
    if fresh {
      let _ = fs::remove_dir_all(&at);
    }
    fs::create_dir_all(&at).expect("a yard of the test");
    let (record, store) = world::store(at.join("record.jsonl"))?;
    let read = Rc::default();
    let words = Rc::new(RefCell::new(words.iter().map(|one| (*one).to_owned()).collect()));
    let scratch = world::Scratch::own();
    let ears = [
      ("provider", provider(at.clone(), words, Rc::clone(&read))),
      ("files", world::files(scratch.clone())),
      ("bash", world::bash(scratch)),
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
fn a_command_runs_on_this_machine_and_speaks_its_exit_from_its_own_thread() {
  let mut lived = Lived::new("commands", &[], true).unwrap();
  let apart = lived.engine.span(-250, -1).unwrap();
  let with = verbs::Bash { show_err: Some(apart), on: on(&lived.root()), ..Default::default() };
  let exit: Exit = block_on(lived.engine.bash("echo hi; echo no >&2", with).unwrap()).unwrap();
  assert_eq!(exit.code, Some(0));
  assert_eq!(exit.stdout.content, "hi\n");
  assert_eq!(exit.stderr.content, "no\n");
  // A command that the engine merges says its stderr in its stdout.
  let with = verbs::Bash { on: on(&lived.root()), ..Default::default() };
  let exit: Exit = block_on(lived.engine.bash("echo hi; echo no >&2", with).unwrap()).unwrap();
  assert_eq!((exit.stdout.content.as_str(), exit.stderr.content.as_str()), ("hi\nno\n", ""));
}

#[test]
fn a_command_whose_stdin_is_neither_a_text_nor_none_has_a_stdin_that_is_empty_and_closed() {
  let mut lived = Lived::new("flag", &[], true).unwrap();
  let root = lived.root();
  // The flag of a record made before the stdin was a text, False or True.
  for flag in [false, true] {
    let kwargs = vec![
      ("stdin", Object::bool(flag)),
      ("timeout", Object::float(5.0)),
      ("on", Object::string(&root)),
    ];
    let id = lived.engine.verb("bash", vec![Object::string("cat; echo end")], kwargs).unwrap();
    let id = id.as_ref().as_str().expect("the name of the command").to_owned();
    let exit =
      Exit::of(lived.settled(&id).unwrap().as_ref()).expect("the command came to its exit");
    assert_eq!((exit.code, exit.stdout.content.as_str()), (Some(0), "end\n"), "the flag {flag}");
  }
}

#[test]
fn the_world_writes_the_stderr_of_a_merged_command_into_its_stdout() {
  let mut lived = Lived::new("merged", &[], true).unwrap();
  let root = lived.root();
  let with = verbs::Bash { on: on(&root), ..Default::default() };
  let act = lived.engine.bash("printf a; printf b >&2; printf c", with).unwrap();
  let id = act.id().to_owned();
  let exit: Exit = block_on(act).unwrap();
  assert_eq!((exit.stdout.content.as_str(), exit.stderr.content.as_str()), ("abc", ""));
  let facts = lived.engine.transcript(verbs::Transcript { on: on(&root), of: None }).unwrap();
  let outs: Vec<(String, String)> = facts
    .iter()
    .filter(|one| one.kind() == "out" && one.about() == id)
    .map(|one| {
      let word =
        |at| one.word(at).and_then(|it| it.as_str().map(str::to_owned)).unwrap_or_default();
      (word(0), word(1))
    })
    .collect();
  // The parts of a stream come as the pipe gives them, so the test joins them.
  assert!(outs.iter().all(|(_, stream)| stream == "stdout"), "no out of the stderr: {outs:?}");
  assert_eq!(outs.iter().map(|(text, _)| text.as_str()).collect::<String>(), "abc");
  let door = || verbs::Read { on: on(&root), ..Default::default() };
  assert_eq!(lived.engine.read(&format!("{id}/stdout"), door()).unwrap().content, "abc");
  assert_eq!(lived.engine.read(&format!("{id}/stderr"), door()).unwrap().content, "");
}

#[test]
fn a_command_is_fed_what_the_operator_writes_into_its_stdin() {
  let mut lived = Lived::new("feeds", &[], true).unwrap();
  let root = lived.root();
  let act = lived
    .engine
    .bash("cat", verbs::Bash { stdin: Some(None), on: on(&root), ..Default::default() })
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
fn a_command_is_fed_a_stdin_of_text_whole_at_its_start_and_then_its_stdin_closes() {
  let mut lived = Lived::new("given", &[], true).unwrap();
  let with = verbs::Bash {
    stdin: Some(Some("one\ntwo\n".to_owned())),
    on: on(&lived.root()),
    ..Default::default()
  };
  // cat ends only at the end of its stdin, so the end of the command says that the stdin closed.
  let exit: Exit = block_on(lived.engine.bash("cat; echo end", with).unwrap()).unwrap();
  assert_eq!((exit.code, exit.stdout.content.as_str()), (Some(0), "one\ntwo\nend\n"));
}

#[test]
fn a_timeout_or_a_wait_past_the_longest_timer_runs_its_full_time() {
  let mut lived = Lived::new("long", &[], true).unwrap();
  let root = lived.root();
  let wait = lived.engine.wait(&[], verbs::Wait { seconds: Some(1e300), on: on(&root) }).unwrap();
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
  // Each process of the command sleeps for LONG, and the one in the background holds the stdout of the command, which
  // the World reads to its end before it says the exit. So an exit before LONG has passed proves that the World ended
  // every process at the timeout, and the bound holds under any load that delays the test by less than LONG.
  const LONG: u64 = 120;
  let begun = Instant::now();
  let exit: Exit =
    block_on(lived.engine.bash(&format!("sleep {LONG} & sleep {LONG}"), with).unwrap()).unwrap();
  assert_eq!(exit.code, None);
  let took = begun.elapsed();
  assert!(
    took < Duration::from_secs(LONG),
    "a process of the command outlived its timeout, for {took:?}"
  );
}

#[test]
fn a_command_that_puts_its_work_in_the_background_ends_when_that_work_ends() {
  let mut lived = Lived::new("behind", &[], true).unwrap();
  let with = verbs::Bash { timeout: Some(60.0), on: on(&lived.root()), ..Default::default() };
  // The shell exits at once, and the work in the background holds the stdout until it ends.
  let exit: Exit = block_on(lived.engine.bash("(sleep 1 && echo x) &", with).unwrap()).unwrap();
  assert_eq!((exit.code, exit.stdout.content.as_str()), (Some(0), "x\n"));
}

#[cfg(unix)]
#[test]
fn a_command_ends_once_its_shell_exited_and_no_process_of_its_group_lives() {
  let mut lived = Lived::new("left", &[], true).unwrap();
  let with = verbs::Bash { timeout: Some(600.0), on: on(&lived.root()), ..Default::default() };
  // The child leaves the group of the command and holds its streams for LONG, as the master of an ssh does, so an
  // exit before LONG has passed proves that the World did not wait for the streams.
  const LONG: u64 = 120;
  let begun = Instant::now();
  let line = format!("perl -e 'use POSIX; setsid(); sleep {LONG}' & echo $!");
  let exit: Exit = block_on(lived.engine.bash(&line, with).unwrap()).unwrap();
  let took = begun.elapsed();
  let pid = exit.stdout.content.trim().to_owned();
  let _ = std::process::Command::new("kill").arg(&pid).status();
  assert_eq!(exit.code, Some(0));
  assert!(pid.parse::<u32>().is_ok(), "the stdout says the child: {:?}", exit.stdout.content);
  assert!(
    took < Duration::from_secs(LONG),
    "the command waited for a child that left its group, for {took:?}"
  );
}

#[test]
fn a_cd_comes_to_the_directory_that_stands_there_and_a_cd_where_none_stands_is_refused() {
  let mut lived = Lived::new("moved", &[], true).unwrap();
  let root = lived.root();
  let into = lived.at.join("into");
  fs::create_dir_all(&into).expect("a directory of the test");
  let into = into.display().to_string();
  assert_eq!(lived.engine.cd(&into, verbs::Cd { on: on(&root) }).unwrap(), into);
  let no = lived.engine.cd("nowhere", verbs::Cd { on: on(&root) }).unwrap_err();
  assert_eq!(no.name, "Refused");
  assert!(no.message().contains("There is no directory at"), "{no}");
  assert_eq!(lived.engine.cwd(verbs::Cwd { on: on(&root) }).unwrap(), into);
}

#[test]
fn a_command_that_does_not_start_is_closed_with_why_and_the_chain_is_told() {
  let mut lived = Lived::new("unstarted", &[], true).unwrap();
  let root = lived.root();
  let gone = lived.at.join("gone");
  fs::create_dir_all(&gone).expect("a directory of the test");
  lived.engine.cd(&gone.display().to_string(), verbs::Cd { on: on(&root) }).unwrap();
  fs::remove_dir(&gone).expect("the directory goes");
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
  let dones = lived.engine.transcript(verbs::Transcript { on: on(&root), of: None }).unwrap();
  let dones = dones.iter().filter(|one| one.kind() == "done" && one.about() == "bash1").count();
  assert_eq!(dones, 1, "the World said no second done of the command a control ended");
}

#[test]
fn a_wait_is_done_when_its_time_is_up_and_it_says_when_it_is_due() {
  let mut lived = Lived::new("waits", &[], true).unwrap();
  let root = lived.root();
  let act = lived.engine.wait(&[], verbs::Wait { seconds: Some(0.1), on: on(&root) }).unwrap();
  let id = act.id().to_owned();
  block_on(act).unwrap();
  let facts = lived.engine.transcript(verbs::Transcript { on: on(&root), of: None }).unwrap();
  let kinds: Vec<(String, String)> = facts
    .iter()
    .filter(|one| one.about() == id)
    .map(|one| (one.kind().to_owned(), one.by().to_owned()))
    .collect();
  let expected = [("wait", "operator"), ("started", "time"), ("due", "time"), ("done", "time")];
  assert_eq!(kinds, expected.map(|(a, b)| (a.to_owned(), b.to_owned())));
}

#[test]
fn a_wait_for_an_act_is_done_with_its_name_when_it_ends_first_and_the_time_ear_stops_its_timer() {
  let mut lived = Lived::new("first", &[], true).unwrap();
  let root = lived.root();
  let with = verbs::Bash { on: on(&root), ..Default::default() };
  let command = lived.engine.bash("sleep 0.1", with).unwrap().id().to_owned();
  let act =
    lived.engine.wait(&[&command], verbs::Wait { seconds: Some(0.5), on: on(&root) }).unwrap();
  let id = act.id().to_owned();
  assert_eq!(
    block_on(act).unwrap(),
    Some(command.clone()),
    "the wait is done with the name of its act"
  );
  let later = lived.engine.wait(&[], verbs::Wait { seconds: Some(0.8), on: on(&root) }).unwrap();
  block_on(later).unwrap();
  let facts = lived.engine.transcript(verbs::Transcript { on: on(&root), of: None }).unwrap();
  let dones: Vec<String> = facts
    .iter()
    .filter(|one| one.kind() == "done" && one.about() == id)
    .map(|one| one.by().to_owned())
    .collect();
  assert_eq!(dones, [id], "the ear of the wait said it done, and the time ear stopped its timer");
}

#[test]
fn a_wait_that_a_wake_starts_again_in_a_later_life_ends_when_it_was_due() {
  let root = {
    let mut first = Lived::new("due", &[], true).unwrap();
    let root = first.root();
    let act = first.engine.wait(&[], verbs::Wait { seconds: Some(0.4), on: on(&root) }).unwrap();
    assert_eq!(act.id(), "wait1");
    root
  };
  let mut second = Lived::new("due", &[], false).unwrap();
  assert!(second.engine.raised().is_none(), "{:?}", second.engine.raised());
  let due = second.engine.transcript(verbs::Transcript { on: on(&root), of: None }).unwrap();
  let due = due
    .iter()
    .find(|one| one.kind() == "due")
    .and_then(|one| one.word(0).and_then(|one| one.as_float()));
  assert!(due.is_some(), "the record said the due again");
  second.engine.wake(&root).unwrap();
  assert_eq!(second.settled("wait1").unwrap().as_ref().type_name(), "NoneType");
  let facts = second.engine.transcript(verbs::Transcript { on: on(&root), of: None }).unwrap();
  assert_eq!(
    facts.iter().filter(|one| one.kind() == "due").count(),
    1,
    "the time ear said no second due"
  );
}

#[test]
fn a_template_string_of_the_host_comes_in_whole_and_a_pair_comes_in_as_its_expression_with_an_equals_sign()
 {
  let mut lived = Lived::new("templates", &[], true).unwrap();
  let whole = crate::value::templated([
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
    crate::value::paired([("n".to_owned(), Object::int(5)), ("s".to_owned(), Object::string("a"))]);
  let got = lived.engine.word("repr(list(t))", vec![("t", pairs)]).unwrap();
  assert_eq!(
    got.as_ref().as_str(),
    Some("['n=', Interpolation(5, 'n', 'r', ''), ' s=', Interpolation('a', 's', 'r', '')]"),
    "each pair came in as the f-string {{expression=}} makes it, with a space between two"
  );
}

#[test]
fn a_function_of_the_host_is_called_back_by_the_sandbox_with_what_the_word_gave_it() {
  let mut lived = Lived::new("callables", &[], true).unwrap();
  let seen = Rc::new(RefCell::new(0usize));
  let held = Rc::clone(&seen);
  let show = callable(move |args| {
    *held.borrow_mut() =
      args.first().and_then(|one| one.as_ref().items()).map_or(0, |lines| lines.len());
    Ok(Object::list([Object::int(2)]))
  });
  let got = lived.engine.word("show(['one', 'two', 'three'])", vec![("show", show)]).unwrap();
  assert_eq!(*seen.borrow(), 3, "the function was given the lines");
  assert_eq!(got.py_repr(), "[2]", "the word got what the function gave");
  let bad = callable(|_| Err(Fault::refused("not lines")));
  let no = lived.engine.word("show(1)", vec![("show", bad)]);
  assert_eq!(
    no.unwrap_err().name,
    "RuntimeError",
    "what the function raised is raised in the word"
  );
}

#[test]
fn what_is_fed_to_a_command_before_a_wake_starts_it_in_a_later_life_reaches_its_process() {
  let id = {
    let mut first = Lived::new("held", &[], true).unwrap();
    let root = first.root();
    let with = verbs::Bash { stdin: Some(None), on: on(&root), ..Default::default() };
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
fn what_an_earlier_life_wrote_to_the_stdin_of_a_command_reaches_it_again_when_a_later_life_starts_it_at_a_wake()
 {
  let id = {
    let mut first = Lived::new("refed", &[], true).unwrap();
    let root = first.root();
    let with = verbs::Bash { stdin: Some(None), on: on(&root), ..Default::default() };
    let id = first.engine.bash("cat", with).unwrap().id().to_owned();
    let fed = Text::new(format!("{id}/stdin"), "a\n");
    first.engine.write(&fed, verbs::Write { on: on(&root) }).unwrap();
    id
  };
  let mut second = Lived::new("refed", &[], false).unwrap();
  let root = second.root();
  second.engine.wake(&root).unwrap();
  second
    .engine
    .write(&Text::new(format!("{id}/stdin"), ""), verbs::Write { on: on(&root) })
    .unwrap();
  let exit = Exit::of(second.settled(&id).unwrap().as_ref()).expect("the command came to its exit");
  assert_eq!(
    exit.stdout.content, "a\n",
    "the later life feeds the command again what the earlier life wrote"
  );
}

#[test]
fn a_command_an_earlier_life_did_not_end_runs_once_at_the_wake_and_its_door_holds_what_it_said() {
  let line = "printf x >> count; [ -e done ] || { printf ready; exec sleep 30; }";
  let (id, at) = {
    let mut first = Lived::new("rerun", &[], true).unwrap();
    let root = first.root();
    let id = first.engine.bash(line, verbs::Bash { on: on(&root), ..Default::default() }).unwrap();
    let id = id.id().to_owned();
    let door = || verbs::Read { on: on(&root), ..Default::default() };
    let waker = Waker::from(Arc::new(Parked(thread::current())));
    while first.engine.read(&format!("{id}/stdout"), door()).unwrap().content != "ready" {
      first.engine.pump(&waker).unwrap();
      thread::park_timeout(Duration::from_millis(20));
    }
    (id, first.at.clone())
  };
  fs::write(at.join("done"), "").unwrap();
  let mut second = Lived::new("rerun", &[], false).unwrap();
  let root = second.root();
  let door = || verbs::Read { on: on(&root), ..Default::default() };
  assert_eq!(fs::read_to_string(at.join("count")).unwrap(), "x", "a life that opens runs nothing");
  assert_eq!(second.engine.read(&format!("{id}/stdout"), door()).unwrap().content, "ready");
  second.engine.wake(&root).unwrap();
  let exit = Exit::of(second.settled(&id).unwrap().as_ref()).expect("the command came to its exit");
  assert_eq!((exit.code, exit.stdout.content.as_str()), (Some(0), ""));
  second.engine.wake(&root).unwrap();
  second.settled(&id).unwrap();
  assert_eq!(fs::read_to_string(at.join("count")).unwrap(), "xx", "a second wake runs nothing");
}

#[test]
fn a_wait_that_a_wake_starts_again_says_nothing_after_a_cancel() {
  {
    let mut first = Lived::new("hushed", &[], true).unwrap();
    let root = first.root();
    first.engine.wait(&[], verbs::Wait { seconds: Some(0.2), on: on(&root) }).unwrap();
  }
  let mut second = Lived::new("hushed", &[], false).unwrap();
  let root = second.root();
  second.engine.wake(&root).unwrap();
  second.engine.cancel("wait1").unwrap();
  assert_eq!(second.settled("wait1").unwrap_err().name, "CancelledError");
  // A later wait outlives the deadline of the first.
  let later = second.engine.wait(&[], verbs::Wait { seconds: Some(0.4), on: on(&root) }).unwrap();
  block_on(later).unwrap();
  let facts = second.engine.transcript(verbs::Transcript { on: on(&root), of: None }).unwrap();
  let dones = facts.iter().filter(|one| one.kind() == "done" && one.about() == "wait1");
  assert_eq!(dones.map(|one| one.by().to_owned()).collect::<Vec<_>>(), ["wait1"]);
}

#[test]
fn a_second_life_on_the_record_the_store_kept_makes_the_same_acts_again() {
  let (root, id) = {
    let mut first = Lived::new("again", &["close(len(read('a.txt').lines))"], true).unwrap();
    let root = first.root();
    fs::write(first.at.join("a.txt"), "one\ntwo\n").unwrap();
    let with = verbs::Prompt {
      message: Some("count the lines".to_owned()),
      on: on(&root),
      ..Default::default()
    };
    let act = first.engine.prompt(Object::string("int"), with).unwrap();
    let id = act.id().to_owned();
    assert_eq!(block_on(act).unwrap().as_ref().as_int(), Some(2));
    // The provider asks for the turns while it hears, and they hold the message of the prompt.
    assert_eq!(first.read.borrow().len(), 1);
    assert!(first.read.borrow()[0].contains("count the lines"));
    (root, id)
  };
  let mut second = Lived::new("again", &[], false).unwrap();
  assert!(second.engine.raised().is_none(), "{:?}", second.engine.raised());
  assert_eq!(second.root(), root);
  assert_eq!(
    second.engine.get(&id).unwrap().map(|one| one.kind().to_owned()).as_deref(),
    Some("prompt")
  );
  assert_eq!(second.read.borrow().len(), 0, "a later life asks no model for what the record holds");
  assert_eq!(second.settled(&id).unwrap().as_ref().as_int(), Some(2));
  let turn = second.engine.transcript(verbs::Transcript { on: on(&root), of: None }).unwrap();
  let turn = turn.iter().find(|one| one.kind() == "done" && one.about() == "reply1");
  let turn = turn.and_then(|one| one.word(0)).map(|one| one.type_name().to_owned());
  assert_eq!(
    turn.as_deref(),
    Some("tuple"),
    "the record gives back the tuple that the provider said"
  );
}

#[test]
fn the_work_an_earlier_life_left_is_pending_until_a_wake_that_this_life_says() {
  let root = {
    let mut first = Lived::new("pending", &[], true).unwrap();
    let root = first.root();
    let word = Some("await wait(600)".to_owned());
    first.engine.rung(verbs::Rung { word, on: on(&root), ..Default::default() }).unwrap();
    let apart = first.engine.chain(verbs::Chain::default()).unwrap().id().to_owned();
    first.engine.pause(&apart).unwrap();
    first.engine.wait(&[], verbs::Wait { seconds: Some(600.0), on: on(&apart) }).unwrap();
    assert!(
      first.engine.pending().unwrap().is_empty(),
      "the work of a life is no work of an earlier one"
    );
    root
  };
  let mut second = Lived::new("pending", &[], false).unwrap();
  second.engine.wait(&[], verbs::Wait { seconds: Some(600.0), on: on(&root) }).unwrap();
  // The wait of the chain that a pause holds waits for the wake of that chain, and the wait of this life is its own.
  let pending =
    second.engine.pending().unwrap().into_iter().map(|(id, kind)| format!("{id} {kind}"));
  assert_eq!(pending.collect::<Vec<_>>(), ["rung1 rung", "wait1 wait"]);
  second.engine.wake(&root).unwrap();
  assert!(second.engine.pending().unwrap().is_empty(), "the wake put that work to the World again");
  drop(second);
  // A life after the one whose wake ran that work again finds it pending again, though the journal says the started
  // of that second run again.
  let mut third = Lived::new("pending", &[], false).unwrap();
  let pending =
    third.engine.pending().unwrap().into_iter().map(|(id, kind)| format!("{id} {kind}"));
  assert_eq!(pending.collect::<Vec<_>>(), ["rung1 rung", "wait1 wait", "wait3 wait"]);
}

#[test]
fn an_act_that_lives_end_while_it_runs_again_and_again_is_parked_until_a_retry() {
  let root = {
    let mut first = Lived::new("parked", &[], true).unwrap();
    let root = first.root();
    first.engine.bash("sleep 30", verbs::Bash { on: on(&root), ..Default::default() }).unwrap();
    root
  };
  let started = |lived: &mut Lived| {
    let facts = lived.engine.transcript(verbs::Transcript { on: on(&root), of: None }).unwrap();
    let ours =
      |one: &&Fact| one.kind() == "started" && one.about() == "bash1" && one.by() != "journal";
    facts.iter().filter(ours).count()
  };
  let bash = || vec![("bash1".to_owned(), "bash".to_owned())];
  // Each life that a host wakes as it opens starts the command again, and ends while it runs.
  for _ in 1..CRASHES {
    let mut again = Lived::new("parked", &[], false).unwrap();
    assert!(again.engine.parked().unwrap().is_empty());
    assert_eq!(again.engine.pending().unwrap(), bash());
    again.engine.wake(&root).unwrap();
    assert_eq!(started(&mut again), 1);
  }
  let mut parked = Lived::new("parked", &[], false).unwrap();
  assert_eq!(parked.engine.parked().unwrap(), bash());
  assert!(parked.engine.pending().unwrap().is_empty(), "a parked act is pending no more");
  parked.engine.wake(&root).unwrap();
  assert_eq!(started(&mut parked), 0, "a wake puts no parked act to the outside");
  let word = Some("x = 1".to_owned());
  let rung = parked.engine.rung(verbs::Rung { word, on: on(&root), ..Default::default() }).unwrap();
  let rung = rung.id().to_owned();
  assert!(parked.settled(&rung).is_ok(), "the ears behind the park hear as before");
  parked.engine.retry("bash1").unwrap();
  assert_eq!(started(&mut parked), 1, "a retry puts it to the outside after the wake");
  assert!(parked.engine.parked().unwrap().is_empty());
  assert_eq!(parked.engine.retry("bash1").unwrap_err().name, "Refused");
  drop(parked);
  // A retry before any wake puts the act to the outside as well.
  let mut retried = Lived::new("parked", &[], false).unwrap();
  assert_eq!(retried.engine.parked().unwrap(), bash());
  retried.engine.retry("bash1").unwrap();
  assert_eq!(started(&mut retried), 1);
  retried.engine.cancel("bash1").unwrap();
  assert_eq!(retried.settled("bash1").unwrap_err().name, "CancelledError");
}

#[test]
fn a_key_of_the_client_makes_its_act_once_across_a_crash_and_a_retry() {
  let word = || vec![("word", Object::string("close(1)"))];
  let (root, id) = {
    let mut first = Lived::new("keyed", &[], true).unwrap();
    let root = first.root();
    let id = first.engine.keyed("k1", "rung", vec![], word()).unwrap();
    assert_eq!(
      first.engine.keyed("k1", "rung", vec![], word()).unwrap(),
      id,
      "a retry in the same life"
    );
    assert_ne!(
      first.engine.keyed("k2", "rung", vec![], word()).unwrap(),
      id,
      "another key makes another act"
    );
    // A host that crashed after it kept the key of k3 and before it kept the act.
    first.engine.say("key", &root, vec![Object::string("k3"), Object::string("rung")]).unwrap();
    (root, id)
  };
  let mut second = Lived::new("keyed", &[], false).unwrap();
  assert!(second.engine.raised().is_none(), "{:?}", second.engine.raised());
  assert_eq!(
    second.engine.keyed("k1", "rung", vec![], word()).unwrap(),
    id,
    "a retry in a later life"
  );
  let made = second.engine.keyed("k3", "rung", vec![], word()).unwrap();
  assert_eq!(made, "rung3", "the key that the crash left with no act makes its act");
  drop(second);
  let mut third = Lived::new("keyed", &[], false).unwrap();
  assert_eq!(third.engine.keyed("k3", "rung", vec![], word()).unwrap(), made);
  let facts = third.engine.transcript(verbs::Transcript { on: on(&root), of: None }).unwrap();
  let rungs = facts.iter().filter(|one| one.kind() == "rung" && one.by() == "operator").count();
  assert_eq!(rungs, 3);
}

/// An ear that takes a wait and ends it by its work, of its own accord, once its gate opens: it tells a pause of the
/// chain of the wait, says the wait due, then says it done.
fn pauser(gate: oneshot::Receiver<()>) -> Box<dyn Ear> {
  ear(move |mut co: Co<(String, String)>| async move {
    let mut gate = Some(gate);
    loop {
      match co.next().await {
        Next::Heard(a) => {
          if a.kind() == "wait"
            && a.question()
            && let Some(opens) = gate.take()
          {
            say(&mut co, Fact::says("started", a.about(), []));
            let (about, on) = (a.about().to_owned(), a.on().to_owned());
            co.work(stream::once(opens.map(|_| (about, on))));
          }
        }
        Next::Worked((about, on)) => {
          let _ = co.call("pause", vec![Object::string(on)], vec![]).await;
          say(&mut co, Fact::says("due", &about, [Object::float(1.0)]));
          say(&mut co, done(&about, Object::none()));
        }
      }
    }
  })
}

/// A life on the provider of the test and on the ear that pauses, and the gate of the work of that ear.
fn paused(yard: &str) -> (Engine, oneshot::Sender<()>) {
  let at = std::env::temp_dir().join(format!("furb-engine-{yard}-{}", std::process::id()));
  let (gate, opens) = oneshot::channel();
  let ears = [("provider", provider(at, Rc::default(), Rc::default())), ("pauser", pauser(opens))];
  (Engine::boot(Vec::<Object>::new(), ears).unwrap(), gate)
}

#[test]
fn the_work_of_an_ear_tells_a_verb_in_its_turn_and_under_its_name() {
  let (mut engine, gate) = paused("uttered");
  let root = engine.root().to_owned();
  let act = engine.wait(&[], verbs::Wait { seconds: Some(9.0), on: on(&root) }).unwrap();
  let id = act.id().to_owned();
  let _ = gate.send(());
  block_on(act).unwrap();
  let facts = engine.transcript(verbs::Transcript { on: on(&root), of: None }).unwrap();
  let said: Vec<(String, String)> = facts
    .iter()
    .filter(|one| one.by() == "pauser")
    .map(|one| (one.kind().to_owned(), one.about().to_owned()))
    .collect();
  let expected = [("started", &id), ("tell", &root), ("pause", &root), ("due", &id), ("done", &id)];
  assert_eq!(said, expected.map(|(kind, about)| (kind.to_owned(), about.clone())));
}

#[test]
fn what_the_ears_say_of_their_own_accord_in_one_drive_is_one_feed_of_the_sandbox() {
  let (mut engine, gate) = paused("drained");
  let root = engine.root().to_owned();
  let id =
    engine.wait(&[], verbs::Wait { seconds: Some(9.0), on: on(&root) }).unwrap().id().to_owned();
  let told = Rc::new(RefCell::new(Vec::new()));
  let held = Rc::clone(&told);
  engine.watch(&id, move |value| held.borrow_mut().push(value.py_repr())).unwrap();
  let before = engine.sand.fed();
  engine.pump(Waker::noop()).unwrap();
  assert_eq!(engine.sand.fed() - before, 0, "a drive where no ear says anything feeds nothing");
  let _ = gate.send(());
  engine.pump(Waker::noop()).unwrap();
  assert_eq!(engine.sand.fed() - before, 1, "one drive of three things said is one feed");
  assert_eq!(*told.borrow(), ["None"], "whoever watches the wait is told in that feed");
}

/// An ear that takes each wait and ends it by a work that is ready at once, which says the wait done.
fn quick() -> Box<dyn Ear> {
  ear(move |mut co: Co<String>| async move {
    loop {
      match co.next().await {
        Next::Heard(a) => {
          if a.kind() == "wait" && a.question() {
            say(&mut co, Fact::says("started", a.about(), []));
            co.work(stream::once(std::future::ready(a.about().to_owned())));
          }
        }
        Next::Worked(about) => {
          say(&mut co, done(&about, Object::none()));
        }
      }
    }
  })
}

#[test]
fn what_the_work_of_an_ear_came_to_lands_after_the_run_that_began_it() {
  let at = std::env::temp_dir().join(format!("furb-engine-landed-{}", std::process::id()));
  let ears = [("provider", provider(at, Rc::default(), Rc::default())), ("quick", quick())];
  let mut engine = Engine::boot(Vec::<Object>::new(), ears).unwrap();
  let root = engine.root().to_owned();
  let word = Some("w = wait(9)".to_owned());
  engine.rung(verbs::Rung { word, on: on(&root), ..Default::default() }).unwrap();
  block_on(Act::<Object>::of(&mut engine, "wait1")).unwrap();
  let facts = engine.transcript(verbs::Transcript { on: on(&root), of: None }).unwrap();
  let at = |kind: &str, about: &str| {
    facts.iter().position(|one| one.kind() == kind && one.about() == about).expect("the fact")
  };
  let (asked, done) = (at("wait", "wait1"), at("done", "wait1"));
  let ran =
    facts[asked..done].iter().any(|one| one.kind() == "done" && one.about().starts_with("run"));
  assert!(ran, "the run that said the wait was over before its done came");
}

#[test]
fn the_work_of_an_ear_speaks_when_the_life_drives_it_and_never_while_the_ear_hears() {
  // Two waits end in one drive. The first wakes a word that waits on a rung that waits on it, the second a word
  // that waits on it alone, and each word then makes a wait. What the second wait came to lands after all that the
  // first one caused, as a later life says it again, so the later life makes each act under the name it had.
  let at = std::env::temp_dir().join(format!("furb-engine-alone-{}", std::process::id()));
  let _ = fs::remove_dir_all(&at);
  fs::create_dir_all(&at).expect("a yard of the test");
  let life = |at: &PathBuf| {
    let (record, store) = world::store(at.join("record.jsonl")).unwrap();
    let ears = [
      ("provider", provider(at.clone(), Rc::default(), Rc::default())),
      ("quick", quick()),
      ("store", store),
    ];
    Engine::boot(record, ears).unwrap()
  };
  let makers = |engine: &mut Engine| {
    ["wait3", "wait4"].map(|id| engine.get(id).unwrap().map(|one| one.by().to_owned()))
  };
  let first = {
    let mut engine = life(&at);
    let root = engine.root().to_owned();
    let words = ["deep = rung(\"await wait(9)\")\nawait deep\nwait(1)", "await wait(9)\nwait(2)"];
    let mut ids = Vec::new();
    for word in words {
      let word = Some(word.to_owned());
      ids.push(
        engine
          .rung(verbs::Rung { word, on: on(&root), ..Default::default() })
          .unwrap()
          .id()
          .to_owned(),
      );
    }
    for id in ids {
      block_on(Act::<Object>::of(&mut engine, &id)).unwrap();
    }
    makers(&mut engine)
  };
  let mut second = life(&at);
  assert!(second.raised().is_none(), "{:?}", second.raised());
  assert_eq!(makers(&mut second), first);
  assert_eq!(first, [Some("rung1".to_owned()), Some("rung3".to_owned())]);
}

/// An ear that takes each wait that the ear `caller` made, and ends it by a work that is ready at once.
fn early() -> Box<dyn Ear> {
  ear(move |mut co: Co<String>| async move {
    loop {
      match co.next().await {
        Next::Heard(a) => {
          if a.kind() == "wait" && a.question() && a.by() == "caller" {
            say(&mut co, Fact::says("started", a.about(), []));
            co.work(stream::once(std::future::ready(a.about().to_owned())));
          }
        }
        Next::Worked(about) => say(&mut co, done(&about, Object::none())),
      }
    }
  })
}

/// An ear that takes each wait of the operator, and ends it by a work that is ready at once, which makes a wait of
/// its own on the same chain first.
fn caller() -> Box<dyn Ear> {
  ear(move |mut co: Co<(String, String)>| async move {
    loop {
      match co.next().await {
        Next::Heard(a) => {
          if a.kind() == "wait" && a.question() && a.by() == "operator" {
            say(&mut co, Fact::says("started", a.about(), []));
            co.work(stream::once(std::future::ready((a.about().to_owned(), a.on().to_owned()))));
          }
        }
        Next::Worked((about, on)) => {
          let _ = co.call("wait", vec![Object::float(2.0)], vec![("on", Object::string(on))]).await;
          say(&mut co, done(&about, Object::none()));
        }
      }
    }
  })
}

/// An ear that says it saw the done of each wait, as it hears it.
fn seer() -> Box<dyn Ear> {
  ear(move |mut co| async move {
    loop {
      let a = hear(&mut co).await;
      if a.kind() == "done" && a.about().starts_with("wait") {
        say(&mut co, Fact::says("seen", a.about(), []));
      }
    }
  })
}

#[test]
fn the_work_of_an_ear_takes_what_its_verb_came_to_when_the_life_drives_it_and_never_while_the_ear_hears()
 {
  // The work of caller makes wait2, which early ends while the work of caller has not taken what its verb gave.
  // The work of caller goes on at the next drive, so the done of wait1 lands after all that the done of wait2 made.
  let at = std::env::temp_dir().join(format!("furb-engine-answered-{}", std::process::id()));
  let ears = [
    ("provider", provider(at, Rc::default(), Rc::default())),
    ("early", early()),
    ("caller", caller()),
    ("seer", seer()),
  ];
  let mut engine = Engine::boot(Vec::<Object>::new(), ears).unwrap();
  let root = engine.root().to_owned();
  let act = engine.wait(&[], verbs::Wait { seconds: Some(1.0), on: on(&root) }).unwrap();
  block_on(act).unwrap();
  let facts = engine.transcript(verbs::Transcript { on: on(&root), of: None }).unwrap();
  let at = |kind: &str, about: &str| {
    facts.iter().position(|one| one.kind() == kind && one.about() == about).expect("the fact")
  };
  assert!(at("seen", "wait2") < at("done", "wait1"), "the done of wait1 came while caller heard");
}

/// A waker that counts how often it is woken.
struct Counted(std::sync::atomic::AtomicUsize);

impl Wake for Counted {
  fn wake(self: Arc<Self>) {
    self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
  }
}

#[test]
fn a_drive_that_finds_nothing_to_do_wakes_nobody() {
  let mut lived = Lived::new("idle", &[], true).unwrap();
  let counted = Arc::new(Counted(std::sync::atomic::AtomicUsize::new(0)));
  let waker = Waker::from(Arc::clone(&counted));
  lived.engine.pump(&waker).unwrap();
  let woken = counted.0.load(std::sync::atomic::Ordering::SeqCst);
  lived.engine.pump(&waker).unwrap();
  lived.engine.pump(&waker).unwrap();
  assert_eq!(
    counted.0.load(std::sync::atomic::Ordering::SeqCst),
    woken,
    "a drive with nothing to do wakes nobody"
  );
}

/// An ear that keeps the kind and the subject of every fact it hears.
fn spy(heard: Rc<RefCell<Vec<(String, String)>>>) -> Box<dyn Ear> {
  ear(move |mut co| async move {
    loop {
      let a = hear(&mut co).await;
      heard.borrow_mut().push((a.kind().to_owned(), a.about().to_owned()));
    }
  })
}

#[test]
fn an_ear_of_the_host_hears_on_after_a_word_that_failed_where_it_reached_that_ear() {
  // The sandbox cannot step an ear of the host from a generator that map() walks, so the word fails at the read,
  // where the act reaches the first ear of the host that the engine drives.
  let word =
    "def h(p):\n  return read(p).content\nclose(len(list(map(str, (h(p) for p in ['a.txt'])))))";
  let mut lived = Lived::new("reached", &[word, "close(8)"], true).unwrap();
  fs::write(lived.at.join("a.txt"), "one\ntwo\n").unwrap();
  let root = lived.root();
  let heard = Rc::new(RefCell::new(Vec::new()));
  lived.engine.drive(spy(Rc::clone(&heard)), "spy").unwrap();
  let with = verbs::Prompt { on: on(&root), ..Default::default() };
  let act = lived.engine.prompt(Object::string("int"), with).unwrap();
  let id = act.id().to_owned();
  assert_eq!(block_on(act).unwrap().as_ref().as_int(), Some(8));
  let facts = lived.engine.transcript(verbs::Transcript { on: on(&root), of: None }).unwrap();
  let raised = facts.iter().find(|one| one.kind() == "done" && one.about() == "rung1");
  let raised = raised.and_then(|one| one.word(0)).and_then(|one| Fault::of(one));
  assert_eq!(raised.map(|one| one.name).as_deref(), Some("NotImplementedError"));
  let heard = heard.borrow();
  assert!(heard.contains(&("read".to_owned(), "read1".to_owned())), "the spy heard the read");
  assert!(heard.contains(&("done".to_owned(), id)), "the spy heard the done of the prompt");
}

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
