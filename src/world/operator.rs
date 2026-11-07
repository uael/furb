//! The operator, as the console of a host puts a prompt to it: the shapes it answers, and a line it writes back, read
//! as a value of the shape the prompt wants. Every console reads a line by these rules, so an answer means the same at
//! every host.

use std::fmt::Display;

use indexmap::IndexMap;
use serde_json::value::RawValue;

use crate::{
  ear::{Co, say},
  fact::Fact,
  value::{Fault, IS, Object, marked},
  wire::inward,
};

/// Every plain shape the operator answers, by its name, in the order a host offers them: a list and a dict as a line
/// of JSON. The operator answers each shape that they make too: a list, a dict or a tuple of shapes that it answers,
/// as a line of JSON, and a union, as `int | None`.
pub const SHAPES: [&str; 7] = ["str", "None", "bool", "int", "float", "list", "dict"];

/// The name of the ear of a host that is the console of the operator: its start of a prompt makes
/// that prompt a question to him.
pub const CONSOLE: &str = "console";

/// A prompt put to the operator: its act, the chain it is on, which is the chain of its thread for
/// a prompt on a thread, its shape and its message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asked {
  pub about: String,
  pub on: String,
  pub shape: String,
  pub message: String,
}

impl Asked {
  /// The prompt a fact asks the operator, when it is one that no ear before the console took, as a
  /// console takes it: the console says its started, and closes at once a prompt of a shape it
  /// answers not, with the refusal of that shape, and gives it not. So every console of furb puts
  /// to the operator the same shapes. A prompt of None asks nothing, so the console closes it with
  /// None at once and gives it, as a notice that the operator reads and answers not.
  pub async fn taken<W: 'static>(co: &mut Co<W>, a: &Fact) -> Result<Option<Self>, Fault> {
    if a.kind() != "prompt" || !a.question() {
      return Ok(None);
    }
    let word = |at: usize| a.word(at).and_then(|one| one.as_str().map(str::to_owned));
    let scope = co.call("scope", vec![Object::string(a.about())], vec![]).await?;
    let asked = Self {
      about: a.about().to_owned(),
      on: scope.as_ref().as_str().unwrap_or(a.on()).to_owned(),
      shape: word(1).unwrap_or_default(),
      message: word(2).unwrap_or_default(),
    };
    say(co, Fact::says("started", &asked.about, []));
    if asked.shape == "None" {
      co.call("close", vec![Object::none()], vec![("id", Object::string(&asked.about))]).await?;
      return Ok(Some(asked));
    }
    if answers(&asked.shape) {
      return Ok(Some(asked));
    }
    let no = answered(&asked.shape, "").unwrap_or_else(|no| no.object());
    co.call("close", vec![no], vec![("id", Object::string(&asked.about))]).await?;
    Ok(None)
  }
}

/// A shape that the operator answers, as its name reads.
#[derive(Debug, Clone)]
pub enum Shape {
  Str,
  None,
  Bool,
  Int,
  Float,
  /// A list, whose every item has a shape, or any shape.
  List(Option<Box<Self>>),
  /// A dict with keys of text, whose every value has a shape, or any shape.
  Dict(Option<Box<Self>>),
  /// A tuple: the shape of each item.
  Tuple(Vec<Self>),
  /// A Literal: its values.
  Choice(Vec<Object>),
  /// A union: the first of its shapes that reads a line.
  Either(Vec<Self>),
}

/// Whether the operator answers a shape: one of SHAPES, a Literal whose every value is a string, a whole number, a
/// truth or None, as python names it, such as Literal['a', 1, True, None], or a list, a dict, a tuple or a union of
/// shapes that it answers, such as list[str], dict[str, int], tuple[int, str] or int | None.
pub fn answers(shape: &str) -> bool {
  Shape::parse(shape).is_some()
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

impl Shape {
  /// A shape as its name reads, or none when the operator answers it not.
  pub fn parse(name: &str) -> Option<Self> {
    let text = name.trim();
    let arms = parts(text, '|');
    if arms.len() > 1 {
      let mut shapes = vec![];
      for arm in arms {
        match Self::parse(arm)? {
          Self::Either(more) => shapes.extend(more),
          one => shapes.push(one),
        }
      }
      return Some(Self::Either(shapes));
    }
    let (head, inside) = match text.split_once('[') {
      Some((head, rest)) => (head.trim(), Some(rest.strip_suffix(']')?)),
      None => (text, None),
    };
    Some(match (head, inside) {
      ("str", None) => Self::Str,
      ("None", None) => Self::None,
      ("bool", None) => Self::Bool,
      ("int", None) => Self::Int,
      ("float", None) => Self::Float,
      ("list", None) => Self::List(None),
      ("dict", None) => Self::Dict(None),
      ("list", Some(item)) => Self::List(Some(Box::new(Self::parse(item)?))),
      ("dict", Some(pair)) => match parts(pair, ',')[..] {
        [key, value] if matches!(Self::parse(key)?, Self::Str) => {
          Self::Dict(Some(Box::new(Self::parse(value)?)))
        }
        _ => return None,
      },
      ("tuple", Some(items)) => {
        Self::Tuple(parts(items, ',').into_iter().map(Self::parse).collect::<Option<_>>()?)
      }
      ("Literal", Some(_)) => Self::Choice(literal(text)?),
      _ => return None,
    })
  }

  /// The lines that answer a shape that is a choice: yes and no for a bool, and each value of a
  /// Literal as `named` gives it. Any other shape gives none.
  pub fn choices(&self) -> Vec<String> {
    match self {
      Self::Bool => vec!["yes".to_owned(), "no".to_owned()],
      Self::Choice(values) => values.iter().map(named).collect(),
      _ => Vec::new(),
    }
  }

  /// Whether a line of no word answers the shape: a union that holds None, which such a line
  /// answers with None.
  pub fn takes_nothing(&self) -> bool {
    matches!(self, Self::Either(arms) if arms.iter().any(|one| matches!(one, Self::None)))
  }

  /// A shape as a prompt is given it, by its name: None itself, or the name of a type.
  pub fn object(name: &str) -> Object {
    if name == "None" { Object::none() } else { Object::string(name) }
  }
}

/// A value of a Literal as the operator names it: a string as its text, and any other value as
/// python shows it.
pub fn named(value: &Object) -> String {
  value.as_ref().as_str().map_or_else(|| value.py_repr(), str::to_owned)
}

/// The values of a Literal that the operator answers, in the order of its shape, or none for any other shape.
fn literal(shape: &str) -> Option<Vec<Object>> {
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
  let held = Shape::parse(shape)
    .ok_or_else(|| Fault::refused(format!("the operator answers no {shape}")))?;
  read(&held, shape.trim(), line)
}

/// A line as a value of a shape, under the name that the prompt gave to the whole shape.
fn read(shape: &Shape, name: &str, line: &str) -> Result<Object, Fault> {
  let no = || Fault::refused(format!("{line:?} is no {name}"));
  let cause = |why: &dyn Display| Fault::refused(format!("{line:?} is no {name}: {why}"));
  let text = line.trim();
  match shape {
    Shape::None => Ok(Object::none()),
    Shape::Str => Ok(Object::string(line)),
    Shape::Int => text.parse().map(Object::int).map_err(|why| cause(&why)),
    Shape::Float => text.parse().map(Object::float).map_err(|why| cause(&why)),
    Shape::Bool => match text.to_lowercase().as_str() {
      "y" | "yes" | "true" | "1" => Ok(Object::bool(true)),
      "n" | "no" | "false" | "0" => Ok(Object::bool(false)),
      _ => Err(Fault::refused(format!("{line:?} is neither yes nor no"))),
    },
    Shape::Choice(values) => {
      let names = |one: &&Object| one.py_repr() == text || one.as_ref().as_str() == Some(text);
      let none = || Fault::refused(format!("{line:?} is none of {name}"));
      values.iter().find(names).cloned().ok_or_else(none)
    }
    Shape::List(_) | Shape::Dict(_) | Shape::Tuple(_) => {
      let raw: &RawValue = serde_json::from_str(text).map_err(|why| cause(&why))?;
      built(shape, raw).map_err(|why| cause(&reason(&why)))
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
  let cause = |why: &dyn Display| Fault::refused(format!("{text} does not fit: {why}"));
  let first = text.as_bytes().first().copied();
  let number = matches!(first, Some(b'-' | b'0'..=b'9'));
  match shape {
    Shape::Str if first == Some(b'"') => written(raw),
    Shape::None if text == "null" => written(raw),
    Shape::Bool if text == "true" || text == "false" => written(raw),
    Shape::Int if number && !text.contains(['.', 'e', 'E']) => written(raw),
    Shape::Float if number => text.parse().map(Object::float).map_err(|why| cause(&why)),
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
      let items: Vec<&RawValue> = serde_json::from_str(text).map_err(|why| cause(&why))?;
      Ok(Object::list(
        items.into_iter().map(|one| built(item, one)).collect::<Result<Vec<_>, _>>()?,
      ))
    }
    Shape::Tuple(shapes) if first == Some(b'[') => {
      let items: Vec<&RawValue> = serde_json::from_str(text).map_err(|why| cause(&why))?;
      if items.len() != shapes.len() {
        return Err(no());
      }
      let items = items.into_iter().zip(shapes).map(|(one, shape)| built(shape, one));
      let items = Object::list(items.collect::<Result<Vec<_>, _>>()?);
      // A host reads a list for a tuple, so a tuple comes as the mark that the engine reads as one.
      Ok(marked("tuple", [("args", Object::list([items]))]))
    }
    Shape::Dict(Some(item)) if first == Some(b'{') => {
      let map: IndexMap<String, &RawValue> =
        serde_json::from_str(text).map_err(|why| cause(&why))?;
      let marks = map.contains_key(IS);
      let pairs = map.into_iter().map(|(key, one)| Ok((Object::string(key), built(item, one)?)));
      Ok(dict(pairs.collect::<Result<Vec<_>, Fault>>()?, marks))
    }
    Shape::Either(arms) => arms.iter().find_map(|arm| built(arm, raw).ok()).ok_or_else(no),
    _ => Err(no()),
  }
}

/// A dict of the operator, whose pairs read as a mark of the engine when its keys hold the key of the mark, so the
/// engine never reads them as one.
fn dict(pairs: Vec<(Object, Object)>, marks: bool) -> Object {
  if !marks {
    return Object::dict(pairs);
  }
  let pairs = pairs.into_iter().map(|(key, one)| Object::tuple([key, one]));
  marked("dict", [("args", Object::list([Object::list(pairs)]))])
}

/// Why a fault refused, as its message says it.
fn reason(fault: &Fault) -> String {
  let said = fault.args.first().and_then(|one| one.as_ref().as_str().map(str::to_owned));
  said.unwrap_or_else(|| fault.name.clone())
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
      Ok(dict(pairs.collect::<Result<Vec<_>, Fault>>()?, marks))
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
