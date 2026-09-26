//! The door to TypeScript, through N-API, which also makes the loader of the package and its declarations.
//!
//! An engine is one [`JsEngine`], whose verbs are the verbs of the contract, made from the contract when the crate is
//! built, as the methods of the crate are. Views, queries and controls give their value at once, and an act is an
//! [`JsAct`], a name that JavaScript awaits. The ears of an engine are generators of JavaScript and the ears the
//! crate writes, each a [`NativeEar`], in the order the engine offers them a question.
pub mod console;
mod host;

use std::{cell::RefCell, rc::Rc};

use napi::{
  Env, JsValue,
  bindgen_prelude::{
    ClassInstance, FnArgs, FromNapiValue, Function, JavaScriptClassExt, JsObjectValue,
    Object as JsObject, Unknown,
  },
};
use napi_derive::napi;
use serde_json::Value;

use host::{Door, Held, Word, refused};

use crate::{Ear, Engine, Fault, Object, wire, world};

#[napi(object)]
pub struct TextValue {
  pub path: String,
  pub content: String,
}

#[napi(object)]
pub struct ExitValue {
  pub code: Option<i64>,
  pub stdout: TextValue,
  pub stderr: TextValue,
}

#[napi(object)]
pub struct Outcome {
  pub done: bool,
  #[napi(ts_type = "unknown")]
  pub value: Value,
}

#[napi(object)]
pub struct Inspection {
  pub name: String,
  pub kind: String,
  pub representation: String,
  #[napi(ts_type = "unknown")]
  pub value: Option<Value>,
}

/// An ear that the crate writes: given once, to the boot of an engine or to a verb that takes an ear.
#[napi]
pub struct NativeEar {
  ear: RefCell<Option<Box<dyn Ear>>>,
}

impl NativeEar {
  fn of(ear: Box<dyn Ear>) -> NativeEar {
    NativeEar { ear: RefCell::new(Some(ear)) }
  }

  /// The ear an object of JavaScript holds, when it is one, taken out of it, since an ear hears in one engine.
  fn taken(env: &Env, object: &JsObject<'_>) -> Result<Option<Box<dyn Ear>>, Fault> {
    if !NativeEar::instance_of(env, object).map_err(refused)? {
      return Ok(None);
    }
    // SAFETY: the object is an instance of the class, which is what the value is read as.
    let held = unsafe { ClassInstance::<NativeEar>::from_napi_value(env.raw(), object.raw()) }
      .map_err(refused)?;
    let ear = held.ear.borrow_mut().take();
    ear
      .map(Some)
      .ok_or_else(|| Fault::refused("an ear of the crate hears in one engine, and this one hears"))
  }
}

#[napi]
impl NativeEar {
  /// The ear is let go before any engine hears it, so what it holds goes: a store lets its record go.
  #[napi]
  pub fn dispose(&self) {
    self.ear.borrow_mut().take();
  }
}

/// The POSIX shell that runs a command of this machine, which a host runs its own commands in too.
#[napi]
pub fn shell() -> &'static str {
  world::SHELL
}

/// The ear of the files, which reads and writes a path.
#[napi]
pub fn files() -> NativeEar {
  NativeEar::of(world::files())
}

/// The ear of commands, which runs each in a shell of this machine.
#[napi]
pub fn bash() -> NativeEar {
  NativeEar::of(world::bash())
}

/// The ear of time, which reads the clock, draws a chance, and ends a wait.
#[napi]
pub fn time() -> NativeEar {
  NativeEar::of(world::time())
}

/// The record at a path, read under its lease, and the ear of the store, which keeps on it what the journal says to
/// keep.
#[napi(ts_return_type = "{ record: unknown[]; ear: NativeEar }")]
pub fn store<'env>(env: &'env Env, path: String) -> napi::Result<JsObject<'env>> {
  let (record, ear) = world::store(&path).map_err(error)?;
  let mut got = JsObject::new(env)?;
  got.set_named_property("record", records(&record))?;
  got.set_named_property("ear", NativeEar::of(ear).into_instance(env)?)?;
  Ok(got)
}

/// What the store kept at a path, read with no lease and changed in nothing.
#[napi(ts_return_type = "unknown[]")]
pub fn kept(path: String) -> napi::Result<Vec<Value>> {
  Ok(records(&world::kept(&path).map_err(error)?))
}

/// Entries of a record, as JavaScript reads them.
fn records(record: &[Object]) -> Vec<Value> {
  record.iter().map(|one| wire::record(one.as_ref())).collect()
}

/// One engine, held on the thread of JavaScript.
#[napi(js_name = "Engine")]
pub struct JsEngine {
  held: Rc<Held>,
  root: String,
  raised: Option<Value>,
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
  pub fn boot(
    env: Env,
    record: Vec<Value>,
    ears: Vec<(String, Unknown<'_>)>,
  ) -> napi::Result<Self> {
    let record = record.iter().map(wire::inward).collect::<Result<Vec<_>, _>>().map_err(error)?;
    let hosted = crate::engine::Hosted::default();
    let door = Door::new(env, hosted.clone())?;
    let mut given = Vec::new();
    for (name, value) in ears {
      let object = value.coerce_to_object()?;
      let ear = match NativeEar::taken(&env, &object).map_err(error)? {
        Some(ear) => ear,
        None => door.ear(object).map_err(error)?,
      };
      given.push((name, ear));
    }
    let engine = Engine::open(hosted, record, given).map_err(error)?;
    let root = engine.root().to_owned();
    let raised = engine.raised().map(|fault| wire::record(fault.object().as_ref()));
    Ok(Self { held: Held::new(&env, engine, door)?, root, raised })
  }

  #[napi(getter)]
  pub fn root(&self) -> String {
    self.root.clone()
  }

  /// What boot raised, and nothing when it raised nothing. After a drift the life goes on, with nothing kept.
  #[napi(getter, ts_return_type = "{ is: string; args: unknown[] } | null")]
  pub fn raised(&self) -> Option<Value> {
    self.raised.clone()
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

  /// What an act comes to, which JavaScript awaits.
  #[napi(ts_generic_types = "T = unknown", ts_return_type = "Promise<T>")]
  pub fn result<'env>(&self, env: &'env Env, id: String) -> napi::Result<JsObject<'env>> {
    self.held.result(env, &id)
  }

  /// What an act came to, and whether it is done.
  #[napi]
  pub fn outcome(&self, id: String) -> napi::Result<Outcome> {
    self.held.call(move |engine| {
      let got = engine.outcome(&id)?;
      Ok(Outcome {
        done: got.is_some(),
        value: got.map_or(Value::Null, |value| wire::outward(value.as_ref())),
      })
    })
  }

  /// One callable the engine made, called back by the handle it crossed under, with these words.
  #[napi(
    ts_generic_types = "T = unknown",
    ts_args_type = "id: number, args: unknown[], kwargs: Record<string, unknown>",
    ts_return_type = "T"
  )]
  pub fn made<'env>(
    &self,
    env: &'env Env,
    id: i64,
    args: Vec<Unknown<'env>>,
    kwargs: JsObject<'env>,
  ) -> napi::Result<Unknown<'env>> {
    let door = self.held.door.clone();
    let args = args
      .into_iter()
      .map(|one| door.inward(env, one, Word::Plain, 1))
      .collect::<Result<Vec<_>, _>>();
    let kwargs = door.named(env, kwargs, &[]);
    let got = self.held.call(|engine| {
      let kwargs = kwargs?;
      let kwargs = kwargs.iter().map(|(key, one)| (key.as_str(), one.clone())).collect();
      engine.made(id, args?, kwargs)
    })?;
    door.outward(env, &got).map_err(error)
  }

  /// A callable the engine made, forgotten: JavaScript holds its handle no more.
  #[napi]
  pub fn forget(&self, id: i64) -> napi::Result<()> {
    self.held.call(move |engine| engine.forget(id))
  }

  /// One name of a chain, read without calling it, with its type and its representation in the sandbox. The value
  /// crosses as every value does, so a map that holds the key `is` crosses as its pairs.
  #[napi]
  pub fn inspect(&self, name: String, chain: Option<String>) -> napi::Result<Inspection> {
    self.held.call(move |engine| {
      let chain = Object::string(chain.unwrap_or_else(|| engine.root().into()));
      let key = Object::string(&name);
      let bound = vec![("__chain", chain), ("__name", key)];
      let raw = engine.word("module(__chain)[__name]", bound.clone())?;
      let value = engine.word("outward(module(__chain)[__name], __engine)", bound)?;
      Ok(Inspection {
        kind: raw.as_ref().type_name().into(),
        representation: raw.as_ref().py_repr(),
        value: Some(wire::outward(value.as_ref())).filter(|value| !value.is_null()),
        name,
      })
    })
  }

  /// Every name the module of a chain binds, in the order it bound them.
  #[napi]
  pub fn names(&self, chain: Option<String>) -> napi::Result<Vec<String>> {
    let value = self.held.call(move |engine| {
      let chain = Object::string(chain.unwrap_or_else(|| engine.root().into()));
      Ok(wire::outward(
        engine.word("[str(x) for x in module(__chain)]", vec![("__chain", chain)])?.as_ref(),
      ))
    })?;
    serde_json::from_value(value).map_err(|error| napi::Error::from_reason(error.to_string()))
  }

  /// The engine is gone, and every result JavaScript awaits of it is refused. Its ears go with it: a command of the
  /// crate ends, a wait ends, and the store lets its record go.
  #[napi]
  pub fn dispose(&self) -> napi::Result<()> {
    self.held.dispose()
  }
}

impl JsEngine {
  /// One verb said with the words JavaScript gave, and what it gave, as JavaScript reads it.
  fn plain<'env>(
    &self,
    env: &'env Env,
    name: &str,
    given: Vec<(Unknown<'env>, Word)>,
    rest: Option<Vec<Unknown<'env>>>,
    options: Option<JsObject<'env>>,
    keys: &[(&str, Word)],
  ) -> napi::Result<Unknown<'env>> {
    let got = self.said(env, name, given, rest, options, keys)?;
    self.held.door.outward(env, &got).map_err(error)
  }

  /// One verb that makes an act, said with the words JavaScript gave, and the act.
  fn acted<'env>(
    &self,
    env: &'env Env,
    name: &str,
    given: Vec<(Unknown<'env>, Word)>,
    rest: Option<Vec<Unknown<'env>>>,
    options: Option<JsObject<'env>>,
    keys: &[(&str, Word)],
  ) -> napi::Result<JsAct> {
    let got = self.said(env, name, given, rest, options, keys)?;
    let id = got.as_ref().as_str().ok_or_else(|| napi::Error::from_reason("a verb gave no act"))?;
    Ok(JsAct { held: Rc::clone(&self.held), id: id.to_owned() })
  }

  fn said<'env>(
    &self,
    env: &'env Env,
    name: &str,
    given: Vec<(Unknown<'env>, Word)>,
    rest: Option<Vec<Unknown<'env>>>,
    options: Option<JsObject<'env>>,
    keys: &[(&str, Word)],
  ) -> napi::Result<Object> {
    let door = self.held.door.clone();
    let mut args = Vec::new();
    for (one, word) in given {
      args.push(door.inward(env, one, word, 0).map_err(error)?);
    }
    for one in rest.unwrap_or_default() {
      args.push(door.inward(env, one, Word::Plain, 0).map_err(error)?);
    }
    let kwargs = match options {
      Some(options) => door.named(env, options, keys).map_err(error)?,
      None => Vec::new(),
    };
    self.held.call(|engine| {
      let kwargs = kwargs.iter().map(|(key, one)| (key.as_str(), one.clone())).collect();
      engine.verb(name, args, kwargs)
    })
  }
}

// The verbs of the contract, one method each, which the build makes from the contract.
include!(concat!(env!("OUT_DIR"), "/ts.rs"));

fn error(fault: Fault) -> napi::Error {
  napi::Error::from_reason(fault.to_string())
}

#[napi]
pub fn engine_source() -> &'static str {
  crate::ENGINE
}

#[napi(ts_return_type = "unknown")]
pub fn decode_record(line: String) -> napi::Result<Value> {
  let value: &serde_json::value::RawValue = serde_json::from_str(&line)
    .map_err(|error| napi::Error::new(napi::Status::InvalidArg, error.to_string()))?;
  wire::decoded(value, 0).map_err(error)
}
