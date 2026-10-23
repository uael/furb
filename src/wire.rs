//! The wire: a value of the sandbox as JSON, which is how the record keeps a fact, how TypeScript reads one, and how
//! a host of rust speaks JSON, as the command line does.
//!
//! JSON holds plain data, and the wire keeps the rest whole: a value that JSON holds only in part crosses as a map
//! that names its type under `is`, with what that type is made again from, so it comes back in as it went out.

use crate::{
  Fault, Object, ObjectRef, Text,
  value::{DEEPEST, IS, marked},
};
use monty_types::unstable::{self, MontyGraph, MontyNode, NodeId};
use serde_json::{Value, json, value::RawValue};

/// The largest integer a JavaScript number holds exactly.
const SAFE: u64 = 9_007_199_254_740_991;

/// A value of JavaScript, as the sandbox takes it. A JavaScript number that is whole is an int, and one with a
/// fraction is a float; a whole number past the safe range is refused, since JavaScript holds it rounded. napi
/// gives a whole number as an integer only within 32 bits, so the integer that reaches here past them is a BigInt,
/// which is exact.
pub(crate) fn inward(value: &Value) -> Result<Object, Fault> {
  Ok(match value {
    Value::Null => Object::none(),
    Value::Bool(value) => Object::bool(*value),
    Value::String(value) => Object::string(value),
    Value::Number(value) => {
      if let Some(value) = value.as_i64() {
        Object::int(value)
      } else if let Some(value) = value.as_u64() {
        marked("int", [("args", Object::list([Object::string(value.to_string())]))])
      } else {
        let value = value.as_f64().ok_or_else(|| Fault::refused("invalid number"))?;
        if value.fract() != 0.0 {
          Object::float(value)
        } else if value.abs() <= SAFE as f64 {
          Object::int(value as i64)
        } else {
          return Err(Fault::refused("the number exceeds the safe integer range of JavaScript"));
        }
      }
    }
    Value::Array(values) => Object::list(values.iter().map(inward).collect::<Result<Vec<_>, _>>()?),
    Value::Object(values) => Object::dict(
      values
        .iter()
        .map(|(key, value)| Ok((Object::string(key), inward(value)?)))
        .collect::<Result<Vec<_>, Fault>>()?,
    ),
  })
}

/// A value of the sandbox, as JSON reads it, and JavaScript with it: a whole float is a number like any other. Every
/// value crosses, so an ear hears every fact.
pub fn outward(value: ObjectRef<'_>) -> Value {
  outward_at(value, 0, false)
}

/// One value of JSON, read with every number exact, as the sandbox takes it: a number with a point stays a float, and
/// a map that names its type under `is` comes in as that type, a fault among them.
pub fn parsed(text: &str) -> Result<Object, Fault> {
  let raw = serde_json::from_str::<&RawValue>(text).map_err(|no| Fault::refused(no.to_string()))?;
  let mut graph = MontyGraph::new();
  let root = built(&mut graph, raw, 0)?;
  whole(graph, root)
}

/// A value of the sandbox, from the graph that holds it and its id there.
pub(crate) fn whole(graph: MontyGraph, root: NodeId) -> Result<Object, Fault> {
  unstable::object_from_graph(graph, root).map_err(|no| Fault::refused(no.to_string()))
}

/// One value of JSON that serde read whole, put into a graph in one pass at the depth it stands at, with every number
/// exact: an integer within the safe range of JavaScript, and a float that stays a float, as a map marked `float` when
/// it is whole. It gives the id of the value in the graph. Serde reads the value first, so this pass meets only JSON
/// that is valid, and serde reads each string in it, so every string comes in as serde reads it.
pub(crate) fn built(graph: &mut MontyGraph, raw: &RawValue, depth: usize) -> Result<NodeId, Fault> {
  Reader { text: raw.get(), at: 0 }.value(graph, depth)
}

/// The place of a pass over the text of a value of JSON.
struct Reader<'a> {
  text: &'a str,
  at: usize,
}

impl Reader<'_> {
  /// The byte at the place, and nothing past the end.
  fn byte(&self) -> u8 {
    self.text.as_bytes().get(self.at).copied().unwrap_or_default()
  }

  /// The place after the whitespace of JSON.
  fn space(&mut self) {
    while matches!(self.byte(), b' ' | b'\t' | b'\n' | b'\r') {
      self.at += 1;
    }
  }

  /// Whether a container holds one more entry: past the comma before it, or false past the mark that ends it.
  fn more(&mut self, end: u8) -> bool {
    self.space();
    let one = self.byte();
    if one == b',' {
      self.at += 1;
      self.space();
    } else if one == end || one == 0 {
      self.at += 1;
      return false;
    }
    true
  }

  /// The value at the place, as a node of the graph.
  fn value(&mut self, graph: &mut MontyGraph, depth: usize) -> Result<NodeId, Fault> {
    if depth > 64 {
      return Err(Fault::refused("the record exceeds 64 levels"));
    }
    self.space();
    let node = match self.byte() {
      b'[' => {
        self.at += 1;
        let mut held = Vec::new();
        while self.more(b']') {
          held.push(self.value(graph, depth + 1)?);
        }
        MontyNode::List(held)
      }
      b'{' => {
        self.at += 1;
        let mut held = indexmap::IndexMap::new();
        while self.more(b'}') {
          let key = self.string()?;
          self.space();
          self.at += 1;
          held.insert(key, self.value(graph, depth + 1)?);
        }
        MontyNode::Dict(
          held.into_iter().map(|(key, one)| (graph.push(MontyNode::String(key)), one)).collect(),
        )
      }
      b'"' => MontyNode::String(self.string()?),
      b't' | b'f' | b'n' => {
        let (node, size) = match self.byte() {
          b't' => (MontyNode::Bool(true), 4),
          b'f' => (MontyNode::Bool(false), 5),
          _ => (MontyNode::None, 4),
        };
        self.at += size;
        node
      }
      _ => return self.number(graph),
    };
    Ok(graph.push(node))
  }

  /// The string at the place, which starts at its quote, as serde reads it, which finds its end and its escapes many
  /// bytes at a time.
  fn string(&mut self) -> Result<String, Fault> {
    let rest = self.text.get(self.at..).unwrap_or_default();
    let mut stream = serde_json::Deserializer::from_str(rest).into_iter::<String>();
    let got = stream.next().unwrap_or_else(|| serde_json::from_str(rest));
    self.at += stream.byte_offset();
    got.map_err(|no| Fault::refused(no.to_string()))
  }

  /// The number at the place: an integer, which must stand in the safe range of JavaScript, or a float, which stays
  /// a float, as a map marked `float` when it is whole.
  fn number(&mut self, graph: &mut MontyGraph) -> Result<NodeId, Fault> {
    let start = self.at;
    while matches!(self.byte(), b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9') {
      self.at += 1;
    }
    let raw = &self.text[start..self.at];
    if !raw.contains(['.', 'e', 'E']) {
      let number = raw.parse::<i64>().ok().filter(|number| number.unsigned_abs() <= SAFE);
      let number = number
        .ok_or_else(|| Fault::refused("the number exceeds the safe integer range of JavaScript"))?;
      return Ok(graph.push(MontyNode::Int(number)));
    }
    let number = raw.parse::<f64>().ok().filter(|number| number.is_finite());
    let number = number.ok_or_else(|| Fault::refused("invalid float"))?;
    if number.fract() != 0.0 {
      return Ok(graph.push(MontyNode::Float(number)));
    }
    let is = graph.push(MontyNode::String(IS.into()));
    let float = graph.push(MontyNode::String("float".into()));
    let args = graph.push(MontyNode::String("args".into()));
    let digits = graph.push(MontyNode::String(number.to_string()));
    let digits = graph.push(MontyNode::List(vec![digits]));
    Ok(graph.push(MontyNode::Dict(vec![(is, float), (args, digits)])))
  }
}

/// A value of the sandbox, as the record holds it: a whole float and a tuple keep their marks, so a later life reads
/// a float and a tuple.
pub(crate) fn record(value: ObjectRef<'_>) -> Value {
  outward_at(value, 0, true)
}

/// Plain data crosses as itself. A value that JSON holds only in part crosses as a map that names its type under
/// `is` and holds under `args` what that type makes it again from, as a fault does, so it comes back in whole: an
/// int past the safe range, a float that is not finite, a map with a key that is no string. Anything else, and a
/// part past the levels that a host is given, `DEEPEST`, crosses as python shows it, as the Python door gives it.
fn outward_at(value: ObjectRef<'_>, depth: usize, durable: bool) -> Value {
  let mark = |name: &str, args: Value| json!({"is": name, "args": [args]});
  if depth >= DEEPEST {
    return json!(value.py_repr());
  }
  if let Some(text) = Text::of(value) {
    return json!({"is": "Text", "path": text.path, "content": text.content});
  }
  if let Some(fault) = Fault::of(value) {
    let args = fault.args.iter().map(|arg| outward_at(arg.as_ref(), depth + 1, durable));
    return json!({"is": fault.name, "args": args.collect::<Vec<_>>()});
  }
  if value.type_name() == "NoneType" {
    return Value::Null;
  }
  if let Some(value) = value.as_bool() {
    return json!(value);
  }
  if let Some(value) = value.as_str() {
    return json!(value);
  }
  // An int past 64 bits has no reading as one, and python shows every int as its digits.
  if value.type_name() == "int" {
    return match value.as_int().filter(|value| value.unsigned_abs() <= SAFE) {
      Some(value) => json!(value),
      None => mark("int", json!(value.py_repr())),
    };
  }
  if let Some(value) = value.as_float() {
    return if !value.is_finite() || (durable && value.fract() == 0.0) {
      mark("float", json!(value.to_string()))
    } else {
      json!(value)
    };
  }
  // A class holds its attributes as pairs, but it is no map of them.
  if value.type_name() == "type" {
    return json!(value.py_repr());
  }
  if let Some(values) = value.items() {
    let values =
      Value::Array(values.into_iter().map(|value| outward_at(value, depth + 1, durable)).collect());
    // The store reads a line as an entry that holds one fact, and boot takes both as the tuples they are, so in the
    // record a tuple keeps its mark only below them.
    return if durable && depth > 1 && value.type_name() == "tuple" {
      mark("tuple", values)
    } else {
      values
    };
  }
  if let Some(values) = value.pairs() {
    let dict = value.type_name() == "dict";
    if values.iter().any(|(key, _)| key.as_str().is_none()) {
      if !dict {
        return json!(value.py_repr());
      }
      let pairs = values.into_iter().map(|(key, value)| {
        json!([outward_at(key, depth + 1, durable), outward_at(value, depth + 1, durable)])
      });
      return mark("dict", Value::Array(pairs.collect()));
    }
    let mut result = serde_json::Map::new();
    if !dict {
      result.insert("is".into(), json!(value.type_name()));
    }
    for (key, value) in values {
      result.insert(key.as_str().unwrap_or_default().into(), outward_at(value, depth + 1, durable));
    }
    return Value::Object(result);
  }
  json!(value.py_repr())
}

#[cfg(test)]
#[path = "wire.test.rs"]
mod test;
