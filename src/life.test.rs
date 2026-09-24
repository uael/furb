//! One life of the real engine, driven by the crate from end to end.
//!
//! Every other test of the crate holds one piece against a double. This holds the whole of it against the engine
//! itself: a life is opened, an operator says verbs, a model answers a prompt, the Kernel gates the word it wrote
//! and runs it, a command runs on this machine and speaks through the Voice from its own thread, a wait ends, the
//! operator answers a prompt, and the record the World kept opens a second life.
//!
//! The one double is [`Yard`], a World of this machine small enough to read.

use std::{
  cell::RefCell,
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
  Actor, Command, Exit, Fault, Later, Life, Object, ObjectRef, Running, Standing, Text, Voice,
  World,
};

/// A World of this machine, for the tests: a directory, a model that answers by a script, and real commands.
struct Yard {
  at: PathBuf,
  words: Rc<RefCell<Vec<String>>>,
  /// What every ask showed the model, shared with the test.
  read: Rc<RefCell<Vec<String>>>,
  /// The entries it kept, shared with the test, which is the record of the life.
  kept: Rc<RefCell<Vec<Object>>>,
  /// Every command it was told to end before its time, shared with the test.
  slain: Rc<RefCell<Vec<String>>>,
  voice: Option<Voice>,
}

impl Yard {
  fn new(at: PathBuf, words: &[&str]) -> Self {
    Yard {
      at,
      words: Rc::new(RefCell::new(words.iter().map(|one| (*one).to_owned()).collect())),
      read: Rc::default(),
      kept: Rc::default(),
      slain: Rc::default(),
      voice: None,
    }
  }
}

/// A command of the yard, which cannot be fed and ends on its own, and which notes that it was told to end.
struct Ran {
  about: String,
  slain: Rc<RefCell<Vec<String>>>,
}

impl Running for Ran {
  fn feed(&mut self, _text: Option<String>) {}
  fn slay(&mut self) {
    self.slain.borrow_mut().push(self.about.clone());
  }
}

impl World for Yard {
  fn opened(&mut self, voice: Voice) {
    self.voice = Some(voice);
  }

  fn stand(&mut self) -> Standing {
    Standing {
      roster: vec![
        Actor { name: "operator".to_owned(), efforts: vec![], window: 200_000 },
        Actor { name: "m".to_owned(), efforts: vec!["low".to_owned()], window: 200_000 },
      ],
      directory: self.at.display().to_string(),
      actor: "m/low".to_owned(),
    }
  }

  fn read(&mut self, here: &str, path: &str) -> Result<Text, Fault> {
    let at = PathBuf::from(here).join(path);
    fs::read_to_string(&at)
      .map(|content| Text::new(at.display().to_string(), content))
      .map_err(|_| Fault::refused(format!("no file at {}", at.display())))
  }

  fn write(&mut self, here: &str, path: &str, content: &str) -> Result<Text, Fault> {
    let at = PathBuf::from(here).join(path);
    fs::write(&at, content).map_err(|no| Fault::refused(no.to_string()))?;
    Ok(Text::new(at.display().to_string(), content))
  }

  fn clock(&mut self) -> f64 {
    0.5
  }

  fn chance(&mut self) -> f64 {
    0.5
  }

  fn keep(&mut self, entry: ObjectRef<'_>) {
    self.kept.borrow_mut().push(entry.to_owned());
  }

  fn ask(&mut self, _rung: &str, _on: &str, _actor: &str, turns: ObjectRef<'_>) -> Later<Object> {
    self.read.borrow_mut().push(turns.py_repr());
    let word = {
      let mut words = self.words.borrow_mut();
      if words.is_empty() { "close(None)".to_owned() } else { words.remove(0) }
    };
    Box::pin(async move {
      Object::tuple([
        Object::string("assistant"),
        Object::string(word),
        Object::none(),
        Object::list([]),
      ])
    })
  }

  fn run(&mut self, command: Command) -> Box<dyn Running> {
    let voice = self.voice.clone().expect("a yard is opened before a command runs");
    let ran = Ran { about: command.about.clone(), slain: Rc::clone(&self.slain) };
    thread::spawn(move || {
      let got = std::process::Command::new("sh")
        .arg("-c")
        .arg(&command.command)
        .current_dir(&command.here)
        .output();
      match got {
        Ok(out) => {
          voice.out(&command.about, &String::from_utf8_lossy(&out.stdout), "stdout");
          if !out.stderr.is_empty() {
            voice.out(&command.about, &String::from_utf8_lossy(&out.stderr), "stderr");
          }
          voice.exited(&command.about, out.status.code().map(i64::from));
        }
        Err(_) => voice.exited(&command.about, None),
      }
    });
    Box::new(ran)
  }

  fn wait(&mut self, _seconds: f64) -> Later<()> {
    Box::pin(async {})
  }

  fn prompt(&mut self, _about: &str, shape: &str, _message: &str) -> Later<Result<Object, Fault>> {
    let answer = if shape == "int" {
      Ok(Object::int(3))
    } else {
      Err(Fault::refused(format!("the yard answers no {shape}")))
    };
    Box::pin(async move { answer })
  }
}

/// A waker that unparks the thread of the test, so a Voice spoken from another thread wakes the poll.
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

/// One life of the real engine, on a yard of the disk, with the model answering these words.
struct Lived {
  life: Life,
  at: PathBuf,
  read: Rc<RefCell<Vec<String>>>,
  kept: Rc<RefCell<Vec<Object>>>,
  slain: Rc<RefCell<Vec<String>>>,
}

impl Lived {
  fn new(yard: &str, words: &[&str], record: Vec<Object>) -> Result<Self, Fault> {
    let at = std::env::temp_dir().join(format!("furb-life-{yard}"));
    if record.is_empty() {
      let _ = fs::remove_dir_all(&at);
    }
    fs::create_dir_all(&at).expect("a yard of the test");
    let world = Yard::new(at.clone(), words);
    let (read, kept, slain) =
      (Rc::clone(&world.read), Rc::clone(&world.kept), Rc::clone(&world.slain));
    let life = Life::boot(world, record)?;
    Ok(Lived { life, at, read, kept, slain })
  }

  /// A life restored from a dump, on a yard of the same name.
  fn restored(yard: &str, words: &[&str], dump: &[u8]) -> Result<Self, Fault> {
    let at = std::env::temp_dir().join(format!("furb-life-{yard}"));
    let world = Yard::new(at.clone(), words);
    let (read, kept, slain) =
      (Rc::clone(&world.read), Rc::clone(&world.kept), Rc::clone(&world.slain));
    let life = Life::open(world).restore(dump)?;
    Ok(Lived { life, at, read, kept, slain })
  }

  fn root(&self) -> String {
    self.life.root().to_owned()
  }

  /// The life driven until it holds an act of this name.
  fn made(&mut self, id: &str) {
    while !self
      .life
      .held("acts", vec![Object::string(id)], "in")
      .unwrap()
      .as_ref()
      .as_bool()
      .unwrap()
    {
      block_on(self.life.drive()).unwrap();
      thread::sleep(Duration::from_millis(5));
    }
  }

  /// What an act came to, once the World has said it.
  fn settled(&mut self, id: &str) -> Object {
    loop {
      if let Some(got) = self.life.outcome(id).unwrap() {
        return got;
      }
      block_on(self.life.drive()).unwrap();
      thread::sleep(Duration::from_millis(5));
    }
  }
}

#[test]
fn a_life_of_the_real_engine_opens_on_its_root_and_answers_what_the_root_stands_on() {
  let mut lived = Lived::new("opens", &[], vec![]).unwrap();
  assert_eq!(lived.root(), "chain1");
  assert!(lived.life.raised().is_none(), "{:?}", lived.life.raised());
  let cwd = lived.life.cwd(&lived.root()).unwrap();
  assert_eq!(cwd.as_ref().as_str(), Some(lived.at.display().to_string().as_str()));
}

#[test]
fn the_world_serves_a_read_and_a_write_of_the_real_engine() {
  let mut lived = Lived::new("reads", &[], vec![]).unwrap();
  let root = lived.root();
  let got = lived.life.write(&Text::new("a.txt", "one\ntwo\n"), &root).unwrap();
  assert_eq!(Text::of(got.as_ref()).unwrap().content, "one\ntwo\n");
  let got = lived.life.read("a.txt", None, &root).unwrap();
  assert_eq!(Text::of(got.as_ref()).unwrap().content, "one\ntwo\n");
  let no = lived.life.read("none.txt", None, &root).unwrap_err();
  assert_eq!(no.name, "Refused");
  assert!(no.message().contains("no file at"), "{no}");
}

#[test]
fn a_model_answers_a_prompt_and_the_kernel_gates_and_runs_the_word_it_wrote() {
  let mut lived = Lived::new("prompts", &["close(len(read('a.txt').lines))"], vec![]).unwrap();
  let root = lived.root();
  lived.life.write(&Text::new("a.txt", "one\ntwo\nthree\n"), &root).unwrap();
  let act = lived.life.prompt("int", "count the lines", "", &root).unwrap();
  let got = block_on(act).unwrap();
  assert_eq!(got.as_ref().as_int(), Some(3));
  assert_eq!(lived.read.borrow().len(), 1);
  assert!(lived.read.borrow()[0].contains("count the lines"));
}

#[test]
fn the_gate_refuses_a_word_and_the_engine_asks_the_model_again() {
  let mut lived = Lived::new("gates", &["close(nowhere)", "close(7)"], vec![]).unwrap();
  let root = lived.root();
  let got = block_on(lived.life.prompt("int", "count", "", &root).unwrap()).unwrap();
  assert_eq!(got.as_ref().as_int(), Some(7));
  assert_eq!(lived.read.borrow().len(), 2, "the model was asked again after the refusal");
  assert!(lived.read.borrow()[1].contains("refused"), "{}", lived.read.borrow()[1]);
}

#[test]
fn a_word_that_raises_is_asked_again_and_the_model_reads_what_it_raised() {
  let mut lived = Lived::new("raises", &["close(1 / 0)", "close(9)"], vec![]).unwrap();
  let root = lived.root();
  let got = block_on(lived.life.prompt("int", "count", "", &root).unwrap()).unwrap();
  assert_eq!(got.as_ref().as_int(), Some(9));
  assert!(lived.read.borrow()[1].contains("ZeroDivisionError"), "{}", lived.read.borrow()[1]);
}

#[test]
fn a_command_runs_on_this_machine_and_speaks_its_exit_from_its_own_thread() {
  let mut lived = Lived::new("commands", &[], vec![]).unwrap();
  let root = lived.root();
  let act = lived.life.bash("echo hi", false, None, &root).unwrap();
  let exit: Exit = block_on(act).unwrap();
  assert_eq!(exit.code, Some(0));
  assert_eq!(exit.stdout.content, "hi\n");
  assert_eq!(exit.stderr.content, "");
}

#[test]
fn the_world_ends_a_command_at_a_cancel_of_its_prompt_and_never_at_a_close_of_it() {
  let words = ["await bash('sleep 0.5')", "c = bash('sleep 0.2; echo late')\nclose(1)"];
  let mut lived = Lived::new("controls", &words, vec![]).unwrap();
  let root = lived.root();
  let cancelled = lived.life.prompt("int", "go", "", &root).unwrap().id().to_owned();
  lived.made("bash1");
  lived.life.cancel(&cancelled).unwrap();
  assert_eq!(*lived.slain.borrow(), ["bash1"]);
  let got = block_on(lived.life.prompt("int", "go", "", &root).unwrap()).unwrap();
  assert_eq!(got.as_ref().as_int(), Some(1));
  let exit = Exit::of(lived.settled("bash2").as_ref()).expect("the command came to its exit");
  assert_eq!((exit.code, exit.stdout.content.as_str()), (Some(0), "late\n"));
  assert_eq!(*lived.slain.borrow(), ["bash1"]);
}

#[test]
fn a_wait_is_done_when_the_world_says_so() {
  let mut lived = Lived::new("waits", &[], vec![]).unwrap();
  let root = lived.root();
  block_on(lived.life.wait(0.0, &root).unwrap()).unwrap();
}

#[test]
fn a_prompt_of_the_operator_is_closed_with_what_the_operator_answered() {
  let mut lived = Lived::new("operator", &[], vec![]).unwrap();
  let root = lived.root();
  let got = block_on(lived.life.prompt("int", "how many?", "operator", &root).unwrap()).unwrap();
  assert_eq!(got.as_ref().as_int(), Some(3));
  let no = block_on(lived.life.prompt("str", "a name?", "operator", &root).unwrap()).unwrap_err();
  assert_eq!(no.name, "Refused");
}

#[test]
fn a_callable_of_the_host_is_called_back_by_the_sandbox_with_what_the_word_gave_it() {
  let mut lived = Lived::new("callables", &[], vec![]).unwrap();
  let seen = Rc::new(RefCell::new(0usize));
  let held = Rc::clone(&seen);
  let show = lived.life.callable(move |args| {
    let lines = args.first().and_then(|one| one.as_ref().items()).map_or(0, |held| held.len());
    *held.borrow_mut() = lines;
    Ok(Object::list([Object::int(2)]))
  });
  let got = lived.life.word("show(['one', 'two', 'three'])", vec![("show", show)]).unwrap();
  assert_eq!(*seen.borrow(), 3, "the callable was given the lines");
  assert_eq!(got.py_repr(), "[2]", "the word got what the callable gave");
  let bad = lived.life.callable(|_| Err(Fault::refused("not lines")));
  let no = lived.life.word("show(1)", vec![("show", bad)]);
  assert_eq!(
    no.unwrap_err().name,
    "RuntimeError",
    "what the callable raised is raised in the word"
  );
}

#[test]
fn what_the_engine_raised_reaches_the_host_as_the_fault_it_is() {
  let mut lived = Lived::new("faults", &[], vec![]).unwrap();
  let no = lived.life.get("bash9").unwrap_err();
  assert_eq!(no.name, "KeyError");
  let no = lived.life.word("nowhere", vec![]).unwrap_err();
  assert_eq!(no.name, "NameError");
}

#[test]
fn a_second_life_on_the_record_the_world_kept_makes_the_same_acts_again() {
  let mut first = Lived::new("again", &["close(len(read('a.txt').lines))"], vec![]).unwrap();
  let root = first.root();
  // The file is put there by hand: a write of the operator is a query, which takes a number of the operator's,
  // and a later life that says it not would name its acts otherwise.
  fs::write(first.at.join("a.txt"), "one\ntwo\n").unwrap();
  let act = first.life.prompt("int", "count", "", &root).unwrap();
  let id = act.id().to_owned();
  assert_eq!(block_on(act).unwrap().as_ref().as_int(), Some(2));
  let exit: Exit = block_on(first.life.bash("echo again", false, None, &root).unwrap()).unwrap();
  assert_eq!(exit.stdout.content, "again\n");
  let kept = first.kept.borrow().clone();
  assert!(
    kept.len() >= 4,
    "the record holds the chain, the prompt, its rung and its answer: {}",
    kept.len()
  );
  let mut second = Lived::new("again", &[], kept).unwrap();
  assert!(second.life.raised().is_none(), "{:?}", second.life.raised());
  assert_eq!(second.root(), root);
  let again = second.life.get(&id).unwrap();
  assert_eq!(again.kind(), "prompt");
  assert_eq!(second.read.borrow().len(), 0, "a later life asks no model for what the record holds");
  assert_eq!(second.life.outcome(&id).unwrap().and_then(|one| one.as_ref().as_int()), Some(2));
}

#[test]
fn a_life_dumped_where_it_stands_still_is_restored_and_goes_on() {
  let mut first = Lived::new("dumped", &[], vec![]).unwrap();
  let root = first.root();
  fs::write(first.at.join("a.txt"), "one\ntwo\n").unwrap();
  block_on(first.life.rung("k = len(read('a.txt').lines)", "", "", &root).unwrap()).unwrap();
  let id = first.life.bash("echo still", false, None, &root).unwrap().id().to_owned();
  let no = first.life.dump().unwrap_err();
  assert!(no.message().contains("a command runs"), "{no}");
  let exit: Exit = block_on(first.life.awaiting(&id)).unwrap();
  assert_eq!(exit.stdout.content, "still\n");
  let dump = first.life.dump().unwrap();
  let mut second = Lived::restored("dumped", &[], &dump).unwrap();
  assert_eq!(second.root(), root);
  assert!(second.life.raised().is_none());
  let k =
    second.life.held("modules", vec![Object::string(&root), Object::string("k")], "at").unwrap();
  assert_eq!(k.as_ref().as_int(), Some(2));
  assert_eq!(second.read.borrow().len(), 0, "a restored life replays nothing and asks no model");
  let got = block_on(second.life.rung("close(k + 1)", "", "", &root).unwrap()).unwrap();
  assert_eq!(got.as_ref().as_int(), Some(3));
}
