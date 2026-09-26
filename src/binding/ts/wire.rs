use crate::{Fault, Object, ObjectRef, Text, value::marked};
use serde_json::{Value, json};

/// The largest integer a JavaScript number holds exactly.
const SAFE: u64 = 9_007_199_254_740_991;

/// A value of JavaScript, as the sandbox takes it. A JavaScript number that is whole is an int, and one with a
/// fraction is a float; a whole number past the safe range is refused, since JavaScript holds it rounded. napi
/// gives a whole number as an integer only within 32 bits, so the integer that reaches here past them is a BigInt,
/// which is exact.
pub fn inward(value: &Value) -> Result<Object, Fault> {
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

/// A value of the sandbox, as JavaScript reads it. Every value crosses, so an ear hears every fact.
pub fn outward(value: ObjectRef<'_>) -> Value {
  outward_at(value, 0, false)
}

/// A value of the sandbox, as the record holds it: a whole float keeps its mark, so a later life reads a float.
pub fn record(value: ObjectRef<'_>) -> Value {
  outward_at(value, 0, true)
}

/// Plain data crosses as itself. A value that JSON holds only in part crosses as a map that names its type under
/// `is` and holds under `args` what that type makes it again from, as a fault does, so it comes back in whole: an
/// int past the safe range, a float that is not finite, a map with a key that is no string. Anything else, and a
/// value past 64 levels, crosses as python shows it, as the Python door does.
fn outward_at(value: ObjectRef<'_>, depth: usize, durable: bool) -> Value {
  let mark = |name: &str, args: Value| json!({"is": name, "args": [args]});
  if depth > 64 {
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
    return Value::Array(
      values.into_iter().map(|value| outward_at(value, depth + 1, durable)).collect(),
    );
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

/// Read numbers from serde's raw values before JavaScript can round them or erase a decimal point.
pub fn decoded(value: &serde_json::value::RawValue, depth: usize) -> Result<Value, Fault> {
  if depth > 64 {
    return Err(Fault::refused("the record exceeds 64 levels"));
  }
  let raw = value.get();
  let invalid = |error: serde_json::Error| Fault::refused(error.to_string());
  Ok(match raw.as_bytes()[0] {
    b'[' => Value::Array(
      serde_json::from_str::<Vec<&serde_json::value::RawValue>>(raw)
        .map_err(invalid)?
        .into_iter()
        .map(|item| decoded(item, depth + 1))
        .collect::<Result<_, _>>()?,
    ),
    b'{' => Value::Object(
      serde_json::from_str::<indexmap::IndexMap<String, &serde_json::value::RawValue>>(raw)
        .map_err(invalid)?
        .into_iter()
        .map(|(key, item)| Ok((key, decoded(item, depth + 1)?)))
        .collect::<Result<_, Fault>>()?,
    ),
    b'-' | b'0'..=b'9' if !raw.contains(['.', 'e', 'E']) => {
      let number =
        raw.parse::<i64>().ok().filter(|number| number.unsigned_abs() <= SAFE).ok_or_else(
          || Fault::refused("the number exceeds the safe integer range of JavaScript"),
        )?;
      json!(number)
    }
    b'-' | b'0'..=b'9' => {
      let number = raw
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
        .ok_or_else(|| Fault::refused("invalid float"))?;
      if number.fract() == 0.0 {
        json!({"is": "float", "args": [number.to_string()]})
      } else {
        json!(number)
      }
    }
    _ => serde_json::from_str(raw).map_err(invalid)?,
  })
}
