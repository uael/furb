//! The World and the gate a host writes in javascript, as the crate reads them.
//!
//! A host of javascript writes an object with `hears` and, when it asks the engine anything, `answered`. The
//! crate takes a World and a gate of its own, so each of these holds the javascript object and stands in for it:
//! every call of the trait becomes one call of the object, and what the object gives back is read as a reply.
//!
//! An object of javascript is reached through the env of the call that is running, and through no other. So the
//! env is given to these at the top of every call of a life and taken away at the end of it, and a call of the
//! object while neither holds one is no call at all: there is no interpreter on the stack to make it in.

#![expect(unsafe_code, reason = "the napi API is unsafe, and every cast below is guarded by the check above it")]

use furb::{Fact, Gate, Reply, Value, World};
use napi::{
  Env, Error, JsValue, Result, Unknown,
  bindgen_prelude::{Array, FnArgs, Function, JavaScriptClassExt, JsObjectValue, Object, ObjectRef},
};

use crate::value::{Fact as Held, made, of_js, refused, to_js};

/// A question of the engine, which a World answers a fact with when it must know something first.
///
/// The boundary puts the question and hands the answer back through `answered`, so a World never calls into a
/// life that stands waiting for it. A host says one as a plain object, `{ ask, on, words }`, since it carries
/// three words and nothing else.
pub const ASK: &str = "ask";

/// A World of javascript, as the crate reads one.
///
/// Every fault of the javascript object stands: a World that throws has said something a life cannot answer, and
/// the life stops with it rather than carrying on with a reply that was never given.
pub struct Worlds {
  /// The object the host wrote.
  held: ObjectRef,
  /// The env of the call that is running, and nothing between two calls.
  env: Option<Env>,
  /// The first fault of the object, which the caller of the life throws once the call is over.
  pub raised: Option<Error>,
}

impl Worlds {
  /// A World of the crate, over the object a host wrote.
  pub fn new(held: ObjectRef) -> Self {
    Worlds { held, env: None, raised: None }
  }

  /// The env of the call that is starting, which is how the object is reached while it runs.
  pub fn on(&mut self, env: &Env) {
    self.env = Some(*env);
  }

  /// The end of the call, after which the object is reached by nothing.
  pub fn off(&mut self) {
    self.env = None;
  }

  /// The object the host wrote, as the host reads it back.
  pub fn object<'env>(&self, env: &Env) -> Result<Unknown<'env>> {
    made(env, self.held.get_value(env)?)
  }

  /// The reference on the object, released, which is the end of this World.
  ///
  /// A reference holds an object of javascript against collection, and nothing releases one for you: the life
  /// that made it releases it when the life itself is collected.
  pub fn done(self, env: &Env) -> Result<()> {
    self.held.unref(env)
  }

  /// One call of the object, and the reply it gave.
  ///
  /// A fault is kept and answered with nothing, so that the life runs to the end of the call it is in and the
  /// caller hears the fault whole. A life that carried on would hear a reply the host never gave.
  fn calls(&mut self, name: &str, args: impl FnOnce(&Env) -> Result<Unknown<'static>>) -> Reply {
    if self.raised.is_some() {
      return Reply::Nothing;
    }
    let Some(env) = self.env else { return Reply::Nothing };
    match self.said(&env, name, args) {
      Ok(reply) => reply,
      Err(fault) => {
        self.raised = Some(fault);
        Reply::Nothing
      }
    }
  }

  /// One call of the object, which is the method of that name on it.
  fn said(&self, env: &Env, name: &str, args: impl FnOnce(&Env) -> Result<Unknown<'static>>) -> Result<Reply> {
    let held = self.held.get_value(env)?;
    let one = args(env)?;
    let call: Function<'_, Unknown<'_>, Unknown<'_>> = held
      .get_named_property(name)
      .map_err(|_| refused(&format!("a World of a host answers {name}, and this one does not")))?;
    let said = call.apply(&held, one)?;
    reply_of(env, &said)
  }
}

impl World for Worlds {
  fn hears(&mut self, fact: &Fact) -> Reply {
    let held = fact.clone();
    self.calls("hears", move |env| made(env, Held { held }))
  }

  fn answered(&mut self, got: &Value) -> Reply {
    let held = got.clone();
    self.calls("answered", move |env| to_js(env, &held))
  }
}

/// A gate of javascript, as the crate reads one.
///
/// A host that reads no python leaves it out, and the word then runs and raises where it stands. One that reads
/// python gives an object with `gate`, and the findings it gives are what the model is told.
pub struct Gates {
  /// The object the host wrote, and nothing for a host that gates no word.
  held: Option<ObjectRef>,
  /// The env of the call that is running, and nothing between two calls.
  env: Option<Env>,
  /// The first fault of the object, which the caller of the life throws once the call is over.
  pub raised: Option<Error>,
}

impl Gates {
  /// A gate of the crate, over the object a host wrote, or over nothing.
  pub fn new(held: Option<ObjectRef>) -> Self {
    Gates { held, env: None, raised: None }
  }

  /// The env of the call that is starting, which is how the object is reached while it runs.
  pub fn on(&mut self, env: &Env) {
    self.env = Some(*env);
  }

  /// The end of the call, after which the object is reached by nothing.
  pub fn off(&mut self) {
    self.env = None;
  }

  /// The reference on the object, released, which is the end of this gate.
  pub fn done(self, env: &Env) -> Result<()> {
    match self.held {
      Some(held) => held.unref(env),
      None => Ok(()),
    }
  }

  /// One call of the object, which is the `gate` method on it.
  fn found(&self, env: &Env, word: &str, ladder: &[String], shape: &str) -> Result<Vec<String>> {
    let Some(held) = self.held.as_ref() else { return Ok(Vec::new()) };
    let held = held.get_value(env)?;
    let call: Function<'_, FnArgs<(String, Vec<String>, String)>, Array<'_>> =
      held.get_named_property("gate").map_err(|_| refused("a gate of a host answers gate, and this one does not"))?;
    let said = call.apply(&held, (word.to_owned(), ladder.to_vec(), shape.to_owned()).into())?;
    let mut out = Vec::with_capacity(said.len() as usize);
    for at in 0..said.len() {
      let one: String = said.get(at)?.ok_or_else(|| refused("a finding of a gate is a text"))?;
      out.push(one);
    }
    Ok(out)
  }
}

impl Gate for Gates {
  fn gate(&mut self, word: &str, ladder: &[String], shape: &str) -> Vec<String> {
    if self.raised.is_some() {
      return Vec::new();
    }
    let Some(env) = self.env else { return Vec::new() };
    match self.found(&env, word, ladder, shape) {
      Ok(found) => found,
      Err(fault) => {
        self.raised = Some(fault);
        Vec::new()
      }
    }
  }
}

/// What a host said of a fact, as the crate reads it.
///
/// Nothing at all is nothing said, one fact or an array of them is what to say in that order, and an object with
/// an `ask` is a question of the engine. Anything else is a fault of the host, which is told what it gave.
fn reply_of(env: &Env, said: &Unknown<'_>) -> Result<Reply> {
  match said.get_type()? {
    napi::ValueType::Undefined | napi::ValueType::Null => return Ok(Reply::Nothing),
    napi::ValueType::Object => {}
    other => return Err(refused(&format!("a World says nothing, a fact, an array of facts or an ask, not a {other}"))),
  }
  if Held::instance_of(env, said)? {
    // Checked by the line above, which is what makes the cast right.
    let one: &Held = unsafe { said.cast()? };
    return Ok(Reply::say(one.held.clone()));
  }
  let object: Object<'_> = unsafe { said.cast()? };
  if object.is_array()? {
    let each: Array<'_> = unsafe { said.cast()? };
    let mut held = Vec::with_capacity(each.len() as usize);
    for at in 0..each.len() {
      let one: &Held = each
        .get::<&Held>(at)?
        .ok_or_else(|| refused("what a World says is a fact, and an array of them holds no hole"))?;
      held.push(one.held.clone());
    }
    return Ok(Reply::Say(held));
  }
  let Some(kind) = object.get::<String>(ASK)? else {
    return Err(refused("a World says nothing, a fact, an array of facts or an ask, and this object is none"));
  };
  let on = object.get::<String>("on")?.unwrap_or_default();
  let words = match object.get::<Unknown<'_>>("words")? {
    Some(held) => of_js(env, &held)?.as_entries().unwrap_or_default().to_vec(),
    None => Vec::new(),
  };
  Ok(Reply::Ask { kind, on, words })
}
