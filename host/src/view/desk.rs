//! The desk of the operator: each question that waits for him and each notice that he did not
//! dismiss, with what the line of his answer is for the shape that each one asks.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use furb::{
  value::Object,
  wire::outward,
  world::{Shape, named},
};

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

/// A value of a Literal as JSON.
fn json_of(value: &Object) -> String {
  outward(value.as_ref()).to_string()
}

/// The lines that answer a shape that is a choice: yes and no for a bool, and each value of a
/// Literal. A shape that takes any line of its kind gives none.
pub fn choices(shape: &str) -> Vec<String> {
  Shape::parse(shape).map(|one| one.choices()).unwrap_or_default()
}

/// Whether a line of no word answers a shape: a union that holds None, which an empty line answers
/// with None.
pub fn takes_nothing(shape: &str) -> bool {
  Shape::parse(shape).is_some_and(|one| one.takes_nothing())
}

/// The values that a list of a Literal takes. A shape that is no such list gives none.
pub fn ticks(shape: &str) -> Vec<Tick> {
  match Shape::parse(shape) {
    Some(Shape::List(Some(of))) => match *of {
      Shape::Choice(values) => {
        values.iter().map(|one| Tick { shown: named(one), json: json_of(one) }).collect()
      }
      _ => Vec::new(),
    },
    _ => Vec::new(),
  }
}

/// What a form is, in words, as a part of a line of JSON.
fn said(form: &Shape) -> String {
  match form {
    Shape::Str => "a text".to_owned(),
    Shape::Int => "a whole number".to_owned(),
    Shape::Float => "a number".to_owned(),
    Shape::Bool => "true or false".to_owned(),
    Shape::None => "null".to_owned(),
    Shape::Choice(values) => {
      format!("one of {}", values.iter().map(json_of).collect::<Vec<_>>().join(", "))
    }
    Shape::List(of) => {
      of.as_ref().map_or("a list".to_owned(), |of| format!("a list, each item {}", said(of)))
    }
    Shape::Dict(of) => {
      of.as_ref().map_or("an object".to_owned(), |of| format!("an object, each value {}", said(of)))
    }
    Shape::Tuple(of) => {
      let noun = if of.len() == 1 { "item" } else { "items" };
      let each: Vec<String> = of.iter().map(said).collect();
      format!("a list of {} {noun}: {}", of.len(), each.join("; "))
    }
    Shape::Either(of) => of.iter().map(said).collect::<Vec<_>>().join(" or "),
  }
}

/// A line of JSON that a form takes, the nth of two examples of it.
fn sample(form: &Shape, n: usize) -> String {
  let two = |one: &str, other: &str| if n == 0 { one.to_owned() } else { other.to_owned() };
  match form {
    Shape::Str => two("\"a\"", "\"b\""),
    Shape::Int => two("1", "2"),
    Shape::Float => two("1.5", "2.5"),
    Shape::Bool => two("true", "false"),
    Shape::None => "null".to_owned(),
    Shape::Choice(values) => json_of(&values[n % values.len()]),
    Shape::List(of) => of
      .as_ref()
      .map_or("[\"a\", 1]".to_owned(), |of| format!("[{}, {}]", sample(of, 0), sample(of, 1))),
    Shape::Dict(of) => {
      format!("{{\"a\": {}}}", of.as_ref().map_or("1".to_owned(), |of| sample(of, n)))
    }
    Shape::Tuple(of) => {
      format!("[{}]", of.iter().map(|one| sample(one, n)).collect::<Vec<_>>().join(", "))
    }
    Shape::Either(of) => {
      sample(of.iter().find(|one| !matches!(one, Shape::None)).unwrap_or(&Shape::None), n)
    }
  }
}

/// What the line of the operator is for a form whose line is more than text, in words.
fn line(form: &Shape) -> String {
  match form {
    Shape::Int => "a whole number".to_owned(),
    Shape::Float => "a number".to_owned(),
    Shape::List(of) | Shape::Dict(of) => {
      let (noun, item) =
        if matches!(form, Shape::List(_)) { ("list", "item") } else { ("object", "value") };
      let of = match of.as_deref() {
        Some(Shape::Str) => " of text".to_owned(),
        Some(Shape::Int) => " of whole numbers".to_owned(),
        Some(Shape::Float) => " of numbers".to_owned(),
        Some(Shape::Bool) => " of true or false".to_owned(),
        Some(one) => format!(", each {item} {}", said(one)),
        None => String::new(),
      };
      format!("a JSON {noun}{of}, as {}", sample(form, 0))
    }
    Shape::Tuple(_) => format!("a JSON {}, as {}", &said(form)[2..], sample(form, 0)),
    Shape::Either(of) => {
      let arms: Vec<&Shape> = of.iter().filter(|one| !matches!(one, Shape::None)).collect();
      let words: Vec<String> = arms
        .iter()
        .map(|arm| match (line(arm), arm) {
          (said, _) if !said.is_empty() => said,
          (_, Shape::Bool) => "yes or no".to_owned(),
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
  Shape::parse(shape).as_ref().map(line).unwrap_or_default()
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
