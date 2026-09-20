//! One life of the real engine, driven by the crate from end to end.
//!
//! Every other test of the crate holds one piece against a scripted double. This holds the whole of it against
//! the engine itself: a life is opened, an operator runs words, a model answers a prompt, the Kernel gates the
//! word it wrote and runs it, an ear of the host does what the word asks of the disk, and the record it kept opens
//! a second life.
//!
//! The one double is [`Yard`], a World of this machine small enough to read: what a life may touch is the host's
//! to decide, so the crate ships the trait and every host writes one of these.

use std::{cell::RefCell, fs, path::PathBuf, rc::Rc};

use furb::{Ear, Ears, Fact, Life, Refusal, Reply, Sand, Value};

/// A World of this machine, for the tests: a directory, and a model that answers by a script.
struct Yard {
  /// Where the chains of the life stand.
  at: PathBuf,
  /// The words its model answers with, in order.
  words: Vec<String>,
  /// What every ask it heard showed the model, shared with the test, so it reads what the model was told.
  read: Rc<RefCell<Vec<String>>>,
  /// The entries it kept, shared with the test, which is the record of the life.
  kept: Rc<RefCell<Vec<Value>>>,
  /// The read or the write it is waiting to answer, once the chain has said where its paths resolve.
  asking: Option<Fact>,
}

impl Yard {
  fn new(at: PathBuf, words: &[&str]) -> Self {
    Yard {
      at,
      words: words.iter().map(|one| (*one).to_owned()).collect(),
      read: Rc::default(),
      kept: Rc::default(),
      asking: None,
    }
  }

  /// What the World stands on: one operator, one model, this directory, and the model as the default actor.
  fn standing(&self) -> Value {
    let actor = |name: &str, efforts: Vec<Value>| {
      Value::Tuple(vec![Value::Str(name.to_owned()), Value::Tuple(efforts), Value::Int(200_000)])
    };
    Value::Tuple(vec![
      Value::Tuple(vec![actor("operator", vec![]), actor("m", vec![Value::Str("low".to_owned())])]),
      Value::Str(self.at.display().to_string()),
      Value::Str("m/low".to_owned()),
    ])
  }
}

impl Ear for Yard {
  fn hears(&mut self, fact: &Fact) -> Reply {
    match fact.kind() {
      "stand" => Reply::Say(Fact::says("done", fact.about(), vec![self.standing()])),
      "clock" | "chance" => Reply::Say(Fact::says("done", fact.about(), vec![Value::Float(0.5)])),
      "read" | "write" => {
        // Where a path resolves is the chain's to say, so it is asked before the disk is touched.
        self.asking = Some(fact.clone());
        Reply::reads(format!("cwd(on={:?})", fact.on().unwrap_or_default()))
      }
      "ask" => {
        self.read.borrow_mut().push(text(fact.words().get(2).unwrap_or(&Value::None)));
        let word = if self.words.is_empty() { "close(None)".to_owned() } else { self.words.remove(0) };
        let turn = Value::Tuple(vec![
          Value::Str("assistant".to_owned()),
          Value::List(vec![Value::Str(word)]),
          Value::None,
          Value::List(vec![]),
        ]);
        Reply::Say(Fact::says("answer", fact.about(), vec![turn]))
      }
      "keep" => {
        self.kept.borrow_mut().extend(fact.words().first().cloned());
        Reply::Nothing
      }
      _ => Reply::Nothing,
    }
  }

  fn answered(&mut self, got: &Value) -> Reply {
    let Some(held) = self.asking.take() else { return Reply::Nothing };
    let here = got.as_str().unwrap_or_default().to_owned();
    if held.kind() == "read" {
      let path = held.words().get(1).and_then(Value::as_str).unwrap_or_default().to_owned();
      let at = self.at.join(&here).join(&path);
      let said = match fs::read_to_string(&at) {
        Ok(one) => Value::text(at.display().to_string(), one),
        Err(_) => Value::refused(format!("no file at {}", at.display())),
      };
      return Reply::Say(Fact::says("done", held.about(), vec![said]));
    }
    let text = held.words().get(1).cloned().unwrap_or(Value::None);
    let path = text.field("path").and_then(Value::as_str).unwrap_or_default();
    let content = text.field("content").and_then(Value::as_str).unwrap_or_default();
    let at = self.at.join(&here).join(path);
    fs::write(&at, content).expect("a file of the test");
    Reply::Say(Fact::says("done", held.about(), vec![Value::text(at.display().to_string(), content)]))
  }
}

/// Every text a value holds, one after the other, which is enough for a test to read what a model was told.
fn text(got: &Value) -> String {
  match got {
    Value::Str(held) => held.clone(),
    Value::List(held) | Value::Tuple(held) => held.iter().map(text).collect::<Vec<String>>().join(" "),
    Value::Map(held) | Value::Shape { fields: held, .. } => {
      held.iter().map(|(_, one)| text(one)).collect::<Vec<String>>().join(" ")
    }
    Value::Error { name, args } => format!("{name} {}", args.iter().map(text).collect::<Vec<String>>().join(" ")),
    _ => String::new(),
  }
}

/// One life of the real engine, on a yard of the disk, with the model answering these words.
struct Lived {
  held: Life<Ears>,
  at: PathBuf,
  read: Rc<RefCell<Vec<String>>>,
  kept: Rc<RefCell<Vec<Value>>>,
}

impl Lived {
  fn new(yard: &str, words: &[&str], record: &Value) -> Result<Self, Refusal> {
    let at = std::env::temp_dir().join(format!("furb-life-{yard}"));
    if record == &Value::List(vec![]) {
      let _ = fs::remove_dir_all(&at);
    }
    fs::create_dir_all(&at).expect("a yard of the test");
    let world = Yard::new(at.clone(), words);
    let (read, kept) = (Rc::clone(&world.read), Rc::clone(&world.kept));
    let ears = Ears::new().with("world", world);
    let held = Life::boot(Sand::default(), ears, &["world".to_owned()], record)?;
    Ok(Lived { held, at, read, kept })
  }

  /// What the World of the life showed its model at each ask.
  fn shown(&self) -> Vec<String> {
    self.read.borrow().clone()
  }
}

/// What an act came to, once every word of the life ran.
fn came(life: &mut Lived, act: &str) -> Value {
  life.held.word(&format!("peek({act:?})")).expect("the life answers a peek")
}

#[test]
fn a_life_of_the_real_engine_opens_on_its_root_and_answers_what_the_root_stands_on() {
  let mut life = Lived::new("open", &[], &Value::List(vec![])).unwrap();
  assert_eq!(life.held.root(), "chain://operator.1");
  let cwd = life.held.word("cwd(on=\"chain://operator.1\")").unwrap();
  assert_eq!(cwd, Value::Str(life.at.display().to_string()));
  let turns = life.held.word("turns(on=\"chain://operator.1\")").unwrap();
  assert_eq!(turns.as_entries().map(<[Value]>::len), Some(1));
}

#[test]
fn an_ear_serves_a_read_and_a_write_of_the_real_engine() {
  let mut life = Lived::new("disk", &[], &Value::List(vec![])).unwrap();
  let got = life.held.word("write(Text(\"a.txt\", \"one\\ntwo\\n\"), on=\"chain://operator.1\")").unwrap();
  assert_eq!(got.field("content"), Some(&Value::Str("one\ntwo\n".to_owned())));
  assert_eq!(fs::read_to_string(life.at.join("a.txt")).unwrap(), "one\ntwo\n");
  let got = life.held.word("read(\"a.txt\", on=\"chain://operator.1\").lines").unwrap();
  assert_eq!(got, Value::List(vec![Value::Str("one".to_owned()), Value::Str("two".to_owned())]));
  let no = life.held.word("read(\"none.txt\", on=\"chain://operator.1\")").unwrap_err();
  assert!(no.refused(), "{no}");
}

#[test]
fn a_model_answers_a_prompt_and_the_kernel_runs_the_word_it_wrote() {
  let mut life = Lived::new("prompt", &["close(len(read('a.txt').lines))"], &Value::List(vec![])).unwrap();
  life.held.word("write(Text(\"a.txt\", \"one\\ntwo\\nthree\\n\"), on=\"chain://operator.1\")").unwrap();
  let act = life.held.word("prompt(int, \"count the lines\", on=\"chain://operator.1\")").unwrap();
  let act = act.as_str().expect("a prompt gives its name").to_owned();
  assert_eq!(came(&mut life, &act), Value::Int(3));
  assert!(life.shown()[0].contains("count the lines"), "{:?}", life.shown());
}

#[test]
fn the_gate_refuses_a_word_and_the_engine_asks_the_model_again() {
  let mut life = Lived::new("gate", &["close(BAD)", "close(7)"], &Value::List(vec![])).unwrap();
  let act = life.held.word("prompt(int, \"count\", on=\"chain://operator.1\")").unwrap();
  let act = act.as_str().unwrap().to_owned();
  assert_eq!(came(&mut life, &act), Value::Int(7));
  assert!(life.shown()[1].contains("BAD"), "{:?}", life.shown()[1]);
}

#[test]
fn a_word_that_raises_is_asked_again_and_the_model_reads_what_it_raised() {
  let mut life = Lived::new("raises", &["close(1 // 0)", "close(5)"], &Value::List(vec![])).unwrap();
  let act = life.held.word("prompt(int, \"count\", on=\"chain://operator.1\")").unwrap();
  let act = act.as_str().unwrap().to_owned();
  assert_eq!(came(&mut life, &act), Value::Int(5));
  assert!(life.shown()[1].contains("ZeroDivisionError"), "{:?}", life.shown()[1]);
}

#[test]
fn what_the_engine_raised_reaches_the_host_as_the_exception_it_is() {
  let mut life = Lived::new("raised", &[], &Value::List(vec![])).unwrap();
  let no = life.held.word("get(\"bash://operator.9\")").unwrap_err();
  assert_eq!(no.name(), "KeyError");
  assert!(!no.refused(), "{no}");
}

#[test]
fn a_second_life_on_the_record_the_world_kept_makes_the_same_acts_again() {
  let mut life = Lived::new("again", &["close(2)"], &Value::List(vec![])).unwrap();
  let act = life.held.word("prompt(int, \"one plus one\", on=\"chain://operator.1\")").unwrap();
  let act = act.as_str().unwrap().to_owned();
  assert_eq!(came(&mut life, &act), Value::Int(2));
  let kept = life.kept.borrow().clone();
  assert!(kept.len() > 1, "the World kept {} entries", kept.len());
  let at = life.at.clone();
  drop(life);

  let mut again = Lived::new("again", &[], &Value::List(kept)).unwrap();
  assert_eq!(again.held.root(), "chain://operator.1");
  assert_eq!(came(&mut again, &act), Value::Int(2));
  assert!(again.shown().is_empty(), "a later life asks no model again");
  let _ = fs::remove_dir_all(at);
}
