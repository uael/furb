//! The door to TypeScript, through N-API, which also makes the declarations of the package.
//!
//! An engine is one [`JsEngine`], whose verbs are the verbs of the contract, made from the contract when the crate is
//! built, as the methods of the crate are. Views, queries and controls give their value at once, and an act is an
//! [`JsAct`], a name that JavaScript awaits. The ears of an engine are generators of JavaScript and the ears the
//! crate writes, each a [`NativeEar`], in the order the engine offers them a question.
mod host;

use std::{collections::HashMap, rc::Rc, sync::Arc};

use napi::{
  Env, JsValue, Status,
  bindgen_prelude::{
    ClassInstance, FnArgs, FromNapiValue, Function, JavaScriptClassExt, JsObjectValue,
    Object as JsObject, Promise, ToNapiValue, TypeName, Unknown, ValidateNapiValue,
  },
  sys,
  threadsafe_function::ThreadsafeFunctionCallMode,
};
use napi_derive::napi;
use serde_json::Value as Json;

use host::{Held, Word};

use super::{Given, Inspection, NativeEar, Outcome, Record, Said, Told, Value, words};
use crate::{
  Engine, Fault, Object,
  life::{Answer, Opening, Stream},
  wire,
};

impl From<Fault> for napi::Error {
  fn from(fault: Fault) -> napi::Error {
    napi::Error::from_reason(fault.to_string())
  }
}

impl From<Fault> for napi::JsError {
  fn from(fault: Fault) -> napi::JsError {
    napi::Error::from(fault).into()
  }
}

/// A fault of napi, as the engine reads it.
fn refused(error: napi::Error) -> Fault {
  Fault::refused(error.reason)
}

impl TypeName for Value {
  fn type_name() -> &'static str {
    "unknown"
  }

  fn value_type() -> napi::ValueType {
    napi::ValueType::Unknown
  }
}

impl ValidateNapiValue for Value {}

impl FromNapiValue for Value {
  unsafe fn from_napi_value(env: sys::napi_env, value: sys::napi_value) -> napi::Result<Self> {
    // SAFETY: napi gives the value in its env, where it is read at once.
    let value = unsafe { Unknown::from_raw_unchecked(env, value) };
    Ok(Value(host::inward(&Env::from_raw(env), value, Word::Plain, 0)?))
  }
}

impl ToNapiValue for Value {
  unsafe fn to_napi_value(env: sys::napi_env, value: Self) -> napi::Result<sys::napi_value> {
    // SAFETY: the env is the one napi gives the value to.
    unsafe { Json::to_napi_value(env, wire::outward(value.0.as_ref())) }
  }
}

impl ToNapiValue for Record {
  unsafe fn to_napi_value(env: sys::napi_env, value: Self) -> napi::Result<sys::napi_value> {
    // SAFETY: the env is the one napi gives the value to.
    unsafe { Json::to_napi_value(env, wire::record(value.0.as_ref())) }
  }
}

impl FromNapiValue for Given {
  /// An ear of the crate as itself, and a generator of JavaScript heard as an ear.
  unsafe fn from_napi_value(env: sys::napi_env, value: sys::napi_value) -> napi::Result<Self> {
    let env = Env::from_raw(env);
    // SAFETY: napi gives the value in its env, where it is read at once.
    let object = unsafe { Unknown::from_raw_unchecked(env.raw(), value) }.coerce_to_object()?;
    match native(&env, &object)? {
      Some(ear) => Ok(Given(ear)),
      None => Ok(Given(host::ear(&env, &object)?)),
    }
  }
}

impl FromNapiValue for Said {
  unsafe fn from_napi_value(env: sys::napi_env, value: sys::napi_value) -> napi::Result<Self> {
    // SAFETY: napi gives the value in its env, where it is read at once.
    Ok(Said(unsafe { Vec::<String>::from_napi_value(env, value) }.ok()))
  }
}

impl FromNapiValue for Told {
  /// A function of JavaScript, told on its own thread, which keeps no host alive that has nothing else to do.
  unsafe fn from_napi_value(env: sys::napi_env, value: sys::napi_value) -> napi::Result<Self> {
    // SAFETY: napi gives the value in its env, where it is read at once.
    let told = unsafe { Function::<String, ()>::from_napi_value(env, value) }?;
    let told = told.build_threadsafe_function().callee_handled::<false>().weak::<true>().build()?;
    Ok(Told(Arc::new(move |event| {
      told.call(event.to_owned(), ThreadsafeFunctionCallMode::NonBlocking) == Status::Ok
    })))
  }
}

impl FromNapiValue for Stream {
  /// A function of JavaScript, told on its own thread, which keeps no host alive that has nothing else to do.
  unsafe fn from_napi_value(env: sys::napi_env, value: sys::napi_value) -> napi::Result<Self> {
    // SAFETY: napi gives the value in its env, where it is read at once.
    let stream = unsafe { Function::<(), ()>::from_napi_value(env, value) }?;
    let told = stream
      .build_threadsafe_function::<(String, String, String, String)>()
      .weak::<true>()
      .build_callback(|call| Ok(FnArgs::from(call.value)))?;
    Ok(Stream(Arc::new(move |rung, chain, text, thinking| {
      let said = (rung.to_owned(), chain.to_owned(), text.to_owned(), thinking.to_owned());
      told.call(said, ThreadsafeFunctionCallMode::NonBlocking);
    })))
  }
}

impl FromNapiValue for Answer {
  /// A function of JavaScript as a model: it is called on the thread of JavaScript with the request and a function
  /// that tells what it writes, and the turn its promise gives is the answer, or what it threw is the refusal.
  unsafe fn from_napi_value(env: sys::napi_env, value: sys::napi_value) -> napi::Result<Self> {
    /// What a call tells as it writes, as one type, which a future of the call carries to the thread of JavaScript.
    struct Writing(crate::world::Told);
    // SAFETY: napi gives the value in its env, where it is read at once.
    let answer = unsafe { Function::<(), Promise<Json>>::from_napi_value(env, value) }?;
    let called = answer
      .build_threadsafe_function::<(Json, Writing)>()
      .weak::<true>()
      .build_callback(|call| {
        let (request, Writing(told)) = call.value;
        let request = call.env.to_js_value(&request)?;
        let write: Function<'_, Json, ()> =
          call.env.create_function_from_closure("write", move |cx| {
            let delta: Json = cx.first_arg()?;
            let part =
              |key: &str| delta.get(key).and_then(Json::as_str).unwrap_or_default().to_owned();
            told(&part("text"), &part("thinking"));
            Ok(())
          })?;
        // The values go as they are on the thread of JavaScript, where the call takes them at once.
        Ok(FnArgs::from((request.raw(), write.raw())))
      })?;
    let called = Arc::new(called);
    Ok(Answer(Arc::new(move |request, told| {
      let called = Arc::clone(&called);
      let writing = Writing(told);
      Box::pin(async move {
        let promise =
          called.call_async_catch((request, writing)).await.map_err(|no| no.reason.clone())?;
        let turn = promise.await.map_err(|no| no.reason.clone())?;
        wire::inward(&turn).map_err(|fault| fault.message())
      })
    })))
  }
}

/// The ear an object of JavaScript holds, when it is an ear of the crate, taken out of it.
fn native(env: &Env, object: &JsObject<'_>) -> Result<Option<Box<dyn crate::Ear>>, Fault> {
  if !NativeEar::instance_of(env, object).map_err(refused)? {
    return Ok(None);
  }
  // SAFETY: the object is an instance of the class, which is what the value is read as.
  let mut held = unsafe { ClassInstance::<NativeEar>::from_napi_value(env.raw(), object.raw()) }
    .map_err(refused)?;
  held.taken().map(Some)
}

/// One engine, held on the thread of JavaScript.
#[napi(js_name = "Engine")]
pub struct JsEngine {
  held: Rc<Held>,
  root: String,
  raised: Option<Fault>,
  record: Vec<Object>,
}

/// An act: its name, which a control takes, and what it comes to, which JavaScript awaits.
#[napi(js_name = "Act")]
pub struct JsAct {
  held: Rc<Held>,
  id: String,
}

impl JsAct {
  /// The name an object of JavaScript holds, when it is an act.
  fn named(env: &Env, object: &JsObject<'_>) -> Result<Option<String>, Fault> {
    if !JsAct::instance_of(env, object).map_err(refused)? {
      return Ok(None);
    }
    // SAFETY: the object is an instance of the class, which is what the value is read as.
    let held = unsafe { ClassInstance::<JsAct>::from_napi_value(env.raw(), object.raw()) }
      .map_err(refused)?;
    Ok(Some(held.id.clone()))
  }
}

#[napi]
impl JsAct {
  #[napi(getter)]
  pub fn id(&self) -> String {
    self.id.clone()
  }

  #[napi(js_name = "toString")]
  pub fn text(&self) -> String {
    self.id.clone()
  }

  #[napi(js_name = "toJSON")]
  pub fn json(&self) -> String {
    self.id.clone()
  }

  #[napi(
    ts_generic_types = "R = unknown, E = never",
    ts_return_type = "Promise<R | E>",
    ts_args_type = "onfulfilled?: ((value: unknown) => R | PromiseLike<R>) | null, onrejected?: ((reason: unknown) => E | PromiseLike<E>) | null"
  )]
  pub fn then<'env>(
    &self,
    env: &'env Env,
    fulfilled: Option<Unknown<'env>>,
    rejected: Option<Unknown<'env>>,
  ) -> napi::Result<Unknown<'env>> {
    let promise = self.held.result(env, &self.id)?;
    type Handlers<'scope> = FnArgs<(Option<Unknown<'scope>>, Option<Unknown<'scope>>)>;
    let then: Function<Handlers<'env>, Unknown<'env>> = promise.get_named_property("then")?;
    then.apply(promise, (fulfilled, rejected).into())
  }
}

#[napi]
impl JsEngine {
  /// An engine, opened from the record, on these ears, each a generator of JavaScript or an ear of the crate under
  /// the name the engine hears it by, in the order the engine offers them a question.
  #[napi(
    factory,
    ts_args_type = "record: unknown[], ears: Array<[string, Generator<unknown, unknown, unknown> | NativeEar]>"
  )]
  pub fn boot(env: Env, record: Vec<Value>, ears: Vec<(String, Given)>) -> napi::Result<Self> {
    let ears = ears.into_iter().map(|(name, ear)| (name, ear.0));
    let engine = Engine::boot(record.into_iter().map(|one| one.0), ears)?;
    JsEngine::held(&env, engine, Vec::new())
  }

  /// A life opened as every host of the crate opens one: on its record, on these ears of the host, each a generator
  /// of JavaScript or an ear of the crate, and then on the ears of the crate and of the extensions. A life whose
  /// record drifted is refused.
  #[napi(
    factory,
    ts_args_type = "ears: Array<[string, Generator<unknown, unknown, unknown> | NativeEar]>, opening: Opening"
  )]
  pub fn open(env: Env, ears: Vec<(String, Given)>, opening: Opening) -> napi::Result<Self> {
    let (engine, record) = opening.boot(ears.into_iter().map(|(name, ear)| (name, ear.0)))?;
    JsEngine::held(&env, engine, record)
  }

  /// The record the life opened on, which the journal said again whole before boot returned.
  #[napi(getter, ts_return_type = "unknown[]")]
  pub fn record(&self) -> Record {
    Record(Object::list(self.record.iter().cloned()))
  }

  #[napi(getter)]
  pub fn root(&self) -> String {
    self.root.clone()
  }

  /// What boot raised, and nothing when it raised nothing. After a drift the life goes on, with nothing kept.
  #[napi(getter, ts_return_type = "{ is: string; args: unknown[] } | null")]
  pub fn raised(&self) -> Option<Record> {
    self.raised.as_ref().map(|fault| Record(fault.object()))
  }

  #[napi(getter)]
  pub fn disposed(&self) -> bool {
    self.held.engine.try_borrow().is_ok_and(|engine| engine.is_none())
  }

  /// Who speaks in the life, and who speaks from now on when a name is given: the site of the contract, which the work
  /// an ear began sets to the name of that ear before it speaks.
  #[napi]
  pub fn site(&self, value: Option<String>) -> napi::Result<String> {
    self.held.call(|engine| engine.site(value.as_deref()))
  }

  /// One name of the engine, said by its name with these words, and what it gave.
  #[napi(
    ts_args_type = "name: string, args?: unknown[], kwargs?: Record<string, unknown>",
    ts_return_type = "unknown"
  )]
  pub fn verb(
    &self,
    name: String,
    args: Option<Vec<Value>>,
    kwargs: Option<HashMap<String, Value>>,
  ) -> napi::Result<Value> {
    let (args, kwargs) = words(args, &kwargs);
    self.said(&name, args, kwargs).map(Value)
  }

  /// One callable the engine made, called back by the number it went out under, with these words.
  #[napi(
    ts_args_type = "n: number, args?: unknown[], kwargs?: Record<string, unknown>",
    ts_return_type = "unknown"
  )]
  pub fn made(
    &self,
    n: i64,
    args: Option<Vec<Value>>,
    kwargs: Option<HashMap<String, Value>>,
  ) -> napi::Result<Value> {
    let (args, kwargs) = words(args, &kwargs);
    self.held.call(|engine| engine.made(n, args, kwargs)).map(Value)
  }

  /// A callable the engine made, forgotten: the host holds its number no more.
  #[napi]
  pub fn forget(&self, n: i64) -> napi::Result<()> {
    self.held.call(|engine| engine.forget(n))
  }

  /// What an act comes to, which JavaScript awaits.
  #[napi(ts_generic_types = "T = unknown", ts_return_type = "Promise<T>")]
  pub fn result<'env>(&self, env: &'env Env, act: String) -> napi::Result<JsObject<'env>> {
    self.held.result(env, &act)
  }

  /// What an act came to, and whether it is done.
  #[napi]
  pub fn outcome(&self, act: String) -> napi::Result<Outcome> {
    let got = self.held.call(|engine| engine.outcome(&act))?;
    Ok(Outcome { done: got.is_some(), value: Value(got.unwrap_or_else(Object::none)) })
  }

  /// The work that an earlier life left, which waits for a wake that this life says, each act by its name and its
  /// kind, as the engine of the crate finds it.
  #[napi]
  pub fn pending(&self) -> napi::Result<Vec<(String, String)>> {
    self.held.call(Engine::pending)
  }

  /// One name of a chain, the root when none is given, read without calling it.
  #[napi]
  pub fn inspect(&self, name: String, chain: Option<String>) -> napi::Result<Inspection> {
    let (kind, representation, value) =
      self.held.call(|engine| engine.inspect(&name, chain.as_deref()))?;
    Ok(Inspection { name, kind, representation, value: Value(value) })
  }

  /// Every name the module of a chain binds, the root when none is given, in the order it bound them.
  #[napi]
  pub fn names(&self, chain: Option<String>) -> napi::Result<Vec<String>> {
    self.held.call(|engine| engine.names(chain.as_deref()))
  }

  /// The engine is gone, and every result JavaScript awaits of it is refused. Its ears go with it: a command of the
  /// crate ends, a wait ends, and the store lets its record go.
  #[napi]
  pub fn dispose(&self) -> napi::Result<()> {
    self.held.dispose()
  }
}

impl JsEngine {
  /// The engine, held on the thread of JavaScript.
  fn held(env: &Env, engine: Engine, record: Vec<Object>) -> napi::Result<Self> {
    let (root, raised) = (engine.root().to_owned(), engine.raised().cloned());
    Ok(JsEngine { held: Held::new(env, engine)?, root, raised, record })
  }

  /// One verb, said by what the life hears now while it hears, and by the operator otherwise.
  fn said(
    &self,
    name: &str,
    args: Vec<Object>,
    kwargs: Vec<(&str, Object)>,
  ) -> napi::Result<Object> {
    self.held.call(|engine| engine.verb(name, args, kwargs))
  }

  /// One verb of the contract with the words JavaScript gave, each as the contract says its type, and what it gave.
  fn worded<'env>(
    &self,
    env: &'env Env,
    name: &str,
    given: Vec<(Unknown<'env>, Word)>,
    rest: Option<Vec<Unknown<'env>>>,
    options: Option<JsObject<'env>>,
    keys: &[(&str, Word)],
  ) -> napi::Result<Object> {
    let mut args = Vec::new();
    for (one, word) in given {
      args.push(host::inward(env, one, word, 0)?);
    }
    for one in rest.unwrap_or_default() {
      args.push(host::inward(env, one, Word::Plain, 0)?);
    }
    let kwargs = match options {
      Some(options) => host::named(env, options, keys)?,
      None => Vec::new(),
    };
    let kwargs = kwargs.iter().map(|(key, one)| (key.as_str(), one.clone())).collect();
    self.said(name, args, kwargs)
  }

  /// One verb of the contract that gives a value, as JavaScript reads it.
  fn plain<'env>(
    &self,
    env: &'env Env,
    name: &str,
    given: Vec<(Unknown<'env>, Word)>,
    rest: Option<Vec<Unknown<'env>>>,
    options: Option<JsObject<'env>>,
    keys: &[(&str, Word)],
  ) -> napi::Result<Value> {
    self.worded(env, name, given, rest, options, keys).map(Value)
  }

  /// One verb of the contract that makes an act, and the act.
  fn acted<'env>(
    &self,
    env: &'env Env,
    name: &str,
    given: Vec<(Unknown<'env>, Word)>,
    rest: Option<Vec<Unknown<'env>>>,
    options: Option<JsObject<'env>>,
    keys: &[(&str, Word)],
  ) -> napi::Result<JsAct> {
    let got = self.worded(env, name, given, rest, options, keys)?;
    let id = got.as_ref().as_str().ok_or_else(|| napi::Error::from_reason("a verb gave no act"))?;
    Ok(JsAct { held: Rc::clone(&self.held), id: id.to_owned() })
  }
}

// The verbs of the contract, one method each, which the build makes from the contract.
include!(concat!(env!("OUT_DIR"), "/ts.rs"));
