//! One life of the real engine, driven by the crate from end to end.
//!
//! Every other test of the crate holds one piece against a double. This holds the whole of it against the engine
//! itself: a life is opened, the life plays the builtin extensions, an operator says verbs, a model answers a
//! prompt, the Kernel gates the word it wrote and runs it, a command runs on this machine and speaks through the
//! Voice from its own thread, a wait ends, the operator answers a prompt, and the record the World kept opens a
//! second life.
//!
//! The one double is [`Yard`], a World of this machine small enough to read: it does what the builtins ask of a
//! World, a read and a write of a file and a command, through the facts it hears.

use std::{
  cell::RefCell,
  collections::HashMap,
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
  Actor, Fact, Fault, Later, Life, Object, ObjectRef, Standing, Voice, World,
  ear::Reply,
  extension,
  value::{entry, field},
};

/// What the yard waits for the value of, after it said a verb.
enum Pending {
  /// A read of a path, which waits for the directory of its chain.
  Read { qid: String, path: String },
  /// A write of a content at a path, which waits for the directory of its chain.
  Write { qid: String, path: String, content: String },
  /// A command to start, which waits for the directory of its chain.
  Start { about: String, command: String },
  /// A control, and the commands it may be over, which waits for whether it is over the first of them.
  Covers { fact: Object, left: Vec<String> },
}

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
  /// The command line of every command the life made, and the chain it is on, by its act.
  commands: HashMap<String, (String, String)>,
  /// The commands that run.
  running: Vec<String>,
  pending: Option<Pending>,
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
      commands: HashMap::new(),
      running: Vec::new(),
      pending: None,
    }
  }

  /// The verb that asks the chain of a question where its paths resolve.
  fn cwd(on: &str) -> Reply {
    Reply::Calls {
      name: "cwd".to_owned(),
      args: vec![],
      kwargs: vec![("on".to_owned(), Object::string(on))],
    }
  }

  /// The verb that asks whether a control is over a command.
  fn covers(fact: &Object, id: &str) -> Reply {
    Reply::Calls {
      name: "covers".to_owned(),
      args: vec![fact.clone(), Object::string(id)],
      kwargs: vec![],
    }
  }

  /// The next command a control may be over, asked, or nothing when none is left.
  fn next(&mut self, fact: Object, mut left: Vec<String>) -> Reply {
    if left.is_empty() {
      return Reply::Nothing;
    }
    let first = left[0].clone();
    let reply = Yard::covers(&fact, &first);
    left.remove(0);
    self.pending = Some(Pending::Covers { fact, left: [vec![first], left].concat() });
    reply
  }
}

/// The plain data a read or a write of a file is answered with: its path and its content.
fn plain(path: &str, content: &str) -> Object {
  Object::dict([
    (Object::string("path"), Object::string(path)),
    (Object::string("content"), Object::string(content)),
  ])
}

/// A command of this machine run on a thread of its own, which speaks its streams and its exit through the Voice.
fn spawned(voice: Voice, about: String, command: String, here: String) {
  thread::spawn(move || {
    let got = std::process::Command::new("sh").arg("-c").arg(&command).current_dir(&here).output();
    match got {
      Ok(out) => {
        let said = |text: &[u8], stream: &str| {
          Fact::says(
            "out",
            &about,
            [Object::string(String::from_utf8_lossy(text)), Object::string(stream)],
          )
        };
        voice.say(said(&out.stdout, "stdout"));
        if !out.stderr.is_empty() {
          voice.say(said(&out.stderr, "stderr"));
        }
        let code = out.status.code().map_or_else(Object::none, |code| Object::int(i64::from(code)));
        voice.say(Fact::says("exited", &about, [code]));
      }
      Err(_) => voice.say(Fact::says("exited", &about, [Object::none()])),
    }
  });
}

impl World for Yard {
  fn opened(&mut self, voice: Voice) {
    self.voice = Some(voice);
  }

  fn kinds(&self) -> Vec<String> {
    vec!["bash".to_owned()]
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

  fn hears(&mut self, fact: &Fact) -> Reply {
    let word =
      |at: usize| fact.word(at).and_then(|one| one.as_str()).unwrap_or_default().to_owned();
    match fact.kind() {
      "bash" if fact.question() => {
        self.commands.insert(fact.about().to_owned(), (word(1), fact.on().to_owned()));
        Reply::Nothing
      }
      "read" if fact.question() => {
        self.pending = Some(Pending::Read { qid: fact.about().to_owned(), path: word(1) });
        Yard::cwd(fact.on())
      }
      "write" if fact.question() => {
        self.pending =
          Some(Pending::Write { qid: fact.about().to_owned(), path: word(1), content: word(2) });
        Yard::cwd(fact.on())
      }
      "start" if self.commands.contains_key(fact.about()) => {
        let about = fact.about().to_owned();
        let (command, on) = self.commands[&about].clone();
        self.running.push(about.clone());
        self.pending = Some(Pending::Start { about, command });
        Yard::cwd(&on)
      }
      "cancel" | "close" => {
        let left = self.running.clone();
        self.next(fact.0.clone(), left)
      }
      "exited" => {
        self.running.retain(|one| one != fact.about());
        Reply::Nothing
      }
      _ => Reply::Nothing,
    }
  }

  fn answered(&mut self, got: ObjectRef<'_>) -> Reply {
    let value = entry(&got, 1).map(|one| one.to_owned()).unwrap_or_else(Object::none);
    let text = value.as_ref().as_str().unwrap_or_default().to_owned();
    match self.pending.take() {
      Some(Pending::Read { qid, path }) => {
        let at = PathBuf::from(&text).join(&path);
        let answer = fs::read_to_string(&at).map_or_else(
          |_| Fault::refused(format!("no file at {}", at.display())).object(),
          |content| plain(&at.display().to_string(), &content),
        );
        Reply::Say(Fact::says("done", &qid, [answer]))
      }
      Some(Pending::Write { qid, path, content }) => {
        let at = PathBuf::from(&text).join(&path);
        let answer = fs::write(&at, &content).map_or_else(
          |no| Fault::refused(no.to_string()).object(),
          |()| plain(&at.display().to_string(), &content),
        );
        Reply::Say(Fact::says("done", &qid, [answer]))
      }
      Some(Pending::Start { about, command }) => {
        let voice = self.voice.clone().expect("a yard is opened before a command runs");
        spawned(voice, about, command, text);
        Reply::Nothing
      }
      Some(Pending::Covers { fact, left }) => {
        let over = value.as_ref().as_bool().unwrap_or_default();
        let mut left = left;
        let first = left.remove(0);
        if over {
          self.running.retain(|one| one != &first);
          self.slain.borrow_mut().push(first);
        }
        self.next(fact, left)
      }
      None => Reply::Nothing,
    }
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

/// The words of the builtins, which a life of the tests plays.
fn builtin() -> Vec<String> {
  extension::words(&extension::builtins())
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
    Lived::playing(yard, words, record, builtin(), vec![])
  }

  fn playing(
    yard: &str,
    words: &[&str],
    record: Vec<Object>,
    played: Vec<String>,
    lives: Vec<String>,
  ) -> Result<Self, Fault> {
    let at = std::env::temp_dir().join(format!("furb-life-{yard}"));
    if record.is_empty() {
      let _ = fs::remove_dir_all(&at);
    }
    fs::create_dir_all(&at).expect("a yard of the test");
    let world = Yard::new(at.clone(), words);
    let (read, kept, slain) =
      (Rc::clone(&world.read), Rc::clone(&world.kept), Rc::clone(&world.slain));
    let life = Life::open(world).words(played).lives(lives).boot(record)?;
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

  /// One verb said on a chain by the operator, with these words.
  fn said(&mut self, name: &str, args: Vec<Object>, on: &str) -> Result<Object, Fault> {
    self.life.verb(name, args, vec![("on", Object::string(on))])
  }

  /// The words of the program of a chain, in order.
  fn program(&mut self, on: &str) -> Vec<String> {
    let got =
      self.life.verb("ask", vec![Object::string("program"), Object::string(on)], vec![]).unwrap();
    let got = got.as_ref();
    let held = entry(&got, 1).and_then(|one| one.pairs()).unwrap_or_default();
    held.into_iter().map(|(_, word)| word.as_str().unwrap_or_default().to_owned()).collect()
  }

  /// Every rung on a chain, as who made it and the rung it retells.
  fn rungs(&mut self, on: &str) -> Vec<(String, String)> {
    let names = self.life.held("acts", vec![], "keys").unwrap();
    let names: Vec<String> = names
      .as_ref()
      .items()
      .unwrap_or_default()
      .into_iter()
      .filter_map(|one| one.as_str().map(str::to_owned))
      .filter(|one| one.starts_with("rung"))
      .collect();
    let mut out = Vec::new();
    for name in names {
      let fact = self.life.get(&name).unwrap();
      if fact.on() == on {
        out.push((
          fact.by().to_owned(),
          fact.word(2).and_then(|one| one.as_str()).unwrap_or_default().to_owned(),
        ));
      }
    }
    out
  }
}

/// One field of an instance a word defined, as the stand-in sends it out: its fields under `value`.
fn of<'a>(got: &ObjectRef<'a>, name: &str) -> Option<ObjectRef<'a>> {
  field(&field(got, "value")?, name)
}

#[test]
fn a_life_of_the_real_engine_opens_on_its_root_and_answers_what_the_root_stands_on() {
  let mut lived = Lived::new("opens", &[], vec![]).unwrap();
  assert_eq!(lived.root(), "chain1");
  assert!(lived.life.raised().is_none(), "{:?}", lived.life.raised());
  let root = lived.root();
  let cwd = lived.said("cwd", vec![], &root).unwrap();
  assert_eq!(cwd.as_ref().as_str(), Some(lived.at.display().to_string().as_str()));
}

#[test]
fn the_world_answers_a_question_of_an_extension_through_a_verb_it_says() {
  let mut lived = Lived::new("reads", &[], vec![]).unwrap();
  let root = lived.root();
  block_on(lived.life.rung("write(Text('a.txt', 'one\\ntwo\\n'))", "", "", &root).unwrap())
    .unwrap();
  assert_eq!(fs::read_to_string(lived.at.join("a.txt")).unwrap(), "one\ntwo\n");
  let got = lived.said("read", vec![Object::string("a.txt")], &root).unwrap();
  let got = got.as_ref();
  assert_eq!(of(&got, "content").and_then(|one| one.as_str()), Some("one\ntwo\n"));
  let no = lived.said("read", vec![Object::string("none.txt")], &root).unwrap_err();
  assert_eq!(no.name, "Refused");
  assert!(no.message().contains("no file at"), "{no}");
}

#[test]
fn a_model_answers_a_prompt_and_the_kernel_gates_and_runs_the_word_it_wrote() {
  let mut lived = Lived::new("prompts", &["close(len(read('a.txt').lines))"], vec![]).unwrap();
  let root = lived.root();
  fs::write(lived.at.join("a.txt"), "one\ntwo\nthree\n").unwrap();
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
  let act = lived.said("bash", vec![Object::string("echo hi")], &root).unwrap();
  let id = act.as_ref().as_str().unwrap().to_owned();
  let exit = block_on(lived.life.awaited(&id)).unwrap();
  let exit = exit.as_ref();
  assert_eq!(of(&exit, "code").and_then(|one| one.as_int()), Some(0));
  let stdout = of(&exit, "stdout").unwrap();
  assert_eq!(of(&stdout, "content").and_then(|one| one.as_str()), Some("hi\n"));
  let stderr = of(&exit, "stderr").unwrap();
  assert_eq!(of(&stderr, "content").and_then(|one| one.as_str()), Some(""));
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
  let exit = lived.settled("bash2");
  let exit = exit.as_ref();
  let stdout = of(&exit, "stdout").unwrap();
  assert_eq!(of(&exit, "code").and_then(|one| one.as_int()), Some(0));
  assert_eq!(of(&stdout, "content").and_then(|one| one.as_str()), Some("late\n"));
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
fn a_start_the_world_does_not_do_is_closed_with_its_refusal() {
  let mut lived = Lived::new("unknown", &[], vec![]).unwrap();
  let root = lived.root();
  let got = lived
    .life
    .word("act('job', __on, started(ending(idle)))", vec![("__on", Object::string(&root))])
    .unwrap();
  let id = got.as_ref().as_str().unwrap().to_owned();
  let no = block_on(lived.life.awaited(&id)).unwrap_err();
  assert_eq!((no.name.as_str(), no.message()), ("Refused", "the World does no job".to_owned()));
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
  let no = lived.life.get("wait9").unwrap_err();
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
  let command = first.said("bash", vec![Object::string("echo again")], &root).unwrap();
  let command = command.as_ref().as_str().unwrap().to_owned();
  let exit = block_on(first.life.awaited(&command)).unwrap();
  let stdout = of(&exit.as_ref(), "stdout").map(|one| one.to_owned()).unwrap();
  assert_eq!(of(&stdout.as_ref(), "content").and_then(|one| one.as_str()), Some("again\n"));
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
fn the_life_plays_the_words_of_the_extensions_on_the_root_as_the_world() {
  let mut lived = Lived::new("plays", &[], vec![]).unwrap();
  let root = lived.root();
  assert_eq!(lived.program(&root), builtin());
  assert_eq!(lived.rungs(&root), vec![("world".to_owned(), String::new()); 3]);
  assert_eq!(lived.life.get("rung1").unwrap().by(), "world");
}

#[test]
fn the_life_plays_the_words_on_a_chain_born_without_a_source() {
  let mut lived = Lived::new("births", &[], vec![]).unwrap();
  let two = block_on(async { lived.life.chain("two", "", None, "").unwrap().id().to_owned() });
  assert_eq!(lived.program(&two), builtin());
  assert!(lived.rungs(&two).iter().all(|(by, retells)| by == "world" && retells.is_empty()));
}

#[test]
fn a_chain_with_a_source_is_played_no_word_of_its_own() {
  let mut lived = Lived::new("sources", &[], vec![]).unwrap();
  let root = lived.root();
  let twin = lived.life.chain("twin", &root, None, "").unwrap().id().to_owned();
  block_on(lived.life.drive()).unwrap();
  assert_eq!(lived.program(&twin), builtin());
  let rungs = lived.rungs(&twin);
  assert_eq!(rungs.len(), 3);
  assert!(
    rungs.iter().all(|(by, retells)| by == &twin && retells.starts_with("rung")),
    "{rungs:?}"
  );
}

#[test]
fn a_later_life_plays_only_the_words_its_record_lacks() {
  let first = Lived::new("later", &[], vec![]).unwrap();
  let root = first.root();
  let kept = first.kept.borrow().clone();
  let mut second = Lived::new("later", &[], kept.clone()).unwrap();
  assert_eq!(second.rungs(&root).len(), 3);
  let mut more = builtin();
  more.push("extra = 1".to_owned());
  let mut third = Lived::playing("later", &[], kept, more.clone(), vec![]).unwrap();
  assert_eq!(third.program(&root), more);
  assert_eq!(third.rungs(&root).len(), 4);
}

#[test]
fn a_life_opened_with_no_words_plays_nothing() {
  let mut lived = Lived::playing("nothing", &[], vec![], vec![], vec![]).unwrap();
  let root = lived.root();
  assert_eq!(lived.program(&root), Vec::<String>::new());
  assert_eq!(lived.rungs(&root), vec![]);
}

#[test]
fn the_life_plays_the_life_words_in_every_life_after_the_words() {
  let lives = vec!["seen = 1".to_owned()];
  let first = Lived::playing("lives", &[], vec![], builtin(), lives.clone()).unwrap();
  let root = first.root();
  let kept = first.kept.borrow().clone();
  let mut first = first;
  assert_eq!(first.program(&root)[3..], ["seen = 1"]);
  let mut second = Lived::playing("lives", &[], kept, builtin(), lives).unwrap();
  assert_eq!(second.program(&root)[3..], ["seen = 1", "seen = 1"]);
  let two = second.life.chain("two", "", None, "").unwrap().id().to_owned();
  assert_eq!(second.program(&two)[3..], ["seen = 1"]);
}

/// A value of json as the sandbox takes it: plain data as itself.
fn json(value: &serde_json::Value) -> Object {
  match value {
    serde_json::Value::Null => Object::none(),
    serde_json::Value::Bool(one) => Object::bool(*one),
    serde_json::Value::Number(one) => {
      one.as_i64().map_or_else(|| Object::float(one.as_f64().unwrap_or_default()), Object::int)
    }
    serde_json::Value::String(one) => Object::string(one),
    serde_json::Value::Array(held) => Object::list(held.iter().map(json)),
    serde_json::Value::Object(held) => {
      Object::dict(held.iter().map(|(key, one)| (Object::string(key), json(one))))
    }
  }
}

#[test]
fn a_record_of_0_1_0_opens_and_the_life_plays_its_words_after() {
  let text = include_str!("../test/outside/record-0.1.0.jsonl");
  let record: Vec<Object> = text
    .lines()
    .filter(|line| !line.trim().is_empty())
    .map(|line| json(&serde_json::from_str(line).unwrap()))
    .collect();
  let at = std::env::temp_dir().join("furb-life-old");
  let _ = fs::remove_dir_all(&at);
  fs::create_dir_all(&at).unwrap();
  fs::write(at.join("a.txt"), "one\n").unwrap();
  let mut lived = Lived::new("old", &[], record).unwrap();
  assert!(lived.life.raised().is_none(), "{:?}", lived.life.raised());
  let root = lived.root();
  assert_eq!(root, "chain1");
  let got = lived.said("read", vec![Object::string("a.txt")], &root).unwrap();
  assert_eq!(of(&got.as_ref(), "content").and_then(|one| one.as_str()), Some("one\n"));
}
