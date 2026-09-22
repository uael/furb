//! Render before crossing to JavaScript, while values still have their Python types.
use crate::{Fault, ObjectRef};

pub fn turns(value: ObjectRef<'_>) -> Result<Vec<String>, Fault> {
  sequence(value)?
    .into_iter()
    .map(|turn| {
      let parts = sequence(turn)?;
      content(*parts.get(1).ok_or_else(|| Fault::refused("invalid turn"))?)
    })
    .collect()
}

fn sequence(value: ObjectRef<'_>) -> Result<Vec<ObjectRef<'_>>, Fault> {
  value.items().ok_or_else(|| Fault::refused("invalid transcript sequence"))
}

fn content(value: ObjectRef<'_>) -> Result<String, Fault> {
  sequence(value)?
    .into_iter()
    .map(|one| one.as_str().map_or_else(|| tag(one, 0), |text| Ok(text.to_owned())))
    .collect::<Result<Vec<_>, _>>()
    .map(|parts| parts.join("\n"))
}

fn tag(value: ObjectRef<'_>, depth: usize) -> Result<String, Fault> {
  if depth > 64 {
    return Err(Fault::refused("the transcript exceeds 64 levels"));
  }
  let values = sequence(value)?;
  let [name, held, body] = values.as_slice() else {
    return Err(Fault::refused("invalid tag"));
  };
  let name = name.as_str().ok_or_else(|| Fault::refused("invalid tag name"))?;
  let mut attrs = String::new();
  let mut parts = Vec::new();
  for field in sequence(*held)? {
    let pair = sequence(field)?;
    let [key, value] = pair.as_slice() else {
      return Err(Fault::refused("invalid tag attribute"));
    };
    let key = key.as_str().ok_or_else(|| Fault::refused("invalid attribute name"))?;
    let said = value.as_str().map_or_else(|| value.py_repr(), str::to_owned);
    if said.contains(['\n', '"']) {
      parts.push(format!("<{key}>\n{said}\n</{key}>"));
    } else {
      attrs.push_str(&format!(" {key}=\"{said}\""));
    }
  }
  if let Some(text) = body.as_str() {
    parts.push(text.to_owned());
  } else if body.type_name() == "list" {
    for one in sequence(*body)? {
      if one.type_name() == "tuple"
        && one
          .items()
          .is_some_and(|items| items.get(1).is_some_and(|held| held.type_name() == "list"))
      {
        parts.push(tag(one, depth + 1)?);
      } else {
        parts.push(one.py_repr());
      }
    }
  } else if body.type_name() != "NoneType" {
    parts.push(body.py_repr());
  }
  let inner = parts.into_iter().filter(|part| !part.is_empty()).collect::<Vec<_>>().join("\n");
  Ok(if inner.is_empty() {
    format!("<{name}{attrs}/>")
  } else {
    format!("<{name}{attrs}>\n{inner}\n</{name}>")
  })
}
