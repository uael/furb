//! What a fact is made of, outside python.
//!
//! Every fact of the engine is a tuple: its kind, the act it is about, who said it, and its words. A host that is
//! not python hears the same fact with every value made plain, which [`Value`] is the shape of: plain data, a
//! tuple, a map, a shape such as a text or an exit, an exception, and the mark of a callable, which no host reads.

use std::fmt;

/// How the plain form says a tuple, which a list is not, since a host that says a fact back says the fact it was.
pub const TUPLE: &str = "()";
/// How the plain form says a callable of the sandbox, which is nothing a host can read.
pub const SHOW: &str = "";
/// How the plain form says the name of an act, which is a string to a host and an act to python.
pub const ACT: &str = "Act";
/// How the plain form says a name of the engine, such as a verb, which is a callable a host cannot read.
pub const NAME: &str = "name";

/// One value as it crosses to a host that is not python.
///
/// A `Tuple` is kept apart from a `List` because the engine and its suite read the difference.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Value {
  /// None.
  #[default]
  None,
  /// A truth.
  Bool(bool),
  /// A whole number.
  Int(i64),
  /// A number with a fraction.
  Float(f64),
  /// A text.
  Str(String),
  /// The name of an act, which a verb gives and a caller holds of the act: a text to a host, and an act to python.
  Act(String),
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
  /// A name of the engine, such as a verb, which a host of python reads as its own and a host of rust cannot call.
  Name(String),
  /// A callable of the sandbox that has no name: a show, a filter or an ear a word made, which no host reads.
  Show,
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
      Value::Str(said) | Value::Act(said) => Some(said),
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

  /// One field of a shape, or one entry of a map, by its name.
  pub fn field(&self, want: &str) -> Option<&Value> {
    match self {
      Value::Shape { fields, .. } | Value::Map(fields) => {
        fields.iter().find(|(name, _)| name == want).map(|(_, held)| held)
      }
      _ => None,
    }
  }

  /// The plain form of the value, which is what crosses to a host: a tuple, a shape, an exception and the mark
  /// of a callable each stand under a name, and everything else is itself.
  ///
  /// This is the form `src/preamble.py` makes and reads, so what a host says back is made again as it was said.
  pub fn plain(&self) -> Value {
    match self {
      Value::None | Value::Bool(_) | Value::Int(_) | Value::Float(_) | Value::Str(_) => self.clone(),
      Value::List(held) => Value::List(held.iter().map(Value::plain).collect()),
      Value::Map(held) => Value::Map(held.iter().map(|(key, one)| (key.clone(), one.plain())).collect()),
      Value::Tuple(held) => marked(TUPLE, Value::List(held.iter().map(Value::plain).collect())),
      Value::Act(name) => marked(ACT, Value::List(vec![Value::Str(name.clone())])),
      Value::Error { name, args } => marked(name, Value::List(args.iter().map(Value::plain).collect())),
      Value::Show => Value::Map(vec![("is".to_owned(), Value::Str(SHOW.to_owned()))]),
      Value::Name(name) => {
        Value::Map(vec![("is".to_owned(), Value::Str(NAME.to_owned())), ("name".to_owned(), Value::Str(name.clone()))])
      }
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
}

/// A value under a name, which is how the plain form says what is no plain data.
fn marked(name: &str, args: Value) -> Value {
  Value::Map(vec![("is".to_owned(), Value::Str(name.to_owned())), ("args".to_owned(), args)])
}

/// One map of the plain form, made again: a tuple, an act, a callable, an exception, a shape, or a map.
fn of_map(held: &[(String, Value)]) -> Value {
  let name = held.iter().find(|(key, _)| key == "is").and_then(|(_, one)| one.as_str());
  let args = || match held.iter().find(|(key, _)| key == "args") {
    Some((_, Value::List(each))) => each.iter().map(Value::of_plain).collect(),
    _ => Vec::new(),
  };
  match name {
    Some(TUPLE) => Value::Tuple(args()),
    Some(SHOW) => Value::Show,
    Some(NAME) => Value::Name(
      held.iter().find(|(key, _)| key == "name").and_then(|(_, one)| one.as_str()).unwrap_or_default().to_owned(),
    ),
    Some(ACT) => {
      Value::Act(args().into_iter().next().and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default())
    }
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

/// One fact: its kind, the act it is about, who said it, and its words.
///
/// A question carries the chain it is on as its first word, which [`Fact::on`] gives, and a fact that is no
/// question carries none.
#[derive(Debug, Clone, PartialEq)]
pub struct Fact(pub Vec<Value>);

impl Fact {
  /// A fact from its kind, the act it is about, who said it, and its words.
  pub fn new(kind: impl Into<String>, about: impl Into<String>, by: impl Into<String>, words: Vec<Value>) -> Self {
    let mut held = vec![Value::Str(kind.into()), Value::Str(about.into()), Value::Str(by.into())];
    held.extend(words);
    Fact(held)
  }

  /// A saying: a fact with nobody said in it, which the bus fills in from the ear that speaks.
  pub fn says(kind: impl Into<String>, about: impl Into<String>, words: Vec<Value>) -> Self {
    Fact::new(kind, about, "", words)
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

  /// The fact as an ear yields it: its kind, the act it is about, and its words.
  ///
  /// Who said it is no slot of a saying. The bus fills it in from the ear that speaks, so a fact of the World
  /// is the World's own without the World saying so, and a fact that says it again shifts every word.
  pub fn said(&self) -> Vec<Value> {
    let mut held = vec![Value::Str(self.kind().to_owned()), Value::Str(self.about().to_owned())];
    held.extend(self.words().to_vec());
    held
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
    let verb = Value::Name("read".to_owned());
    assert_eq!(Value::of_plain(&verb.plain()), verb);
  }

  #[test]
  fn the_name_of_an_act_is_a_text_to_a_host_and_keeps_its_mark() {
    let act = Value::Act("prompt://operator.2".to_owned());
    assert_eq!(act.as_str(), Some("prompt://operator.2"));
    assert_eq!(Value::of_plain(&act.plain()), act);
  }
}
