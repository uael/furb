//! The sandbox: one session of monty, which is where the engine runs.
//!
//! Monty is a python interpreter written in rust that runs untrusted code, which is what the word of a model is.
//! One [`Sand`] holds one session, and that session holds one life: the engine, the sheet and the Kernel, each in a
//! namespace of its own, and the word of every rung in the module of its chain.
//!
//! Monty runs a piece of code until it is done or until it needs the host: a name it cannot resolve, a call of a
//! method of an object of the host or of a function of the host, a future only the host can settle. The host answers
//! each call with a [`Host`], and while it answers, it may call into the sandbox again through the same [`Sand`], to
//! any depth, since the session holds each call of the host that waits. A value of the sandbox with no data form, and
//! a class of the sandbox, crosses to the host as a handle, which the session holds until the host releases it.

use monty::{MontyRepl, ReplFunctionCall, ReplProgress, ReplStartError};
use monty_types::{
  CallArgs, CompileOptions, ExcType, MontyException, MontyUuid, NameLookupResult, NamedValues,
  PrintWriter, ResourceLimits, ResourceTracker,
};

use crate::value::{Fault, Object};

/// The name of the function of the host that an entry calls first, whose answer is the body of the entry.
const ENTER: &str = "__enter";

/// What answers the sandbox while its code runs.
pub(crate) trait Host {
  /// One call of the sandbox, answered: the object of the host it is on, by its id, the method or function it called,
  /// and what it handed over. The host may call into the sandbox with `sand` while it answers.
  fn answer(
    &mut self,
    sand: &mut Sand,
    on: Option<MontyUuid>,
    name: &str,
    args: Vec<Object>,
  ) -> Answer;
}

/// What the host answers a call of the sandbox with.
pub(crate) enum Answer {
  /// What the call gave.
  Value(Object),
  /// An exception of the sandbox, by its class and its fields, raised where the call was made.
  Raise(Object),
  /// A fault, raised where the call was made as the builtin exception it names, or as a runtime error.
  Fault(Fault),
  /// A fault that ends the run as the builtin exception it names, or as a runtime error, which nothing in the
  /// sandbox catches.
  Abort(Fault),
}

/// One monty session, which is one life.
pub(crate) struct Sand {
  /// The session between two runs.
  idle: Option<MontyRepl>,
  /// The call of the sandbox that the host answers now, while a run goes on; the session holds each call it waits on
  /// beneath it.
  answering: Option<ReplFunctionCall>,
  /// Why the run ended while the host answered a call, which ended every call it answered with it.
  ended: Option<Fault>,
  /// How many runs the session began. It holds a name for each one until the life ends.
  fed: usize,
}

impl Sand {
  /// A sandbox with the limits of monty, which holds handles, and nothing in it yet.
  pub(crate) fn new() -> Self {
    let limits = ResourceTracker::new(ResourceLimits::default());
    let repl = MontyRepl::new("furb", limits, CompileOptions::default()).with_handles();
    Self { idle: Some(repl), answering: None, ended: None, fed: 0 }
  }

  /// How many runs the session began.
  #[cfg_attr(not(test), allow(dead_code))]
  pub(crate) fn fed(&self) -> usize {
    self.fed
  }

  /// One piece of code, run in the sandbox with these names bound, and what its last expression gave.
  ///
  /// The host answers every call the code makes while it runs. What the code raises is the fault.
  pub(crate) fn run(
    &mut self,
    code: &str,
    inputs: NamedValues,
    host: &mut dyn Host,
  ) -> Result<Object, Fault> {
    self.fed += 1;
    self.ended = None;
    let repl = self.idle.take().expect("a sandbox holds its session between two runs");
    let step = repl.feed_start(code, inputs, PrintWriter::Disabled);
    self.drive(step, host)
  }

  /// One entry of the host into the sandbox, which is a run of its own: `body` runs as the answer of the one call
  /// the run makes, so every call it makes into the sandbox is a call before that one, and the session compiles one
  /// snippet for the whole entry. What `body` gave is the entry's, and the run gives nothing back.
  pub(crate) fn enter<H: Host, T>(
    &mut self,
    host: &mut H,
    body: impl FnOnce(&mut Sand, &mut H) -> T,
  ) -> Result<T, Fault> {
    let mut entering = Entering { body: Some(body), host, got: None };
    let mut inputs = NamedValues::new();
    inputs.push(ENTER, Object::function(ENTER, None));
    self.run(&format!("{ENTER}()"), inputs, &mut entering)?;
    Ok(entering.got.expect("an entry runs its body before its run is done"))
  }

  /// A callable of the sandbox, called with these words while the host answers a call, and what it gave.
  pub(crate) fn call(
    &mut self,
    host: &mut dyn Host,
    callable: &Object,
    args: Vec<Object>,
    kwargs: Vec<(&str, Object)>,
  ) -> Result<Object, Fault> {
    let Some(call) = self.answering.take() else {
      return Err(
        self
          .ended
          .clone()
          .expect("the host calls into the sandbox inside an entry, until its run ends"),
      );
    };
    let kwargs = kwargs.into_iter().map(|(key, value)| (Object::string(key), value)).collect();
    let step =
      call.call_first(callable.clone(), CallArgs::from((args, kwargs)), PrintWriter::Disabled);
    self.drive(step, host)
  }

  /// The host holds the handle `id` no more, so the session lets go of what it held under it.
  pub(crate) fn release(&mut self, id: &MontyUuid) {
    if let Some(call) = self.answering.as_mut() {
      call.release(id);
    } else if let Some(repl) = self.idle.as_mut() {
      repl.release(id);
    } else {
      unreachable!("the host releases a handle between two runs or while it answers a call");
    }
  }

  /// A run driven until it is done, or until the call the host made first returns: each call of the sandbox is
  /// answered by the host, with the call standing where the host reaches it.
  fn drive(
    &mut self,
    mut step: Result<ReplProgress, Box<ReplStartError>>,
    host: &mut dyn Host,
  ) -> Result<Object, Fault> {
    loop {
      let progress = match step {
        Ok(progress) => progress,
        Err(failed) => return Err(self.failed(*failed)),
      };
      step = match progress {
        ReplProgress::Complete { repl, value } => {
          self.idle = Some(repl);
          return Ok(value);
        }
        ReplProgress::Returned { result, call } => {
          self.answering = Some(call);
          return result.map_err(|error| fault(&error));
        }
        ReplProgress::FunctionCall(call) => {
          let args: Vec<Object> = call.args.args().map(|one| one.to_owned()).collect();
          let (on, name) = (call.object_id, call.function_name.clone());
          self.answering = Some(call);
          let answer = host.answer(self, on, &name, args);
          // The call is gone when the run ended while the host answered it.
          let Some(call) = self.answering.take() else {
            return Err(self.ended.clone().expect("a run that ended says why"));
          };
          match answer {
            Answer::Value(value) => call.resume(value, PrintWriter::Disabled),
            Answer::Raise(exception) => call.raise(exception, PrintWriter::Disabled),
            Answer::Fault(fault) => call.resume(raised(&fault), PrintWriter::Disabled),
            Answer::Abort(fault) => call.abort(raised(&fault), PrintWriter::Disabled),
          }
        }
        // A name the sandbox cannot resolve is undefined, and so is an attribute of an object of the host that the
        // sandbox does not hold.
        ReplProgress::NameLookup(asked) => {
          asked.resume(NameLookupResult::Undefined, PrintWriter::Disabled)
        }
        // A life reaches neither: nothing in the sandbox opens a file, and what waits, waits on the host through a
        // fact, which is the engine's way and not a future of the interpreter.
        ReplProgress::OsCall(call) => {
          call.abort(refusal("a life asks nothing of the operating system"), PrintWriter::Disabled)
        }
        ReplProgress::ResolveFutures(waiting) => waiting.abort(
          refusal("a life waits by a fact and not by a future of the host"),
          PrintWriter::Disabled,
        ),
      };
    }
  }

  /// What the sandbox raised, as the fault a host reads, with the session kept for the next run. Every call the host
  /// was answering ended with the run.
  fn failed(&mut self, failed: ReplStartError) -> Fault {
    self.idle = Some(failed.repl);
    self.answering = None;
    let fault = fault(&failed.error);
    self.ended = Some(fault.clone());
    fault
  }
}

/// What the sandbox raised, as a fault: its name, the name of the class of the session it was raised from when it was
/// one, and what it says.
fn fault(error: &MontyException) -> Fault {
  Fault::new(error.type_name(), vec![Object::string(error.message().unwrap_or_default())])
}

/// What a life is refused for asking something no life asks.
fn refusal(why: &str) -> MontyException {
  MontyException::new(ExcType::RuntimeError, Some(why.to_owned()))
}

/// A fault of the host, raised where the sandbox called: the builtin exception it names, with its message, or a
/// runtime error that names it, since a class of the host is no class the sandbox holds.
fn raised(fault: &Fault) -> MontyException {
  match fault.name.parse::<ExcType>() {
    Ok(kind) => {
      MontyException::new(kind, Some(fault.message()).filter(|message| !message.is_empty()))
    }
    Err(_) => MontyException::new(ExcType::RuntimeError, Some(fault.to_string())),
  }
}

/// An object of the host, as the sandbox holds one: an instance of a class the host defined, under this id, whose
/// methods the sandbox calls by name and the host answers.
pub(crate) fn object(class: &str, id: MontyUuid) -> Object {
  let class = Object::class_type(class, id, true, false, Vec::<(Object, Object)>::new());
  Object::class_instance(class, id, Vec::<(Object, Object)>::new())
}

/// An id of the host, from a number: each object of the host that the sandbox holds, and each value the host hands in
/// by its fields, stands under one of its own.
pub(crate) fn id(of: u128) -> MontyUuid {
  MontyUuid::try_from_slice(&of.to_be_bytes()).expect("sixteen bytes are an id")
}

/// The host of an entry: its body answers the first call, and the host answers every other.
struct Entering<'a, H, F, T> {
  body: Option<F>,
  host: &'a mut H,
  got: Option<T>,
}

impl<H: Host, F: FnOnce(&mut Sand, &mut H) -> T, T> Host for Entering<'_, H, F, T> {
  fn answer(
    &mut self,
    sand: &mut Sand,
    on: Option<MontyUuid>,
    name: &str,
    args: Vec<Object>,
  ) -> Answer {
    match self.body.take() {
      Some(body) if on.is_none() && name == ENTER => {
        self.got = Some(body(sand, self.host));
        Answer::Value(Object::none())
      }
      body => {
        self.body = body;
        self.host.answer(sand, on, name, args)
      }
    }
  }
}

/// A host that answers no call: a call of the sandbox ends the run with a refusal that names what called.
pub(crate) struct Nobody(pub &'static str);

impl Host for Nobody {
  fn answer(&mut self, _: &mut Sand, _: Option<MontyUuid>, name: &str, _: Vec<Object>) -> Answer {
    Answer::Abort(Fault::refused(format!("{} calls no host, and it called {name}", self.0)))
  }
}
