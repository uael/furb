//! What a fact is made of, outside python.
//!
//! Every fact of the engine is a tuple: its kind, the act it is about, who said it, and its words. A host that is
//! not python hears the same fact with every value made plain, which [`Value`] is the shape of. The set is closed,
//! and it was read off the suite rather than guessed: plain data, a tuple, a map, a shape such as a text or an
//! exit, an exception, and the mark of a show, which no host reads and no record holds.

use std::fmt;

use serde::{Deserialize, Serialize};

/// How the plain form says a tuple, which a list is not, since a host that says a fact back says the fact it was.
pub const TUPLE: &str = "()";
/// How the plain form says a show or a filter: a mark of what it was, and nothing a host can read.
pub const SHOW: &str = "";

/// One value as it crosses to a host that is not python.
///
/// A `Tuple` is kept apart from a `List` because the engine and its suite read the difference. The record on disk
/// does not: [`Value::record`] writes a tuple as an array, which is the form the python World has always kept.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
  /// None.
  None,
  /// A truth.
  Bool(bool),
  /// A whole number.
  Int(i64),
  /// A number with a fraction.
  Float(f64),
  /// A text, which is also every name of an act.
  Str(String),
  /// A list, which stays a list.
  List(Vec<Value>),
  /// A tuple, which every fact is one of.
  Tuple(Vec<Value>),
  /// A map, by the order its keys were put.
  Map(Vec<(String, Value)>),
  /// A shape of the engine by its name and its fields, which a text and an exit are.
  Shape {
    /// The name of the shape, as the engine holds it.
    name: String,
    /// The fields of the shape, in the order the shape declares them.
    fields: Vec<(String, Value)>,
  },
  /// An exception by its name and what it was made with.
  Error {
    /// The name of the exception.
    name: String,
    /// What the exception was made with.
    args: Vec<Value>,
  },
  /// A show or a filter: the verb that was given it keeps it, and nothing crosses but the mark.
  Show,
  /// Something the host holds, by the name it holds it under, which crosses back to the host as itself.
  ///
  /// A show is one of these while a word carries it: the word takes it from a verb and hands it to another, and
  /// the host keeps the callable itself, since nothing of it can cross.
  Held(String),
}

impl Default for Value {
  /// A value that says nothing, which is what a word left unsaid carries.
  fn default() -> Self {
    Value::None
  }
}

impl Value {
  /// A text of the engine, by its path and what stands at it.
  pub fn text(path: impl Into<String>, content: impl Into<String>) -> Self {
    Value::Shape {
      name: "Text".to_owned(),
      fields: vec![("path".to_owned(), Value::Str(path.into())), ("content".to_owned(), Value::Str(content.into()))],
    }
  }

  /// A refusal, which is what a call the engine will not make raises in the one that made it.
  pub fn refused(why: impl Into<String>) -> Self {
    Value::Error { name: "Refused".to_owned(), args: vec![Value::Str(why.into())] }
  }

  /// The text of a value that is one, and nothing for a value that is none.
  pub fn as_str(&self) -> Option<&str> {
    match self {
      Value::Str(said) => Some(said),
      _ => None,
    }
  }

  /// The entries of a value that holds entries, which a tuple and a list both do.
  pub fn as_entries(&self) -> Option<&[Value]> {
    match self {
      Value::Tuple(held) | Value::List(held) => Some(held),
      _ => None,
    }
  }

  /// One field of a shape, by its name.
  pub fn field(&self, want: &str) -> Option<&Value> {
    match self {
      Value::Shape { fields, .. } => fields.iter().find(|(name, _)| name == want).map(|(_, held)| held),
      _ => None,
    }
  }

  /// The plain form of the value, which is what crosses to a host: a tuple, a shape, an exception and the mark
  /// of a show each stand under a name, and everything else is itself.
  ///
  /// This is the form `src/preamble.py` makes and reads, so what a host says back is made again as it was said.
  pub fn plain(&self) -> Value {
    match self {
      Value::None | Value::Bool(_) | Value::Int(_) | Value::Float(_) | Value::Str(_) => self.clone(),
      Value::List(held) => Value::List(held.iter().map(Value::plain).collect()),
      Value::Map(held) => Value::Map(held.iter().map(|(key, one)| (key.clone(), one.plain())).collect()),
      Value::Tuple(held) => marked(TUPLE, Value::List(held.iter().map(Value::plain).collect())),
      Value::Error { name, args } => marked(name, Value::List(args.iter().map(Value::plain).collect())),
      Value::Show | Value::Held(_) => Value::Map(vec![("is".to_owned(), Value::Str(SHOW.to_owned()))]),
      Value::Shape { name, fields } => {
        let mut held = vec![("is".to_owned(), Value::Str(name.clone()))];
        held.extend(fields.iter().map(|(key, one)| (key.clone(), one.plain())));
        Value::Map(held)
      }
    }
  }

  /// The value again from its plain form, as a host says it back.
  pub fn of_plain(said: &Value) -> Value {
    match said {
      Value::List(held) => Value::List(held.iter().map(Value::of_plain).collect()),
      Value::Tuple(held) => Value::Tuple(held.iter().map(Value::of_plain).collect()),
      Value::Map(held) => of_map(held),
      other => other.clone(),
    }
  }

  /// The value as the record holds it: the form the python World writes, where a tuple is an array.
  pub fn record(&self) -> serde_json::Value {
    use serde_json::Value as Json;
    match self {
      Value::None => Json::Null,
      Value::Bool(said) => Json::Bool(*said),
      Value::Int(said) => Json::from(*said),
      Value::Float(said) => serde_json::Number::from_f64(*said).map_or(Json::Null, Json::Number),
      Value::Str(said) => Json::String(said.clone()),
      Value::List(held) | Value::Tuple(held) => Json::Array(held.iter().map(Value::record).collect()),
      Value::Map(held) => Json::Object(held.iter().map(|(k, v)| (k.clone(), v.record())).collect()),
      Value::Shape { name, fields } => {
        let mut out = serde_json::Map::new();
        out.insert("is".to_owned(), Json::String(name.clone()));
        for (key, held) in fields {
          out.insert(key.clone(), held.record());
        }
        Json::Object(out)
      }
      Value::Error { name, args } => {
        let mut out = serde_json::Map::new();
        out.insert("is".to_owned(), Json::String(name.clone()));
        out.insert("args".to_owned(), Json::Array(args.iter().map(Value::record).collect()));
        Json::Object(out)
      }
      Value::Show | Value::Held(_) => Json::Object(serde_json::Map::new()),
    }
  }

  /// The value again from the record, as the python World reads it back.
  ///
  /// An array is a list here: the record keeps no tuple apart, and what reads an entry of it makes the tuple that
  /// entry is, exactly as the python World does when it gives a record to a later life.
  pub fn of_record(said: &serde_json::Value) -> Self {
    use serde_json::Value as Json;
    match said {
      Json::Null => Value::None,
      Json::Bool(held) => Value::Bool(*held),
      Json::Number(held) => held.as_i64().map_or_else(|| Value::Float(held.as_f64().unwrap_or(0.0)), Value::Int),
      Json::String(held) => Value::Str(held.clone()),
      Json::Array(held) => Value::List(held.iter().map(Value::of_record).collect()),
      Json::Object(held) => match held.get("is").and_then(Json::as_str) {
        Some(TUPLE) => Value::Tuple(shaped(held, "args")),
        Some(SHOW) => Value::Show,
        Some(name) if held.contains_key("args") => Value::Error { name: name.to_owned(), args: shaped(held, "args") },
        Some(name) => Value::Shape {
          name: name.to_owned(),
          fields: held
            .iter()
            .filter(|(key, _)| key.as_str() != "is")
            .map(|(key, one)| (key.clone(), Value::of_record(one)))
            .collect(),
        },
        None => Value::Map(held.iter().map(|(key, one)| (key.clone(), Value::of_record(one))).collect()),
      },
    }
  }
}

/// A value under a name, which is how the plain form says what is no plain data.
fn marked(name: &str, args: Value) -> Value {
  Value::Map(vec![("is".to_owned(), Value::Str(name.to_owned())), ("args".to_owned(), args)])
}

/// One map of the plain form, made again: a tuple, a show, an exception, a shape, or a map that is a map.
fn of_map(held: &[(String, Value)]) -> Value {
  let name = held.iter().find(|(key, _)| key == "is").and_then(|(_, one)| one.as_str());
  let args = || match held.iter().find(|(key, _)| key == "args") {
    Some((_, Value::List(each))) => each.iter().map(Value::of_plain).collect(),
    _ => Vec::new(),
  };
  match name {
    Some(TUPLE) => Value::Tuple(args()),
    Some(SHOW) => Value::Show,
    Some(name) if held.iter().any(|(key, _)| key == "args") => Value::Error { name: name.to_owned(), args: args() },
    Some(name) => Value::Shape {
      name: name.to_owned(),
      fields: held
        .iter()
        .filter(|(key, _)| key != "is")
        .map(|(key, one)| (key.clone(), Value::of_plain(one)))
        .collect(),
    },
    None => Value::Map(held.iter().map(|(key, one)| (key.clone(), Value::of_plain(one))).collect()),
  }
}

/// The entries a key of a map holds, each made again, and none at all when the key holds no array.
fn shaped(held: &serde_json::Map<String, serde_json::Value>, key: &str) -> Vec<Value> {
  held
    .get(key)
    .and_then(serde_json::Value::as_array)
    .map(|args| args.iter().map(Value::of_record).collect())
    .unwrap_or_default()
}

/// One fact: its kind, the act it is about, who said it, and its words.
///
/// A question carries the chain it is on as its first word, which [`Fact::on`] gives, and a fact that is no
/// question carries none.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fact(pub Vec<Value>);

impl Fact {
  /// A fact from its kind, the act it is about, who said it, and its words.
  pub fn new(kind: impl Into<String>, about: impl Into<String>, by: impl Into<String>, words: Vec<Value>) -> Self {
    let mut held = vec![Value::Str(kind.into()), Value::Str(about.into()), Value::Str(by.into())];
    held.extend(words);
    Fact(held)
  }

  /// The kind of the fact, which is its first slot.
  pub fn kind(&self) -> &str {
    self.0.first().and_then(Value::as_str).unwrap_or_default()
  }

  /// The act the fact is about.
  pub fn about(&self) -> &str {
    self.0.get(1).and_then(Value::as_str).unwrap_or_default()
  }

  /// Who said the fact: a rung, the operator, the World or the Kernel.
  pub fn by(&self) -> &str {
    self.0.get(2).and_then(Value::as_str).unwrap_or_default()
  }

  /// The words of the fact, which are everything after who said it.
  pub fn words(&self) -> &[Value] {
    self.0.get(3..).unwrap_or_default()
  }

  /// Whether the fact is a question, which its name says: a question is about itself, under its kind.
  pub fn question(&self) -> bool {
    let (kind, about) = (self.kind(), self.about());
    !kind.is_empty() && about.starts_with(kind) && about[kind.len()..].starts_with("://")
  }

  /// The chain a question is on, which is its first word, and nothing for a fact that is no question.
  pub fn on(&self) -> Option<&str> {
    self.question().then(|| self.words().first().and_then(Value::as_str)).flatten()
  }
}

impl fmt::Display for Fact {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{} {} by {}", self.kind(), self.about(), self.by())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_fact_says_its_kind_the_act_it_is_about_who_said_it_and_its_words() {
    let a = Fact::new("done", "read://operator.1.1", "world", vec![Value::text("/w/a.txt", "one\n")]);
    assert_eq!(a.kind(), "done");
    assert_eq!(a.about(), "read://operator.1.1");
    assert_eq!(a.by(), "world");
    assert_eq!(a.words().len(), 1);
    assert_eq!(a.words()[0].field("path"), Some(&Value::Str("/w/a.txt".to_owned())));
  }

  #[test]
  fn a_question_is_about_itself_under_its_kind_and_its_first_word_is_the_chain() {
    let q = Fact::new(
      "read",
      "read://operator.1.1",
      "operator",
      vec![Value::Str("chain://operator.1".to_owned()), Value::Str("a.txt".to_owned())],
    );
    assert!(q.question());
    assert_eq!(q.on(), Some("chain://operator.1"));
    let a = Fact::new("done", "read://operator.1.1", "world", vec![Value::None]);
    assert!(!a.question());
    assert_eq!(a.on(), None);
  }

  #[test]
  fn the_record_keeps_a_tuple_as_an_array_and_a_shape_by_its_name() {
    let held = Value::Tuple(vec![Value::Str("done".to_owned()), Value::text("a.txt", "x")]);
    let said = held.record();
    assert_eq!(said.to_string(), r#"["done",{"is":"Text","path":"a.txt","content":"x"}]"#);
    let back = Value::of_record(&said);
    assert_eq!(back, Value::List(vec![Value::Str("done".to_owned()), Value::text("a.txt", "x")]));
  }

  #[test]
  fn an_exception_crosses_as_its_name_and_what_it_was_made_with() {
    let held = Value::refused("a.txt is the door of nothing that lives");
    assert_eq!(held.record().to_string(), r#"{"is":"Refused","args":["a.txt is the door of nothing that lives"]}"#);
    assert_eq!(Value::of_record(&held.record()), held);
  }
}

#[cfg(test)]
mod plain {
  use super::*;

  /// One fact as it crosses: a tell of a read, which carries a text and the show it was told by.
  fn told() -> Value {
    Value::Tuple(vec![
      Value::Str("tell".to_owned()),
      Value::Str("rung://operator.1.1".to_owned()),
      Value::List(vec![Value::Tuple(vec![
        Value::Str("read".to_owned()),
        Value::List(vec![Value::Tuple(vec![Value::Str("path".to_owned()), Value::Str("a.txt".to_owned())])]),
        Value::List(vec![Value::Tuple(vec![Value::text("/w/a.txt", "one\n"), Value::Show])]),
      ])]),
    ])
  }

  #[test]
  fn what_crosses_plain_is_made_again_as_it_was_said() {
    let held = told();
    assert_eq!(Value::of_plain(&held.plain()), held);
  }

  #[test]
  fn a_tuple_stands_apart_from_a_list_in_the_plain_form() {
    let held = Value::Tuple(vec![Value::Int(1)]);
    assert_ne!(held.plain(), Value::List(vec![Value::Int(1)]).plain());
    assert_eq!(Value::of_plain(&held.plain()), held);
    assert_eq!(Value::of_plain(&Value::List(vec![Value::Int(1)])), Value::List(vec![Value::Int(1)]));
  }

  #[test]
  fn an_exception_and_a_shape_each_stand_under_their_name() {
    let refused = Value::refused("a.txt is the door of nothing that lives");
    assert_eq!(Value::of_plain(&refused.plain()), refused);
    let text = Value::text("a.txt", "one\n");
    assert_eq!(Value::of_plain(&text.plain()), text);
  }

  #[test]
  fn a_map_that_is_a_map_crosses_as_a_map() {
    let held = Value::Map(vec![("k".to_owned(), Value::Int(1))]);
    assert_eq!(Value::of_plain(&held.plain()), held);
  }

  #[test]
  fn what_no_host_reads_crosses_as_the_mark_of_what_it_was() {
    assert_eq!(Value::of_plain(&Value::Show.plain()), Value::Show);
    assert_eq!(Value::of_plain(&Value::Held("held.1".to_owned()).plain()), Value::Show);
  }
}
