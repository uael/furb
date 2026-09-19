//! The sandbox the word of a rung runs in: one session of monty for each chain.
//!
//! A model writes the word, so the word is the one part of a life that is not trusted. It runs here, where it
//! reaches the engine and nothing else: no disk, no machine, no name of the outside. Every verb it calls goes out
//! through [`Reach`], which the host answers from the engine it holds.
//!
//! A chain has a module of its own, so a chain has a session of its own, and what one word binds the next word of
//! that chain reads. What the engine gives back that is no plain value, a text among it, crosses as an instance
//! the host holds: its fields go with it, and a field it was not given the sandbox asks the host for, so nothing
//! of the engine is copied that the word never reads.

use std::collections::HashMap;

use monty::{MontyRepl, ReplProgress};
use monty_types::{
  CompileOptions, ExcType, ExtFunctionResult, MontyException, MontyObject, MontyUuid, NameLookupResult, ObjectRef,
  PrintWriter, ResourceTracker,
};

use crate::{
  fact::Value,
  kernel::{Sandbox, Step},
};

/// What the engine holds under a name that a word says.
#[derive(Debug, Clone, PartialEq)]
pub enum Held {
  /// A verb, which the word may call.
  Verb,
  /// A value, which the word reads.
  Is(Value),
  /// Nothing at all, which raises a NameError where the word said it.
  Nothing,
}

/// What a verb of the engine gave a word.
#[derive(Debug, Clone, PartialEq)]
pub enum Called {
  /// A value.
  Gave(Value),
  /// An act, by its name, which the word may await.
  Act(String),
  /// An exception, which is raised where the word made the call.
  Raised {
    /// The name of the exception.
    name: String,
    /// What the exception says.
    why: String,
  },
}

/// The engine, as the word reaches it.
///
/// The host holds the engine, in whatever language it is written in, and answers for it here. A name the word
/// says is looked up once for each session that says it, and a verb it calls is performed where the engine lives.
pub trait Reach {
  /// What the engine holds under a name.
  fn holds(&mut self, name: &str) -> Held;

  /// One call of a verb of the engine, made by the word of a rung, on the chain of that rung.
  ///
  /// What the word says is said by that rung, so the host stands the engine on the rung for the call: the act the
  /// verb makes is named under it, and the fact it says is on the chain of it.
  fn call(&mut self, rung: &str, chain: &str, name: &str, args: Vec<Value>, kwargs: Vec<(String, Value)>) -> Called;

  /// One field of a value the engine gave, which the word asked for and the value did not carry.
  ///
  /// The value is given whole, so a host answers from the value itself and holds nothing between the calls.
  fn field(&mut self, of: &Value, name: &str) -> Held;
}

/// One suspended run, held between the step that suspended it and the step that carries it on.
enum Waiting {
  /// The run waits for an act, which the call id stands for.
  Futures(monty::ReplResolveFutures, u32),
}

/// The sandbox of monty: one session for each chain, and the runs that stand.
pub struct Monty<R> {
  /// The engine, as the word reaches it.
  pub reach: R,
  /// One session for each chain, which holds what the words of that chain bound.
  sessions: HashMap<String, MontyRepl>,
  /// The chain of each run that stands.
  chains: HashMap<String, String>,
  /// The runs that wait, each by its name.
  waiting: HashMap<String, Waiting>,
  /// The act that each call id of a pending future stands for.
  acts: HashMap<u32, String>,
  /// Every value the host holds for the sandbox, by the identity the sandbox knows it as.
  held: HashMap<MontyUuid, Value>,
  /// How many identities the sandbox has given out, which names the next one.
  named: u128,
}

impl<R: Reach> Monty<R> {
  /// A sandbox over one engine, for one life.
  pub fn new(reach: R) -> Self {
    Monty {
      reach,
      sessions: HashMap::new(),
      chains: HashMap::new(),
      waiting: HashMap::new(),
      acts: HashMap::new(),
      held: HashMap::new(),
      named: 0,
    }
  }

  /// The next identity the host gives a value it holds, which a later life gives again.
  fn names(&mut self) -> MontyUuid {
    self.named += 1;
    MontyUuid::from_u128(self.named)
  }

  /// The session of a chain, which is made the first time a word of that chain runs.
  fn session(&mut self, chain: &str) -> MontyRepl {
    self.sessions.remove(chain).unwrap_or_else(|| {
      MontyRepl::new(chain, ResourceTracker::default(), CompileOptions::default())
    })
  }

  /// The session of a chain, put back wherever a run of it left off.
  fn recover(&mut self, chain: &str, said: Result<ReplProgress, Box<monty::ReplStartError>>) {
    let repl = match said {
      Ok(ReplProgress::Complete { repl, .. }) => repl,
      Ok(ReplProgress::NameLookup(one)) => one.into_repl(),
      Ok(ReplProgress::FunctionCall(one)) => one.into_repl(),
      Ok(ReplProgress::ResolveFutures(one)) => one.into_repl(),
      Ok(ReplProgress::OsCall(one)) => one.into_repl(),
      Err(error) => (*error).repl,
    };
    self.sessions.insert(chain.to_owned(), repl);
  }

  /// One run, driven to its next await or to its end.
  fn drive(&mut self, rung: &str, chain: &str, mut said: Result<ReplProgress, Box<monty::ReplStartError>>) -> Step {
    loop {
      let progress = match said {
        Ok(progress) => progress,
        Err(error) => {
          let error = *error;
          let raised = Value::Error {
            name: format!("{:?}", error.error.exc_type()),
            args: vec![Value::Str(error.error.to_string())],
          };
          self.sessions.insert(chain.to_owned(), error.repl);
          return Step::Ran(Some(raised));
        }
      };
      said = match progress {
        ReplProgress::Complete { repl, value } => {
          self.sessions.insert(chain.to_owned(), repl);
          let _ = value;
          return Step::Ran(None);
        }
        ReplProgress::NameLookup(one) => {
          let got = match one.object_id().and_then(|id| self.held.get(&id).cloned()) {
            Some(of) => self.reach.field(&of, &one.name),
            None => self.reach.holds(&one.name),
          };
          let answer = match got {
            Held::Verb => NameLookupResult::Value(MontyObject::function(one.name.clone(), None)),
            Held::Is(value) => NameLookupResult::Value(self.into_monty(&value)),
            Held::Nothing => NameLookupResult::Undefined,
          };
          one.resume(answer, PrintWriter::Stdout)
        }
        ReplProgress::FunctionCall(one) => {
          let (args, kwargs) = self.words_of(&one.args);
          let call_id = one.call_id;
          match self.reach.call(rung, chain, &one.function_name, args, kwargs) {
            Called::Gave(value) => {
              let got = self.into_monty(&value);
              one.resume(ExtFunctionResult::Return(got), PrintWriter::Stdout)
            }
            Called::Act(name) => {
              self.acts.insert(call_id, name);
              one.resume_pending(PrintWriter::Stdout)
            }
            Called::Raised { name, why } if name == "CancelledError" => {
              // A close of the prompt of the running word stops that word where it stands, and nothing after the
              // call runs, so the cancel is uncatchable and the run is over with it.
              let raised = MontyException::new(ExcType::RuntimeError, Some(why));
              let said = one.abort(raised, PrintWriter::Stdout);
              self.recover(chain, said);
              return Step::Ran(Some(Value::Error { name, args: Vec::new() }));
            }
            Called::Raised { name, why } => {
              let raised = MontyException::new(exception_of(&name), Some(why));
              one.resume(ExtFunctionResult::Error(raised), PrintWriter::Stdout)
            }
          }
        }
        ReplProgress::ResolveFutures(one) => {
          let Some(&call_id) = one.pending_call_ids().first() else {
            self.sessions.insert(chain.to_owned(), one.into_repl());
            return Step::Ran(None);
          };
          let act = self.acts.get(&call_id).cloned().unwrap_or_default();
          self.waiting.insert(rung.to_owned(), Waiting::Futures(one, call_id));
          return Step::Wants(act);
        }
        ReplProgress::OsCall(one) => {
          // The word reaches the machine through the verbs of the engine alone, so the os is closed to it.
          let refused = MontyException::new(ExcType::RuntimeError, Some("the sandbox reaches the engine alone".to_owned()));
          one.abort(refused, PrintWriter::Stdout)
        }
      };
    }
  }

  /// The words of one call: what stood by place, and what stood by name.
  fn words_of(&self, args: &monty_types::CallArgs) -> (Vec<Value>, Vec<(String, Value)>) {
    let held = args.args().map(|one| self.of_sandbox(&one)).collect();
    let named = args
      .kwargs()
      .map(|(key, one)| (key.as_str().unwrap_or_default().to_owned(), self.of_sandbox(&one)))
      .collect();
    (held, named)
  }

  /// One value of the sandbox as the engine reads it.
  ///
  /// An act the word holds is a pending future of the session, so the name of the act goes back to the engine
  /// wherever the word hands one to a verb.
  fn of_sandbox(&self, said: &ObjectRef<'_>) -> Value {
    let got = of_monty(said);
    match &got {
      Value::Error { name, args } if name == "Repr" => {
        let repr = args.first().and_then(Value::as_str).unwrap_or_default();
        match awaited(repr).and_then(|id| self.acts.get(&id)) {
          Some(act) => Value::Str(act.clone()),
          None => got,
        }
      }
      _ => got,
    }
  }

  /// A value of the engine as the sandbox holds it.
  ///
  /// What is plain crosses whole. What is not, a text among it, crosses as an instance the host still holds: it
  /// carries the fields it has, and a field the word asks for that it does not carry goes back to the host.
  fn into_monty(&mut self, value: &Value) -> MontyObject {
    match value {
      Value::None | Value::Show => MontyObject::none(),
      Value::Bool(said) => MontyObject::bool(*said),
      Value::Int(said) => MontyObject::int(*said),
      Value::Float(said) => MontyObject::float(*said),
      Value::Str(said) => MontyObject::string(said.clone()),
      Value::List(held) => MontyObject::list(held.iter().map(|one| self.into_monty(one)).collect::<Vec<_>>()),
      Value::Tuple(held) => MontyObject::tuple(held.iter().map(|one| self.into_monty(one)).collect::<Vec<_>>()),
      Value::Map(held) => {
        let pairs: Vec<(MontyObject, MontyObject)> = held
          .iter()
          .map(|(key, one)| (MontyObject::string(key.clone()), self.into_monty(one)))
          .collect();
        MontyObject::dict(pairs)
      }
      Value::Error { name, args } => {
        let why = args.first().and_then(Value::as_str).map(str::to_owned);
        MontyObject::exception(exception_of(name), why)
      }
      Value::Shape { name, fields } => {
        let id = self.names();
        self.held.insert(id, value.clone());
        let kind = MontyObject::class_type(name.clone(), self.names(), true, true, Vec::new());
        let attrs: Vec<(MontyObject, MontyObject)> = fields
          .iter()
          .map(|(key, one)| (MontyObject::string(key.clone()), self.into_monty(one)))
          .collect();
        MontyObject::class_instance(kind, id, attrs)
      }
    }
  }
}

/// Which act a pending future of the sandbox stands for, which its own repr says.
fn awaited(said: &str) -> Option<u32> {
  let held = said.split("external_future(").nth(1)?;
  held.split(')').next()?.parse().ok()
}

/// One value of the sandbox, as the engine reads it.
fn of_monty(said: &ObjectRef<'_>) -> Value {
  if let Some(held) = said.as_bool() {
    return Value::Bool(held);
  }
  if let Some(held) = said.as_int() {
    return Value::Int(held);
  }
  if let Some(held) = said.as_float() {
    return Value::Float(held);
  }
  if let Some(held) = said.as_str() {
    return Value::Str(held.to_owned());
  }
  if let Some(held) = said.pairs() {
    let pairs = held
      .iter()
      .map(|(key, one)| (key.as_str().unwrap_or_default().to_owned(), of_monty(one)))
      .collect();
    return Value::Map(pairs);
  }
  if let Some(held) = said.items() {
    let items = held.iter().map(of_monty).collect();
    return if said.type_name() == "tuple" { Value::Tuple(items) } else { Value::List(items) };
  }
  match said.type_name() {
    "NoneType" => Value::None,
    name => Value::Error { name: name.to_owned(), args: vec![Value::Str(said.py_repr())] },
  }
}

/// The exception of a name, and a plain one for a name the sandbox does not know.
fn exception_of(name: &str) -> ExcType {
  match name {
    "ValueError" => ExcType::ValueError,
    "TypeError" => ExcType::TypeError,
    "KeyError" => ExcType::KeyError,
    "IndexError" => ExcType::IndexError,
    "NameError" => ExcType::NameError,
    "AttributeError" => ExcType::AttributeError,
    "NotImplementedError" => ExcType::NotImplementedError,
    _ => ExcType::RuntimeError,
  }
}

impl<R: Reach> Sandbox for Monty<R> {
  fn gate(&mut self, _word: &str, _ladder: &[String], _shape: &str) -> Vec<String> {
    // The gate that reads a word against the shape it must give is the type checker's, and it is not here yet.
    Vec::new()
  }

  fn begin(&mut self, rung: &str, chain: &str, word: &str) -> Step {
    let session = self.session(chain);
    self.chains.insert(rung.to_owned(), chain.to_owned());
    let said = session.feed_start(word, Vec::new(), PrintWriter::Stdout);
    self.drive(rung, chain, said)
  }

  fn carry(&mut self, rung: &str, got: &Value) -> Step {
    let Some(Waiting::Futures(one, call_id)) = self.waiting.remove(rung) else {
      return Step::Ran(None);
    };
    let chain = self.chains.get(rung).cloned().unwrap_or_default();
    let value = self.into_monty(got);
    let said = one.resume(vec![(call_id, ExtFunctionResult::Return(value))], PrintWriter::Stdout);
    self.drive(rung, &chain, said)
  }

  fn drop_frame(&mut self, rung: &str) {
    if let Some(Waiting::Futures(one, _)) = self.waiting.remove(rung) {
      let chain = self.chains.get(rung).cloned().unwrap_or_default();
      self.sessions.insert(chain, one.into_repl());
    }
    self.chains.remove(rung);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// An engine of the test: it holds the verbs of a word and keeps every call the word made.
  #[derive(Default)]
  struct Engine {
    calls: Vec<(String, Vec<Value>)>,
  }

  impl Reach for Engine {
    fn holds(&mut self, name: &str) -> Held {
      match name {
        "close" | "bash" | "read" => Held::Verb,
        "TIMEOUT" => Held::Is(Value::Float(600.0)),
        _ => Held::Nothing,
      }
    }

    fn call(&mut self, _rung: &str, _chain: &str, name: &str, args: Vec<Value>, _kwargs: Vec<(String, Value)>) -> Called {
      self.calls.push((name.to_owned(), args.clone()));
      match name {
        "bash" => Called::Act("bash://operator.1.1.1".to_owned()),
        "read" => Called::Gave(Value::text("/w/a.txt", "one\ntwo\n")),
        _ => Called::Gave(Value::None),
      }
    }

    fn field(&mut self, _of: &Value, name: &str) -> Held {
      match name {
        "lines" => Held::Is(Value::List(vec![Value::Str("one".to_owned()), Value::Str("two".to_owned())])),
        _ => Held::Nothing,
      }
    }
  }

  #[test]
  fn the_word_of_a_rung_runs_in_the_sandbox_and_calls_the_verbs_of_the_engine() {
    let mut sandbox = Monty::new(Engine::default());
    let step = sandbox.begin("rung://operator.1.1", "chain://operator.1", "close(1)");
    assert_eq!(step, Step::Ran(None));
    assert_eq!(sandbox.reach.calls, vec![("close".to_owned(), vec![Value::Int(1)])]);
  }

  #[test]
  fn what_a_rung_binds_stays_bound_for_every_later_rung_of_the_chain() {
    let mut sandbox = Monty::new(Engine::default());
    assert_eq!(sandbox.begin("rung://operator.1.1", "chain://operator.1", "k = 41"), Step::Ran(None));
    assert_eq!(sandbox.begin("rung://operator.1.2", "chain://operator.1", "close(k + 1)"), Step::Ran(None));
    assert_eq!(sandbox.reach.calls, vec![("close".to_owned(), vec![Value::Int(42)])]);
  }

  #[test]
  fn a_word_reads_a_field_of_what_a_verb_gave_it() {
    let mut sandbox = Monty::new(Engine::default());
    let word = "t = read('a.txt')\nclose(len(t.lines))";
    assert_eq!(sandbox.begin("rung://operator.1.1", "chain://operator.1", word), Step::Ran(None));
    // len is the sandbox's own, so the word never asks the engine for it.
    assert_eq!(sandbox.reach.calls[0].0, "read");
    assert_eq!(sandbox.reach.calls[1], ("close".to_owned(), vec![Value::Int(2)]));
  }

  #[test]
  fn the_word_of_a_rung_runs_to_its_next_await_and_continues_when_what_it_waits_for_comes() {
    let mut sandbox = Monty::new(Engine::default());
    let word = "x = bash('echo hi')\nclose((await x).code)";
    let step = sandbox.begin("rung://operator.1.1", "chain://operator.1", word);
    assert_eq!(step, Step::Wants("bash://operator.1.1.1".to_owned()));
    let exit = Value::Shape {
      name: "Exit".to_owned(),
      fields: vec![
        ("code".to_owned(), Value::Int(0)),
        ("stdout".to_owned(), Value::text("out", "ran echo hi\n")),
        ("stderr".to_owned(), Value::text("err", "")),
      ],
    };
    assert_eq!(sandbox.carry("rung://operator.1.1", &exit), Step::Ran(None));
    assert_eq!(sandbox.reach.calls[1], ("close".to_owned(), vec![Value::Int(0)]));
  }

  #[test]
  fn a_word_that_raises_is_over_with_what_it_raised() {
    let mut sandbox = Monty::new(Engine::default());
    let step = sandbox.begin("rung://operator.1.1", "chain://operator.1", "raise ValueError('boom')");
    match step {
      Step::Ran(Some(Value::Error { name, args })) => {
        assert_eq!(name, "ValueError");
        assert!(args[0].as_str().unwrap_or_default().contains("boom"));
      }
      other => panic!("a word that raises is over with what it raised, and gave {other:?}"),
    }
  }
}
