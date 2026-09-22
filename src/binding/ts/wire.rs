use crate::{Fault, Object, ObjectRef, Text};
use serde_json::{Value, json};

pub fn inward(value: &Value) -> Result<Object, Fault> {
  Ok(match value {
    Value::Null => Object::none(),
    Value::Bool(value) => Object::bool(*value),
    Value::String(value) => Object::string(value),
    Value::Number(value) => {
      if let Some(value) = value.as_i64() {
        if value.unsigned_abs() > 9_007_199_254_740_991 {
          return Err(Fault::refused("the number exceeds the safe integer range of JavaScript"));
        }
        Object::int(value)
      } else {
        Object::float(value.as_f64().ok_or_else(|| Fault::refused("invalid number"))?)
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

pub fn outward(value: ObjectRef<'_>) -> Result<Value, Fault> {
  outward_at(value, 0, false)
}

pub fn record(value: ObjectRef<'_>) -> Result<Value, Fault> {
  outward_at(value, 0, true)
}

fn outward_at(value: ObjectRef<'_>, depth: usize, durable: bool) -> Result<Value, Fault> {
  if depth > 64 {
    return Err(Fault::refused("the value is cyclic or exceeds 64 levels"));
  }
  if let Some(text) = Text::of(value) {
    return Ok(json!({"is": "Text", "path": text.path, "content": text.content}));
  }
  if let Some(fault) = Fault::of(value) {
    return fault_value(&fault, depth + 1, durable);
  }
  if value.type_name() == "NoneType" {
    return Ok(Value::Null);
  }
  if let Some(value) = value.as_bool() {
    return Ok(json!(value));
  }
  if let Some(value) = value.as_int() {
    if value.unsigned_abs() > 9_007_199_254_740_991 {
      return Err(Fault::refused("the number exceeds the safe integer range of JavaScript"));
    }
    return Ok(json!(value));
  }
  if let Some(value) = value.as_float() {
    if !value.is_finite() {
      return Err(Fault::refused("the number is not finite"));
    }
    return Ok(if durable && value.fract() == 0.0 {
      json!({"is": "float", "args": [value.to_string()]})
    } else {
      json!(value)
    });
  }
  if let Some(value) = value.as_str() {
    return Ok(json!(value));
  }
  if let Some(values) = value.items() {
    return values
      .into_iter()
      .map(|value| outward_at(value, depth + 1, durable))
      .collect::<Result<Vec<_>, _>>()
      .map(Value::Array);
  }
  if let Some(values) = value.pairs() {
    let mut result = serde_json::Map::new();
    if value.type_name() != "dict" {
      result.insert("is".into(), json!(value.type_name()));
    }
    for (key, value) in values {
      let key = key.as_str().ok_or_else(|| Fault::refused("a plain map needs string keys"))?;
      result.insert(key.into(), outward_at(value, depth + 1, durable)?);
    }
    return Ok(Value::Object(result));
  }
  Err(Fault::refused(format!("{} cannot cross as plain data", value.type_name())))
}

fn fault_value(fault: &Fault, depth: usize, durable: bool) -> Result<Value, Fault> {
  let args = fault
    .args
    .iter()
    .map(|arg| outward_at(arg.as_ref(), depth, durable))
    .collect::<Result<Vec<_>, _>>()?;
  Ok(json!({"is": fault.name, "args": args}))
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
      let number = raw
        .parse::<i64>()
        .ok()
        .filter(|number| number.unsigned_abs() <= 9_007_199_254_740_991)
        .ok_or_else(|| Fault::refused("the number exceeds the safe integer range of JavaScript"))?;
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
