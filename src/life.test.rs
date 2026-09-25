//! One life of the real engine, driven by the crate from end to end.
//!
//! Every other test of the crate holds one piece against a double. This holds the whole of it against the engine
//! itself: a life is opened, an operator says verbs, a model answers a prompt, the Kernel gates the word it wrote
//! and runs it, a command runs on this machine and speaks through the Voice of its ear from its own thread, a wait
//! ends, the operator answers a prompt, and the record the ear kept opens a second life.
//!
//! The one double is [`Yard`], the World of this machine as one ear, small enough to read.

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

use crate::{Ears, Exit, Fact, Fault, Life, Object, ObjectRef, Reply, Text, Voice, value::entry};

/// What the yard does with the answer of a verb it said: given the fact it heard, the words of the verb, and the
/// answer.
type Then = fn(&mut Yard, &Fact, &[Object], ObjectRef<'_>);

/// One verb the yard says while it hears a fact, and what it does with the answer.
struct Step {
  name: &'static str,
  args: Vec<Object>,
  kwargs: Vec<(String, Object)>,
  then: Then,
}

/// One fact said by say, whose answer the yard does not read, and then what it does once it is said.
fn say(kind: &str, about: &str, words: impl IntoIterator<Item = Object>, then: Then) -> Step {
  let mut args = vec![Object::string(kind), Object::string(about)];
  args.extend(words);
  Step { name: "say", args, kwargs: vec![], then }
}

/// Nothing to do with an answer.
fn nothing(_: &mut Yard, _: &Fact, _: &[Object], _: ObjectRef<'_>) {}

/// The chain a question is on, as the keyword of a view.
fn on(fact: &Fact) -> Vec<(String, Object)> {
  vec![("on".to_owned(), Object::string(fact.on()))]
}

/// The World of this machine, for the tests, as the one ear of the host: a directory, a model that answers by a
/// script, and real commands.
struct Yard {
  at: PathBuf,
  words: Rc<RefCell<Vec<String>>>,
  /// What every ask showed the model, shared with the test.
  read: Rc<RefCell<Vec<String>>>,
  /// The entries it kept, shared with the test, which is the record of the life.
  kept: Rc<RefCell<Vec<Object>>>,
  /// Every command it was told to end before its time, shared with the test.
  slain: Rc<RefCell<Vec<String>>>,
  /// The voice its work speaks with once a hearing is over.
  voice: Voice,
  /// The commands it runs, by their acts.
  running: Vec<String>,
  /// The verbs it has still to say while it hears the fact it hears.
  steps: VecDeque<Step>,
  hearing: Option<Fact>,
}

impl Yard {
  fn new(at: PathBuf, words: &[&str]) -> Self {
    Yard {
      at,
      words: Rc::new(RefCell::new(words.iter().map(|one| (*one).to_owned()).collect())),
      read: Rc::default(),
      kept: Rc::default(),
      slain: Rc::default(),
      voice: Voice::default(),
      running: Vec::new(),
      steps: VecDeque::new(),
      hearing: None,
    }
  }

  /// What a chain stands on: the operator and one model, the directory of the yard, and that model.
  fn standing(&self) -> Object {
    let actor = |name: &str, efforts: &[&str]| {
      Object::list([
        Object::string(name),
        Object::list(efforts.iter().map(|one| Object::string(*one))),
        Object::int(200_000),
      ])
    };
    Object::list([
      Object::list([actor("operator", &[]), actor("m", &["low"])]),
      Object::string(self.at.display().to_string()),
      Object::string("m/low"),
    ])
  }

  /// The verbs the yard says of one fact: it answers what is its own at once, and takes a reply, a command, a wait
  /// and a prompt to the operator with a started, whose done its work says later.
  fn heard(&mut self, fact: &Fact) -> Vec<Step> {
    let id = fact.about().to_owned();
    match fact.kind() {
      "stand" => vec![say("done", &id, [self.standing()], nothing)],
      "clock" | "chance" => vec![say("done", &id, [Object::float(0.5)], nothing)],
      "read" | "write" => {
        vec![Step { name: "cwd", args: vec![], kwargs: on(fact), then: Yard::served }]
      }
      "keep" => {
        self.kept.borrow_mut().push(fact.word(0).map_or_else(Object::none, |one| one.to_owned()));
        vec![]
      }
      "reply" => vec![
        say("started", &id, [], nothing),
        Step { name: "turns", args: vec![], kwargs: on(fact), then: Yard::answers },
      ],
      "bash" => vec![
        say("started", &id, [], nothing),
        Step { name: "cwd", args: vec![], kwargs: on(fact), then: Yard::runs },
      ],
      "wait" => vec![say("started", &id, [], |yard, fact, _, _| {
        yard.voice.say("done", fact.about(), [Object::none()]);
      })],
      "prompt" => vec![say("started", &id, [], |yard, fact, _, _| {
        let shape = fact.word(1).and_then(|one| one.as_str()).unwrap_or_default();
        let answer = if shape == "int" {
          Object::int(3)
        } else {
          Fault::refused(format!("the yard answers no {shape}")).object()
        };
        yard.voice.verb("close", vec![answer, Object::string(fact.about())]);
      })],
      "cancel" | "close" => self
        .running
        .iter()
        .map(|command| Step {
          name: "covers",
          args: vec![fact.0.clone(), Object::string(command.clone())],
          kwargs: vec![],
          then: Yard::slays,
        })
        .collect(),
      "done" => {
        self.running.retain(|one| *one != id);
        vec![]
      }
      _ => vec![],
    }
  }

  /// A read or a write, served at the directory the chain stands in.
  fn served(yard: &mut Yard, fact: &Fact, _: &[Object], here: ObjectRef<'_>) {
    let here = PathBuf::from(here.as_str().unwrap_or_default());
    let got = if fact.kind() == "read" {
      let at = here.join(fact.word(1).and_then(|one| one.as_str()).unwrap_or_default());
      fs::read_to_string(&at)
        .map(|content| Text::new(at.display().to_string(), content).object())
        .unwrap_or_else(|_| Fault::refused(format!("no file at {}", at.display())).object())
    } else {
      let text = fact.word(1).and_then(Text::of).expect("a write carries a text");
      let at = here.join(&text.path);
      match fs::write(&at, &text.content) {
        Ok(()) => Text::new(at.display().to_string(), text.content).object(),
        Err(no) => Fault::refused(no.to_string()).object(),
      }
    };
    yard.steps.push_back(say("done", fact.about(), [got], nothing));
  }

  /// A reply, answered with the next word of the script, which the work of the yard says later.
  fn answers(yard: &mut Yard, fact: &Fact, _: &[Object], turns: ObjectRef<'_>) {
    yard.read.borrow_mut().push(turns.py_repr());
    let word = {
      let mut words = yard.words.borrow_mut();
      if words.is_empty() { "close(None)".to_owned() } else { words.remove(0) }
    };
    let turn = Object::tuple([
      Object::string("assistant"),
      Object::string(word),
      Object::none(),
      Object::list([]),
    ]);
    yard.voice.say("done", fact.about(), [turn]);
  }

  /// A command, run on a thread of its own in the directory the chain stands in, which says what it wrote and what
  /// it came to through the voice of the yard.
  fn runs(yard: &mut Yard, fact: &Fact, _: &[Object], here: ObjectRef<'_>) {
    let (voice, about) = (yard.voice.clone(), fact.about().to_owned());
    let command = fact.word(1).and_then(|one| one.as_str()).unwrap_or_default().to_owned();
    let here = here.as_str().unwrap_or_default().to_owned();
    yard.running.push(about.clone());
    thread::spawn(move || {
      let got =
        std::process::Command::new("sh").arg("-c").arg(&command).current_dir(&here).output();
      let stream = |name: &str, text: &str| Text::new(format!("{about}/{name}"), text);
      let exit = match got {
        Ok(out) => {
          let (stdout, stderr) =
            (String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
          voice.say("out", &about, [Object::string(&*stdout), Object::string("stdout")]);
          if !stderr.is_empty() {
            voice.say("out", &about, [Object::string(&*stderr), Object::string("stderr")]);
          }
          let code = out.status.code().map(i64::from);
          Exit { code, stdout: stream("stdout", &stdout), stderr: stream("stderr", &stderr) }
        }
        Err(_) => Exit { code: None, stdout: stream("stdout", ""), stderr: stream("stderr", "") },
      };
      voice.say("done", &about, [exit.object()]);
    });
  }

  /// A command a control is over, ended before its time.
  fn slays(yard: &mut Yard, _: &Fact, args: &[Object], covers: ObjectRef<'_>) {
    let command =
      args.get(1).and_then(|one| one.as_ref().as_str().map(str::to_owned)).unwrap_or_default();
    if covers.as_bool() == Some(true) {
      yard.running.retain(|one| *one != command);
      yard.slain.borrow_mut().push(command);
    }
  }

  /// The next verb the yard says, or nothing when it is done hearing.
  fn next(&self) -> Reply {
    match self.steps.front() {
      Some(step) => Reply::Calls {
        name: step.name.to_owned(),
        args: step.args.clone(),
        kwargs: step.kwargs.clone(),
      },
      None => Reply::Nothing,
    }
  }
}

impl Ears for Yard {
  fn opened(&mut self, voice: Voice) {
    self.voice = voice.of("world");
  }

  fn hears(&mut self, _name: &str, fact: Option<&Fact>) -> Reply {
    let Some(fact) = fact else { return Reply::Nothing };
    self.steps = self.heard(fact).into();
    self.hearing = Some(fact.clone());
    self.next()
  }

  fn answered(&mut self, _name: &str, got: ObjectRef<'_>) -> Reply {
    if let (Some(step), Some(fact)) = (self.steps.pop_front(), self.hearing.clone())
      && let Some(value) = entry(&got, 1)
    {
      (step.then)(self, &fact, &step.args, value);
    }
    self.next()
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
    let life = Life::open(world, ["world"]).boot(record)?;
    Ok(Lived { life, at, read, kept, slain })
  }

  fn root(&self) -> String {
    self.life.root().to_owned()
  }

  /// The life driven until it holds an act of this name.
  fn made(&mut self, id: &str) {
    while self.life.get(id).is_err() {
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
  assert_eq!(no.name, "Refused", "a name of no act is no act");
  let no = lived.life.word("{}['bash9']", vec![]).unwrap_err();
  assert_eq!(no.name, "KeyError");
  let no = lived.life.word("nowhere", vec![]).unwrap_err();
  assert_eq!(no.name, "NameError");
}

#[test]
fn a_second_life_on_the_record_the_world_kept_makes_the_same_acts_again() {
  let mut first = Lived::new("again", &["close(len(read('a.txt').lines))"], vec![]).unwrap();
  let root = first.root();
  // The file is put there by hand: a write of the operator is an act that the record keeps, and what this test
  // holds are the acts of a prompt alone.
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
