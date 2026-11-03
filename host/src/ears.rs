//! The ears that the host gives each life, before the ears of the crate: one list, so each life of
//! a machine opens the same way, and the record of one host opens in another.

use std::collections::{HashMap, HashSet};

use furb::{
  Ear, Fact, Object,
  ear::{ear, hear, hearing},
  wire::outward,
  world::Asked,
};
use serde_json::json;
use tokio::sync::mpsc::UnboundedSender;

use crate::event::{Act, Event, State};

/// The ears of the host, each under its name, in the order of boot: the console of the operator,
/// and the ear that tells each fact as events.
pub(crate) fn ears(told: UnboundedSender<Event>) -> Vec<(&'static str, Box<dyn Ear>)> {
  vec![("console", console()), ("events", events(told))]
}

/// The console of the operator, which takes each prompt to the operator as every console of furb
/// takes it. A client sees the prompt in the events of its act, and answers it with a command.
fn console() -> Box<dyn Ear> {
  hearing(&["prompt"], |mut co| async move {
    loop {
      let a = hear(&mut co).await;
      Asked::taken(&mut co, &a).await?;
    }
  })
}

/// The ear that hears each fact, and tells it, the act that it changes, and the chain that it
/// changes. An act that an ear of the engine took is said to no other ear, so its first fact tells
/// it first, as the log of a life shows it.
fn events(told: UnboundedSender<Event>) -> Box<dyn Ear> {
  ear(move |mut co| async move {
    let mut acts: HashMap<String, Act> = HashMap::new();
    let mut seen: HashSet<String> = HashSet::new();
    loop {
      let a = hear(&mut co).await;
      let mut facts = Vec::new();
      if seen.insert(a.about().to_owned()) && !a.question() && !a.about().is_empty() {
        let made = co.call("get", vec![Object::string(a.about())], vec![]).await?;
        facts.extend(Fact::owned(made));
      }
      facts.push(a);
      for one in facts.iter().flat_map(|one| heard(&mut acts, one)) {
        if told.send(one).is_err() {
          return Ok(());
        }
      }
    }
  })
}

/// What one fact tells: itself, the act that it asks or whose state it moves, and the chain of the
/// act that it is about. The fact of a module tells no words, since a module is no plain value.
fn heard(acts: &mut HashMap<String, Act>, a: &Fact) -> Vec<Event> {
  let fact = match a.kind() {
    "module" => json!(["module", a.about(), a.by()]),
    _ => outward(a.0.as_ref()),
  };
  let mut told = vec![Event::Fact(fact)];
  let moved = matches!(a.kind(), "started" | "done");
  if a.question() {
    let chain = match a.kind() {
      "chain" => a.about().to_owned(),
      _ => acts.get(a.on()).map_or_else(|| a.on().to_owned(), |on| on.chain.clone()),
    };
    let act = Act {
      kind: a.kind().to_owned(),
      id: a.about().to_owned(),
      by: a.by().to_owned(),
      on: a.on().to_owned(),
      chain,
      words: (1..).map_while(|at| a.word(at)).map(outward).collect(),
      state: State::Asked,
    };
    acts.insert(act.id.clone(), act);
  } else if let Some(act) = acts.get_mut(a.about()).filter(|_| moved) {
    act.state = match a.word(0) {
      Some(value) if a.kind() == "done" => State::Done(outward(value)),
      _ => State::Started,
    };
  }
  if let Some(act) = acts.get(a.about()) {
    if a.question() || moved {
      told.push(Event::Act(act.clone()));
    }
    told.push(Event::Changed(act.chain.clone()));
  }
  told
}
