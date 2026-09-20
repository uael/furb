//! What crosses between a life and a host that is javascript.
//!
//! The crate says every value plainly, and javascript says most of them itself: null, a truth, a number, a text,
//! an array and an object are the javascript ones. Three are not, so the module gives a class for each: [`Tuple`]
//! is a tuple, which javascript has no word for and the engine reads apart from a list, [`Shape`] is a shape of
//! the engine such as a text or an exit, and [`Fault`] is an exception by its name and what it was made with. A
//! show carries nothing, so it crosses as the one mark [`Show`].
//!
//! Nothing of javascript but these crosses. A host that hands over anything else is told what it handed over,
//! since a value the engine cannot hold is a fault of the host and not of the life.
#![expect(unsafe_code, reason = "the napi API is unsafe, and every cast below is guarded by the check above it")]

use furb::Value;
use napi::{
  Env, Error, JsValue, Result, Status, Unknown, ValueType,
  bindgen_prelude::{Array, JavaScriptClassExt, Null, Object, ToNapiValue},
};
use napi_derive::napi;

/// A tuple, which the engine reads apart from a list and javascript has no word for.
///
/// Every fact is one of these, and so is the standing of a chain and the turn of a model. An array of javascript
/// is a list, and this is the other one.
#[napi]
#[derive(Debug, Clone, PartialEq)]
pub struct Tuple {
  /// What the tuple holds, in order.
  pub(crate) items: Vec<Value>,
}

#[napi]
impl Tuple {
  /// A tuple of these values, in this order.
  #[napi(constructor)]
  pub fn new(env: &Env, items: Vec<Unknown<'_>>) -> Result<Self> {
    Ok(Tuple { items: each_of(env, &items)? })
  }

  /// What the tuple holds, in order.
  #[napi(getter)]
  pub fn items<'env>(&self, env: &Env) -> Result<Vec<Unknown<'env>>> {
    each_js(env, &self.items)
  }

  /// How many values the tuple holds.
  #[napi(getter)]
  pub fn length(&self) -> u32 {
    self.items.len() as u32
  }
}

/// A shape of the engine, by its name and its fields, which a text and an exit are.
///
/// The fields keep the order the shape declares them in, and `shape.fields` reads them all at once, by name.
#[napi]
#[derive(Debug, Clone, PartialEq)]
pub struct Shape {
  /// The name of the shape, as the engine holds it.
  pub(crate) name: String,
  /// The fields, in the order the shape declares them.
  pub(crate) fields: Vec<(String, Value)>,
}

#[napi]
impl Shape {
  /// A shape of this name, with these fields.
  #[napi(constructor)]
  pub fn new(env: &Env, name: String, fields: Object<'_>) -> Result<Self> {
    Ok(Shape { name, fields: mapped(env, &fields)? })
  }

  /// The name of the shape, as the engine holds it.
  #[napi(getter)]
  pub fn name(&self) -> &str {
    &self.name
  }

  /// The fields of the shape, by name, in the order the shape declares them.
  #[napi(getter)]
  pub fn fields(&self, env: &Env) -> Result<Object<'_>> {
    let mut held = Object::new(env)?;
    for (name, one) in &self.fields {
      held.set(name, to_js(env, one)?)?;
    }
    Ok(held)
  }
}

/// An exception of the engine, by its name and what it was made with.
///
/// It is a value and not a throw: a fault the engine hands back is a thing a host reads, and only a fault of the
/// call a host made is thrown in the host. `Refused` is the name of a call the engine will not make.
#[napi]
#[derive(Debug, Clone, PartialEq)]
pub struct Fault {
  /// The name of the exception.
  pub(crate) name: String,
  /// What the exception was made with.
  pub(crate) args: Vec<Value>,
}

#[napi]
impl Fault {
  /// An exception of this name, made with these values.
  #[napi(constructor)]
  pub fn new(env: &Env, name: String, args: Option<Vec<Unknown<'_>>>) -> Result<Self> {
    Ok(Fault { name, args: each_of(env, args.as_deref().unwrap_or_default())? })
  }

  /// The name of the exception.
  #[napi(getter)]
  pub fn name(&self) -> &str {
    &self.name
  }

  /// What the exception was made with.
  #[napi(getter)]
  pub fn args<'env>(&self, env: &Env) -> Result<Vec<Unknown<'env>>> {
    each_js(env, &self.args)
  }

  /// Whether the engine refused the call, which is the one fault a word of a model makes on purpose.
  #[napi]
  pub fn refused(&self) -> bool {
    self.name == "Refused"
  }
}

/// The mark of a show, which carries nothing a host can read.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Show;

#[napi]
impl Show {
  /// The one mark of a show.
  #[napi(constructor)]
  pub fn new() -> Self {
    Show
  }
}

impl Default for Show {
  fn default() -> Self {
    Show
  }
}

/// One fact of the engine: its kind, the act it is about, who said it, and its words.
#[napi]
#[derive(Debug, Clone, PartialEq)]
pub struct Fact {
  /// The fact, as the crate holds it.
  pub(crate) held: furb::Fact,
}

#[napi]
impl Fact {
  /// A fact from its kind, the act it is about, who said it, and its words.
  #[napi(constructor)]
  pub fn new(
    env: &Env,
    kind: String,
    about: String,
    by: Option<String>,
    words: Option<Vec<Unknown<'_>>>,
  ) -> Result<Self> {
    let held = each_of(env, words.as_deref().unwrap_or_default())?;
    Ok(Fact { held: furb::Fact::new(kind, about, by.unwrap_or_default(), held) })
  }

  /// The kind of the fact, which is its first slot.
  #[napi(getter)]
  pub fn kind(&self) -> &str {
    self.held.kind()
  }

  /// The act the fact is about.
  #[napi(getter)]
  pub fn about(&self) -> &str {
    self.held.about()
  }

  /// Who said the fact: a rung, the operator, the World or the Kernel.
  #[napi(getter)]
  pub fn by(&self) -> &str {
    self.held.by()
  }

  /// The words of the fact, which are everything after who said it.
  #[napi(getter)]
  pub fn words<'env>(&self, env: &Env) -> Result<Vec<Unknown<'env>>> {
    each_js(env, self.held.words())
  }

  /// Whether the fact is a question, which its name says: a question is about itself, under its kind.
  #[napi(getter)]
  pub fn question(&self) -> bool {
    self.held.question()
  }

  /// The chain a question is on, which is its first word, and nothing for a fact that is no question.
  #[napi(getter)]
  pub fn on(&self) -> Option<&str> {
    self.held.on()
  }
}

/// One value of the engine, as javascript reads it.
pub fn to_js<'env>(env: &Env, value: &Value) -> Result<Unknown<'env>> {
  match value {
    // Nothing crosses as `null`, which is what javascript says nothing with, and a host that says either `null`
    // or `undefined` back says nothing too.
    Value::None => made(env, Null),
    Value::Bool(one) => made(env, *one),
    Value::Int(one) => made(env, *one),
    Value::Float(one) => made(env, *one),
    Value::Str(one) => made(env, one.as_str()),
    Value::List(each) => made(env, each_js(env, each)?),
    Value::Tuple(each) => made(env, Tuple { items: each.clone() }),
    Value::Map(each) => {
      let mut held = Object::new(env)?;
      for (name, one) in each {
        held.set(name, to_js(env, one)?)?;
      }
      made(env, held)
    }
    Value::Shape { name, fields } => made(env, Shape { name: name.clone(), fields: fields.clone() }),
    Value::Error { name, args } => made(env, Fault { name: name.clone(), args: args.clone() }),
    Value::Show => made(env, Show),
  }
}

/// One value of javascript, as the engine holds it.
///
/// A class of this module is read before a plain object, since each one is an object too, and an array before
/// either, since the engine reads a list apart from a map.
pub fn of_js(env: &Env, held: &Unknown<'_>) -> Result<Value> {
  match held.get_type()? {
    ValueType::Undefined | ValueType::Null => return Ok(Value::None),
    ValueType::Boolean => {
      // Checked by the arm it is in, which is what makes the cast right.
      let one: bool = unsafe { held.cast()? };
      return Ok(Value::Bool(one));
    }
    ValueType::Number => {
      let one: f64 = unsafe { held.cast()? };
      // A number of javascript is one type, and the engine holds a whole number apart from a fraction, so a
      // number that is whole crosses as a whole one.
      return Ok(if one.fract() == 0.0 && one.abs() < 9.007_199_254_740_992e15 {
        Value::Int(one as i64)
      } else {
        Value::Float(one)
      });
    }
    ValueType::String => {
      let one: String = unsafe { held.cast()? };
      return Ok(Value::Str(one));
    }
    ValueType::Object => {}
    other => return Err(refused(&format!("a life holds no {other}"))),
  }

  if Tuple::instance_of(env, held)? {
    let one: &Tuple = unsafe { held.cast()? };
    return Ok(Value::Tuple(one.items.clone()));
  }
  if Shape::instance_of(env, held)? {
    let one: &Shape = unsafe { held.cast()? };
    return Ok(Value::Shape { name: one.name.clone(), fields: one.fields.clone() });
  }
  if Fault::instance_of(env, held)? {
    let one: &Fault = unsafe { held.cast()? };
    return Ok(Value::Error { name: one.name.clone(), args: one.args.clone() });
  }
  if Show::instance_of(env, held)? {
    return Ok(Value::Show);
  }
  let object: Object<'_> = unsafe { held.cast()? };
  if object.is_array()? {
    let each: Array<'_> = unsafe { held.cast()? };
    let mut out = Vec::with_capacity(each.len() as usize);
    for at in 0..each.len() {
      let one: Unknown<'_> = each.get(at)?.ok_or_else(|| refused("a list of a host holds no hole"))?;
      out.push(of_js(env, &one)?);
    }
    return Ok(Value::List(out));
  }
  Ok(Value::Map(mapped(env, &object)?))
}

/// The entries of one object of javascript, in the order the object holds them.
fn mapped(env: &Env, held: &Object<'_>) -> Result<Vec<(String, Value)>> {
  let names = Object::keys(held)?;
  let mut out = Vec::with_capacity(names.len());
  for name in names {
    let one: Unknown<'_> = held.get(&name)?.ok_or_else(|| refused(&format!("{name} names nothing")))?;
    out.push((name, of_js(env, &one)?));
  }
  Ok(out)
}

/// Every value of a list, as javascript reads them, which is an array.
pub fn each_js<'env>(env: &Env, each: &[Value]) -> Result<Vec<Unknown<'env>>> {
  each.iter().map(|one| to_js(env, one)).collect()
}

/// Every value of an array of javascript, as the engine holds them.
fn each_of(env: &Env, each: &[Unknown<'_>]) -> Result<Vec<Value>> {
  each.iter().map(|one| of_js(env, one)).collect()
}

/// One value of the module, as the value of no type that a host reads.
pub fn made<'env, T: ToNapiValue>(env: &Env, one: T) -> Result<Unknown<'env>> {
  // The value was just made by napi from `one`, so it is a value of this env and of no type this side names.
  let raw = unsafe { T::to_napi_value(env.raw(), one) }?;
  Ok(unsafe { Unknown::from_raw_unchecked(env.raw(), raw) })
}

/// What a host is told when it hands over a value a life holds none of.
pub fn refused(why: &str) -> Error {
  Error::new(Status::InvalidArg, why.to_owned())
}
