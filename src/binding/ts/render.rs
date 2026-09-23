//! Render before crossing to JavaScript, while values still have their Python types.
//!
//! A turn reads here as `shown` and `rendered` of `src/furb/world.py` read it. A part of a turn that is no tag,
//! on which those raise, stands here as python shows it, so the World of this door renders every turn. So does a
//! tag nested past 64 levels, which bounds the depth of the render.
use crate::ObjectRef;

/// The text of each turn, as the model reads it.
pub fn turns(value: ObjectRef<'_>) -> Vec<String> {
  let turns = value.items().unwrap_or_default();
  turns
    .into_iter()
    .map(|turn| match turn.items().and_then(|parts| parts.get(1).copied()) {
      Some(content) => rendered(content),
      None => turn.py_repr(),
    })
    .collect()
}

/// What one turn holds, as one text: a text as itself, and anything else as its tag.
fn rendered(content: ObjectRef<'_>) -> String {
  let Some(parts) = content.items() else { return content.to_string() };
  let parts =
    parts.into_iter().map(|one| one.as_str().map_or_else(|| shown(one, 0), str::to_owned));
  parts.collect::<Vec<_>>().join("\n")
}

/// One tag as the model reads it, or what python shows of a value that is no tag.
fn shown(value: ObjectRef<'_>, depth: usize) -> String {
  tag(value, depth).unwrap_or_else(|| value.py_repr())
}

/// A tag: its name, its short attributes beside the name, and everything else inside it. A value that holds a
/// line break or a quotation mark stands in the body and not beside the name, and nothing is ever escaped.
fn tag(value: ObjectRef<'_>, depth: usize) -> Option<String> {
  if depth > 64 {
    return None;
  }
  let [name, held, body] = value.items()?.try_into().ok()?;
  let mut attrs = String::new();
  let mut parts = Vec::new();
  for field in held.items()? {
    let [key, value] = field.items()?.try_into().ok()?;
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
    // A tag says its attributes as a list, which nothing else a body holds does, a showing among it.
    for one in body.items()? {
      let tagged = one.type_name() == "tuple"
        && one
          .items()
          .is_some_and(|items| items.get(1).is_some_and(|held| held.type_name() == "list"));
      parts.push(if tagged { shown(one, depth + 1) } else { one.py_repr() });
    }
  } else if body.type_name() != "NoneType" {
    parts.push(body.py_repr());
  }
  let inner = parts.into_iter().filter(|part| !part.is_empty()).collect::<Vec<_>>().join("\n");
  Some(if inner.is_empty() {
    format!("<{name}{attrs}/>")
  } else {
    format!("<{name}{attrs}>\n{inner}\n</{name}>")
  })
}
