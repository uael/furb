//! The operator, as the console of a host puts a thread to it: the shapes it answers, and a line it writes back, read
//! as a value of the shape the thread wants. Every console reads a line by these rules, so an answer means the same at
//! every host.

use indexmap::IndexMap;
use serde_json::value::RawValue;

use crate::{
  value::{Fault, IS, Object, marked},
  wire::inward,
};

/// Every shape the operator answers, by its name, in the order a host offers them: a list and a dict as a line of
/// JSON.
pub const SHAPES: [&str; 7] = ["str", "None", "bool", "int", "float", "list", "dict"];

/// A line of the operator as a value of the shape a thread wants, or why it is none. A number and a truth are read
/// from the line less the spaces around it, and a truth is yes, y, true or 1, or no, n, false or 0, in any case. A
/// list and a dict are a line of JSON, whose every number is exact.
pub fn answered(shape: &str, line: &str) -> Result<Object, Fault> {
  let no = || Fault::refused(format!("{line:?} is no {shape}"));
  let text = line.trim();
  match shape {
    "None" => Ok(Object::none()),
    "str" => Ok(Object::string(line)),
    "int" => text.parse().map(Object::int).map_err(|_| no()),
    "float" => text.parse().map(Object::float).map_err(|_| no()),
    "bool" => match text.to_lowercase().as_str() {
      "y" | "yes" | "true" | "1" => Ok(Object::bool(true)),
      "n" | "no" | "false" | "0" => Ok(Object::bool(false)),
      _ => Err(Fault::refused(format!("{line:?} is neither yes nor no"))),
    },
    "list" | "dict" => {
      let value = serde_json::from_str(text).map_err(|_| no()).and_then(written)?;
      if value.as_ref().type_name() == shape { Ok(value) } else { Err(no()) }
    }
    _ => Err(Fault::refused(format!("the operator answers no {shape}"))),
  }
}

/// A value of JSON as the operator writes it, which is plain data: every number exact, which JSON read as a number
/// already, a whole number past 64 bits as its digits, and a map that holds the key of the mark as its pairs, so the
/// engine never reads it as a mark.
fn written(raw: &RawValue) -> Result<Object, Fault> {
  let invalid = |no: serde_json::Error| Fault::refused(no.to_string());
  match raw.get().as_bytes().first() {
    Some(b'[') => {
      let items: Vec<&RawValue> = serde_json::from_str(raw.get()).map_err(invalid)?;
      Ok(Object::list(items.into_iter().map(written).collect::<Result<Vec<_>, _>>()?))
    }
    Some(b'{') => {
      let map: IndexMap<String, &RawValue> = serde_json::from_str(raw.get()).map_err(invalid)?;
      let marks = map.contains_key(IS);
      let pairs = map.into_iter().map(|(key, one)| Ok((Object::string(key), written(one)?)));
      let pairs = pairs.collect::<Result<Vec<_>, Fault>>()?;
      if !marks {
        return Ok(Object::dict(pairs));
      }
      let pairs = pairs.into_iter().map(|(key, one)| Object::tuple([key, one]));
      Ok(marked("dict", [("args", Object::list([Object::list(pairs)]))]))
    }
    Some(b'-' | b'0'..=b'9') => {
      let text = raw.get();
      Ok(match text.parse() {
        Ok(whole) => Object::int(whole),
        Err(_) if text.contains(['.', 'e', 'E']) => Object::float(text.parse().unwrap_or_default()),
        Err(_) => marked("int", [("args", Object::list([Object::string(text)]))]),
      })
    }
    _ => inward(&serde_json::from_str(raw.get()).map_err(invalid)?),
  }
}

#[cfg(test)]
#[path = "operator.test.rs"]
mod test;
