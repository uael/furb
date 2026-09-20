//! furb for javascript: one life of the engine, in the sandbox, with a World you write in javascript.
//!
//! The crate runs the engine in monty and gives its surface to a host. This is that surface for a host of
//! javascript: [`Life`] is one life, a host writes a World and hands it over, and [`Voice`] is how the host says
//! into the life the work that finishes later.
//!
//! Nothing of the engine crosses: the word of a rung runs where the engine runs, and every value that crosses is
//! plain, which `value` is the shape of. The crate takes care of the Kernel, so a host writes the World alone.

mod outside;
mod value;

use std::time::Duration;

use furb::{Ears, Entry, Life as Held, Refusal, Sand, Voice as Says, record};
use napi::{Env, Error, Result, Status, Unknown, bindgen_prelude::Object};
use napi_derive::napi;

use crate::{
  outside::{Gates, Worlds},
  value::{of_js, refused, to_js},
};

/// One life: the engine in the sandbox, and the outside it reaches.
///
/// A host opens one with a World of its own and drives it by calling words on it, the way an operator calls a
/// verb. What the host started and has not finished it says through its [`Voice`], and the life hears all of it
/// in the order it was said.
#[napi]
pub struct Life {
  /// The life, which holds the sandbox, the World and the gate.
  held: Held<Worlds, Gates>,
}

#[napi]
impl Life {
  /// A life, opened on a World, a gate and what a World kept of the life before it.
  ///
  /// The Voice is the one the host says into; a life made without one hears nothing that finishes later, which
  /// is enough for a World that answers everything where it hears it.
  #[napi(constructor)]
  pub fn new(
    env: &Env,
    world: Object<'_>,
    gate: Option<Object<'_>>,
    record: Option<String>,
    voice: Option<&mut Voice>,
  ) -> Result<Self> {
    let kept = match record {
      Some(held) => record::read(&held).map_err(|drift| refused(&drift.0))?,
      None => Vec::new(),
    };
    let ears = match voice {
      Some(held) => held.takes()?,
      None => Ears::made().1,
    };
    let mut worlds = Worlds::new(world.create_ref()?);
    let mut gates = Gates::new(gate.map(|one| one.create_ref()).transpose()?);
    worlds.on(env);
    gates.on(env);
    let held = Held::boot(Sand::default(), worlds, gates, ears, &kept);
    let mut life = match held {
      Ok(one) => Life { held: one },
      Err(why) => return Err(raised(&why)),
    };
    life.caught()?;
    life.held.world_mut().off();
    life.held.gate_mut().off();
    Ok(life)
  }

  /// The root chain of the life, which is the first act of any record.
  #[napi(getter)]
  pub fn root(&self) -> &str {
    self.held.root()
  }

  /// One word of the operator, run on a chain, and what it gave.
  ///
  /// This is how a host works a life: a verb of the engine is called by running a word that calls it, which is
  /// what an operator does from python too.
  #[napi]
  pub fn word<'env>(&mut self, env: &Env, word: String) -> Result<Unknown<'env>> {
    let got = self.calling(env, |held| held.word(&word));
    to_js(env, &got?)
  }

  /// Everything the host has said into its Voice, done in the life, in the order it was said.
  ///
  /// A command says what it wrote while it runs, a model answers an ask long after the ask was heard, and the
  /// operator answers a prompt whenever the operator answers it. This is the one door they come through.
  #[napi]
  pub fn heard(&mut self, env: &Env) -> Result<u32> {
    let got = self.calling(env, Held::heard)?;
    Ok(got as u32)
  }

  /// Wait until the host says something, or until this long has passed, and say whether anything waits.
  ///
  /// A life goes on when a fact is said in it, so while it waits for a model, a command or a person, nothing of
  /// it moves until its host speaks.
  #[napi]
  pub fn waits(&mut self, seconds: f64) -> bool {
    self.held.waits(Duration::from_secs_f64(seconds.max(0.0)))
  }

  /// What an act came to, and nothing at all while it waits.
  ///
  /// A host says what it owes first, since a fact it is holding may be the very one that settles the act, and
  /// then asks. It never waits in here: how long to wait is the host's to decide.
  #[napi]
  pub fn came<'env>(&mut self, env: &Env, act: String) -> Result<Option<Unknown<'env>>> {
    match self.calling(env, |held| held.came(&act))? {
      Some(value) => Ok(Some(to_js(env, &value)?)),
      None => Ok(None),
    }
  }

  /// The World of this life, which is the object the host handed over.
  #[napi(getter)]
  pub fn world<'env>(&self, env: &Env) -> Result<Unknown<'env>> {
    self.held.world().object(env)
  }
}

impl Life {
  /// One call of the engine, with the World and the gate able to reach javascript for the length of it.
  ///
  /// A World of javascript is an object of the interpreter, and the interpreter is reached by the env of the
  /// call that is running. So the env is given to both before the call and taken away after it, and neither
  /// holds one while nothing of javascript is on the stack.
  fn calling<T>(
    &mut self,
    env: &Env,
    call: impl FnOnce(&mut Held<Worlds, Gates>) -> std::result::Result<T, Refusal>,
  ) -> Result<T> {
    self.held.world_mut().on(env);
    self.held.gate_mut().on(env);
    let got = call(&mut self.held);
    self.held.world_mut().off();
    self.held.gate_mut().off();
    self.caught()?;
    got.map_err(|why| raised(&why))
  }

  /// The first fault of the World or of the gate, thrown now that the call they were in is over.
  ///
  /// A fault of the host is the host's and not the life's, so it stands as it was thrown, and a life that heard
  /// nothing where the host threw goes on for whoever catches it.
  fn caught(&mut self) -> Result<()> {
    if let Some(fault) = self.held.world_mut().raised.take() {
      return Err(fault);
    }
    if let Some(fault) = self.held.gate_mut().raised.take() {
      return Err(fault);
    }
    Ok(())
  }
}

/// How a host says into a life when nothing asked it to.
///
/// A Voice may be carried anywhere the host does its work, since what it says waits in order until the life is
/// ready to hear it. One is handed to a life when the life is opened, and the life hears what it holds at every
/// [`Life::heard`].
#[napi]
pub struct Voice {
  /// What the host says into.
  says: Says,
  /// The Ears of this Voice, until the life it was made for takes them.
  ///
  /// A Voice is heard by one life: the life that is opened on it takes these, and a second life opened on the
  /// same Voice finds nothing and is refused.
  ears: Option<Ears>,
}

#[napi]
impl Voice {
  /// A Voice and the Ears that hear it, made together.
  #[napi(constructor)]
  pub fn new() -> Self {
    let (says, ears) = Ears::made();
    Voice { says, ears: Some(ears) }
  }

  /// One fact, said into the life.
  ///
  /// A word said after the life is gone is dropped, since there is nobody to hear it.
  #[napi]
  pub fn fact(&self, one: &value::Fact) {
    self.says.send(one.held.clone());
  }

  /// One act, closed with a value, which is how the work a host started answers the act that asked for it.
  #[napi]
  pub fn close(&self, env: &Env, act: String, value: Unknown<'_>) -> Result<()> {
    self.says.close(act, of_js(&Env::from_raw(env.raw()), &value)?);
    Ok(())
  }

  /// One chain, paused, which stops it until the operator wakes it.
  #[napi]
  pub fn pause(&self, chain: String) {
    self.says.pause(chain);
  }

  /// Whether anything said into this Voice is still waiting to be heard.
  #[napi]
  pub fn waiting(&self) -> bool {
    self.says.waiting()
  }
}

impl Default for Voice {
  fn default() -> Self {
    Self::new()
  }
}

impl Voice {
  /// The Ears of this Voice, which the life it was made for takes once.
  fn takes(&mut self) -> Result<Ears> {
    self.ears.take().ok_or_else(|| refused("a Voice is heard by one life"))
  }
}

/// The line a World writes for one entry it was told to keep.
///
/// The record of a life is the lines a World kept, and a life is opened again on the text of them. What the
/// engine tells a World to keep is one entry, and this is the one line of it, so that every host writes the same
/// record and no host derives the shape of one for itself.
#[napi]
pub fn line(env: &Env, entry: Unknown<'_>) -> Result<String> {
  let held = of_js(env, &entry)?;
  let each = held.as_entries().unwrap_or_default();
  let made = Entry {
    before: each.first().and_then(furb::Value::as_str).unwrap_or_default().to_owned(),
    fact: furb::Fact(each.get(1).and_then(furb::Value::as_entries).unwrap_or_default().to_vec()),
    answer: each.get(2).cloned(),
  };
  Ok(made.line())
}

/// The name the World hears under, which is the name the engine takes it under.
#[napi]
pub const WORLD: &str = furb::host::WORLD;

/// One refusal of the engine, as the fault a host catches.
fn raised(why: &Refusal) -> Error {
  Error::new(Status::GenericFailure, why.to_string())
}
