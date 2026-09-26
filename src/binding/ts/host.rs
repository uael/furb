//! The door of TypeScript, on the thread of JavaScript: one engine held there, a value of JavaScript as the engine
//! takes it and back, a generator of JavaScript heard as an ear, and a function of JavaScript called back.
//!
//! What JavaScript throws is caught by one function of its own, so a generator or a function that throws gives the
//! value it threw, which the engine reads as the fault it is: a map that names its class under `is` with what it
//! was made with, or an error by its message.

use std::{
  cell::RefCell,
  rc::{Rc, Weak},
  sync::Arc,
  task::{Wake, Waker},
};

use napi::{
  Env, JsDeferred, JsValue, ValueType,
  bindgen_prelude::{
    FnArgs, FromNapiValue, Function, FunctionRef, JsObjectValue, Object as JsObject, ObjectRef,
    ToNapiValue, Unknown,
  },
  threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode},
};
use serde_json::Value;

use crate::{
  Ear, Engine, Fault, Heard, Object, Step,
  engine::Hosted,
  value::{field, marked},
  wire::{inward, outward},
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

/// The function of JavaScript that calls another, as a method of `self` or alone, and catches what it throws.
const CAUGHT: &str = "({ step(f, self, args) { try { return { ok: f.apply(self, args) } } catch (no) { return { no } } } })";

/// The thread of JavaScript, as the door reaches it: its env, the function that steps what may throw, and the ears
/// and the functions of the host.
#[derive(Clone)]
pub struct Door(Rc<Doorway>);

struct Doorway {
  env: Env,
  step: Option<ObjectRef<false>>,
  hosted: Hosted,
}

impl Drop for Doorway {
  fn drop(&mut self) {
    if let Some(step) = self.step.take() {
      let _ = step.unref(&self.env);
    }
  }
}

impl Door {
  /// Every ear and every function of the host that the door holds goes.
  pub fn close(&self) {
    self.0.hosted.clear();
  }

  pub fn new(env: Env, hosted: Hosted) -> napi::Result<Door> {
    let holder: JsObject<'_> = env.run_script(CAUGHT)?;
    Ok(Door(Rc::new(Doorway { env, step: Some(holder.create_ref()?), hosted })))
  }

  pub fn env(&self) -> Env {
    self.0.env
  }

  /// A function of JavaScript called, as a method of `this` when it is one, with these arguments: what it gave, or
  /// the fault of what it threw.
  fn call<'env>(
    &self,
    env: &'env Env,
    f: Unknown<'env>,
    this: Option<Unknown<'env>>,
    args: Vec<Unknown<'env>>,
  ) -> Result<Unknown<'env>, Fault> {
    let held = self.0.step.as_ref().ok_or_else(|| Fault::refused("the door is closed"))?;
    let holder = held.get_value(env).map_err(refused)?;
    let this = match this {
      Some(this) => this,
      None => ().into_unknown(env).map_err(refused)?,
    };
    let step: Function<'env, FnArgs<(Unknown, Unknown, Vec<Unknown>)>, JsObject> =
      holder.get_named_property("step").map_err(refused)?;
    let got = step.apply(holder, (f, this, args).into()).map_err(refused)?;
    if got.has_named_property("no").map_err(refused)? {
      let no: Unknown = got.get_named_property("no").map_err(refused)?;
      return Err(self.fault(env, no));
    }
    let ok: Unknown = got.get_named_property("ok").map_err(refused)?;
    if ok.is_promise().unwrap_or_default() {
      return Err(Fault::refused(
        "an ear or a function of the host answers at once, and this one gave a promise",
      ));
    }
    Ok(ok)
  }

  /// What JavaScript threw, as the fault it is: a map that names its class under `is` with what it was made with,
  /// or an error by its message.
  fn fault(&self, env: &Env, no: Unknown<'_>) -> Fault {
    if let Ok(held) = self.inward(env, no, Word::Plain, 0)
      && let Some(fault) = marked_fault(&held)
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

  /// A value of JavaScript, as the engine takes it: a function as a function of the host, a generator as an ear, an
  /// ear of the crate as itself, an act as its name, and plain data as the wire reads it, each entry the same way.
  pub fn inward(
    &self,
    env: &Env,
    value: Unknown<'_>,
    word: Word,
    depth: usize,
  ) -> Result<Object, Fault> {
    if depth > DEEPEST {
      return Err(Fault::refused(format!("a value deeper than {DEEPEST} does not go in")));
    }
    match value.get_type().map_err(refused)? {
      ValueType::Undefined => return Ok(Object::none()),
      ValueType::Function => {
        let held =
          value.coerce_to_object().and_then(|one| one.create_ref::<false>()).map_err(refused)?;
        let called = Called { door: self.clone(), held: Some(held) };
        return Ok(self.0.hosted.callable(Box::new(move |args| called.call(args))));
      }
      ValueType::Object => {}
      _ => {
        // SAFETY: the value is one of this env, read as the plain value napi reads a parameter as, whose numbers
        // keep what JavaScript holds.
        let plain = unsafe { Value::from_napi_value(env.raw(), value.raw()) }.map_err(refused)?;
        return inward(&plain);
      }
    }
    let object = value.coerce_to_object().map_err(refused)?;
    if let Some(ear) = super::NativeEar::taken(env, &object)? {
      return Ok(self.0.hosted.ear(ear, false));
    }
    if let Some(id) = super::JsAct::named(env, &object)? {
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
      return Ok(self.0.hosted.ear(self.ear(object)?, false));
    }
    if object.is_array().map_err(refused)? {
      let mut held = Vec::new();
      for i in 0..object.get_array_length().map_err(refused)? {
        let one: Unknown = object.get_element(i).map_err(refused)?;
        held.push(self.inward(env, one, Word::Plain, depth + 1)?);
      }
      if let Word::Template = word {
        let pairs = held.into_iter().map(|pair| {
          let items = pair.as_ref().items().unwrap_or_default();
          let at = |i: usize| items.get(i).map_or_else(Object::none, |one| one.to_owned());
          Object::list([at(1), at(0)])
        });
        return Ok(marked("Templated", [("interpolations", Object::list(pairs))]));
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
      pairs.push((Object::string(key), self.inward(env, one, Word::Plain, depth + 1)?));
    }
    Ok(Object::dict(pairs))
  }

  /// A generator of JavaScript that boot is given, heard as an ear.
  pub fn ear(&self, generator: JsObject<'_>) -> Result<Box<dyn Ear>, Fault> {
    Ok(Box::new(JsEar {
      door: self.clone(),
      generator: Some(generator.create_ref().map_err(refused)?),
    }))
  }

  /// The words of an object of options, by the keys of the verb and as each goes in; with no keys, every word.
  pub fn named(
    &self,
    env: &Env,
    options: JsObject<'_>,
    keys: &[(&str, Word)],
  ) -> Result<Vec<(String, Object)>, Fault> {
    let names = options.get_property_names().map_err(refused)?;
    let mut named = Vec::new();
    for i in 0..names.get_array_length().map_err(refused)? {
      let key: String = names.get_element(i).map_err(refused)?;
      let word = match keys.iter().find(|(name, _)| *name == key) {
        Some((_, word)) => *word,
        None if keys.is_empty() => Word::Plain,
        None => return Err(Fault::refused(format!("{key} is no word of this verb"))),
      };
      let one: Unknown = options.get_named_property(&key).map_err(refused)?;
      if matches!(one.get_type().map_err(refused)?, ValueType::Undefined | ValueType::Null) {
        continue;
      }
      named.push((key, self.inward(env, one, word, 0)?));
    }
    Ok(named)
  }

  /// A value of the engine, as JavaScript reads it.
  pub fn outward<'env>(&self, env: &'env Env, value: &Object) -> Result<Unknown<'env>, Fault> {
    env.to_js_value(&outward(value.as_ref())).map_err(refused)
  }
}

/// A fault that JavaScript threw as a map that names its class under `is` with what it was made with.
fn marked_fault(held: &Object) -> Option<Fault> {
  let held = held.as_ref();
  let name = field(&held, "is")?.as_str()?.to_owned();
  let args = field(&held, "args")?.items()?.into_iter().map(|one| one.to_owned()).collect();
  Some(Fault::new(name, args))
}

/// A fault of napi, as the engine reads it.
pub fn refused(error: napi::Error) -> Fault {
  Fault::refused(error.reason)
}

/// A function of JavaScript, called back by the sandbox with what the word gave it.
struct Called {
  door: Door,
  held: Option<ObjectRef<false>>,
}

impl Called {
  fn call(&self, args: Vec<Object>) -> Result<Object, Fault> {
    let door = &self.door;
    let env = door.env();
    let held = self.held.as_ref().ok_or_else(|| Fault::refused("the function is gone"))?;
    let f = held.get_value(&env).map_err(refused)?.to_unknown();
    let args = args.iter().map(|one| door.outward(&env, one)).collect::<Result<Vec<_>, _>>()?;
    let got = door.call(&env, f, None, args)?;
    door.inward(&env, got, Word::Plain, 0)
  }
}

impl Drop for Called {
  fn drop(&mut self) {
    if let Some(held) = self.held.take() {
      let _ = held.unref(&self.door.env());
    }
  }
}

/// A generator of JavaScript, heard as an ear: each step of it taken where what it throws is caught.
struct JsEar {
  door: Door,
  generator: Option<ObjectRef<false>>,
}

impl Drop for JsEar {
  fn drop(&mut self) {
    if let Some(held) = self.generator.take() {
      let _ = held.unref(&self.door.env());
    }
  }
}

impl Ear for JsEar {
  fn resume(&mut self, heard: Heard) -> Step {
    self.stepped(heard).unwrap_or_else(Step::Raised)
  }
}

impl JsEar {
  fn stepped(&mut self, heard: Heard) -> Result<Step, Fault> {
    let door = self.door.clone();
    let env = door.env();
    let held = self.generator.as_ref().ok_or_else(|| Fault::refused("the ear is over"))?;
    let generator = held.get_value(&env).map_err(refused)?;
    let (method, value) = match heard {
      Heard::Born(_) => ("next", None),
      Heard::Fact(fact) => ("next", Some(fact.0)),
      Heard::Value(value) => ("next", Some(value)),
      Heard::Raised(fault) => ("throw", Some(fault.object())),
    };
    let f: Unknown = generator.get_named_property(method).map_err(refused)?;
    let args = match value {
      Some(one) => vec![door.outward(&env, &one)?],
      None => vec![],
    };
    let got = match door.call(&env, f, Some(generator.to_unknown()), args) {
      Ok(got) => got.coerce_to_object().map_err(refused)?,
      Err(fault) => return Ok(Step::Raised(fault)),
    };
    if got.get_named_property::<bool>("done").unwrap_or_default() {
      return Ok(Step::Over);
    }
    let value: Unknown = got.get_named_property("value").map_err(refused)?;
    Ok(Step::of(&door.inward(&env, value, Word::Plain, 0)?).unwrap_or_else(Step::Raised))
  }
}

type Resolver = Box<dyn FnOnce(Env) -> napi::Result<Value>>;
type Pending = Rc<RefCell<Option<JsDeferred<Value, Resolver>>>>;

/// A function of JavaScript that another thread may call, which keeps JavaScript alive when it is strong.
type Waking<const WEAK: bool> = ThreadsafeFunction<(), (), (), napi::Status, false, WEAK>;

/// What wakes the thread of JavaScript to drive the engine, when a voice spoke from another thread.
struct Wakes(Waking<true>);

impl Wake for Wakes {
  fn wake(self: Arc<Self>) {
    self.0.call((), ThreadsafeFunctionCallMode::NonBlocking);
  }
}

/// One engine, held on the thread of JavaScript, the results that JavaScript awaits of it, and what wakes the thread
/// when a voice speaks.
pub struct Held {
  pub engine: RefCell<Option<Engine>>,
  pub door: Door,
  pending: RefCell<Vec<Pending>>,
  wakes: Arc<Wakes>,
  /// The function that drives the engine, from which the one that keeps JavaScript alive is made.
  drives: FunctionRef<(), ()>,
  /// What keeps JavaScript alive while it awaits a result, which only a voice of another thread may give.
  alive: RefCell<Option<Waking<false>>>,
}

impl Held {
  pub fn new(env: &Env, engine: Engine, door: Door) -> napi::Result<Rc<Held>> {
    let slot: Rc<RefCell<Weak<Held>>> = Rc::default();
    let driven = Rc::clone(&slot);
    let drive = env.create_function_from_closure("drive", move |_| {
      if let Some(held) = driven.borrow().upgrade() {
        held.drive();
      }
      Ok(())
    })?;
    let wakes =
      drive.build_threadsafe_function::<()>().callee_handled::<false>().weak::<true>().build()?;
    let held = Rc::new(Held {
      engine: RefCell::new(Some(engine)),
      door,
      pending: RefCell::default(),
      wakes: Arc::new(Wakes(wakes)),
      drives: drive.create_ref()?,
      alive: RefCell::default(),
    });
    *slot.borrow_mut() = Rc::downgrade(&held);
    held.drive();
    Ok(held)
  }

  /// The engine driven as far as it goes: what the voices said is said into it, and each awaited result told.
  pub fn drive(&self) {
    let waker = Waker::from(Arc::clone(&self.wakes));
    let _ = self.call(|engine| engine.pump(&waker));
    self.kept();
  }

  /// JavaScript is kept alive while it awaits a result, and let go when it awaits none.
  fn kept(&self) {
    self.pending.borrow_mut().retain(|waiting| waiting.borrow().is_some());
    let awaited = !self.pending.borrow().is_empty();
    let mut alive = self.alive.borrow_mut();
    if !awaited {
      alive.take();
    } else if alive.is_none() {
      let env = self.door.env();
      *alive = self
        .drives
        .borrow_back(&env)
        .and_then(|drive| {
          drive.build_threadsafe_function::<()>().callee_handled::<false>().weak::<false>().build()
        })
        .ok();
    }
  }

  /// One call of the engine, which no ear may make while the engine hears it.
  pub fn call<T>(&self, call: impl FnOnce(&mut Engine) -> Result<T, Fault>) -> napi::Result<T> {
    let mut held = self.engine.try_borrow_mut().map_err(|_| {
      napi::Error::from_reason(
        "An ear yields a call of a verb: the engine cannot be entered while it hears.",
      )
    })?;
    let engine =
      held.as_mut().ok_or_else(|| napi::Error::from_reason("The engine is disposed."))?;
    call(engine).map_err(|fault| napi::Error::from_reason(fault.to_string()))
  }

  /// What an act comes to, as a promise of JavaScript.
  pub fn result<'env>(&self, env: &'env Env, id: &str) -> napi::Result<JsObject<'env>> {
    let (deferred, promise) = env.create_deferred::<Value, Resolver>()?;
    let waiting = Rc::new(RefCell::new(Some(deferred)));
    self.pending.borrow_mut().push(waiting.clone());
    let settled = waiting.clone();
    let result = self.call(|engine| {
      engine.watch(id, move |value| {
        if let Some(deferred) = settled.borrow_mut().take() {
          match Fault::of(value.as_ref()) {
            Some(fault) => deferred.reject(napi::Error::from_reason(fault.to_string())),
            None => {
              let value = outward(value.as_ref());
              deferred.resolve(Box::new(move |_| Ok(value)));
            }
          }
        }
      })
    });
    if let Err(error) = result
      && let Some(deferred) = waiting.borrow_mut().take()
    {
      deferred.reject(error);
    }
    self.kept();
    Ok(promise)
  }

  pub fn dispose(&self) -> napi::Result<()> {
    let mut engine = self
      .engine
      .try_borrow_mut()
      .map_err(|_| napi::Error::from_reason("The engine is in a call of the host."))?;
    engine.take();
    self.door.close();
    for waiting in self.pending.borrow_mut().drain(..) {
      if let Some(deferred) = waiting.borrow_mut().take() {
        deferred.reject(napi::Error::from_reason("CancelledError: the engine was disposed"));
      }
    }
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
