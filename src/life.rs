//! One life of the engine, driven from a host.
//!
//! A life runs in a sandbox: the preamble first, in a module of its own, then the engine, then `boot`, which is
//! given the Kernel of the crate and one generator of the preamble for each ear of the host. From then the host
//! drives it by running one word at a time in that sandbox, the way an operator calls a verb, and by answering
//! what the sandbox asks of its ears.

use crate::{
  ENGINE, PREAMBLE, SHEET,
  ear::Host,
  fact::Value,
  gate::{Ty, named},
  sand::Sand,
};

/// The name the sandbox reaches its host by, which the preamble calls and nothing else does.
pub const HOST: &str = "host";
/// The name the Kernel hears by, whose one question, the gate, the crate answers itself.
pub const KERNEL: &str = "kernel";

/// What a life could not do.
#[derive(Debug, Clone, PartialEq)]
pub enum Refusal {
  /// What the engine raised, as the exception it is: its name, and what it was made with.
  ///
  /// A call the engine will not make raises `Refused` in the one that made it, and a host reads which it was by
  /// the name, so that it tells a call it may not make from one it made wrong.
  Raised(Value),
  /// What the crate could not read of what the engine gave.
  Read(String),
}

impl Refusal {
  /// The name of what the engine raised, and nothing for a fault of the reading.
  pub fn name(&self) -> &str {
    match self {
      Refusal::Raised(Value::Error { name, .. }) => name,
      _ => "",
    }
  }

  /// Whether the engine refused the call, which is the one fault a word of a model makes on purpose.
  pub fn refused(&self) -> bool {
    self.name() == "Refused"
  }
}

impl std::fmt::Display for Refusal {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Refusal::Raised(Value::Error { name, args }) => {
        let each: Vec<&str> = args.iter().filter_map(Value::as_str).collect();
        if each.is_empty() { f.write_str(name) } else { write!(f, "{name}: {}", each.join(" ")) }
      }
      Refusal::Raised(held) => write!(f, "{held:?}"),
      Refusal::Read(held) => f.write_str(held),
    }
  }
}

impl std::error::Error for Refusal {}

/// One life: a sandbox with the engine in it, and the host it reaches.
#[derive(Debug)]
pub struct Life<H> {
  /// Where the engine runs, which is monty.
  sand: Sand,
  /// The host, behind the gate of the Kernel.
  gated: Gated<H>,
  /// The root chain, which every life opens under the one name.
  root: String,
  /// What boot raised, if it raised, which comes out of the entry the host went in by while the life goes on.
  raised: Option<Value>,
}

impl<H: Host> Life<H> {
  /// A life, opened from what a World kept of the life before it, on the ears of these names, with the gate it
  /// reads a word by.
  ///
  /// The record is the entries the World kept, as a list, each entry the act made last before its fact, the
  /// fact, and for a query of a run what it was answered. The preamble runs in a module of its own, so the
  /// globals of a chain hold what the engine defines and nothing more; the engine runs next, and `boot` is given
  /// the Kernel and one generator for each name. The gate is a host's to share across its lives, since the
  /// reading it holds costs once and a clone of it is the same gate.
  pub fn boot(mut sand: Sand, host: H, ears: &[String], record: &Value, gate: Ty) -> Result<Self, Refusal> {
    let mut gated = Gated { host, gate };
    sand.run(PREAMBLE, &mut gated).map_err(one)?;
    let held = Value::List(named(ENGINE).into_iter().map(Value::Str).collect());
    let names = Value::List(ears.iter().map(|name| Value::Str(name.clone())).collect());
    let opening = format!(
      "__engine = module({})\n__sheet = module({})\n__unwiring = unwiring(__engine, {HOST})\n__root = opened(__engine, __sheet, {}, {}, {HOST}, {})\n__root\n",
      shown(&Value::Str(ENGINE.to_owned())),
      shown(&Value::Str(SHEET.to_owned())),
      shown(&held),
      shown(&record.plain()),
      shown(&names)
    );
    let got = sand.run(&opening, &mut gated).map_err(one)?;
    let Some(root) = got.field("root").and_then(Value::as_str).map(str::to_owned) else {
      return Err(Refusal::Read(format!("a life opens on a chain, and {got:?} is none")));
    };
    let raised = got.field("raised").filter(|one| **one != Value::None).cloned();
    Ok(Life { sand, gated, root, raised })
  }

  /// What boot raised, if it raised: a drift, which breaks the journal while the life goes on with nothing kept,
  /// or a refusal of the ears it was given.
  pub fn raised(&self) -> Option<&Value> {
    self.raised.as_ref()
  }

  /// The root chain of the life, which is the first act of any record.
  pub fn root(&self) -> &str {
    &self.root
  }

  /// One word of the operator, run in the names of the engine, and what it gave.
  ///
  /// This is how a host works a life: a verb of the engine is called by running a word that calls it, which is
  /// what an operator does from python too. What the word binds stays bound, as a word of the operator does.
  pub fn word(&mut self, word: &str) -> Result<Value, Refusal> {
    let asked = format!("asked(__engine, {}, __unwiring)", shown(&Value::Str(word.to_owned())));
    let said = self.sand.run(&asked, &mut self.gated).map_err(one)?;
    Ok(Value::of_plain(&said))
  }

  /// The host of this life, which a host reads what its ears kept off.
  pub fn host(&self) -> &H {
    &self.gated.host
  }

  /// The host of this life, to change.
  pub fn host_mut(&mut self) -> &mut H {
    &mut self.gated.host
  }
}

/// The host, with the one call of the Kernel, which hands over the sheet of a word, answered by the gate of the crate
/// before any ear hears it.
struct Gated<H> {
  host: H,
  gate: Ty,
}

impl<H> std::fmt::Debug for Gated<H> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("Gated")
  }
}

impl<H: Host> Host for Gated<H> {
  fn called(&mut self, name: &str, said: &Value) -> Value {
    if name != KERNEL {
      return self.host.called(name, said);
    }
    let sheet = said.as_str().unwrap_or_default();
    let found = self.gate.checked(sheet);
    Value::List(
      found
        .into_iter()
        .map(|(line, why)| Value::List(vec![Value::Int(i64::try_from(line).unwrap_or_default()), Value::Str(why)]))
        .collect(),
    )
  }
}

/// One fault of the sandbox, as the life reads it.
fn one(raised: Value) -> Refusal {
  Refusal::Raised(Value::of_plain(&raised))
}

/// One plain value as the python literal that says it again, which is how a host hands a value to the sandbox.
pub(crate) fn shown(value: &Value) -> String {
  match value {
    Value::None => "None".to_owned(),
    Value::Bool(held) => if *held { "True" } else { "False" }.to_owned(),
    Value::Int(held) => held.to_string(),
    Value::Float(held) => {
      if held.is_finite() && held.fract() == 0.0 && held.abs() < 1e15 {
        format!("{held:.1}")
      } else {
        format!("{held:?}")
      }
    }
    Value::Str(held) => quoted(held),
    Value::List(held) => {
      let each: Vec<String> = held.iter().map(shown).collect();
      format!("[{}]", each.join(", "))
    }
    Value::Map(held) => {
      let each: Vec<String> = held.iter().map(|(key, one)| format!("{}: {}", quoted(key), shown(one))).collect();
      format!("{{{}}}", each.join(", "))
    }
    Value::Tuple(_) | Value::Act(_) | Value::Name(_) | Value::Shape { .. } | Value::Error { .. } | Value::Show => {
      shown(&value.plain())
    }
  }
}

/// One text as the python literal that says it again.
///
/// The escapes of python and of rust are not the same, so this writes the literal itself: a text of any character
/// crosses to the sandbox as the text it was.
fn quoted(said: &str) -> String {
  let mut held = String::with_capacity(said.len() + 2);
  held.push('"');
  for one in said.chars() {
    match one {
      '"' => held.push_str("\\\""),
      '\\' => held.push_str("\\\\"),
      '\n' => held.push_str("\\n"),
      '\r' => held.push_str("\\r"),
      '\t' => held.push_str("\\t"),
      one if (one as u32) < 0x20 || one as u32 == 0x7F => held.push_str(&format!("\\x{:02x}", one as u32)),
      one => held.push(one),
    }
  }
  held.push('"');
  held
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_text_of_any_character_crosses_to_the_sandbox_as_the_text_it_was() {
    assert_eq!(quoted("one\ntwo"), "\"one\\ntwo\"");
    assert_eq!(quoted("a \"word\" and a \\"), "\"a \\\"word\\\" and a \\\\\"");
    assert_eq!(quoted("a bell \u{7}"), "\"a bell \\x07\"");
    assert_eq!(quoted("a face \u{1F600}"), "\"a face \u{1F600}\"");
  }

  #[test]
  fn a_value_a_host_hands_over_is_the_word_that_says_it_again() {
    assert_eq!(shown(&Value::None), "None");
    assert_eq!(shown(&Value::Bool(true)), "True");
    assert_eq!(shown(&Value::Float(30.0)), "30.0");
    assert_eq!(shown(&Value::List(vec![Value::Int(1), Value::Str("two".to_owned())])), "[1, \"two\"]");
    assert_eq!(shown(&Value::refused("no file")), "{\"is\": \"Refused\", \"args\": [\"no file\"]}");
    assert_eq!(shown(&Value::Tuple(vec![Value::Int(1)])), "{\"is\": \"()\", \"args\": [1]}");
  }
}
