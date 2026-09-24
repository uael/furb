//! Native TypeScript bindings. N-API generates the package loader and declarations from this surface.
//! Queries and controls are synchronous. Acts carry their name and are awaited through a native Promise.
pub mod console;
pub mod extension;
mod host;
mod lease;
mod wire;

use napi::{
  Env,
  bindgen_prelude::{FnArgs, Function, JsObjectValue, Object as JsObject, Unknown},
};
use std::rc::Rc;

use crate::Fault;
use napi_derive::napi;
use serde_json::{Value, json};

use host::{Held, Host, invoke, on};
use wire::{inward, outward};

#[napi(object)]
pub struct PromptOptions {
  pub to: Option<String>,
  pub on: Option<String>,
}

#[napi(object)]
pub struct Outcome {
  pub done: bool,
  #[napi(ts_type = "unknown")]
  pub value: Value,
}

#[napi(object)]
pub struct RungOptions {
  pub retells: Option<String>,
  pub actor: Option<String>,
  pub on: Option<String>,
}

#[napi(object)]
pub struct Inspection {
  pub name: String,
  pub kind: String,
  pub representation: String,
  #[napi(ts_type = "unknown")]
  pub value: Option<Value>,
}

#[napi(js_name = "Life")]
pub struct JsLife {
  held: Rc<Held>,
  root: String,
  raised: Option<Value>,
  words: Vec<String>,
}

/// A named act. Keep its id for controls, or await the act for its outcome.
#[napi(js_name = "Act")]
pub struct JsAct {
  held: Rc<Held>,
  id: String,
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
impl JsLife {
  /// Open on JavaScript ears, using the same call and reply protocol as the Python binding. The module of the engine
  /// runs the words of the extensions after the engine: those the record pins, or else these, which the life pins as
  /// the World. The life plays the life words as rungs, as the World, on every chain without a source, once boot
  /// stands on its record and at the birth of each such chain after.
  #[napi(
    factory,
    ts_args_type = "callback: (request: unknown[]) => unknown, names: string[], record?: unknown[] | null, words?: string[] | null, lives?: string[] | null"
  )]
  pub fn boot(
    env: Env,
    callback: Function<Value, Value>,
    names: Vec<String>,
    record: Option<Vec<Value>>,
    words: Option<Vec<String>>,
    lives: Option<Vec<String>>,
  ) -> napi::Result<Self> {
    let record = record
      .unwrap_or_default()
      .iter()
      .map(inward)
      .collect::<Result<Vec<_>, _>>()
      .map_err(error)?;
    let life = crate::Life::open_on(Host { env, callback: callback.create_ref()? }, names)
      .words(words.unwrap_or_default())
      .lives(lives.unwrap_or_default())
      .boot(record)
      .map_err(error)?;
    let root = life.root().to_owned();
    let raised = life.raised().map(|fault| outward(fault.object().as_ref()));
    let words = life.words().to_vec();
    Ok(Self { held: Held::new(life), root, raised, words })
  }

  #[napi(getter)]
  pub fn root(&self) -> String {
    self.root.clone()
  }

  /// The words of the extensions the life runs after the engine, which the system prompt reads after the engine.
  #[napi(getter)]
  pub fn words(&self) -> Vec<String> {
    self.words.clone()
  }

  /// What boot raised, and nothing when it raised nothing. After a drift the life goes on, with nothing kept.
  #[napi(getter, ts_return_type = "{ is: string; args: unknown[] } | null")]
  pub fn raised(&self) -> Option<Value> {
    self.raised.clone()
  }

  #[napi(getter)]
  pub fn disposed(&self) -> bool {
    self.held.life.try_borrow().is_ok_and(|life| life.is_none())
  }

  #[napi]
  pub fn site(&self, value: Option<String>) -> napi::Result<String> {
    self.held.call(|life| life.site(value.as_deref()))
  }

  /// Call any public engine verb, including verbs used by extensions.
  #[napi(
    ts_generic_types = "T = unknown",
    ts_args_type = "name: string, args: unknown[], kwargs: Record<string, unknown>",
    ts_return_type = "T"
  )]
  pub fn call(&self, name: String, args: Vec<Value>, kwargs: Value) -> napi::Result<Value> {
    self.held.call(move |life| invoke(life, &name, args, kwargs))
  }

  /// Await one act. JavaScript continues to drive the World while the Promise waits.
  #[napi(ts_generic_types = "T = unknown", ts_return_type = "Promise<T>")]
  pub fn result<'env>(&self, env: &'env Env, id: String) -> napi::Result<JsObject<'env>> {
    self.held.result(env, &id)
  }

  #[napi]
  pub fn outcome(&self, id: String) -> napi::Result<Outcome> {
    self.held.call(move |life| {
      let got = life.outcome(&id)?;
      Ok(Outcome {
        done: got.is_some(),
        value: got.map_or(Value::Null, |value| outward(value.as_ref())),
      })
    })
  }

  #[napi(
    ts_generic_types = "T = unknown",
    ts_args_type = "name: string, keys: unknown[], ask: string",
    ts_return_type = "T"
  )]
  pub fn held(&self, name: String, keys: Vec<Value>, ask: String) -> napi::Result<Value> {
    self.held.call(move |life| {
      let keys = keys.iter().map(inward).collect::<Result<Vec<_>, _>>()?;
      Ok(outward(life.held(&name, keys, &ask)?.as_ref()))
    })
  }

  #[napi(
    ts_generic_types = "T = unknown",
    ts_args_type = "id: number, args: unknown[], kwargs: Record<string, unknown>",
    ts_return_type = "T"
  )]
  pub fn made(&self, id: i64, args: Vec<Value>, kwargs: Value) -> napi::Result<Value> {
    self.held.call(move |life| {
      let args = args.iter().map(inward).collect::<Result<Vec<_>, _>>()?;
      let held = kwargs
        .as_object()
        .ok_or_else(|| Fault::refused("keywords must be an object"))?
        .iter()
        .map(|(key, value)| Ok((key.clone(), inward(value)?)))
        .collect::<Result<Vec<_>, Fault>>()?;
      let kwargs = held.iter().map(|(key, value)| (key.as_str(), value.clone())).collect();
      Ok(outward(life.made(id, args, kwargs)?.as_ref()))
    })
  }

  #[napi]
  pub fn forget(&self, id: i64) -> napi::Result<()> {
    self.held.call(move |life| life.forget(id))
  }

  #[napi(ts_generic_types = "T = unknown", ts_return_type = "Act & PromiseLike<T>")]
  pub fn prompt(
    &self,
    shape: String,
    message: String,
    options: Option<PromptOptions>,
  ) -> napi::Result<JsAct> {
    self
      .held
      .call(move |life| {
        let options = options.unwrap_or(PromptOptions { to: None, on: None });
        let named = on(life, options.on);
        invoke(
          life,
          "prompt",
          vec![json!(shape), json!(message), json!(options.to.unwrap_or_default())],
          named,
        )
      })
      .and_then(string)
      .map(|id| JsAct { id, held: self.held.clone() })
  }

  #[napi(ts_return_type = "Act & PromiseLike<unknown>")]
  pub fn rung(&self, word: String, options: Option<RungOptions>) -> napi::Result<JsAct> {
    self
      .held
      .call(move |life| {
        let options = options.unwrap_or(RungOptions { retells: None, actor: None, on: None });
        let named = on(life, options.on);
        invoke(
          life,
          "rung",
          vec![
            json!(word),
            json!(options.retells.unwrap_or_default()),
            json!(options.actor.unwrap_or_default()),
          ],
          named,
        )
      })
      .and_then(string)
      .map(|id| JsAct { id, held: self.held.clone() })
  }

  /// Read one name from the chain without calling it, with its Python type and representation. The value crosses
  /// as every value of the life does, through the stand-in, so a map that holds the key `is` crosses as its pairs.
  #[napi]
  pub fn inspect(&self, name: String, chain: Option<String>) -> napi::Result<Inspection> {
    self.held.call(move |life| {
      let chain = crate::Object::string(chain.unwrap_or_else(|| life.root().into()));
      let key = crate::Object::string(&name);
      let raw = life.word(
        "modules[__chain][__name]",
        vec![("__chain", chain.clone()), ("__name", key.clone())],
      )?;
      let value = life.held("modules", vec![chain, key], "at")?;
      Ok(Inspection {
        kind: raw.as_ref().type_name().into(),
        representation: raw.as_ref().py_repr(),
        value: Some(outward(value.as_ref())).filter(|value| !value.is_null()),
        name,
      })
    })
  }

  #[napi(
    ts_args_type = "label: string, source?: string | null, filter?: unknown, on?: string | null",
    ts_return_type = "Act & PromiseLike<never>"
  )]
  pub fn chain(
    &self,
    label: String,
    source: Option<String>,
    filter: Option<Value>,
    chain: Option<String>,
  ) -> napi::Result<JsAct> {
    self
      .held
      .call(move |life| {
        // A chain the operator opens stands on no chain unless it names one, as the contract's default says: on
        // the root it would be an act of the root, which every control over the root reaches.
        let named = json!({ "on": chain.unwrap_or_default(), "filter": filter });
        invoke(life, "chain", vec![json!(label), json!(source.unwrap_or_default())], named)
      })
      .and_then(string)
      .map(|id| JsAct { id, held: self.held.clone() })
  }

  #[napi(ts_return_type = "Act & PromiseLike<null>")]
  pub fn wait(&self, seconds: f64, chain: Option<String>) -> napi::Result<JsAct> {
    self
      .held
      .call(move |life| {
        let named = on(life, chain);
        invoke(life, "wait", vec![json!(seconds)], named)
      })
      .and_then(string)
      .map(|id| JsAct { id, held: self.held.clone() })
  }

  #[napi(ts_generic_types = "T = unknown", ts_return_type = "T | null")]
  pub fn peek(&self, id: String, chain: Option<String>) -> napi::Result<Value> {
    self.held.call(move |life| {
      let named = on(life, chain);
      invoke(life, "peek", vec![json!(id)], named)
    })
  }

  #[napi(ts_return_type = "[string, string, string, string, ...unknown[]]")]
  pub fn get(&self, id: String) -> napi::Result<Value> {
    self.held.call(move |life| Ok(outward(life.get(&id)?.0.as_ref())))
  }

  /// The turns of a chain, each the python a model reads, which the engine wrote.
  #[napi(
    ts_return_type = "Array<['user' | 'assistant', string, [number, number, number, number, number] | null, unknown]>"
  )]
  pub fn turns(&self, chain: Option<String>) -> napi::Result<Value> {
    self.held.call(move |life| {
      let named = on(life, chain);
      invoke(life, "turns", vec![], named)
    })
  }

  #[napi]
  pub fn scope(&self, id: String) -> napi::Result<String> {
    self.held.call(move |life| invoke(life, "scope", vec![json!(id)], json!({}))).and_then(string)
  }

  #[napi]
  pub fn clock(&self, chain: Option<String>) -> napi::Result<f64> {
    self
      .held
      .call(move |life| {
        let named = on(life, chain);
        invoke(life, "clock", vec![], named)
      })
      .and_then(number)
  }

  #[napi]
  pub fn chance(&self, chain: Option<String>) -> napi::Result<f64> {
    self
      .held
      .call(move |life| {
        let named = on(life, chain);
        invoke(life, "chance", vec![], named)
      })
      .and_then(number)
  }

  #[napi]
  pub fn gate(&self, word: String, chain: Option<String>) -> napi::Result<Vec<String>> {
    let value = self.held.call(move |life| {
      let named = on(life, chain);
      invoke(life, "gate", vec![json!(word)], named)
    })?;
    serde_json::from_value(value).map_err(|error| napi::Error::from_reason(error.to_string()))
  }

  #[napi]
  pub fn pause(&self, id: String) -> napi::Result<()> {
    self.held.call(move |life| life.pause(&id))
  }
  #[napi]
  pub fn wake(&self, id: String) -> napi::Result<()> {
    self.held.call(move |life| life.wake(&id))
  }
  #[napi]
  pub fn cancel(&self, id: String) -> napi::Result<()> {
    self.held.call(move |life| life.cancel(&id))
  }
  #[napi(ts_args_type = "value: unknown, id: string")]
  pub fn close(&self, value: Value, id: String) -> napi::Result<()> {
    self.held.call(move |life| life.close(inward(&value)?, &id))
  }

  #[napi(ts_return_type = "[string, string, string, ...unknown[]]")]
  pub fn send(
    &self,
    kind: String,
    about: String,
    words: Vec<Value>,
    by: Option<String>,
  ) -> napi::Result<Value> {
    self.held.call(move |life| {
      let mut args = vec![json!(kind), json!(about)];
      args.extend(words);
      invoke(life, "send", args, json!({"by": by.unwrap_or_default()}))
    })
  }

  #[napi(ts_return_type = "{ is: 'made'; id: number }")]
  pub fn take(&self, ids: Vec<String>, inside: Option<bool>) -> napi::Result<Value> {
    self.held.call(move |life| {
      invoke(
        life,
        "take",
        ids.into_iter().map(Value::String).collect(),
        json!({"inside": inside.unwrap_or(true)}),
      )
    })
  }

  /// Drop this sandbox and reject pending waits. The World must stop its own processes and timers.
  #[napi]
  pub fn dispose(&self) -> napi::Result<()> {
    self.held.dispose()
  }
}

fn error(fault: Fault) -> napi::Error {
  napi::Error::from_reason(fault.to_string())
}

fn string(value: Value) -> napi::Result<String> {
  value
    .as_str()
    .map(str::to_owned)
    .ok_or_else(|| napi::Error::from_reason("the engine returned no string"))
}
fn number(value: Value) -> napi::Result<f64> {
  value.as_f64().ok_or_else(|| napi::Error::from_reason("the engine returned no number"))
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
