//! The operator, as the console of a host puts a prompt to it: the shapes it answers, and a line it writes back, read
//! as a value of the shape the prompt wants. Every console reads a line by these rules, so an answer means the same at
//! every host.

use indexmap::IndexMap;
use serde_json::value::RawValue;

use crate::{
  value::{Fault, IS, Object, marked},
  wire::inward,
};

/// Every plain shape the operator answers, by its name, in the order a host offers them: a list and a dict as a line
/// of JSON. The operator answers each shape that they make too: a list, a dict or a tuple of shapes that it answers,
/// as a line of JSON, and a union, as `int | None`.
pub const SHAPES: [&str; 7] = ["str", "None", "bool", "int", "float", "list", "dict"];

/// A shape that the operator answers, as its name reads.
enum Shape {
  Str,
  None,
  Bool,
  Int,
  Float,
  /// A list, whose every item has a shape, or any shape.
  List(Option<Box<Shape>>),
  /// A dict with keys of text, whose every value has a shape, or any shape.
  Dict(Option<Box<Shape>>),
  /// A tuple: the shape of each item.
  Tuple(Vec<Shape>),
  /// A Literal: its values.
  Choice(Vec<Object>),
  /// A union: the first of its shapes that reads a line.
  Either(Vec<Shape>),
}

/// Whether the operator answers a shape: one of SHAPES, a Literal whose every value is a string, a whole number, a
/// truth or None, as python names it, such as Literal['a', 1, True, None], or a list, a dict, a tuple or a union of
/// shapes that it answers, such as list[str], dict[str, int], tuple[int, str] or int | None.
pub fn answers(shape: &str) -> bool {
  parse(shape).is_some()
}

/// The parts of a text between the marks that stand in no brackets and in no quote.
fn parts(text: &str, mark: char) -> Vec<&str> {
  let (mut found, mut depth, mut quote, mut start, mut skip) = (vec![], 0, None, 0, false);
  for (at, c) in text.char_indices() {
    if skip {
      skip = false;
      continue;
    }
    match (quote, c) {
      (Some(_), '\\') => skip = true,
      (Some(open), c) if c == open => quote = None,
      (Some(_), _) => {}
      (None, '\'' | '"') => quote = Some(c),
      (None, '[' | '(' | '{') => depth += 1,
      (None, ']' | ')' | '}') => depth -= 1,
      (None, c) if c == mark && depth == 0 => {
        found.push(&text[start..at]);
        start = at + c.len_utf8();
      }
      _ => {}
    }
  }
  found.push(&text[start..]);
  found
}

/// A shape as its name reads, or none when the operator answers it not.
fn parse(name: &str) -> Option<Shape> {
  let text = name.trim();
  let arms = parts(text, '|');
  if arms.len() > 1 {
    let mut shapes = vec![];
    for arm in arms {
      match parse(arm)? {
        Shape::Either(more) => shapes.extend(more),
        one => shapes.push(one),
      }
    }
    return Some(Shape::Either(shapes));
  }
  let (head, inside) = match text.split_once('[') {
    Some((head, rest)) => (head.trim(), Some(rest.strip_suffix(']')?)),
    None => (text, None),
  };
  Some(match (head, inside) {
    ("str", None) => Shape::Str,
    ("None", None) => Shape::None,
    ("bool", None) => Shape::Bool,
    ("int", None) => Shape::Int,
    ("float", None) => Shape::Float,
    ("list", None) => Shape::List(None),
    ("dict", None) => Shape::Dict(None),
    ("list", Some(item)) => Shape::List(Some(Box::new(parse(item)?))),
    ("dict", Some(pair)) => match parts(pair, ',')[..] {
      [key, value] if matches!(parse(key)?, Shape::Str) => {
        Shape::Dict(Some(Box::new(parse(value)?)))
      }
      _ => return None,
    },
    ("tuple", Some(items)) => {
      Shape::Tuple(parts(items, ',').into_iter().map(parse).collect::<Option<_>>()?)
    }
    ("Literal", Some(_)) => Shape::Choice(choices(text)?),
    _ => return None,
  })
}

/// The values of a Literal that the operator answers, in the order of its shape, or none for any other shape.
fn choices(shape: &str) -> Option<Vec<Object>> {
  let mut rest = shape.strip_prefix("Literal[")?.strip_suffix(']')?;
  let mut values = vec![];
  loop {
    let (value, after) = choice(rest.trim_start())?;
    values.push(value);
    match after.trim_start().strip_prefix(',') {
      Some(next) => rest = next,
      None => return after.trim().is_empty().then_some(values),
    }
  }
}

/// One value of a Literal as python shows it, at the start of a text, and the text after it.
fn choice(text: &str) -> Option<(Object, &str)> {
  let Some(quote) = text.chars().next().filter(|c| *c == '\'' || *c == '"') else {
    let end = text.find(|c: char| c == ',' || c.is_whitespace()).unwrap_or(text.len());
    let value = match &text[..end] {
      "None" => Object::none(),
      "True" => Object::bool(true),
      "False" => Object::bool(false),
      word => Object::int(word.parse().ok()?),
    };
    return Some((value, &text[end..]));
  };
  let mut value = String::new();
  let mut chars = text.char_indices().skip(1);
  while let Some((at, c)) = chars.next() {
    match c {
      '\\' => {
        let (_, escaped) = chars.next()?;
        let digits = match escaped {
          'x' => 2,
          'u' => 4,
          'U' => 8,
          _ => 0,
        };
        if digits == 0 {
          value.push(match escaped {
            'n' => '\n',
            't' => '\t',
            'r' => '\r',
            '\\' | '\'' | '"' => escaped,
            _ => return None,
          });
        } else {
          let hex: String = (0..digits).filter_map(|_| chars.next().map(|(_, d)| d)).collect();
          value.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
        }
      }
      c if c == quote => return Some((Object::string(value), &text[at + 1..])),
      c => value.push(c),
    }
  }
  None
}

/// A line of the operator as a value of the shape a prompt wants, or why it is none. A number and a truth are read
/// from the line less the spaces around it, and a truth is yes, y, true or 1, or no, n, false or 0, in any case. A
/// list, a dict and a tuple are a line of JSON, whose every number is exact and whose every item has the shape of its
/// place: a tuple has the length of its shape, and a dict has keys of text. A tuple comes as the mark of a tuple, since
/// a host that reads the value reads a list for it. A Literal is one of its values, which the
/// line less the spaces around it names as python shows it, or, for a string, as its text. A union is the first of its
/// shapes that reads the line, and None for a line with no word when the union holds None.
pub fn answered(shape: &str, line: &str) -> Result<Object, Fault> {
  let held =
    parse(shape).ok_or_else(|| Fault::refused(format!("the operator answers no {shape}")))?;
  read(&held, shape.trim(), line)
}

/// A line as a value of a shape, under the name that the prompt gave to the whole shape.
fn read(shape: &Shape, name: &str, line: &str) -> Result<Object, Fault> {
  let no = || Fault::refused(format!("{line:?} is no {name}"));
  let text = line.trim();
  match shape {
    Shape::None => Ok(Object::none()),
    Shape::Str => Ok(Object::string(line)),
    Shape::Int => text.parse().map(Object::int).map_err(|_| no()),
    Shape::Float => text.parse().map(Object::float).map_err(|_| no()),
    Shape::Bool => match text.to_lowercase().as_str() {
      "y" | "yes" | "true" | "1" => Ok(Object::bool(true)),
      "n" | "no" | "false" | "0" => Ok(Object::bool(false)),
      _ => Err(Fault::refused(format!("{line:?} is neither yes nor no"))),
    },
    Shape::Choice(values) => {
      let named = |one: &&Object| one.py_repr() == text || one.as_ref().as_str() == Some(text);
      let none = || Fault::refused(format!("{line:?} is none of {name}"));
      values.iter().find(named).cloned().ok_or_else(none)
    }
    Shape::List(_) | Shape::Dict(_) | Shape::Tuple(_) => {
      let raw: &RawValue = serde_json::from_str(text).map_err(|_| no())?;
      built(shape, raw).map_err(|_| no())
    }
    Shape::Either(arms) => {
      if text.is_empty() && arms.iter().any(|one| matches!(one, Shape::None)) {
        return Ok(Object::none());
      }
      let reads = arms.iter().filter(|one| !matches!(one, Shape::None));
      reads.into_iter().find_map(|arm| read(arm, name, line).ok()).ok_or_else(no)
    }
  }
}

/// A value of JSON as a value of a shape, which each item of a list, a dict and a tuple has by its place.
fn built(shape: &Shape, raw: &RawValue) -> Result<Object, Fault> {
  let text = raw.get();
  let no = || Fault::refused(format!("{text} does not fit"));
  let first = text.as_bytes().first().copied();
  let number = matches!(first, Some(b'-' | b'0'..=b'9'));
  match shape {
    Shape::Str if first == Some(b'"') => written(raw),
    Shape::None if text == "null" => written(raw),
    Shape::Bool if text == "true" || text == "false" => written(raw),
    Shape::Int if number && !text.contains(['.', 'e', 'E']) => written(raw),
    Shape::Float if number => text.parse().map(Object::float).map_err(|_| no()),
    Shape::Choice(values) => {
      let one = written(raw)?;
      if values.iter().any(|value| value.py_repr() == one.py_repr()) { Ok(one) } else { Err(no()) }
    }
    Shape::List(None) | Shape::Dict(None) if matches!(first, Some(b'[' | b'{')) => {
      let one = written(raw)?;
      if one.as_ref().type_name() == if matches!(shape, Shape::List(_)) { "list" } else { "dict" } {
        Ok(one)
      } else {
        Err(no())
      }
    }
    Shape::List(Some(item)) if first == Some(b'[') => {
      let items: Vec<&RawValue> = serde_json::from_str(text).map_err(|_| no())?;
      Ok(Object::list(
        items.into_iter().map(|one| built(item, one)).collect::<Result<Vec<_>, _>>()?,
      ))
    }
    Shape::Tuple(shapes) if first == Some(b'[') => {
      let items: Vec<&RawValue> = serde_json::from_str(text).map_err(|_| no())?;
      if items.len() != shapes.len() {
        return Err(no());
      }
      let items = items.into_iter().zip(shapes).map(|(one, shape)| built(shape, one));
      let items = Object::list(items.collect::<Result<Vec<_>, _>>()?);
      // A host reads a list for a tuple, so a tuple comes as the mark that the engine reads as one.
      Ok(marked("tuple", [("args", Object::list([items]))]))
    }
    Shape::Dict(Some(item)) if first == Some(b'{') => {
      let map: IndexMap<String, &RawValue> = serde_json::from_str(text).map_err(|_| no())?;
      let marks = map.contains_key(IS);
      let pairs = map.into_iter().map(|(key, one)| Ok((Object::string(key), built(item, one)?)));
      dict(pairs.collect::<Result<Vec<_>, Fault>>()?, marks)
    }
    Shape::Either(arms) => arms.iter().find_map(|arm| built(arm, raw).ok()).ok_or_else(no),
    _ => Err(no()),
  }
}

/// A dict of the operator, whose pairs read as a mark of the engine when its keys hold the key of the mark, so the
/// engine never reads them as one.
fn dict(pairs: Vec<(Object, Object)>, marks: bool) -> Result<Object, Fault> {
  if !marks {
    return Ok(Object::dict(pairs));
  }
  let pairs = pairs.into_iter().map(|(key, one)| Object::tuple([key, one]));
  Ok(marked("dict", [("args", Object::list([Object::list(pairs)]))]))
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
      dict(pairs.collect::<Result<Vec<_>, Fault>>()?, marks)
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
