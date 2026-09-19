//! Every verb of the crate, as the word that calls it, one to a line.
//!
//! `script/verbs.py` reads these and holds each of them against the engine: the name of every call must be a
//! name of the engine, and every word of the call must be one the engine takes, in the place it takes it. The
//! crate is the one source of the words, so nothing of this is written twice.
//!
//!     cargo run --quiet --example verbs

use furb::{
  Filter, Show, Text, Value,
  verb::{
    Bash, Cancel, Cd, Chain, Chance, Clock, Close, Cwd, Gate, Get, Grant, Pause, Peek, Prompt, Read, Rung, Tell, Turns,
    Verb, Wait, Wake, Write,
  },
};

/// The chain every word of this stands on.
const ON: &str = "chain://operator.1";

fn main() {
  let mut said: Vec<String> = Vec::new();
  said.push(Read { path: "a.txt", show: Some(Show::Span(1, 10)), on: ON }.word());
  said.push(Read { path: "a.txt", show: Some(Show::Grep("^def ".to_owned())), on: "" }.word());
  said.push(Read { path: "a.txt", show: Some(Show::Differs(vec!["one".to_owned()])), on: "" }.word());
  said.push(Write { text: Text::new("a.txt", "one\n"), on: ON }.word());
  said.push(Peek { at: "bash://operator.1.1", on: ON }.word());
  said.push(Turns { on: ON }.word());
  said.push(Get { about: "bash://operator.1.1" }.word());
  said.push(Clock { on: ON }.word());
  said.push(Chance { on: ON }.word());
  said.push(Gate { word: "close(1)", returns: "int", on: ON }.word());
  said.push(Cd { path: "/w", on: ON }.word());
  said.push(Cwd { on: ON }.word());
  said.push(Pause { id: ON }.word());
  said.push(Wake { id: ON }.word());
  said.push(Cancel { id: ON }.word());
  said.push(Close { value: Value::Int(3), id: "prompt://operator.2" }.word());
  said
    .push(Tell { name: "noted", attrs: vec![("by".to_owned(), Value::Str("operator".to_owned()))], body: None }.word());
  said.push(Tell { name: "noted", attrs: vec![], body: Some("the work is done".to_owned()) }.word());
  said.push(Wait { seconds: 1.5, on: ON }.word());
  said.push(Rung { word: "close(1)", retells: "bash://operator.1.1", actor: "m/low", returns: "int", on: ON }.word());
  said.push(Prompt { shape: "int", message: "count the lines", to: "m/low", on: ON }.word());
  said.push(Prompt::default().word());
  said.push(Chain { label: "work", source: ON, filter: Some(Filter::take(["bash://operator.1.1"])), on: ON }.word());
  said.push(Chain { filter: Some(Filter::take(["bash://operator.1.1"]).outside()), ..Chain::default() }.word());
  said.push(Grant { usd: Some(8.0), share: Some(0.5), on: ON }.word());
  said.push(
    Bash {
      command: "ls -la",
      fed: true,
      timeout: Some(30.0),
      show: Some(Show::Tail),
      show_err: Some(Show::Hidden),
      on: ON,
    }
    .word(),
  );
  said.push(Bash { command: "ls", ..Bash::default() }.word());
  for one in said {
    println!("{one}");
  }
}
