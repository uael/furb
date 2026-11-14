//! The cards of a life: each thread that the operator started, as a list of threads shows it, with
//! the rule of its part and of its place in that list.

use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
  act::{Act, Kind, Maker, operator},
  held::Held,
  state::State,
  threads::{Rows, ThreadNode, unseen},
};
use crate::engine::OPERATOR;

/// One thread that the operator started, as a card of a list shows it: its prompt, its title, how
/// it stands, the model that wrote its last word, how many agents work under it, and the site of
/// each operator who read it to its end, `operator` for each of them and `operator/<login>` for
/// one member. A question of an agent under it is a question of the card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Card {
  pub prompt: String,
  pub title: String,
  pub standing: Standing,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub model: Option<String>,
  #[serde(default)]
  pub agents: usize,
  #[serde(default)]
  pub readers: Vec<String>,
}

/// How the thread of a card stands. It asks one member by his login, or each operator. A pause
/// names who said it. A thread whose model closed its prompt is resolved, and one that a cancel
/// ended is stopped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "is", rename_all = "snake_case")]
pub enum Standing {
  Asks { of: Option<String> },
  Failed,
  Paused { by: String },
  Thinks,
  Runs,
  Delegates,
  Stopped,
  Rests,
  Resolved,
}

/// The part of a list of cards that a card stands in, in the order of the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Part {
  NeedsYou,
  Working,
  Resting,
  Resolved,
}

impl Standing {
  /// How a thread stands from its state, whether it is over, and who said the pause that holds
  /// it. A question asks each operator.
  pub(super) fn of(state: State, over: bool, paused_by: String) -> Self {
    match state {
      State::Asks => Self::Asks { of: None },
      State::Failed => Self::Failed,
      State::Paused => Self::Paused { by: paused_by },
      State::Thinks => Self::Thinks,
      State::Runs => Self::Runs,
      State::Delegates => Self::Delegates,
      State::Cancelled => Self::Stopped,
      State::Done | State::Unread | State::Idle if over => Self::Resolved,
      State::Done | State::Unread | State::Idle => Self::Rests,
    }
  }
}

impl Card {
  /// Whether a member read the thread to its end.
  pub fn read_by(&self, login: &str) -> bool {
    let reads = |site: &String| {
      site == OPERATOR
        || site.strip_prefix(OPERATOR).and_then(|one| one.strip_prefix('/')) == Some(login)
    };
    self.readers.iter().any(reads)
  }

  /// Whether the thread works.
  pub fn works(&self) -> bool {
    matches!(self.standing, Standing::Thinks | Standing::Runs | Standing::Delegates)
  }

  /// The shape of a message of the operator on the thread: none while its prompt is open, so its
  /// model reads the message at its next step, and a text once the thread ended, which opens the
  /// thread again as a question.
  pub fn reply(&self) -> &'static str {
    match self.standing {
      Standing::Resolved | Standing::Stopped | Standing::Failed => "str",
      _ => "None",
    }
  }

  /// The card of a thread whose record no life holds: nothing works there, so a thread that worked
  /// rests.
  #[must_use]
  pub fn at_rest(mut self) -> Self {
    if self.works() {
      self.standing = Standing::Rests;
    }
    self
  }

  /// Whether the thread failed or ended after what a member read of it.
  pub fn unread(&self, login: &str) -> bool {
    matches!(self.standing, Standing::Failed | Standing::Resolved) && !self.read_by(login)
  }

  /// Whether the thread waits for a member: it asks him or each operator, it failed and he did not
  /// read it, or a pause holds it that no operator said. An end that he did not read waits for
  /// nobody: it rests, with its mark, whether a life holds the session or not.
  pub fn needs(&self, login: &str) -> bool {
    match &self.standing {
      Standing::Asks { of } => of.as_deref().is_none_or(|of| of == login),
      Standing::Paused { by } => !operator(by),
      Standing::Failed => self.unread(login),
      _ => false,
    }
  }

  /// The part of the list where a member finds the card, and its rank in that part: in what needs
  /// him, a question stands before a failure and a pause. A thread that ended and that he did not
  /// read rests.
  pub fn part(&self, login: &str) -> (Part, u8) {
    let needs = self.needs(login);
    match &self.standing {
      Standing::Asks { .. } if needs => (Part::NeedsYou, 0),
      Standing::Failed | Standing::Paused { .. } if needs => (Part::NeedsYou, 1),
      Standing::Resolved if self.unread(login) => (Part::Resting, 0),
      Standing::Resolved => (Part::Resolved, 0),
      _ if self.works() => (Part::Working, 0),
      _ => (Part::Resting, 0),
    }
  }
}

impl Held {
  /// The cards of the life: each thread at the top of its rail that the operator started, in the
  /// order that they began.
  pub fn cards(&self) -> Vec<Card> {
    self.then(None, |acts| {
      let rows: Rows<'_> = acts.iter().map(|act| (act.id.as_str(), *act)).collect();
      let mut models: HashMap<&str, &str> = HashMap::new();
      for act in acts.iter().filter(|act| act.kind == Kind::Rung) {
        if let Maker::Model { model, .. } = &act.maker {
          models.insert(act.by.as_str(), model);
        }
      }
      let sites: Vec<(&str, HashMap<String, u64>)> =
        self.sites().map(|site| (site, self.seen_by(site))).collect();
      let mut nodes: Vec<ThreadNode> =
        self.rail(&[], None).into_iter().filter(|node| node.thread.yours).collect();
      nodes.sort_by_key(|node| rows.get(node.thread.prompt.as_str()).map_or(0, |act| act.began));
      let card = |node: ThreadNode| {
        let readers = sites.iter().filter(|(_, seen)| !unseen(&node.thread, &rows, seen));
        let said = |id: &String| models.get(id.as_str()).map(|one| (*one).to_owned());
        let model =
          node.thread.messages.iter().rev().find_map(said).or_else(|| said(&node.thread.prompt));
        Card {
          standing: standing(&node, &rows),
          model,
          agents: working(&node.agents),
          readers: readers.map(|(site, _)| (*site).to_owned()).collect(),
          prompt: node.thread.prompt,
          title: node.thread.title,
        }
      };
      nodes.into_iter().map(card).collect()
    })
  }
}

/// How many threads of a tree work.
fn working(agents: &[ThreadNode]) -> usize {
  agents.iter().map(|one| usize::from(one.state.busy()) + working(&one.agents)).sum()
}

/// How the thread of a node stands, from the one state of the fold.
fn standing(node: &ThreadNode, rows: &Rows<'_>) -> Standing {
  let thread = &node.thread;
  let acts =
    || [&thread.prompt].into_iter().chain(&thread.messages).filter_map(|id| rows.get(id.as_str()));
  match node.state {
    State::Asks => Standing::Asks { of: asked(node, rows) },
    State::Paused => {
      let by = acts().find_map(|act| act.pause.as_ref()).map(|pause| pause.by.clone());
      Standing::of(State::Paused, thread.over, by.unwrap_or_default())
    }
    state => Standing::of(state, thread.over, String::new()),
  }
}

/// The member that the first question of a thread, or of an agent under it, waits for, and none
/// when it waits for each operator.
fn asked(node: &ThreadNode, rows: &Rows<'_>) -> Option<String> {
  let own = node.thread.asks.first().and_then(|ask| rows.get(ask.as_str()));
  let to = |act: &&Act| act.words.get(2).and_then(Value::as_str).map(str::to_owned);
  match own {
    Some(ask) => to(ask)?.strip_prefix(OPERATOR)?.strip_prefix('/').map(str::to_owned),
    None => {
      node.agents.iter().find(|one| one.state == State::Asks).and_then(|one| asked(one, rows))
    }
  }
}

#[cfg(test)]
#[path = "cards.test.rs"]
mod test;
