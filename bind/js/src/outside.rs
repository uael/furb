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

use std::{
  cell::{Cell, RefCell},
  rc::Rc,
};

use furb::{Fact, Gate, Reply, Value, World};
use napi::{
  Env, Error, JsValue, Result, Unknown,
  bindgen_prelude::{Array, FnArgs, Function, JavaScriptClassExt, JsObjectValue, Object, ObjectRef},
};

use crate::value::{Fact as Held, made, of_js, refused, to_js};

/// The name a question of the engine is said under.
///
/// A World that must know something before it answers a fact says a plain object, `{ ask, on, words }`, which
/// carries three words and nothing else, so it needs no class of its own. The boundary puts the question and
/// hands the answer back through `answered`, and a World never calls into a life that stands waiting for it.
const ASK: &str = "ask";

/// What a host of javascript gave a life: an object of the interpreter, and what came of calling it.
///
/// A life takes its World and gives it back only when it opened, so a World that throws while the life is
/// opening would take its fault with it, and the reference it holds on the host's object would never be
/// released. Both stand here instead, where the life and the crate's side of it reach the same one, whether the
/// life opened or not.
pub struct Gave {
  /// The object the host wrote, until it is released. A gate the host left out has none.
  object: RefCell<Option<ObjectRef>>,
  /// The first fault of the object, which the caller of the life throws once the call is over.
  raised: RefCell<Option<Error>>,
  /// The env of the call that is running, and nothing between two calls.
  ///
  /// An object of javascript is reached through the env of the call that is running, and through no other, so a
  /// call made while this holds nothing is no call at all: there is no interpreter on the stack to make it in.
  env: Cell<Option<Env>>,
}

/// One [`Gave`], as the life and the crate's side of it both hold it.
pub type Shared = Rc<Gave>;

impl Gave {
  /// What a host gave, held: the object, or nothing for a gate the host left out.
  pub fn new(object: Option<ObjectRef>) -> Shared {
    Rc::new(Gave { object: RefCell::new(object), raised: RefCell::new(None), env: Cell::new(None) })
  }

  /// The env of the call that is starting, which is how the object is reached while it runs.
  pub fn on(&self, env: &Env) {
    self.env.set(Some(*env));
  }

  /// The end of the call, after which the object is reached by nothing.
  pub fn off(&self) {
    self.env.set(None);
  }

  /// The first fault of the object, taken, which the caller throws once the call it was in is over.
  pub fn caught(&self) -> Option<Error> {
    self.raised.borrow_mut().take()
  }

  /// The object the host wrote, as the host reads it back.
  pub fn object<'env>(&self, env: &Env) -> Result<Unknown<'env>> {
    match self.object.borrow().as_ref() {
      Some(held) => made(env, held.get_value(env)?),
      None => made(env, napi::bindgen_prelude::Null),
    }
  }

  /// The reference on the object, released, which is the end of what the host gave.
  ///
  /// A reference holds an object of javascript against collection, and nothing releases one for you, so the
  /// life that made it releases it: when the life is collected, or when the life never opened at all.
  pub fn done(&self, env: &Env) -> Result<()> {
    match self.object.borrow_mut().take() {
      Some(held) => held.unref(env),
      None => Ok(()),
    }
  }

  /// One call of the object, and what it gave, or `none` where there was no call to make.
  ///
  /// A fault is kept and answered with nothing, so that the life runs to the end of the call it is in and the
  /// caller hears the fault whole. A life that carried on would hear a reply the host never gave.
  fn calls<T>(&self, none: T, said: impl FnOnce(&Env, &Object<'_>) -> Result<T>) -> T {
    if self.raised.borrow().is_some() {
      return none;
    }
    let Some(env) = self.env.get() else { return none };
    let got = self
      .object
      .borrow()
      .as_ref()
      .ok_or_else(|| refused("a host gave this life no object"))
      .and_then(|held| held.get_value(&env))
      .and_then(|held| said(&env, &held));
    match got {
      Ok(one) => one,
      Err(fault) => {
        // The first fault is the one that says what went wrong; every call after it was made on a host that had
        // already thrown.
        let mut held = self.raised.borrow_mut();
        if held.is_none() {
          *held = Some(fault);
        }
        none
      }
    }
  }
}

/// A World of javascript, as the crate reads one.
///
/// Every fault of the javascript object stands: a World that throws has said something a life cannot answer, and
/// the life stops with it rather than carrying on with a reply that was never given.
pub struct Worlds(pub Shared);

impl World for Worlds {
  fn hears(&mut self, fact: &Fact) -> Reply {
    let held = fact.clone();
    self.0.calls(Reply::Nothing, move |env, on| {
      let one = made(env, Held { held })?;
      answers(env, on, "hears", one)
    })
  }

  fn answered(&mut self, got: &Value) -> Reply {
    let held = got.clone();
    self.0.calls(Reply::Nothing, move |env, on| {
      let one = to_js(env, &held)?;
      answers(env, on, "answered", one)
    })
  }
}

/// A gate of javascript, as the crate reads one.
///
/// A host that reads no python leaves it out, and the word then runs and raises where it stands. One that reads
/// python gives an object with `gate`, and the findings it gives are what the model is told.
pub struct Gates(pub Shared);

impl Gate for Gates {
  fn gate(&mut self, word: &str, ladder: &[String], shape: &str) -> Vec<String> {
    if self.0.object.borrow().is_none() {
      return Vec::new();
    }
    let (word, ladder, shape) = (word.to_owned(), ladder.to_vec(), shape.to_owned());
    self.0.calls(Vec::new(), move |_env, on| {
      let call: Function<'_, FnArgs<(String, Vec<String>, String)>, Array<'_>> =
        on.get_named_property("gate").map_err(|_| refused("a gate of a host answers gate, and this one does not"))?;
      let said = call.apply(on, (word, ladder, shape).into())?;
      let mut out = Vec::with_capacity(said.len() as usize);
      for at in 0..said.len() {
        out.push(said.get::<String>(at)?.ok_or_else(|| refused("a finding of a gate is a text"))?);
      }
      Ok(out)
    })
  }
}

/// One call of a World, made and read: the method of that name, called with that one value.
fn answers(env: &Env, on: &Object<'_>, name: &str, one: Unknown<'_>) -> Result<Reply> {
  let call: Function<'_, Unknown<'_>, Unknown<'_>> = on
    .get_named_property(name)
    .map_err(|_| refused(&format!("a World of a host answers {name}, and this one does not")))?;
  let said = call.apply(on, one)?;
  reply_of(env, &said)
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
