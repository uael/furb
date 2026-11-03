//! The desk of the operator: each question that waits for him and each notice that he did not
//! dismiss, with what the line of his answer is for the shape that each one asks.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::act::Act;

/// A prompt to the operator that waits for him. A question waits for an answer of its shape. A
/// notice is a prompt of None, which the console closed as it took it, and which waits only until
/// the operator dismisses it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Ask {
  pub id: String,
  /// The chain that the prompt stands in.
  pub chain: String,
  /// Who made the prompt: a rung, an ear, or the operator.
  pub by: String,
  pub shape: String,
  pub message: String,
  pub notice: bool,
  /// The lines that answer a choice, one for each value that the shape allows.
  pub choices: Vec<String>,
  /// The values of a list of a Literal, which the operator ticks.
  pub ticks: Vec<Tick>,
  /// What the line of the operator is, in words, when its shape is more than text or a choice.
  pub hint: String,
  pub began: u64,
}

/// A value of a list of a Literal: as the operator reads it, and as the line of the list writes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Tick {
  pub shown: String,
  pub json: String,
}

/// What waits for the operator in a life: each ask, the oldest first, and how many of them are
/// questions and notices.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Desk {
  pub asks: Vec<Ask>,
  pub questions: usize,
  pub notices: usize,
}

/// The desk of a life: each question that the operator owes, and each notice that a window still
/// shows, of the acts that the life made, the oldest first.
pub fn desk(acts: &[Act], notices: &[&str]) -> Desk {
  let asked = |act: &Act, notice: bool| {
    let word = |at: usize| act.words.get(at).and_then(|one| one.as_str()).unwrap_or_default();
    let shape = if notice { "None" } else { word(0) };
    Ask {
      id: act.id.clone(),
      chain: act.chain.clone(),
      by: act.by.clone(),
      shape: shape.to_owned(),
      message: word(1).to_owned(),
      notice,
      choices: if notice { Vec::new() } else { choices(shape) },
      ticks: if notice { Vec::new() } else { ticks(shape) },
      hint: if notice { String::new() } else { hint(shape) },
      began: act.began,
    }
  };
  let questions = acts.iter().filter(|act| act.owed()).map(|act| asked(act, false));
  let shown = acts.iter().filter(|act| notices.contains(&act.id.as_str()));
  let mut asks: Vec<Ask> = questions.chain(shown.map(|act| asked(act, true))).collect();
  asks.sort_by_key(|ask| ask.began);
  let questions = asks.iter().filter(|ask| !ask.notice).count();
  Desk { notices: asks.len() - questions, questions, asks }
}

/// A shape that the operator answers, as the console reads its name.
#[derive(Debug, Clone, PartialEq)]
enum Form {
  Str,
  Int,
  Float,
  Bool,
  Nothing,
  List(Option<Box<Form>>),
  Dict(Option<Box<Form>>),
  Tuple(Vec<Form>),
  Literal(Vec<String>),
  Either(Vec<Form>),
}

/// The parts of a text between the marks that stand in no brackets and in no quote.
fn parts(text: &str, mark: char) -> Vec<&str> {
  let (mut found, mut depth, mut quote, mut start, mut escaped) = (Vec::new(), 0, None, 0, false);
  for (at, char) in text.char_indices() {
    match quote {
      Some(_) if escaped => escaped = false,
      Some(_) if char == '\\' => escaped = true,
      Some(open) if char == open => quote = None,
      None if char == '\'' || char == '"' => quote = Some(char),
      None if "([{".contains(char) => depth += 1,
      None if ")]}".contains(char) => depth -= 1,
      None if char == mark && depth == 0 => {
        found.push(&text[start..at]);
        start = at + 1;
      }
      _ => {}
    }
  }
  found.push(&text[start..]);
  found
}

/// Whether a value of a Literal is a string, between quotes of one kind.
fn quoted(token: &str) -> bool {
  let mut chars = token.chars();
  let ends = (chars.next(), chars.next_back());
  matches!(ends, (Some(open @ ('\'' | '"')), Some(close)) if open == close)
}

/// A value of a Literal as the operator writes it: a string with no quotes, and any other value as
/// Python writes it.
fn shown(token: &str) -> String {
  if !quoted(token) {
    return token.to_owned();
  }
  let mut out = String::new();
  let mut chars = token[1..token.len() - 1].chars();
  while let Some(char) = chars.next() {
    out.push(if char == '\\' { chars.next().unwrap_or(char) } else { char });
  }
  out
}

/// A value of a Literal as JSON.
fn json_of(token: &str) -> String {
  match token {
    _ if quoted(token) => serde_json::Value::String(shown(token)).to_string(),
    "True" => "true".to_owned(),
    "False" => "false".to_owned(),
    "None" => "null".to_owned(),
    _ => token.to_owned(),
  }
}

/// Whether a value of a Literal is one that the console reads: a string, a whole number, a bool or
/// None.
fn literal(token: &str) -> bool {
  let digits = token.strip_prefix('-').unwrap_or(token);
  quoted(token)
    || (!digits.is_empty() && digits.bytes().all(|one| one.is_ascii_digit()))
    || ["True", "False", "None"].contains(&token)
}

/// A shape as its name reads, or none when the operator does not answer it.
fn form(name: &str) -> Option<Form> {
  let text = name.trim();
  let text = text.strip_prefix("typing.").unwrap_or(text);
  let arms = parts(text, '|');
  if arms.len() > 1 {
    let forms = arms.into_iter().map(form).collect::<Option<Vec<Form>>>()?;
    let flat = forms.into_iter().flat_map(|one| match one {
      Form::Either(of) => of,
      one => vec![one],
    });
    return Some(Form::Either(flat.collect()));
  }
  let Some(open) = text.find('[') else {
    return Some(match text {
      "str" => Form::Str,
      "int" => Form::Int,
      "float" => Form::Float,
      "bool" => Form::Bool,
      "None" => Form::Nothing,
      "list" => Form::List(None),
      "dict" => Form::Dict(None),
      _ => return None,
    });
  };
  let inside = text.strip_suffix(']')?.get(open + 1..)?;
  match text[..open].trim() {
    "list" => Some(Form::List(Some(Box::new(form(inside)?)))),
    "dict" => match parts(inside, ',')[..] {
      [key, value] if form(key) == Some(Form::Str) => {
        Some(Form::Dict(Some(Box::new(form(value)?))))
      }
      _ => None,
    },
    "tuple" => parts(inside, ',').into_iter().map(form).collect::<Option<_>>().map(Form::Tuple),
    "Literal" => {
      let values: Vec<String> = parts(inside, ',')
        .into_iter()
        .map(str::trim)
        .filter(|one| !one.is_empty())
        .map(str::to_owned)
        .collect();
      (!values.is_empty() && values.iter().all(|one| literal(one))).then_some(Form::Literal(values))
    }
    _ => None,
  }
}

/// The lines that answer a shape that is a choice: yes and no for a bool, and each value of a
/// Literal. A shape that takes any line of its kind gives none.
pub fn choices(shape: &str) -> Vec<String> {
  match form(shape) {
    _ if shape == "bool" => vec!["yes".to_owned(), "no".to_owned()],
    Some(Form::Literal(values)) => values.iter().map(|one| shown(one)).collect(),
    _ => Vec::new(),
  }
}

/// Whether a line of no word answers a shape: a union that holds None, which an empty line answers
/// with None.
pub fn takes_nothing(shape: &str) -> bool {
  matches!(form(shape), Some(Form::Either(of)) if of.contains(&Form::Nothing))
}

/// The values that a list of a Literal takes. A shape that is no such list gives none.
pub fn ticks(shape: &str) -> Vec<Tick> {
  match form(shape) {
    Some(Form::List(Some(of))) => match *of {
      Form::Literal(values) => {
        values.iter().map(|one| Tick { shown: shown(one), json: json_of(one) }).collect()
      }
      _ => Vec::new(),
    },
    _ => Vec::new(),
  }
}

/// What a form is, in words, as a part of a line of JSON.
fn said(form: &Form) -> String {
  match form {
    Form::Str => "a text".to_owned(),
    Form::Int => "a whole number".to_owned(),
    Form::Float => "a number".to_owned(),
    Form::Bool => "true or false".to_owned(),
    Form::Nothing => "null".to_owned(),
    Form::Literal(values) => {
      format!("one of {}", values.iter().map(|one| json_of(one)).collect::<Vec<_>>().join(", "))
    }
    Form::List(of) => {
      of.as_ref().map_or("a list".to_owned(), |of| format!("a list, each item {}", said(of)))
    }
    Form::Dict(of) => {
      of.as_ref().map_or("an object".to_owned(), |of| format!("an object, each value {}", said(of)))
    }
    Form::Tuple(of) => {
      let noun = if of.len() == 1 { "item" } else { "items" };
      let each: Vec<String> = of.iter().map(said).collect();
      format!("a list of {} {noun}: {}", of.len(), each.join("; "))
    }
    Form::Either(of) => of.iter().map(said).collect::<Vec<_>>().join(" or "),
  }
}

/// A line of JSON that a form takes, the nth of two examples of it.
fn sample(form: &Form, n: usize) -> String {
  let two = |one: &str, other: &str| if n == 0 { one.to_owned() } else { other.to_owned() };
  match form {
    Form::Str => two("\"a\"", "\"b\""),
    Form::Int => two("1", "2"),
    Form::Float => two("1.5", "2.5"),
    Form::Bool => two("true", "false"),
    Form::Nothing => "null".to_owned(),
    Form::Literal(values) => json_of(&values[n % values.len()]),
    Form::List(of) => of
      .as_ref()
      .map_or("[\"a\", 1]".to_owned(), |of| format!("[{}, {}]", sample(of, 0), sample(of, 1))),
    Form::Dict(of) => {
      format!("{{\"a\": {}}}", of.as_ref().map_or("1".to_owned(), |of| sample(of, n)))
    }
    Form::Tuple(of) => {
      format!("[{}]", of.iter().map(|one| sample(one, n)).collect::<Vec<_>>().join(", "))
    }
    Form::Either(of) => {
      sample(of.iter().find(|one| **one != Form::Nothing).unwrap_or(&Form::Nothing), n)
    }
  }
}

/// What the line of the operator is for a form whose line is more than text, in words.
fn line(form: &Form) -> String {
  match form {
    Form::Int => "a whole number".to_owned(),
    Form::Float => "a number".to_owned(),
    Form::List(of) | Form::Dict(of) => {
      let (noun, item) =
        if matches!(form, Form::List(_)) { ("list", "item") } else { ("object", "value") };
      let of = match of.as_deref() {
        Some(Form::Str) => " of text".to_owned(),
        Some(Form::Int) => " of whole numbers".to_owned(),
        Some(Form::Float) => " of numbers".to_owned(),
        Some(Form::Bool) => " of true or false".to_owned(),
        Some(one) => format!(", each {item} {}", said(one)),
        None => String::new(),
      };
      format!("a JSON {noun}{of}, as {}", sample(form, 0))
    }
    Form::Tuple(_) => format!("a JSON {}, as {}", &said(form)[2..], sample(form, 0)),
    Form::Either(of) => {
      let arms: Vec<&Form> = of.iter().filter(|one| **one != Form::Nothing).collect();
      let words: Vec<String> = arms
        .iter()
        .map(|arm| match (line(arm), arm) {
          (said, _) if !said.is_empty() => said,
          (_, Form::Bool) => "yes or no".to_owned(),
          (_, arm) => said(arm),
        })
        .collect();
      let nothing = if arms.len() < of.len() { ", or no word for nothing" } else { "" };
      format!("{}{nothing}", words.join(" or "))
    }
    _ => String::new(),
  }
}

/// What the operator writes for a shape whose line is more than text: a whole number, a number, a
/// list, an object or a tuple as a line of JSON with an example of it, or a union of them, which
/// takes no word for nothing. A shape that takes any text, or a choice, gives none.
pub fn hint(shape: &str) -> String {
  form(shape).as_ref().map(line).unwrap_or_default()
}

/// What an ask of a shape wants of the operator, in words: nothing for a notice, the choices for a
/// choice, any of the values of a list of them, and else its shape.
pub fn wants(shape: &str, notice: bool) -> String {
  let or = |all: &[String]| match all {
    [] => String::new(),
    [one] => one.clone(),
    [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
  };
  let choices = choices(shape);
  let ticks: Vec<String> = ticks(shape).into_iter().map(|one| one.shown).collect();
  if notice {
    "nothing".to_owned()
  } else if choices.len() > 1 {
    or(&choices)
  } else if !ticks.is_empty() {
    format!("any of {}", or(&ticks))
  } else {
    choices.into_iter().next().unwrap_or_else(|| shape.to_owned())
  }
}
