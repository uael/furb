//! The view of a life that the UI draws: the threads, the programs and their messages, the ladder of each thread as
//! code with the value of each act beside the line that made it, and the stack and the watch of each chain. The app
//! makes it from the model of the life at the cursor, and hands it to the UI as JSON, whole at each change.

use serde_json::{Value, json};

use crate::{
  model::{Act, Life, OPERATOR, Rung, State, kind_of},
  source::{clip, python, source, summary},
};

/// The word of a state, as the UI names it.
pub fn state(state: State) -> &'static str {
  match state {
    State::Asks => "asks",
    State::Failed => "failed",
    State::Paused => "paused",
    State::Runs => "runs",
    State::Waits => "waits",
    State::Done => "done",
    State::Rest => "rest",
  }
}

/// The state of an act, as the mark beside its line says it.
pub fn act_state(life: &Life, act: &Act) -> State {
  if life.owed(&act.id) {
    State::Asks
  } else if act.failed() {
    State::Failed
  } else if act.over() {
    State::Done
  } else if life.holds(&act.id) {
    State::Paused
  } else if act.kind == "prompt" || act.kind == "wait" {
    State::Waits
  } else {
    State::Runs
  }
}

/// What one step did, in a few words: the acts it made, counted by what they are.
pub fn said(life: &Life, rung: &Rung, place: &str) -> String {
  if rung.thinks {
    return "Writing the next step".to_owned();
  }
  if rung.findings.is_some() {
    return "The gate refused this word".to_owned();
  }
  if let Some(why) = &rung.raised {
    let message =
      why.split_once("('").map_or(why.as_str(), |(_, rest)| rest.trim_end_matches("')"));
    let message = match message.find(&format!("/{place}/")) {
      Some(at) => {
        let start = message[..at].rfind(' ').map_or(0, |one| one + 1);
        format!("{}{}", &message[..start], &message[at + place.len() + 2..])
      }
      None => message.to_owned(),
    };
    return clip(message.trim_end_matches('.'), 80);
  }
  let mut counts: Vec<(&str, usize)> = Vec::new();
  for act in rung.acts.iter().filter_map(|one| life.act(one)) {
    let verb = match act.kind.as_str() {
      "read" => "Read",
      "write" => "Wrote",
      "bash" => "Ran",
      "prompt" if act.text(2) == OPERATOR => "Asked you",
      "prompt" => "Asked",
      "wait" => "Waited",
      _ => continue,
    };
    match counts.iter_mut().find(|(one, _)| *one == verb) {
      Some((_, n)) => *n += 1,
      None => counts.push((verb, 1)),
    }
  }
  let words: Vec<String> = counts
    .into_iter()
    .enumerate()
    .map(|(at, (verb, n))| (if at == 0 { verb.to_owned() } else { verb.to_lowercase() }, n))
    .map(|(verb, n)| match verb.as_str() {
      "Read" | "read" | "Wrote" | "wrote" => format!("{verb} {}", plural(n, "file")),
      "Ran" | "ran" => format!("{verb} {}", plural(n, "command")),
      "Asked" | "asked" => format!("{verb} {}", plural(n, "thread")),
      _ => verb,
    })
    .collect();
  if words.is_empty() {
    let first = rung.word.lines().find(|one| !one.trim().is_empty()).unwrap_or_default();
    return clip(first.trim(), 60);
  }
  words.join(", ")
}

fn plural(n: usize, one: &str) -> String {
  if n == 1 { format!("1 {one}") } else { format!("{n} {one}s") }
}

/// The text of a value as a reader reads it: a string as its text, and any other value as python shows it.
pub fn prose(value: &Value) -> String {
  value.as_str().map_or_else(|| python(value), str::to_owned)
}

/// The ladder of one thread: each step with what it did and its code, each line with its acts.
fn ladder(life: &Life, prompt: &str, place: &str) -> Value {
  let rungs = life.ladder(prompt);
  let steps: Vec<Value> = rungs
    .iter()
    .map(|rung| {
      let lines: Vec<Value> = source(life, std::slice::from_ref(rung))
        .iter()
        .map(|line| {
          let acts: Vec<Value> = line
            .acts
            .iter()
            .filter_map(|one| life.act(one))
            .map(|act| json!({"id": act.id, "kind": act.kind, "state": state(act_state(life, act)), "summary": summary(life, act)}))
            .collect();
          json!({"text": line.text, "here": line.here, "acts": acts})
        })
        .collect();
      let asks: Vec<&String> = rung.acts.iter().filter(|one| kind_of(one) == "prompt").collect();
      json!({
        "id": rung.id,
        "state": state(rung.state),
        "said": said(life, rung, place),
        "lines": lines,
        "findings": rung.findings,
        "raised": rung.raised,
        "thinks": rung.thinks,
        "asks": asks,
        "awaits": rung.awaits,
      })
    })
    .collect();
  Value::Array(steps)
}

/// The view of a life, as the UI draws it.
pub fn view(life: &Life, place: &str, works: bool) -> Value {
  let threads: Vec<Value> = life
    .threads()
    .iter()
    .filter_map(|thread| {
      let act = life.act(&thread.prompt)?;
      let rungs = life.ladder(&thread.prompt);
      let done = rungs.iter().filter(|one| one.state == State::Done).count();
      let failed = rungs.iter().filter(|one| one.state == State::Failed).count();
      let asker = life.act(&act.by).filter(|one| one.kind == "rung").map(|one| one.on.clone());
      let to = if act.taker == "console" { OPERATOR.to_owned() } else { act.on.clone() };
      Some(json!({
        "id": thread.prompt,
        "depth": thread.depth,
        "state": state(thread.state),
        "title": act.text(1),
        "shape": act.text(0),
        "chain": act.on,
        "to": to,
        "from": asker.unwrap_or_else(|| OPERATOR.to_owned()),
        "rung": act.by,
        "steps": {"done": done, "failed": failed, "all": rungs.len()},
        "answer": act.outcome.as_ref().filter(|_| act.over()).map(prose),
        "failed": act.failed(),
        "messages": thread.messages.iter().filter_map(|one| life.act(one)).map(|one| one.text(1)).collect::<Vec<_>>(),
        "ladder": ladder(life, &thread.prompt, place),
      }))
    })
    .collect();
  let chains: Vec<Value> = life
    .chains
    .iter()
    .map(|id| {
      let chain = &life.chain[id];
      let shown: Vec<&String> = chain.prompts.iter().filter(|one| life.act(one).is_some_and(|act| !act.by.starts_with("chain"))).collect();
      let stack: Vec<Value> = life.stack(id).iter().map(|one| json!({"act": one.act, "chain": one.chain, "awaits": one.awaits})).collect();
      let watch: Vec<Value> = life
        .bindings(id)
        .iter()
        .rev()
        .map(|one| json!({"name": one.name, "value": one.value, "fresh": life.at.saturating_sub(one.at) < 12}))
        .collect();
      json!({
        "id": id,
        "label": chain.label,
        "parent": chain.parent,
        "depth": chain.depth,
        "state": state(life.state(id)),
        "actor": chain.actor,
        "cost": chain.cost,
        "share": chain.share,
        "threads": shown,
        "paused": life.holds(id),
        "stack": stack,
        "watch": watch,
      })
    })
    .collect();
  let edges: Vec<Value> = life
    .edges()
    .iter()
    .map(|edge| json!({"id": edge.prompt, "from": edge.from, "to": edge.to, "rung": edge.rung, "state": state(edge.state)}))
    .collect();
  let spent: f64 = life.chain.values().map(|one| one.cost).sum();
  let root = life.chains.first().cloned().unwrap_or_default();
  let ceiling = life
    .order
    .iter()
    .filter_map(|one| life.act(one))
    .filter(|act| act.kind == "grant" && act.on == root && !act.over())
    .filter_map(|act| act.words.first().and_then(Value::as_f64))
    .next_back();
  json!({
    "life": {"place": place, "root": root, "at": life.at, "len": life.len, "spent": spent, "ceiling": ceiling, "works": works, "owed": life.debts()},
    "threads": threads,
    "chains": chains,
    "edges": edges,
  })
}

#[cfg(test)]
#[path = "view.test.rs"]
mod test;
