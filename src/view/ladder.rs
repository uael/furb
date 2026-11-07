//! The ladder of a chain as a person reads it at a glance: the last steps at each level of its
//! threads, and a fold in place of each run of earlier ones, which a key that the window opened
//! opens. At the top and under a rung, a step is a prompt, or another act that heads lines of its
//! own, and the last prompts show; under a prompt, a step is a rung, and the last rungs show. A
//! step under which a prompt or a rung asks the operator or works stays shown, and a fold holds
//! two steps in a row at least.

use std::collections::{HashMap, HashSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
  act::{Act, Kind, RunState, operator},
  fold::followed,
  state::State,
};

/// How many of the last prompts of a thread, and of the last rungs of each ladder, show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Shown {
  pub prompts: usize,
  pub rungs: usize,
}

/// The steps that a ladder shows when the window asks no other count.
pub const SHOWN: Shown = Shown { prompts: 6, rungs: 4 };

/// One line of a ladder, in the order that the window draws it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Rung {
  /// A step, with the steps under it after it, each one level deeper.
  Step { id: String, depth: usize },
  /// The rungs in a row that one maker wrote on the chain with no word of its own, as the words of
  /// the skills and the setup of a chain.
  Ear { rungs: Vec<String>, depth: usize },
  /// A fold of `count` earlier steps of a level, from the step `head`, with the kinds of the steps
  /// that it holds in the order they first come. `key` opens it: prompts: or rungs: and the act
  /// that the steps stand under, or the chain at the top.
  More { head: String, depth: usize, count: usize, of: String, key: String, heads: Vec<String> },
}

/// What a ladder takes of an act of a level: a step, a rung of an ear, or nothing.
#[derive(PartialEq)]
enum Take {
  Step,
  Ear,
  No,
}

/// The steps of one level, each with the rungs of an ear that stand as one step, the place of each
/// step by its act, and the places of the steps of each kind.
#[derive(Default)]
struct Listed<'a> {
  steps: Vec<(&'a Act, Option<Vec<&'a str>>)>,
  place: HashMap<&'a str, usize>,
  kinds: Vec<(&'a str, Vec<usize>)>,
}

fn listed<'a>(acts: impl IntoIterator<Item = &'a Act>, take: impl Fn(&Act) -> Take) -> Listed<'a> {
  let mut held = Listed::default();
  for act in acts {
    let taken = take(act);
    if taken == Take::No {
      continue;
    }
    // The rungs in a row that one maker wrote with no word of its own stand as one step.
    if let (Take::Ear, Some((last, Some(ear)))) = (&taken, held.steps.last_mut())
      && last.by == act.by
    {
      ear.push(&act.id);
      continue;
    }
    let place = held.steps.len();
    held.steps.push((act, (taken == Take::Ear).then(|| vec![act.id.as_str()])));
    held.place.insert(&act.id, place);
    match held.kinds.iter_mut().find(|one| one.0 == act.kind.as_str()) {
      Some(kind) => kind.1.push(place),
      None => held.kinds.push((act.kind.as_str(), vec![place])),
    }
  }
  held
}

/// What a ladder reads to fold a chain.
struct Walk<'a, 's> {
  chain: &'s str,
  made: HashMap<&'a str, Vec<&'a Act>>,
  alive: HashMap<&'a str, Vec<&'a str>>,
  opened: HashSet<&'s str>,
  shown: Shown,
  out: Vec<Rung>,
}

impl<'a> Walk<'a, '_> {
  /// The steps under a step: each act that a prompt made, and each prompt that a rung made on the
  /// chain.
  fn under(&self, act: &Act) -> Listed<'a> {
    let made = self.made.get(act.id.as_str()).into_iter().flatten().copied();
    match act.kind {
      Kind::Prompt => listed(made, |_| Take::Step),
      Kind::Rung => listed(made, |child| {
        if child.kind == Kind::Prompt && child.chain == self.chain { Take::Step } else { Take::No }
      }),
      _ => Listed::default(),
    }
  }

  fn level(&mut self, list: &Listed<'a>, depth: usize, key: &str, above: &str) {
    let rungs = key.starts_with("rungs:");
    let count = list.steps.len();
    let keep = if rungs { self.shown.rungs } else { self.shown.prompts };
    let end = if self.opened.contains(key) { 0 } else { count.saturating_sub(keep) };
    let mut shows: Vec<usize> = self
      .alive
      .get(above)
      .into_iter()
      .flatten()
      .filter_map(|id| list.place.get(id).copied().filter(|place| *place < end))
      .collect();
    shows.sort_unstable();
    shows.dedup();
    shows.extend(end..=count);
    let mut from = 0;
    for place in shows {
      if place >= from + 2 {
        let mut heads: Vec<(usize, &str)> = list
          .kinds
          .iter()
          .filter_map(|(kind, places)| {
            let at = places.get(places.partition_point(|one| *one < from))?;
            (*at < place).then_some((*at, *kind))
          })
          .collect();
        heads.sort_unstable();
        self.out.push(Rung::More {
          head: list.steps[from].0.id.clone(),
          depth,
          count: place - from,
          of: (if rungs { "rungs" } else { "prompts" }).to_owned(),
          key: key.to_owned(),
          heads: heads.into_iter().map(|one| one.1.to_owned()).collect(),
        });
      } else if place == from + 1 {
        self.show(list, from, depth);
      }
      if place < count {
        self.show(list, place, depth);
      }
      from = place + 1;
    }
  }

  fn show(&mut self, list: &Listed<'a>, place: usize, depth: usize) {
    let (act, ear) = &list.steps[place];
    if let Some(ear) = ear {
      let rungs = ear.iter().map(|id| (*id).to_owned()).collect();
      self.out.push(Rung::Ear { rungs, depth });
      return;
    }
    self.out.push(Rung::Step { id: act.id.clone(), depth });
    let under = self.under(act);
    let of = if act.kind == Kind::Prompt { "rungs" } else { "prompts" };
    self.level(&under, depth + 1, &format!("{of}:{}", act.id), &act.id);
  }
}

/// Whether a word is blank: none, an empty text or false.
fn blank(word: Option<&Value>) -> bool {
  word.is_none_or(|word| word.is_null() || word == "" || word == false)
}

/// The ladder of a chain from the acts of a life in the order they began. `opened` holds the keys
/// of the folds that the window opened, `owed` the prompts that the operator owes, and `taken`,
/// when given, the only acts of the chain that stand at its top, as the messages that a branch took
/// from its source.
pub fn ladder(
  acts: &[Act],
  chain: &str,
  opened: &[String],
  owed: &[String],
  taken: Option<&[String]>,
  shown: Shown,
) -> Vec<Rung> {
  let opened: HashSet<&str> = opened.iter().map(String::as_str).collect();
  let taken: Option<HashSet<&str>> = taken.map(|taken| taken.iter().map(String::as_str).collect());
  let ids: HashMap<&str, &Act> = acts.iter().map(|act| (act.id.as_str(), act)).collect();
  let local =
    |id: &str| ids.get(id).is_some_and(|act| act.chain == chain && act.kind != Kind::Chain);
  // A rung that no act wrote: an ear of the outside wrote it, or the chain wrote it on itself and
  // it retells no word of another rung.
  let ear = |act: &Act| {
    act.kind == Kind::Rung
      && !operator(&act.by)
      && (!ids.contains_key(act.by.as_str()) || (act.by == act.chain && blank(act.words.get(1))))
  };
  let mut made: HashMap<&str, Vec<&Act>> = HashMap::new();
  // The questions on the way to what an act does, which no person follows, are no steps.
  let steps = || acts.iter().filter(|act| followed(act.kind.as_str()));
  for act in steps() {
    made.entry(act.by.as_str()).or_default().push(act);
  }
  // A step stays shown when a prompt or a rung under it asks the operator or works: each such act
  // marks each step from it up to the top, in the level of the step above it.
  let mut alive: HashMap<&str, Vec<&str>> = HashMap::new();
  let live = acts.iter().filter(|act| !act.done() || act.run == Some(RunState::Running));
  let asked = owed.iter().filter_map(|id| ids.get(id.as_str()).copied());
  for act in live.chain(asked) {
    let working = matches!(act.state, State::Asks | State::Thinks | State::Runs | State::Idle);
    if !matches!(act.kind, Kind::Prompt | Kind::Rung) || !working {
      continue;
    }
    let mut path: Vec<&str> = Vec::new();
    let mut at = Some(act);
    while let Some(now) = at.filter(|_| path.len() <= ids.len()) {
      path.push(&now.id);
      if now.chain == chain && now.kind != Kind::Chain && !local(&now.by) {
        if !ear(now) {
          for (place, id) in path.iter().enumerate() {
            let marked = alive.entry(path.get(place + 1).copied().unwrap_or("")).or_default();
            if !marked.contains(id) {
              marked.push(id);
            }
          }
        }
        break;
      }
      let maker = ids.get(now.by.as_str()).copied();
      at = maker.filter(|maker| {
        maker.kind == Kind::Prompt
          || (maker.kind == Kind::Rung && now.kind == Kind::Prompt && now.chain == chain)
      });
    }
  }
  let on = steps()
    .filter(|act| act.chain == chain && act.kind != Kind::Chain)
    .filter(|act| taken.as_ref().is_none_or(|taken| taken.contains(act.id.as_str())));
  let top = listed(on, |act| match (local(&act.by), ear(act)) {
    (true, _) => Take::No,
    (false, true) => Take::Ear,
    (false, false) => Take::Step,
  });
  let key = format!("prompts:{}", top.steps.first().map_or("", |one| one.0.chain.as_str()));
  let mut walk = Walk { chain, made, alive, opened, shown, out: Vec::new() };
  walk.level(&top, 0, &key, "");
  walk.out
}

#[cfg(test)]
#[path = "ladder.test.rs"]
mod tests;
