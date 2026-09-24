//! JavaScript ears cross like Python ears: a saying, a call to the bus, or nothing.
use super::wire::{inward, outward, record};
use crate::{Ears, Fact, Fault, Life, Object, ObjectRef, Reply};
use napi::{
  Env, JsDeferred,
  bindgen_prelude::{FunctionRef, Object as JsObject},
};
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc};

pub struct Host {
  pub env: Env,
  pub callback: FunctionRef<Value, Value>,
}
impl Host {
  fn call(&self, value: Value) -> Result<Value, Fault> {
    let value = self
      .callback
      .borrow_back(&self.env)
      .and_then(|callback| callback.call(value))
      .map_err(|error| Fault::refused(error.to_string()))?;
    if let Some(error) = value.get("error") {
      return Err(Fault::new(
        error["is"].as_str().unwrap_or("Refused"),
        error["args"]
          .as_array()
          .into_iter()
          .flatten()
          .map(inward)
          .collect::<Result<Vec<_>, _>>()?,
      ));
    }
    Ok(value)
  }
  fn reply(&self, value: Value) -> Reply {
    let result = (|| {
      let value = self.call(value)?;
      Ok(match value[0].as_str() {
        Some("over") => Reply::Over,
        Some("say") => Reply::Say(Fact(Object::tuple(args(&value[1])?))),
        Some("calls") => Reply::Calls {
          name: value[1].as_str().unwrap_or_default().into(),
          args: args(&value[2])?,
          kwargs: kwargs(&value[3])?,
        },
        _ => Reply::Nothing,
      })
    })();
    result.unwrap_or_else(Reply::Raised)
  }
}
impl Ears for Host {
  /// Every ear hears every fact.
  fn hears(&mut self, name: &str, fact: Option<&Fact>) -> Reply {
    let value = fact.map(|fact| {
      if fact.kind() == "keep" { record(fact.0.as_ref()) } else { outward(fact.0.as_ref()) }
    });
    self.reply(json!(["hears", name, value]))
  }
  fn answered(&mut self, name: &str, value: ObjectRef<'_>) -> Reply {
    self.reply(json!(["answered", name, outward(value)]))
  }
  fn called(
    &mut self,
    name: &str,
    args: Vec<Object>,
    kwargs: Vec<(String, Object)>,
  ) -> Result<Object, Fault> {
    let args = args.iter().map(|value| outward(value.as_ref())).collect::<Vec<_>>();
    let kwargs = kwargs
      .iter()
      .map(|(key, value)| (key.clone(), outward(value.as_ref())))
      .collect::<serde_json::Map<_, _>>();
    inward(&self.call(json!(["called", name, args, kwargs]))?)
  }
}

type Resolver = Box<dyn FnOnce(Env) -> napi::Result<Value>>;
type Pending = Rc<RefCell<Option<JsDeferred<Value, Resolver>>>>;

pub struct Held {
  pub life: RefCell<Option<Life>>,
  pending: RefCell<Vec<Pending>>,
}
impl Held {
  pub fn new(life: Life) -> Rc<Self> {
    Rc::new(Self { life: RefCell::new(Some(life)), pending: RefCell::new(Vec::new()) })
  }
  pub fn call<T>(&self, call: impl FnOnce(&mut Life) -> Result<T, Fault>) -> napi::Result<T> {
    self.pending.borrow_mut().retain(|waiting| waiting.borrow().is_some());
    let mut held = self.life.try_borrow_mut().map_err(|_| {
      napi::Error::from_reason(
        "Yield a bus call from an ear; the life cannot be entered recursively.",
      )
    })?;
    let life = held.as_mut().ok_or_else(|| napi::Error::from_reason("life is disposed"))?;
    call(life).map_err(|fault| napi::Error::from_reason(fault.to_string()))
  }
  pub fn result<'env>(&self, env: &'env Env, id: &str) -> napi::Result<JsObject<'env>> {
    self.call(|life| life.get(id))?;
    let (deferred, promise) = env.create_deferred::<Value, Resolver>()?;
    let waiting = Rc::new(RefCell::new(Some(deferred)));
    self.pending.borrow_mut().push(waiting.clone());
    let settled = waiting.clone();
    let result = self.call(|life| {
      life.watch(id, move |value| {
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
    Ok(promise)
  }
  pub fn dispose(&self) -> napi::Result<()> {
    let mut life = self
      .life
      .try_borrow_mut()
      .map_err(|_| napi::Error::from_reason("The life is in a host callback."))?;
    life.take();
    for waiting in self.pending.borrow_mut().drain(..) {
      if let Some(deferred) = waiting.borrow_mut().take() {
        deferred.reject(napi::Error::from_reason("CancelledError: life was disposed"));
      }
    }
    Ok(())
  }
}
impl Drop for Held {
  fn drop(&mut self) {
    let _ = self.dispose();
  }
}

fn args(value: &Value) -> Result<Vec<Object>, Fault> {
  value
    .as_array()
    .ok_or_else(|| Fault::refused("arguments must be an array"))?
    .iter()
    .map(inward)
    .collect()
}
fn kwargs(value: &Value) -> Result<Vec<(String, Object)>, Fault> {
  value
    .as_object()
    .ok_or_else(|| Fault::refused("keywords must be an object"))?
    .iter()
    .map(|(key, value)| Ok((key.clone(), inward(value)?)))
    .collect()
}
pub fn invoke(
  life: &mut Life,
  name: &str,
  values: Vec<Value>,
  named: Value,
) -> Result<Value, Fault> {
  let args = args(&json!(values))?;
  let held = kwargs(&named)?;
  let kwargs = held.iter().map(|(key, value)| (key.as_str(), value.clone())).collect();
  Ok(outward(life.verb(name, args, kwargs)?.as_ref()))
}
pub fn on(life: &Life, given: Option<String>) -> Value {
  json!({"on": given.unwrap_or_else(|| life.root().into())})
}
