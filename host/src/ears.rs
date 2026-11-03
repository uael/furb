//! The ears that the host gives each life, before the ears of the crate: one list, so each life of
//! a machine opens the same way, and the record of one host opens in another.

use std::collections::HashSet;

use furb::{
  Ear, Fact, Fault, Object,
  ear::{Co, ear, hear, hearing},
  wire::outward,
  world::Asked,
};
use serde_json::{Value, json};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
  event::Event,
  view::{Fold, Pause, followed},
};

/// The name of the console of the operator, whose start of a prompt makes it a question to him.
pub(crate) const CONSOLE: &str = "console";

/// The ears of the host, each under its name, in the order of boot: the console of the operator,
/// and the ear that tells each fact as events.
pub(crate) fn ears(told: UnboundedSender<Event>) -> Vec<(&'static str, Box<dyn Ear>)> {
  vec![(CONSOLE, console()), ("events", events(told))]
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

/// The ear that hears each fact, and tells it, each act that it changes with its meaning, and the
/// chain that it changes. An act that an ear of the engine took is said to no other ear, so its
/// first fact tells it first, as the log of a life shows it.
fn events(told: UnboundedSender<Event>) -> Box<dyn Ear> {
  ear(move |mut co| async move {
    let mut fold = Fold::default();
    let mut controls: Vec<Fact> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    loop {
      let a = hear(&mut co).await;
      let mut facts = Vec::new();
      if seen.insert(a.about().to_owned()) && !a.question() && !a.about().is_empty() {
        let made = co.call("get", vec![Object::string(a.about())], vec![]).await?;
        facts.extend(Fact::owned(made));
      }
      facts.push(a);
      for one in &facts {
        let (covered, born) = covered(&mut co, &fold, &mut controls, one).await?;
        let fact = json_of(one);
        let acts = fold.hear(&fact, &covered, born);
        let chain = fold.chain_of(one.about()).map(|chain| Event::Changed(chain.to_owned()));
        let events = [Event::Fact(fact)]
          .into_iter()
          .chain(acts.into_iter().map(|act| Event::Act(Box::new(act))));
        for event in events.chain(chain) {
          if told.send(event).is_err() {
            return Ok(());
          }
        }
      }
    }
  })
}

/// A fact as JSON reads it. The fact of a module tells no words, since a module is no plain value.
fn json_of(a: &Fact) -> Value {
  match a.kind() {
    "module" => json!(["module", a.about(), a.by()]),
    _ => outward(a.0.as_ref()),
  }
}

/// What the engine alone says of a fact: each live act that a pause or a wake covers, and the pause
/// that holds a new act, which is the last control that covers it when that control is a pause.
async fn covered<W: 'static>(
  co: &mut Co<W>,
  fold: &Fold,
  controls: &mut Vec<Fact>,
  a: &Fact,
) -> Result<(Vec<String>, Option<Pause>), Fault> {
  let mut found = Vec::new();
  if !a.question() && matches!(a.kind(), "pause" | "wake") {
    controls.retain(|one| one.about() != a.about());
    controls.push(a.clone());
    for id in fold.open(a.kind() == "pause") {
      if covers(co, a, &id).await? {
        found.push(id);
      }
    }
  } else if a.question() && followed(a.kind()) {
    let oldest = controls.iter().position(|one| one.kind() == "pause").unwrap_or(controls.len());
    for control in controls[oldest..].iter().rev() {
      if covers(co, control, a.about()).await? {
        return Ok((found, Pause::of(&json_of(control))));
      }
    }
  }
  Ok((found, None))
}

/// Whether a pause or a wake covers an act, as the engine reads it.
async fn covers<W: 'static>(co: &mut Co<W>, control: &Fact, id: &str) -> Result<bool, Fault> {
  let got = co.call("covers", vec![control.0.clone(), Object::string(id)], vec![]).await?;
  Ok(got.as_ref().as_bool().unwrap_or_default())
}
