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

use std::{mem, time::Duration};

use monty::{MontyRepl, ReplFunctionCall, ReplProgress, ReplStartError};
use monty_types::{
  CallArgs, CompileOptions, ExcType, MontyException, MontyUuid, NameLookupResult, NamedValues,
  OsFunctionCall, PrintWriter, ResourceLimits, ResourceTracker,
};

use crate::value::{Fault, Object};

/// How many allocations the collector of cycles lets pass between two of its runs. Monty runs it after 100000
/// allocations by default, and each run walks every candidate of a cycle and all that it reaches, which in a life
/// is most of the heap.
const GC_INTERVAL: usize = 10_000_000;

/// The longest stretch of the code of the sandbox between two round trips to the host. Past it, monty raises a
/// `TimeoutError` where the code stands, which the code may catch, so its `finally` blocks run on a new stretch; a
/// second excess in the same stretch ends the run. So a word that never yields ends there with the exception, and the
/// life goes on. The longest stretch of a legal life is about 180 ms in the dev profile, over the replay of a record of
/// 96322 entries, so the bound stands far above every legal one, on a slower machine too.
pub(crate) const TURN: Duration = Duration::from_secs(30);

#[cfg(test)]
thread_local! {
  /// The bound of each sandbox that this thread makes, which a test sets short.
  pub(crate) static BOUND: std::cell::Cell<Duration> = const { std::cell::Cell::new(TURN) };
}

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

/// Where the session of a [`Sand`] stands.
enum State {
  /// The session between two runs.
  Idle(MontyRepl),
  /// The call of the sandbox that the host answers now, while a run goes on. The session holds each
  /// call it waits on beneath it.
  Answering(Box<ReplFunctionCall>),
  /// The run ended while the host answered a call, which ended every call it answered with it. The
  /// fault says why. The session waits for the next run, as it does when idle.
  Ended(MontyRepl, Fault),
  /// No session here: a run holds it, or this sandbox stands in for one that is lent to code of a
  /// host.
  Lent,
}

/// One monty session, which is one life.
pub(crate) struct Sand {
  state: State,
  /// How many runs the session began. It holds a name for each one until the life ends.
  fed: usize,
}

impl Sand {
  /// A sandbox with the limits of monty, which holds handles, and nothing in it yet.
  pub(crate) fn new() -> Self {
    let limits = ResourceLimits::default().gc_interval(GC_INTERVAL).max_turn_duration(turn());
    let limits = ResourceTracker::new(limits);
    let repl = MontyRepl::new("furb", limits, CompileOptions::default()).with_handles();
    Self { state: State::Idle(repl), fed: 0 }
  }

  /// The session between two runs, as monty dumps it.
  pub(crate) fn dump(&self) -> Result<Vec<u8>, Fault> {
    let (State::Idle(repl) | State::Ended(repl, _)) = &self.state else {
      return Err(Fault::refused("a sandbox dumps its session between two runs"));
    };
    monty::dump("furb", None, &[], monty::SessionRef::Idle(repl))
      .map_err(|no| Fault::refused(no.to_string()))
  }

  /// A sandbox on the session of a dump, between two runs. The dump holds the limits of its session, so the sandbox
  /// keeps the bound of the turn that the life that made it set.
  pub(crate) fn loaded(bytes: &[u8]) -> Result<Self, Fault> {
    let dump = monty::Dump::load(bytes).map_err(|no| Fault::refused(no.to_string()))?;
    match dump.state {
      monty::Session::Idle(repl) => Ok(Self { state: State::Idle(*repl), fed: 0 }),
      _ => Err(Fault::refused("a snapshot holds a session between two runs")),
    }
  }

  /// A sandbox of nothing, which holds no session, and stands in for one that is lent to code of a host.
  pub(crate) fn none() -> Self {
    Self { state: State::Lent, fed: 0 }
  }

  /// How many runs the session began.
  #[cfg_attr(not(test), expect(dead_code, reason = "only the tests call it"))]
  pub(crate) fn fed(&self) -> usize {
    self.fed
  }

  /// One piece of code, run in the sandbox with these names bound, and what its last expression gave.
  ///
  /// The host answers every call the code makes while it runs. What the code raises is the fault.
  pub(crate) fn run<H: Host>(
    &mut self,
    code: &str,
    inputs: NamedValues,
    host: &mut H,
  ) -> Result<Object, Fault> {
    let repl = match self.take() {
      State::Idle(repl) | State::Ended(repl, _) => repl,
      held => {
        self.state = held;
        return Err(Fault::refused("a sandbox runs only between two runs"));
      }
    };
    self.fed += 1;
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
    entering.got.ok_or_else(|| Fault::refused("an entry runs its body before its run is done"))
  }

  /// A callable of the sandbox, called with these words while the host answers a call, and what it gave.
  pub(crate) fn call<H: Host>(
    &mut self,
    host: &mut H,
    callable: &Object,
    args: Vec<Object>,
    kwargs: Vec<(&str, Object)>,
  ) -> Result<Object, Fault> {
    let call = self.taken()?;
    let kwargs = kwargs.into_iter().map(|(key, value)| (Object::string(key), value)).collect();
    let step =
      call.call_first(callable.clone(), CallArgs::from((args, kwargs)), PrintWriter::Disabled);
    self.drive(step, host)
  }

  /// The host holds the handle `id` no more, so the session lets go of what it held under it.
  /// A sandbox that stands in for a lent one holds nothing, so it lets go of nothing.
  pub(crate) fn release(&mut self, id: &MontyUuid) {
    match &mut self.state {
      State::Answering(call) => {
        call.release(id);
      }
      State::Idle(repl) | State::Ended(repl, _) => {
        repl.release(id);
      }
      State::Lent => {}
    }
  }

  /// A run driven until it is done, or until the call the host made first returns: each call of the sandbox is
  /// answered by the host, with the call standing where the host reaches it.
  fn drive<H: Host>(
    &mut self,
    mut step: Result<ReplProgress, Box<ReplStartError>>,
    host: &mut H,
  ) -> Result<Object, Fault> {
    loop {
      let progress = match step {
        Ok(progress) => progress,
        Err(failed) => return Err(self.failed(*failed)),
      };
      step = match progress {
        ReplProgress::Complete { repl, value } => {
          self.state = State::Idle(repl);
          return Ok(value);
        }
        ReplProgress::Returned { result, call } => {
          self.state = State::Answering(Box::new(call));
          return result.map_err(|error| fault(&error));
        }
        ReplProgress::FunctionCall(call) => {
          let args: Vec<Object> = call.args.args().map(|one| one.to_owned()).collect();
          let (on, name) = (call.object_id, call.function_name.clone());
          self.state = State::Answering(Box::new(call));
          let answer = host.answer(self, on, &name, args);
          // The call is gone when the run ended while the host answered it.
          let call = self.taken()?;
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
        // A module that monty does not hold is one no life holds, so its import raises `ModuleNotFoundError`, as
        // monty raises it with no host.
        ReplProgress::OsCall(call) if matches!(call.function_call, OsFunctionCall::Import(_)) => {
          let missing = call.function_call.on_no_handler();
          call.resume(missing, PrintWriter::Disabled)
        }
        // A life asks nothing of the operating system, so the call raises where the word made it: an abort would
        // end every frame of the run, the bus among them, and the life would hear nothing more.
        ReplProgress::OsCall(call) => {
          let asked: &'static str = (&call.function_call).into();
          let why = format!("{asked} asks the operating system, and a life asks nothing of it");
          call.resume(refusal(&why), PrintWriter::Disabled)
        }
        // A life reaches none: what waits, waits on the host through a fact, which is the engine's way and not a
        // future of the interpreter, and the host makes no future in the sandbox.
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
    let fault = fault(&failed.error);
    self.state = State::Ended(failed.repl, fault.clone());
    fault
  }

  /// The state of the session, which a run or a call takes out and puts back.
  fn take(&mut self) -> State {
    mem::replace(&mut self.state, State::Lent)
  }

  /// The call that the host answers now, taken out of the session, or why there is none. A run that
  /// ended says why, and the session keeps that for each later call of the same run.
  fn taken(&mut self) -> Result<ReplFunctionCall, Fault> {
    match self.take() {
      State::Answering(call) => Ok(*call),
      State::Ended(repl, why) => {
        self.state = State::Ended(repl, why.clone());
        Err(why)
      }
      held => {
        self.state = held;
        Err(Fault::refused("the host calls into the sandbox only while it answers a call"))
      }
    }
  }
}

/// The bound of the turn of a sandbox: [`TURN`], or what the test set.
fn turn() -> Duration {
  #[cfg(test)]
  let bound = BOUND.get();
  #[cfg(not(test))]
  let bound = TURN;
  bound
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
  MontyUuid::from_u128(of)
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

#[cfg(test)]
#[path = "sand.test.rs"]
mod test;
