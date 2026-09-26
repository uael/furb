//! The sandbox: one session of monty, which is where the engine runs.
//!
//! Monty is a python interpreter written in rust that runs untrusted code, which is what the word of a model is.
//! One [`Sand`] holds one session, and that session holds one life: the stand-in in a module of its own, the
//! engine in another, and the word of every rung in the module of its chain.
//!
//! Monty runs a piece of code until it is done or until it needs the host: a name it cannot resolve, a call of a
//! method of an object of the host or of a function of the host, a future only the host can settle. The host
//! reaches the sandbox as objects: a value that is an instance of a class the host defined, whose methods the
//! sandbox calls by name, and the loop below hands every such call to the closure the caller gave, with the id
//! of the object it is on.

use monty::{MontyRepl, ReplProgress, ReplStartError};
use monty_types::{
  CompileOptions, ExcType, MontyException, MontyUuid, NameLookupResult, NamedValues, PrintWriter,
  ResourceLimits, ResourceTracker,
};

use crate::value::{Fault, Object};

/// What answers the sandbox while its code runs: the object it called, by its id, the method or function it
/// called, and what it handed over; and what it gives back, or the fault to raise where the call was made.
pub(crate) type Answering<'a> =
  dyn FnMut(Option<MontyUuid>, &str, Vec<Object>) -> Result<Object, Fault> + 'a;

/// One monty session, which is one life.
pub(crate) struct Sand {
  /// The session, which is taken out for the length of a run and put back when the run is over.
  repl: Option<MontyRepl>,
}

impl Sand {
  /// A sandbox with the limits of monty, and nothing in it yet.
  pub(crate) fn new() -> Self {
    let limits = ResourceTracker::new(ResourceLimits::default());
    Self { repl: Some(MontyRepl::new("furb", limits, CompileOptions::default())) }
  }

  /// One piece of code, run in the sandbox with these names bound, and what its last expression gave.
  ///
  /// The host answers every call the code makes while it runs. What the code raises is the fault.
  pub(crate) fn run(
    &mut self,
    code: &str,
    inputs: NamedValues,
    host: &mut Answering<'_>,
  ) -> Result<Object, Fault> {
    let mut step = self.repl().feed_start(code, inputs, PrintWriter::Disabled);
    loop {
      let progress = match step {
        Ok(progress) => progress,
        Err(failed) => return Err(self.failed(*failed)),
      };
      match progress {
        ReplProgress::Complete { repl, value } => {
          self.repl = Some(repl);
          return Ok(value);
        }
        // A name the sandbox cannot resolve is undefined: the stand-in is given what it needs as objects, and an
        // attribute of an object of the host the sandbox does not hold is none it has.
        ReplProgress::NameLookup(asked) => {
          step = asked.resume(NameLookupResult::Undefined, PrintWriter::Disabled);
        }
        ReplProgress::FunctionCall(call) => {
          let args: Vec<Object> = call.args.args().map(|one| one.to_owned()).collect();
          step = match host(call.object_id, &call.function_name, args) {
            Ok(value) => call.resume(value, PrintWriter::Disabled),
            Err(fault) => call.abort(raised(&fault), PrintWriter::Disabled),
          };
        }
        // A life reaches neither: nothing in the sandbox opens a file, and what waits, waits on the host through
        // a fact, which is the engine's way and not a future of the interpreter.
        ReplProgress::OsCall(call) => {
          let refused = refusal("a life asks nothing of the operating system");
          step = call.abort(refused, PrintWriter::Disabled);
        }
        ReplProgress::ResolveFutures(waiting) => {
          let refused = refusal("a life waits by a fact and not by a future of the host");
          step = waiting.abort(refused, PrintWriter::Disabled);
        }
      }
    }
  }

  /// The session, which is there except between two steps of one run.
  fn repl(&mut self) -> MontyRepl {
    self.repl.take().expect("a sandbox holds its session between two runs")
  }

  /// What the sandbox raised, as the fault a host reads, with the session kept for the next run.
  fn failed(&mut self, failed: ReplStartError) -> Fault {
    self.repl = Some(failed.repl);
    Fault::new(
      failed.error.type_name(),
      vec![Object::string(failed.error.message().unwrap_or_default())],
    )
  }
}

/// What a life is refused for asking something no life asks.
fn refusal(why: &str) -> MontyException {
  MontyException::new(ExcType::RuntimeError, Some(why.to_owned()))
}

/// A fault of the host, raised where the sandbox called: a runtime error that names it, since what a host raises
/// is no class the sandbox holds.
fn raised(fault: &Fault) -> MontyException {
  MontyException::new(ExcType::RuntimeError, Some(fault.to_string()))
}

/// An object of the host, as the sandbox holds one: an instance of a class the host defined, under this id, whose
/// methods the sandbox calls by name and the host answers.
pub(crate) fn object(class: &str, id: MontyUuid) -> Object {
  let class = Object::class_type(class, id, true, false, Vec::<(Object, Object)>::new());
  Object::class_instance(class, id, Vec::<(Object, Object)>::new())
}

/// The id of an object of the host, from one byte: the crate holds a few, and each is one of a kind.
pub(crate) fn id(of: u8) -> MontyUuid {
  MontyUuid::try_from_slice(&[of; 16]).expect("sixteen bytes are an id")
}
