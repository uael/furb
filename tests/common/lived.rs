//! One life of the real engine in a yard of the disk, on the ears of the World that the crate
//! writes and on a provider that answers from a script, as a host opens one. The suite of the crate
//! and the tests of the bound of a turn beside the engine share it: each includes it as a module,
//! and gives it `answered`, which the provider calls at each answer of its script.

#![allow(dead_code, reason = "each side takes the part of the harness that it needs")]

use std::{cell::RefCell, collections::VecDeque, env, fs, path::PathBuf, process, rc::Rc};

use futures::{FutureExt, stream};
use unsync::oneshot;

use furb::{
  Act, Ear, Engine, Fact, Fault, Object,
  ear::{Co, Next, ear, hear, say},
  world,
};

use super::{Yard, answered, block_on};

/// A fact of a done of an act with a value.
pub(crate) fn done(about: &str, value: Object) -> Fact {
  Fact::says("done", about, [value])
}

/// The provider of the test: it answers what the chains stand on, and each reply with the next word
/// of its script, but for the word WAIT, which leaves the reply started and not done.
pub(crate) fn provider(
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
          answered(read.borrow().len());
          let word = words.borrow_mut().pop_front().unwrap_or_else(|| "close(None)".to_owned());
          if word == "WAIT" {
            continue;
          }
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
pub(crate) struct Lived {
  pub(crate) engine: Engine,
  pub(crate) at: PathBuf,
  pub(crate) read: Rc<RefCell<Vec<String>>>,
  /// The folder of a fresh life, which goes after its engine; a later life on it holds none.
  pub(crate) yard: Option<Yard>,
}

impl Lived {
  /// A life in a yard of its own, fresh, or opened on what the store kept there.
  pub(crate) fn new(yard: &str, words: &[&str], fresh: bool) -> Result<Self, Fault> {
    let name = format!("furb-engine-{yard}-{}", process::id());
    let yard = fresh.then(|| Yard::new(&name));
    let at = env::temp_dir().join(name);
    fs::create_dir_all(&at).unwrap_or_else(|no| panic!("a yard of the test: {no}"));
    let (record, store) = world::store(at.join("record.jsonl"))?;
    let read = Rc::default();
    let words = Rc::new(RefCell::new(words.iter().map(|one| (*one).to_owned()).collect()));
    let scratch = world::Scratch::own();
    let ears = [
      ("provider", provider(at.clone(), words, Rc::clone(&read))),
      ("files", world::files(scratch.clone(), at.join("config"), None)),
      ("bash", world::bash(scratch)),
      ("time", world::time()),
      ("store", store),
    ];
    let engine = Engine::boot(record, ears)?;
    Ok(Self { engine, at, read, yard })
  }

  /// The end of a fresh life, and its folder, for a later life on what the store kept there.
  pub(crate) fn end(mut self) -> Yard {
    self.yard.take().unwrap_or_else(|| panic!("a fresh life holds its yard"))
  }

  pub(crate) fn root(&self) -> String {
    self.engine.root().to_owned()
  }

  /// What an act came to, once the ears have said it, or the fault it completed with.
  pub(crate) fn settled(&mut self, id: &str) -> Result<Object, Fault> {
    block_on(Act::<Object>::of(&mut self.engine, id))
  }
}

#[expect(clippy::unnecessary_wraps, reason = "the field on of each verb takes an Option")]
pub(crate) fn on(root: &str) -> Option<String> {
  Some(root.to_owned())
}

/// A fact as a later life says it again: its kind, its act and its words, and not who said it,
/// since the journal says the done of what the record holds, with no started before it.
pub(crate) fn said(fact: &Fact) -> Option<String> {
  let words = (0..).map_while(|at| fact.word(at)).map(|one| one.to_owned().py_repr());
  (fact.kind() != "started")
    .then(|| format!("{} {} {:?}", fact.kind(), fact.about(), words.collect::<Vec<_>>()))
}

/// An ear that takes a wait and ends it by its work, of its own accord, once its gate opens: it
/// tells a pause of the chain of the wait, says the wait due, then says it done.
pub(crate) fn pauser(gate: oneshot::Receiver<()>) -> Box<dyn Ear> {
  ear(move |mut co: Co<(String, String)>| async move {
    let mut gate = Some(gate);
    loop {
      match co.next().await {
        Next::Heard(a) | Next::Resumed(a) => {
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
          drop(co.call("pause", vec![Object::string(on)], vec![]).await);
          say(&mut co, Fact::says("due", &about, [Object::float(1.0)]));
          say(&mut co, done(&about, Object::none()));
        }
      }
    }
  })
}

/// A life on the provider of the test and on the ear that pauses, and the gate of the work of that
/// ear.
pub(crate) fn paused(yard: &str) -> (Engine, oneshot::Sender<()>) {
  let at = env::temp_dir().join(format!("furb-engine-{yard}-{}", process::id()));
  let (gate, opens) = oneshot::channel();
  let ears = [("provider", provider(at, Rc::default(), Rc::default())), ("pauser", pauser(opens))];
  (Engine::boot(Vec::<Object>::new(), ears).unwrap_or_else(|no| panic!("{no}")), gate)
}
