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

/// Preserve Python JSON floats before JavaScript can erase their decimal point.
pub fn decoded(value: Value, depth: usize) -> Result<Value, Fault> {
  if depth > 64 {
    return Err(Fault::refused("the record exceeds 64 levels"));
  }
  Ok(match value {
    Value::Number(number) => {
      if number.to_string().contains(['.', 'e', 'E']) {
        let value = number
          .as_f64()
          .filter(|value| value.is_finite())
          .ok_or_else(|| Fault::refused("invalid float"))?;
        if value.fract() == 0.0 {
          json!({"is": "float", "args": [value.to_string()]})
        } else {
          json!(value)
        }
      } else {
        let value = number
          .as_i64()
          .filter(|value| value.unsigned_abs() <= 9_007_199_254_740_991)
          .ok_or_else(|| {
          Fault::refused("the number exceeds the safe integer range of JavaScript")
        })?;
        json!(value)
      }
    }
    Value::Array(items) => Value::Array(
      items.into_iter().map(|item| decoded(item, depth + 1)).collect::<Result<_, _>>()?,
    ),
    Value::Object(items) => Value::Object(
      items
        .into_iter()
        .map(|(key, item)| Ok((key, decoded(item, depth + 1)?)))
        .collect::<Result<_, Fault>>()?,
    ),
    value => value,
  })
}

/// JSON has already been parsed. Check integer lexemes before a large one can cross as a rounded float.
pub fn check_numbers(line: &str) -> Result<(), Fault> {
  let bytes = line.as_bytes();
  let mut at = 0;
  while at < bytes.len() {
    if bytes[at] == b'"' {
      at += 1;
      while at < bytes.len() && bytes[at] != b'"' {
        at += if bytes[at] == b'\\' { 2 } else { 1 };
      }
      at += 1;
    } else if bytes[at] == b'-' || bytes[at].is_ascii_digit() {
      let start = at;
      while at < bytes.len() && matches!(bytes[at], b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-')
      {
        at += 1;
      }
      let number = &line[start..at];
      if !number.contains(['.', 'e', 'E'])
        && !number.parse::<i64>().is_ok_and(|value| value.unsigned_abs() <= 9_007_199_254_740_991)
      {
        return Err(Fault::refused("the number exceeds the safe integer range of JavaScript"));
      }
    } else {
      at += 1;
    }
  }
  Ok(())
}
