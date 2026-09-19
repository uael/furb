//! What a model reads of a chain: its turns, and the tags they hold.
//!
//! A chain tells its acts, and what it told is the transcript of that chain. The engine folds that transcript
//! into turns, which are what a provider is given: a user turn of the tags the chain told, and an assistant turn
//! of what the model wrote. A host that asks a model reads the turns and makes the messages of its provider, so
//! this is the shape it reads them in.
//!
//! Nothing here is a shape of the engine. A turn is a tuple, a tag is a tuple, and both cross plain, so what is
//! here is the reading of a plain value and never a value of its own.
//!
//! [`Tag::shown`] and [`Turn::rendered`] make the text of a turn, word for word as the python World makes it, so
//! a chain reads the same to a model whichever host runs it, and the cache of a provider holds across the two.

use std::fmt::Write;

use crate::fact::Value;

/// What one turn of a model cost: the tokens it read, the tokens it wrote, and the money of it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Usage {
  /// The tokens the model read.
  pub input: i64,
  /// The tokens the model wrote.
  pub output: i64,
  /// The tokens the model read from the cache of the provider.
  pub cached: i64,
  /// The tokens the provider wrote to its cache.
  pub wrote: i64,
  /// What the turn cost, in dollars.
  pub cost: f64,
}

impl Usage {
  /// What a turn cost, from what the engine holds of it, and nothing for a turn that cost nothing.
  pub fn of(got: &Value) -> Option<Self> {
    let held = got.as_entries()?;
    let count = |at: usize| match held.get(at) {
      Some(Value::Int(one)) => *one,
      Some(Value::Float(one)) => *one as i64,
      _ => 0,
    };
    let cost = match held.get(4) {
      Some(Value::Float(one)) => *one,
      Some(Value::Int(one)) => *one as f64,
      _ => 0.0,
    };
    Some(Usage { input: count(0), output: count(1), cached: count(2), wrote: count(3), cost })
  }
}

/// One tag of a chain: its name, what stands beside the name, and what stands inside it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Tag {
  /// The name of the tag.
  pub name: String,
  /// What stands beside the name, each an attribute and what it holds.
  pub attrs: Vec<(String, Value)>,
  /// What stands inside the tag.
  pub body: Body,
}

impl Tag {
  /// The tag a value holds, and nothing for a value that is no tag.
  ///
  /// A tag is a tuple of three whose second entry holds entries, which is how the World tells a tag from
  /// anything else a body holds.
  pub fn of(got: &Value) -> Option<Self> {
    let held = got.as_entries()?;
    let [name, attrs, body] = held else { return None };
    let attrs = attrs.as_entries()?;
    let attrs = attrs
      .iter()
      .filter_map(|one| match one.as_entries() {
        Some([key, held]) => Some((key.as_str().unwrap_or_default().to_owned(), held.clone())),
        _ => None,
      })
      .collect();
    Some(Tag { name: name.as_str()?.to_owned(), attrs, body: Body::of(body) })
  }

  /// One tag as the model reads it: its name, its short attributes beside the name, and everything else inside.
  ///
  /// A value that holds a line break or a quotation mark stands in the body and not beside the name, and nothing
  /// is ever escaped, so a text crosses to the model byte for byte.
  pub fn shown(&self) -> String {
    let (mut attrs, mut parts) = (String::new(), Vec::new());
    for (key, value) in &self.attrs {
      let said = match value {
        Value::Str(held) => held.clone(),
        held => repr(held),
      };
      if said.contains('\n') || said.contains('"') {
        parts.push(format!("<{key}>\n{said}\n</{key}>"));
      } else {
        let _ = write!(attrs, " {key}=\"{said}\"");
      }
    }
    match &self.body {
      Body::Nothing => {}
      Body::Text(held) => parts.push(held.clone()),
      Body::Parts(held) => parts.extend(held.iter().map(Shown::inside)),
    }
    let inner: Vec<String> = parts.into_iter().filter(|one| !one.is_empty()).collect();
    let name = &self.name;
    if inner.is_empty() {
      format!("<{name}{attrs}/>")
    } else {
      format!("<{name}{attrs}>\n{}\n</{name}>", inner.join("\n"))
    }
  }
}

/// What stands inside a tag.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Body {
  /// Nothing at all, so the tag says itself and closes.
  #[default]
  Nothing,
  /// One text.
  Text(String),
  /// The parts of it, each a tag of its own or a value the tag shows.
  Parts(Vec<Shown>),
}

impl Body {
  /// What stands inside a tag, from what the engine holds of it.
  pub fn of(got: &Value) -> Self {
    match got {
      Value::Str(held) => Body::Text(held.clone()),
      Value::List(held) | Value::Tuple(held) => Body::Parts(held.iter().map(Shown::of).collect()),
      _ => Body::Nothing,
    }
  }
}

/// One part of what a turn or a tag holds: a tag of its own, or a value it shows.
#[derive(Debug, Clone, PartialEq)]
pub enum Shown {
  /// One tag.
  Tag(Tag),
  /// One value, which the tag shows as python shows it.
  Held(Value),
}

impl Shown {
  /// One part, from what the engine holds of it.
  pub fn of(got: &Value) -> Self {
    Tag::of(got).map_or_else(|| Shown::Held(got.clone()), Shown::Tag)
  }

  /// The part as it stands in the body of a tag, where a value that is no tag stands as python shows it.
  pub fn inside(&self) -> String {
    match self {
      Shown::Tag(held) => held.shown(),
      Shown::Held(held) => repr(held),
    }
  }

  /// The part as it stands in a turn, where a text stands as itself.
  pub fn said(&self) -> String {
    match self {
      Shown::Tag(held) => held.shown(),
      Shown::Held(Value::Str(held)) => held.clone(),
      Shown::Held(held) => repr(held),
    }
  }
}

/// One turn of a chain: who spoke, what they said, what it cost, and what the provider gave.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Turn {
  /// Who spoke, which is the operator and the World as the user, or the model as the assistant.
  pub role: String,
  /// What was said, as the tags of the chain and the words of the model.
  pub content: Vec<Shown>,
  /// What the turn cost, and nothing for a turn of the user.
  pub usage: Option<Usage>,
  /// What the provider gave of its own, which the host that asked it reads and nobody else.
  pub blocks: Value,
}

impl Turn {
  /// The turn a value holds, and nothing for a value that is no turn.
  pub fn of(got: &Value) -> Option<Self> {
    let held = got.as_entries()?;
    let [role, content, usage, blocks] = held else { return None };
    Some(Turn {
      role: role.as_str()?.to_owned(),
      content: content.as_entries()?.iter().map(Shown::of).collect(),
      usage: Usage::of(usage),
      blocks: blocks.clone(),
    })
  }

  /// Every turn a list holds, and nothing for the entries of it that are no turns.
  pub fn every(got: &[Value]) -> Vec<Turn> {
    got.iter().filter_map(Turn::of).collect()
  }

  /// What the turn holds, as one text: a tag as its block, and a text as itself.
  pub fn rendered(&self) -> String {
    let each: Vec<String> = self.content.iter().map(Shown::said).collect();
    each.join("\n")
  }
}

/// One value as python shows it, which is what a tag shows of a value that is no text.
pub fn repr(value: &Value) -> String {
  match value {
    Value::None | Value::Show => "None".to_owned(),
    Value::Bool(held) => if *held { "True" } else { "False" }.to_owned(),
    Value::Int(held) => held.to_string(),
    Value::Float(held) => floated(*held),
    Value::Str(held) | Value::Held(held) => quoted(held),
    Value::List(held) => {
      let each: Vec<String> = held.iter().map(repr).collect();
      format!("[{}]", each.join(", "))
    }
    Value::Tuple(held) => {
      let each: Vec<String> = held.iter().map(repr).collect();
      // A tuple of one carries a comma, since that comma is what makes it a tuple.
      if each.len() == 1 { format!("({},)", each[0]) } else { format!("({})", each.join(", ")) }
    }
    Value::Map(held) => {
      let each: Vec<String> = held.iter().map(|(key, one)| format!("{}: {}", quoted(key), repr(one))).collect();
      format!("{{{}}}", each.join(", "))
    }
    Value::Shape { name, fields } => {
      let each: Vec<String> = fields.iter().map(|(key, one)| format!("{key}={}", repr(one))).collect();
      format!("{name}({})", each.join(", "))
    }
    Value::Error { name, args } => {
      let each: Vec<String> = args.iter().map(repr).collect();
      format!("{name}({})", each.join(", "))
    }
  }
}

/// One text as python shows it: single quotes, unless the text holds one and no quotation mark.
fn quoted(said: &str) -> String {
  let mark = if said.contains('\'') && !said.contains('"') { '"' } else { '\'' };
  let mut held = String::with_capacity(said.len() + 2);
  held.push(mark);
  for one in said.chars() {
    match one {
      '\\' => held.push_str("\\\\"),
      '\n' => held.push_str("\\n"),
      '\r' => held.push_str("\\r"),
      '\t' => held.push_str("\\t"),
      one if one == mark => {
        held.push('\\');
        held.push(one);
      }
      one if (one as u32) < 0x20 || one as u32 == 0x7F => {
        let _ = write!(held, "\\x{:02x}", one as u32);
      }
      one => held.push(one),
    }
  }
  held.push(mark);
  held
}

/// One number as python shows it, which always holds a point or an exponent, so that no number reads as a whole.
fn floated(held: f64) -> String {
  if held.is_nan() {
    return "nan".to_owned();
  }
  if held.is_infinite() {
    return if held > 0.0 { "inf" } else { "-inf" }.to_owned();
  }
  let size = held.abs();
  if size != 0.0 && (size < 1e-4 || size >= 1e16) {
    let said = format!("{held:e}");
    let (lead, power) = said.split_once('e').unwrap_or((said.as_str(), "0"));
    let held: i32 = power.parse().unwrap_or(0);
    return format!("{lead}e{}{:02}", if held < 0 { '-' } else { '+' }, held.abs());
  }
  let said = format!("{held:?}");
  if said.contains('.') || said.contains('e') { said } else { format!("{said}.0") }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// One tag, as the engine hands it over.
  fn tag(name: &str, attrs: Vec<(&str, Value)>, body: Value) -> Value {
    let held = attrs.into_iter().map(|(key, one)| Value::Tuple(vec![Value::Str(key.to_owned()), one])).collect();
    Value::Tuple(vec![Value::Str(name.to_owned()), Value::List(held), body])
  }

  #[test]
  fn a_tag_says_its_name_its_short_attributes_beside_the_name_and_everything_else_inside_it() {
    let held = Tag::of(&tag("read", vec![("path", Value::Str("a.txt".to_owned()))], Value::None)).unwrap();
    assert_eq!(held.shown(), "<read path=\"a.txt\"/>");
    let held = Tag::of(&tag(
      "read",
      vec![("path", Value::Str("a.txt".to_owned())), ("lines", Value::Int(3))],
      Value::Str("one\ntwo".to_owned()),
    ))
    .unwrap();
    assert_eq!(held.shown(), "<read path=\"a.txt\" lines=\"3\">\none\ntwo\n</read>");
  }

  #[test]
  fn a_value_that_holds_a_line_break_or_a_quotation_mark_stands_in_the_body() {
    let held = Tag::of(&tag("out", vec![("text", Value::Str("one\ntwo".to_owned()))], Value::None)).unwrap();
    assert_eq!(held.shown(), "<out>\n<text>\none\ntwo\n</text>\n</out>");
    let held = Tag::of(&tag("out", vec![("text", Value::Str("a \"word\"".to_owned()))], Value::None)).unwrap();
    assert_eq!(held.shown(), "<out>\n<text>\na \"word\"\n</text>\n</out>");
  }

  #[test]
  fn a_tag_of_a_body_of_tags_holds_each_of_them_and_a_value_of_it_as_python_shows_it() {
    let inside = Value::List(vec![
      tag("out", vec![], Value::Str("hi".to_owned())),
      Value::Int(3),
      Value::Str("a word".to_owned()),
    ]);
    let held = Tag::of(&tag("bash", vec![("over", Value::Str("bash://operator.1.1".to_owned()))], inside)).unwrap();
    assert_eq!(held.shown(), "<bash over=\"bash://operator.1.1\">\n<out>\nhi\n</out>\n3\n'a word'\n</bash>");
  }

  #[test]
  fn one_turn_holds_a_tag_as_its_block_and_a_text_as_itself() {
    let held = Value::Tuple(vec![
      Value::Str("user".to_owned()),
      Value::List(vec![tag("noted", vec![], Value::Str("go on".to_owned())), Value::Str("and this".to_owned())]),
      Value::None,
      Value::None,
    ]);
    let got = Turn::of(&held).unwrap();
    assert_eq!(got.role, "user");
    assert_eq!(got.usage, None);
    assert_eq!(got.rendered(), "<noted>\ngo on\n</noted>\nand this");
  }

  #[test]
  fn a_turn_of_a_model_holds_what_it_wrote_and_what_the_turn_cost() {
    let held = Value::Tuple(vec![
      Value::Str("assistant".to_owned()),
      Value::List(vec![Value::Str("close(3)".to_owned())]),
      Value::Tuple(vec![Value::Int(120), Value::Int(8), Value::Int(4000), Value::Int(0), Value::Float(0.002)]),
      Value::List(vec![]),
    ]);
    let got = Turn::of(&held).unwrap();
    assert_eq!(got.rendered(), "close(3)");
    assert_eq!(got.usage, Some(Usage { input: 120, output: 8, cached: 4000, wrote: 0, cost: 0.002 }));
    assert_eq!(Turn::of(&Value::None), None);
  }

  #[test]
  fn a_value_that_is_no_text_stands_as_python_shows_it() {
    assert_eq!(repr(&Value::None), "None");
    assert_eq!(repr(&Value::Bool(false)), "False");
    assert_eq!(repr(&Value::Str("a word".to_owned())), "'a word'");
    assert_eq!(repr(&Value::Str("it's".to_owned())), "\"it's\"");
    assert_eq!(repr(&Value::Str("it's a \"word\"".to_owned())), "'it\\'s a \"word\"'");
    assert_eq!(repr(&Value::Tuple(vec![Value::Int(1)])), "(1,)");
    assert_eq!(repr(&Value::Tuple(vec![Value::Int(1), Value::Int(2)])), "(1, 2)");
    assert_eq!(repr(&Value::text("/w/a.txt", "one\n")), "Text(path='/w/a.txt', content='one\\n')");
    assert_eq!(repr(&Value::refused("no file")), "Refused('no file')");
  }

  #[test]
  fn a_number_always_holds_a_point_or_an_exponent_so_that_no_number_reads_as_a_whole() {
    assert_eq!(floated(200000.0), "200000.0");
    assert_eq!(floated(0.5), "0.5");
    assert_eq!(floated(0.0001), "0.0001");
    assert_eq!(floated(0.00001), "1e-05");
    assert_eq!(floated(1e16), "1e+16");
    assert_eq!(floated(-0.0), "-0.0");
  }

  #[test]
  fn a_standing_stands_beside_the_name_as_python_shows_it() {
    let roster = Value::Tuple(vec![Value::Tuple(vec![
      Value::Str("m".to_owned()),
      Value::Tuple(vec![Value::Str("low".to_owned())]),
      Value::Int(200_000),
    ])]);
    let standing = Value::Tuple(vec![roster, Value::Str("/w".to_owned()), Value::Str("m/low".to_owned())]);
    assert_eq!(repr(&standing), "((('m', ('low',), 200000),), '/w', 'm/low')");
  }
}
