//! The thread of JavaScript, as the door reaches it: a value of JavaScript as the engine takes it, a generator of
//! JavaScript heard as an ear, a function of JavaScript called back, and one engine held there and driven.
//!
//! What JavaScript throws is caught where the door calls it, so a generator or a function that throws gives the
//! value it threw, which the engine reads as the fault it is: a map that names its class under `is` with what it
//! was made with, or an error by its message.

use std::{
  cell::{OnceCell, RefCell},
  ptr,
  rc::{Rc, Weak},
  sync::Arc,
  task::{Context, Wake, Waker},
};

use futures::FutureExt;
use napi::{
  Env, JsDeferred, JsValue, ValueType,
  bindgen_prelude::{
    FromNapiValue, FunctionRef, JsObjectValue, Object as JsObject, ObjectRef, ToNapiValue, Unknown,
  },
  sys,
  threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode},
};
use serde_json::Value as Json;
use unsync::oneshot;

use super::{JsAct, native, refused};
use crate::{
  Ear, Engine, Fault, Heard, Object, Step,
  engine::{callable, handed},
  value::{field, templated},
  wire,
};

/// How a value of JavaScript goes in where the contract says its type: as it is, as a text, which JavaScript writes
/// as its path and its content, or as a template, which it writes as each expression beside its value.
#[derive(Clone, Copy)]
pub enum Word {
  Plain,
  Text,
  Template,
}

/// The deepest a value of JavaScript goes in, which a value that holds itself would pass.
const DEEPEST: usize = 64;

/// A function of JavaScript called, as a method of `this` when it is one, with these arguments: what it gave, or
/// the fault of what it threw.
fn call<'env>(
  env: &'env Env,
  f: Unknown<'env>,
  this: Option<Unknown<'env>>,
  args: Vec<Unknown<'env>>,
) -> Result<Unknown<'env>, Fault> {
  let this = match this {
    Some(this) => this.raw(),
    None => ().into_unknown(env).map_err(refused)?.raw(),
  };
  let args: Vec<sys::napi_value> = args.iter().map(JsValue::raw).collect();
  let mut got = ptr::null_mut();
  // SAFETY: every value is one of this env, and the call runs on its thread.
  let status = unsafe {
    sys::napi_call_function(env.raw(), this, f.raw(), args.len(), args.as_ptr(), &mut got)
  };
  if status == sys::Status::napi_pending_exception {
    let mut thrown = ptr::null_mut();
    // SAFETY: an exception is pending in this env, which this takes and clears.
    unsafe { sys::napi_get_and_clear_last_exception(env.raw(), &mut thrown) };
    // SAFETY: the thrown value is one of this env.
    return Err(fault(env, unsafe { Unknown::from_raw_unchecked(env.raw(), thrown) }));
  }
  if status != sys::Status::napi_ok {
    return Err(Fault::refused(format!("a call of JavaScript failed: {status}")));
  }
  // SAFETY: the value the call gave is one of this env.
  let got = unsafe { Unknown::from_raw_unchecked(env.raw(), got) };
  if got.is_promise().unwrap_or_default() {
    return Err(Fault::refused(
      "an ear or a function of the host answers at once, and this one gave a promise",
    ));
  }
  Ok(got)
}

/// What JavaScript threw, as the fault it is: a map that names its class under `is` with what it was made with,
/// or an error by its message.
fn fault(env: &Env, no: Unknown<'_>) -> Fault {
  if let Ok(held) = inward(env, no, Word::Plain, 0)
    && let Some(fault) = marked(&held)
  {
    return fault;
  }
  let text = |one: Unknown<'_>| {
    one
      .coerce_to_string()
      .ok()
      .and_then(|one| one.into_utf8().ok())
      .and_then(|one| one.into_owned().ok())
  };
  let message = no
    .coerce_to_object()
    .ok()
    .filter(|one| one.has_named_property("message").unwrap_or_default())
    .and_then(|one| one.get_named_property::<Unknown>("message").ok())
    .and_then(text);
  Fault::refused(message.or_else(|| text(no)).unwrap_or_default())
}

/// A fault that JavaScript threw as a map that names its class under `is` with what it was made with.
fn marked(held: &Object) -> Option<Fault> {
  let held = held.as_ref();
  let name = field(&held, "is")?.as_str()?.to_owned();
  let args = field(&held, "args")?.items()?.into_iter().map(|one| one.to_owned()).collect();
  Some(Fault::new(name, args))
}

/// A value of JavaScript, as the engine takes it: a function as a function of the host, a generator as an ear, an
/// ear of the crate as itself, each handed over to the life it comes into, an act as its name, and plain data as the
/// wire reads it, each entry the same way.
pub fn inward(env: &Env, value: Unknown<'_>, word: Word, depth: usize) -> Result<Object, Fault> {
  if depth > DEEPEST {
    return Err(Fault::refused(format!("a value deeper than {DEEPEST} does not go in")));
  }
  match value.get_type().map_err(refused)? {
    ValueType::Undefined => return Ok(Object::none()),
    ValueType::Function => {
      let held =
        value.coerce_to_object().and_then(|one| one.create_ref::<false>()).map_err(refused)?;
      let called = Called { env: *env, held: Some(held) };
      return Ok(callable(move |args| called.call(args)));
    }
    ValueType::Object => {}
    _ => {
      // SAFETY: the value is one of this env, read as the plain value napi reads a parameter as, whose numbers
      // keep what JavaScript holds.
      let plain = unsafe { Json::from_napi_value(env.raw(), value.raw()) }.map_err(refused)?;
      return wire::inward(&plain);
    }
  }
  let object = value.coerce_to_object().map_err(refused)?;
  if let Some(ear) = native(env, &object)? {
    return Ok(handed(ear, false));
  }
  if let Some(id) = JsAct::named(env, &object)? {
    return Ok(Object::string(id));
  }
  if object.is_promise().unwrap_or_default() {
    return Err(Fault::refused("a promise does not go in: await it first"));
  }
  let callable = |name: &str| {
    object.has_named_property(name).unwrap_or_default()
      && object.get_named_property::<Unknown>(name).ok().and_then(|one| one.get_type().ok())
        == Some(ValueType::Function)
  };
  if callable("next") && callable("throw") {
    // JavaScript tells no one whether a generator was started, so one that crosses was not.
    return Ok(handed(ear(env, &object)?, false));
  }
  if object.is_array().map_err(refused)? {
    let mut held = Vec::new();
    for i in 0..object.get_array_length().map_err(refused)? {
      let one: Unknown = object.get_element(i).map_err(refused)?;
      held.push(inward(env, one, Word::Plain, depth + 1)?);
    }
    if let Word::Template = word {
      let pairs = held.into_iter().map(|pair| {
        let items = pair.as_ref().items().unwrap_or_default();
        let at = |i: usize| items.get(i).map_or_else(Object::none, |one| one.to_owned());
        (at(1), at(0))
      });
      return Ok(templated(pairs));
    }
    return Ok(Object::list(held));
  }
  let names = object.get_property_names().map_err(refused)?;
  let mut pairs = Vec::new();
  if let Word::Text = word
    && !object.has_named_property("is").map_err(refused)?
  {
    pairs.push((Object::string("is"), Object::string("Text")));
  }
  for i in 0..names.get_array_length().map_err(refused)? {
    let key: String = names.get_element(i).map_err(refused)?;
    let one: Unknown = object.get_named_property(&key).map_err(refused)?;
    if one.get_type().map_err(refused)? == ValueType::Undefined {
      continue;
    }
    pairs.push((Object::string(key), inward(env, one, Word::Plain, depth + 1)?));
  }
  Ok(Object::dict(pairs))
}

/// The words of an object of options, by the keys of the verb and as each goes in.
pub fn named(
  env: &Env,
  options: JsObject<'_>,
  keys: &[(&str, Word)],
) -> Result<Vec<(String, Object)>, Fault> {
  let names = options.get_property_names().map_err(refused)?;
  let mut named = Vec::new();
  for i in 0..names.get_array_length().map_err(refused)? {
    let key: String = names.get_element(i).map_err(refused)?;
    let Some((_, word)) = keys.iter().find(|(name, _)| *name == key) else {
      return Err(Fault::refused(format!("{key} is no word of this verb")));
    };
    let one: Unknown = options.get_named_property(&key).map_err(refused)?;
    if matches!(one.get_type().map_err(refused)?, ValueType::Undefined | ValueType::Null) {
      continue;
    }
    named.push((key, inward(env, one, *word, 0)?));
  }
  Ok(named)
}

/// A generator of JavaScript, heard as an ear.
pub fn ear(env: &Env, generator: &JsObject<'_>) -> Result<Box<dyn Ear>, Fault> {
  Ok(Box::new(JsEar { env: *env, generator: Some(generator.create_ref().map_err(refused)?) }))
}

/// A function of JavaScript, called back by the sandbox with what the word gave it.
struct Called {
  env: Env,
  held: Option<ObjectRef<false>>,
}

impl Called {
  fn call(&self, args: Vec<Object>) -> Result<Object, Fault> {
    let env = &self.env;
    let held = self.held.as_ref().ok_or_else(|| Fault::refused("the function is gone"))?;
    let f = held.get_value(env).map_err(refused)?.to_unknown();
    let args = args.iter().map(|one| outward(env, one)).collect::<Result<Vec<_>, _>>()?;
    inward(env, call(env, f, None, args)?, Word::Plain, 0)
  }
}

impl Drop for Called {
  fn drop(&mut self) {
    if let Some(held) = self.held.take() {
      let _ = held.unref(&self.env);
    }
  }
}

/// A value of the engine, as JavaScript reads it.
fn outward<'env>(env: &'env Env, value: &Object) -> Result<Unknown<'env>, Fault> {
  env.to_js_value(&wire::outward(value.as_ref())).map_err(refused)
}

/// A generator of JavaScript, heard as an ear: each step of it taken where what it throws is caught.
struct JsEar {
  env: Env,
  generator: Option<ObjectRef<false>>,
}

impl Drop for JsEar {
  fn drop(&mut self) {
    if let Some(held) = self.generator.take() {
      let _ = held.unref(&self.env);
    }
  }
}

impl Ear for JsEar {
  fn resume(&mut self, heard: Heard, _: &mut Context<'_>) -> Step {
    self.stepped(heard).unwrap_or_else(Step::Raised)
  }
}

impl JsEar {
  fn stepped(&mut self, heard: Heard) -> Result<Step, Fault> {
    let env = &self.env;
    let held = self.generator.as_ref().ok_or_else(|| Fault::refused("the ear is over"))?;
    let generator = held.get_value(env).map_err(refused)?;
    // What a verb gave is the value of the yield that called it, and what it raised is thrown in there.
    let (way, args) = match heard {
      Heard::Born => ("next", vec![]),
      Heard::Fact(fact) => ("next", vec![outward(env, &fact.0)?]),
      Heard::Answer(Ok(got)) => ("next", vec![outward(env, &got)?]),
      Heard::Answer(Err(fault)) => ("throw", vec![outward(env, &fault.object())?]),
    };
    let next: Unknown = generator.get_named_property(way).map_err(refused)?;
    let got = match call(env, next, Some(generator.to_unknown()), args) {
      Ok(got) => got.coerce_to_object().map_err(refused)?,
      Err(fault) => return Ok(Step::Raised(fault)),
    };
    if got.get_named_property::<bool>("done").unwrap_or_default() {
      return Ok(Step::Over);
    }
    let value: Unknown = got.get_named_property("value").map_err(refused)?;
    Ok(Step::of(&inward(env, value, Word::Plain, 0)?).unwrap_or_else(Step::Raised))
  }
}

type Resolver = Box<dyn FnOnce(Env) -> napi::Result<Json>>;

/// A result that JavaScript awaits: what the act comes to, which its watcher sends, and the promise it settles. The
/// watcher goes with the engine, so a result whose engine was disposed comes to nothing.
struct Pending {
  came: oneshot::Receiver<Object>,
  deferred: JsDeferred<Json, Resolver>,
}

/// A function of JavaScript that another thread may call, which keeps JavaScript alive when it is strong.
type Waking<const WEAK: bool> = ThreadsafeFunction<(), (), (), napi::Status, false, WEAK>;

/// What wakes the thread of JavaScript to drive the engine, when what an ear waits for is ready.
struct Wakes(Waking<true>);

impl Wake for Wakes {
  fn wake(self: Arc<Self>) {
    self.0.call((), ThreadsafeFunctionCallMode::NonBlocking);
  }
}

/// One engine, held on the thread of JavaScript, the results that JavaScript awaits of it, and what wakes the thread
/// when an ear can go on.
pub struct Held {
  pub engine: RefCell<Option<Engine>>,
  env: Env,
  pending: RefCell<Vec<Pending>>,
  wakes: Arc<Wakes>,
  /// The function that drives the engine, from which the one that keeps JavaScript alive is made.
  drives: FunctionRef<(), ()>,
  /// What keeps JavaScript alive while it awaits a result, which only an ear that waits may give.
  alive: RefCell<Option<Waking<false>>>,
}

impl Held {
  pub fn new(env: &Env, engine: Engine) -> napi::Result<Rc<Held>> {
    // The function that drives the engine is made before what it drives, which it is given once that stands.
    let slot: Rc<OnceCell<Weak<Held>>> = Rc::default();
    let driven = Rc::clone(&slot);
    let drive = env.create_function_from_closure("drive", move |_| {
      if let Some(held) = driven.get().and_then(Weak::upgrade) {
        held.drive();
      }
      Ok(())
    })?;
    let wakes =
      drive.build_threadsafe_function::<()>().callee_handled::<false>().weak::<true>().build()?;
    let held = Rc::new(Held {
      engine: RefCell::new(Some(engine)),
      env: *env,
      pending: RefCell::default(),
      wakes: Arc::new(Wakes(wakes)),
      drives: drive.create_ref()?,
      alive: RefCell::default(),
    });
    let _ = slot.set(Rc::downgrade(&held));
    held.drive();
    Ok(held)
  }

  /// The engine driven as far as it goes: what its ears say of their own accord is said into it, and each awaited
  /// result told.
  pub fn drive(&self) {
    let waker = Waker::from(Arc::clone(&self.wakes));
    let _ = self.call(|engine| engine.pump(&waker));
  }

  /// Each awaited result that came settles its promise, and JavaScript is kept alive while it awaits a result, and
  /// let go when it awaits none.
  fn kept(&self) {
    let pending = std::mem::take(&mut *self.pending.borrow_mut());
    for mut one in pending {
      match (&mut one.came).now_or_never() {
        None => self.pending.borrow_mut().push(one),
        Some(Some(value)) => match Fault::of(value.as_ref()) {
          Some(fault) => one.deferred.reject(fault.into()),
          None => {
            let value = wire::outward(value.as_ref());
            one.deferred.resolve(Box::new(move |_| Ok(value)));
          }
        },
        Some(None) => {
          one.deferred.reject(napi::Error::from_reason("CancelledError: the engine was disposed"));
        }
      }
    }
    let awaited = !self.pending.borrow().is_empty();
    let mut alive = self.alive.borrow_mut();
    if !awaited {
      alive.take();
    } else if alive.is_none() {
      *alive = self
        .drives
        .borrow_back(&self.env)
        .and_then(|drive| {
          drive.build_threadsafe_function::<()>().callee_handled::<false>().weak::<false>().build()
        })
        .ok();
    }
  }

  /// One call of the engine, which JavaScript makes only while the engine runs none of its code.
  pub fn call<T>(&self, call: impl FnOnce(&mut Engine) -> Result<T, Fault>) -> napi::Result<T> {
    let mut held = self.engine.try_borrow_mut().map_err(|_| {
      napi::Error::from_reason(
        "The engine runs code of JavaScript now: an ear calls a verb by yielding call(verb, args, kwargs), and a \
         function answers from its words alone.",
      )
    })?;
    let engine =
      held.as_mut().ok_or_else(|| napi::Error::from_reason("The engine is disposed."))?;
    let got = call(engine);
    drop(held);
    self.kept();
    Ok(got?)
  }

  /// What an act comes to, as a promise of JavaScript.
  pub fn result<'env>(&self, env: &'env Env, id: &str) -> napi::Result<JsObject<'env>> {
    let (deferred, promise) = env.create_deferred::<Json, Resolver>()?;
    let (told, came) = oneshot::channel();
    let mut told = Some(told);
    let watched = self.call(|engine| {
      engine.watch(id, move |value| {
        if let Some(told) = told.take() {
          let _ = told.send(value.clone());
        }
      })
    });
    match watched {
      Ok(()) => {
        self.pending.borrow_mut().push(Pending { came, deferred });
        self.kept();
      }
      Err(error) => deferred.reject(error),
    }
    Ok(promise)
  }

  pub fn dispose(&self) -> napi::Result<()> {
    let mut engine = self
      .engine
      .try_borrow_mut()
      .map_err(|_| napi::Error::from_reason("The engine is in a call of the host."))?;
    engine.take();
    drop(engine);
    self.kept();
    Ok(())
  }
}

impl Drop for Held {
  fn drop(&mut self) {
    let _ = self.dispose();
  }
}
