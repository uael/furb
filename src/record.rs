//! The record, as the python World keeps it and as a later life is given it.
//!
//! The engine holds the record as entries and a World holds it as lines, one json array to an entry. An entry is
//! the act made last before its fact, and the fact; for a query of a run, the query and what it was answered
//! stand beside it, since a query is answered at once and its answer travels with it.
//!
//! A crash tears the last line alone, which is cut away. A line anywhere else that is no entry is a drift, which
//! is a hard error: a record that a life cannot read is no record.

use std::fmt;

use crate::fact::{Fact, Value};

/// One entry of the record.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
  /// The act made last before the fact, which is what puts the entry back in its place in a later life.
  pub before: String,
  /// The fact of the entry.
  pub fact: Fact,
  /// What the query was answered, for a query of a run, and nothing for every other entry.
  pub answer: Option<Value>,
}

impl Entry {
  /// The entry as the line a World writes, which is one json array.
  pub fn line(&self) -> String {
    let mut held = vec![
      serde_json::Value::String(self.before.clone()),
      Value::List(self.fact.0.clone()).record(),
    ];
    if let Some(answer) = &self.answer {
      held.push(answer.record());
    }
    serde_json::Value::Array(held).to_string()
  }
}

/// What a life fails with when the record holds what it cannot read.
#[derive(Debug, Clone, PartialEq)]
pub struct Drift(pub String);

impl fmt::Display for Drift {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&self.0)
  }
}

impl std::error::Error for Drift {}

/// The record of an earlier life, as the entries it holds, which is what a life is opened from.
///
/// A line that will not parse is the torn last line of a crash when nothing follows it, and a drift anywhere else.
pub fn read(text: &str) -> Result<Vec<Entry>, Drift> {
  let lines: Vec<&str> = text.lines().filter(|line| !line.trim().is_empty()).collect();
  let mut said = Vec::with_capacity(lines.len());
  for (n, line) in lines.iter().enumerate() {
    let Ok(got) = serde_json::from_str::<serde_json::Value>(line) else {
      if n + 1 < lines.len() {
        return Err(Drift(format!("line {} of the record is no entry", n + 1)));
      }
      break;
    };
    said.push(entry(&got, n + 1)?);
  }
  Ok(said)
}

/// One entry from what a line said, which is an array of two parts or of three.
fn entry(said: &serde_json::Value, n: usize) -> Result<Entry, Drift> {
  let held = said.as_array().filter(|held| held.len() == 2 || held.len() == 3);
  let Some(held) = held else {
    return Err(Drift(format!("line {n} of the record is no entry")));
  };
  let (before, fact) = (&held[0], &held[1]);
  let words = fact.as_array().filter(|words| !words.is_empty());
  let (Some(before), Some(words)) = (before.as_str(), words) else {
    return Err(Drift(format!("line {n} of the record is no entry")));
  };
  Ok(Entry {
    before: before.to_owned(),
    fact: Fact(words.iter().map(Value::of_record).collect()),
    answer: held.get(2).map(Value::of_record),
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  /// One real record of the harness: a life that read a file, ran a command and was answered.
  const KEPT: &str = r#"["",["chain","chain://operator.1","operator","","root",""]]
["chain://operator.1",["prompt","prompt://operator.2","operator","chain://operator.1","int","read and run",""]]
["rung://operator.2.1",["rung","rung://operator.2.1","prompt://operator.2","chain://operator.1","","","","int"]]
["rung://operator.2.1",["answer","rung://operator.2.1","world",["assistant",["t = read('a.txt')\nx = bash('echo hi')\nk = len(t.lines)\nclose((await x).code)"],[0,0,0,0,0.0],["signed 76"]]]]
["rung://operator.2.1",["read","read://operator.2.1.1","rung://operator.2.1","chain://operator.1","a.txt"],{"is":"Text","path":"/w/a.txt","content":"one\ntwo\n"}]
["bash://operator.2.1.2",["bash","bash://operator.2.1.2","rung://operator.2.1","chain://operator.1","echo hi",false,600.0]]
["bash://operator.2.1.2",["out","bash://operator.2.1.2","world","ran echo hi\n","stdout"]]
["bash://operator.2.1.2",["exited","bash://operator.2.1.2","world",0]]
["rung://operator.1.4.1",["rung","rung://operator.1.4.1","prompt://operator.1.4","chain://operator.1","","","","None"]]
["rung://operator.1.4.1",["answer","rung://operator.1.4.1","world",["assistant",["close(None)"],[0,0,0,0,0.0],["signed 11"]]]]
"#;

  #[test]
  fn an_entry_says_which_act_was_made_last_before_it_and_the_fact() {
    let said = read(KEPT).unwrap();
    assert_eq!(said.len(), 10);
    assert_eq!(said[0].before, "");
    assert_eq!(said[0].fact.kind(), "chain");
    assert_eq!(said[0].fact.about(), "chain://operator.1");
    assert_eq!(said[1].before, "chain://operator.1");
    assert_eq!(said[1].fact.on(), Some("chain://operator.1"));
  }

  #[test]
  fn a_query_of_a_run_keeps_what_it_was_answered_beside_it() {
    let said = read(KEPT).unwrap();
    let read_entry = said.iter().find(|one| one.fact.kind() == "read").unwrap();
    let answer = read_entry.answer.as_ref().unwrap();
    assert_eq!(answer.field("path"), Some(&Value::Str("/w/a.txt".to_owned())));
    assert_eq!(answer.field("content"), Some(&Value::Str("one\ntwo\n".to_owned())));
    assert!(said.iter().find(|one| one.fact.kind() == "chain").unwrap().answer.is_none());
  }

  #[test]
  fn the_record_a_world_writes_is_the_record_it_reads_back() {
    let said = read(KEPT).unwrap();
    let again: String = said.iter().map(|one| one.line() + "\n").collect();
    assert_eq!(again, KEPT);
  }

  #[test]
  fn a_crash_tears_the_last_line_alone_which_is_cut_away() {
    let torn = format!("{KEPT}[\"bash://operator.2.1.2\",[\"out\",\"bash");
    assert_eq!(read(&torn).unwrap().len(), 10);
  }

  #[test]
  fn a_line_that_is_no_entry_anywhere_else_is_a_drift() {
    let bad = format!("[\"\",[]]\n{KEPT}");
    assert_eq!(read(&bad), Err(Drift("line 1 of the record is no entry".to_owned())));
    let broken = format!("not json\n{KEPT}");
    assert_eq!(read(&broken), Err(Drift("line 1 of the record is no entry".to_owned())));
  }
}
