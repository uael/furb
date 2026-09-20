//! A [`Session`] of monty: the sandbox the engine is meant to run in.
//!
//! Monty is a python interpreter written in rust that runs untrusted code, which is what the word of a model is.
//! One [`Sand`] holds one monty session, and that session holds one life: the preamble in a module of its own,
//! the engine in another, and the word of every rung in the module of its chain. Nothing of the host is in there.
//!
//! The one name the sandbox reaches out by is [`HOST`]. Monty has no such function, so it asks for the name and
//! then asks for every call of it, and the loop below answers both from the [`Host`] the caller handed in. That
//! is the whole of the boundary: one name and one plain value go in, and one plain value comes back.

use monty::{MontyRepl, ReplProgress, ReplStartError};
use monty_types::{
  CallArgs, CompileOptions, ExcType, MontyException, MontyObject, NameLookupResult, NamedValues, ObjectRef,
  PrintWriter, ResourceLimits, ResourceTracker,
};

use crate::{
  fact::Value,
  life::{HOST, Host, Session},
};

/// One monty session, which is one life.
///
/// The session stands from one call to the next, so what the preamble binds the engine reads, and what a word of
/// a rung binds its chain reads. It is made empty: a caller runs the preamble and the engine through it in the
/// order [`Life`](crate::life::Life) runs them.
pub struct Sand {
  /// The session, which is taken out for the length of a run and put back when the run is over.
  ///
  /// Monty hands the session to whatever is driving a snippet and gives it back when the snippet is done, so it
  /// is absent only while [`Session::run`] is between two steps of one run.
  repl: Option<MontyRepl>,
}

impl Sand {
  /// A sandbox with the limits a host chooses, and nothing in it yet.
  ///
  /// The limits are monty's: how much memory a life may hold, how long it may run, and how many times it may
  /// suspend. They bound the word of a model, which is the reason the engine runs in a sandbox at all.
  #[must_use]
  pub fn new(limits: ResourceLimits) -> Self {
    Self { repl: Some(MontyRepl::new("furb", ResourceTracker::new(limits), CompileOptions::default())) }
  }

  /// The session, which is there except between two steps of one run.
  fn repl(&mut self) -> MontyRepl {
    self.repl.take().expect("a sandbox holds its session between two runs")
  }
}

impl Default for Sand {
  fn default() -> Self {
    Self::new(ResourceLimits::default())
  }
}

impl Session for Sand {
  fn run(&mut self, code: &str, host: &mut dyn Host) -> Result<Value, Value> {
    let mut step = self.repl().feed_start(code, NamedValues::default(), PrintWriter::Disabled);
    loop {
      let progress = match step {
        Ok(progress) => progress,
        Err(failed) => return Err(self.failed(*failed)),
      };
      match progress {
        ReplProgress::Complete { repl, value } => {
          self.repl = Some(repl);
          return Ok(of_monty(value.as_ref()));
        }
        // Monty has no `host`, so it asks who does before the first call of it. The name is bound to a function
        // of the host, and every call of that function comes back as the arm below.
        ReplProgress::NameLookup(asked) => {
          let found = if asked.name == HOST {
            NameLookupResult::Value(MontyObject::function(HOST, None))
          } else {
            NameLookupResult::Undefined
          };
          step = asked.resume(found, PrintWriter::Disabled);
        }
        ReplProgress::FunctionCall(call) => {
          let Some((name, said)) = called(&call.args) else {
            let wrong =
              MontyException::new(ExcType::TypeError, Some("host() takes a name and one plain value".to_owned()));
            step = call.abort(wrong, PrintWriter::Disabled);
            continue;
          };
          let answer = host.called(&name, &said);
          step = call.resume(to_monty(&answer), PrintWriter::Disabled);
        }
        // A life reaches neither: nothing in the sandbox opens a file or waits on a future of the host, since
        // every question of the outside goes through `host` and is answered where it is asked.
        ReplProgress::OsCall(call) => {
          let refused = refusal("a life asks nothing of the operating system");
          step = call.abort(refused, PrintWriter::Disabled);
        }
        ReplProgress::ResolveFutures(waiting) => {
          let refused = refusal("a life waits on no future of the host");
          step = waiting.abort(refused, PrintWriter::Disabled);
        }
      }
    }
  }
}

impl Sand {
  /// What the sandbox raised, as the plain exception a host reads, with the session kept for the next run.
  ///
  /// A fault of a run is the life's and not the session's: the engine goes on, so the session comes back from the
  /// failure and is put away for the call after this one.
  fn failed(&mut self, failed: ReplStartError) -> Value {
    self.repl = Some(failed.repl);
    Value::Error {
      name: failed.error.exc_type().to_string(),
      args: vec![Value::Str(failed.error.message().unwrap_or_default().to_owned())],
    }
  }
}

/// What a life is refused for asking something no life asks.
fn refusal(why: &str) -> MontyException {
  MontyException::new(ExcType::RuntimeError, Some(why.to_owned()))
}

/// The two arguments of one `host(name, said)` call, or nothing for a call of another shape.
fn called(args: &CallArgs) -> Option<(String, Value)> {
  let mut given = args.args();
  let name = given.next()?.as_str()?.to_owned();
  let said = of_monty(given.next()?);
  given.next().is_none().then_some((name, said))
}

/// One value of the sandbox, read as the plain form it is.
///
/// What crosses is what the preamble made plain, so it is data and nothing else: a scalar, a list, or a map. The
/// marks a tuple, a shape and an exception carry are read by [`Value::of_plain`], where every boundary reads
/// them, so this only says what the shape of the data is.
fn of_monty(said: ObjectRef<'_>) -> Value {
  let read = match said.type_name() {
    "bool" => said.as_bool().map(Value::Bool),
    "int" => said.as_int().map(Value::Int),
    "float" => said.as_float().map(Value::Float),
    "str" => said.as_str().map(|held| Value::Str(held.to_owned())),
    "list" | "tuple" => said.items().map(|held| Value::List(held.into_iter().map(of_monty).collect())),
    "dict" => said.pairs().map(|held| {
      Value::Map(
        held.into_iter().map(|(key, one)| (key.as_str().unwrap_or_default().to_owned(), of_monty(one))).collect(),
      )
    }),
    // `None` is one of these, and so is anything that is no plain form at all.
    _ => None,
  }
  .unwrap_or(Value::None);
  Value::of_plain(&read)
}

/// One value of a host, made plain and handed to the sandbox.
fn to_monty(said: &Value) -> MontyObject {
  match said.plain() {
    Value::None => MontyObject::none(),
    Value::Bool(held) => MontyObject::bool(held),
    Value::Int(held) => MontyObject::int(held),
    Value::Float(held) => MontyObject::float(held),
    Value::Str(held) => MontyObject::string(held),
    Value::List(held) => MontyObject::list(held.iter().map(to_monty)),
    Value::Map(held) => {
      MontyObject::dict(held.iter().map(|(key, one)| (MontyObject::string(key.clone()), to_monty(one))))
    }
    // `plain` leaves none of these, since it is what makes them a map.
    other => MontyObject::string(format!("{other:?}")),
  }
}
