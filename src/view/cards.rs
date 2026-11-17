//! The cards of a life: each thread that the operator started, as a list of threads shows it, with
//! the rule of its part and of its place in that list.

use std::collections::HashMap;

use serde_json::Value;

use furb_wire::{
  act::{Act, Kind, Maker, Phase, operator},
  card::{Card, Sent, Skill, Standing},
  state::State,
  threads::ThreadNode,
};

use super::{
  held::Held,
  threads::{Lookup, Rows, unseen},
};
use crate::{
  engine::{OPERATOR, ROOT},
  world::skills,
};

impl Held {
  /// The cards of the life: each thread at the top of its rail that the operator started, in the
  /// order that they began.
  pub fn cards(&self) -> Vec<Card> {
    self.then(None, |acts| {
      let rows: Rows<'_> = acts.iter().map(|act| (act.id.as_ref(), *act)).collect();
      let mut models: HashMap<&str, &str> = HashMap::new();
      for act in acts.iter().filter(|act| act.kind == Kind::Rung) {
        if let Maker::Model { model, .. } = &act.maker {
          models.insert(act.by.as_ref(), model);
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

impl Held {
  /// What each operator sent to the life, in the order that he sent it: the message of each
  /// prompt that he made, and the word of each rung that he ran.
  pub fn sent(&self) -> Vec<Sent> {
    self.then(None, |acts| {
      let sent = |act: &&Act| {
        let text = match act.kind {
          Kind::Prompt => act.words.get(1),
          Kind::Rung => act.words.first(),
          _ => None,
        };
        let text = text.and_then(Value::as_str).filter(|_| operator(&act.by))?;
        let python = act.kind == Kind::Rung;
        Some(Sent { by: act.by.to_string(), text: text.to_owned(), python })
      };
      acts.iter().filter_map(sent).collect()
    })
  }

  /// The skills that the last read of `skills://` on the root told, in the order of their names,
  /// and none before the first.
  pub fn skills(&self) -> Vec<Skill> {
    self.then(None, |acts| {
      let read = |act: &&&Act| {
        act.kind.as_str() == "read"
          && *act.chain == *ROOT
          && act.words.first().and_then(Value::as_str) == Some("skills://")
      };
      let index = acts.iter().rev().filter(read).find_map(|act| match &act.phase {
        Phase::Done(told) => told.get("content").and_then(Value::as_str),
        _ => None,
      });
      let skill = |(name, description): (&str, &str)| Skill {
        name: name.to_owned(),
        description: description.to_owned(),
        ..Skill::default()
      };
      index.map(|index| skills::indexed(index).map(skill).collect()).unwrap_or_default()
    })
  }

  /// The acts that keep a thread of the operator open, at any depth of the tree of the threads,
  /// each with its shape, in the order that they began: its prompt and each message on it that is
  /// not done. A close of each, the last first, resolves the thread, since a close of a prompt
  /// cancels each prompt under it.
  pub fn open_in(&self, thread: &str) -> Vec<(String, String)> {
    // A sub thread, or an agent, stands under the thread that it is on.
    let mut nodes = self.rail(&[], None);
    let node = loop {
      let Some(node) = nodes.pop() else { return Vec::new() };
      if node.thread.prompt == thread {
        break node;
      }
      nodes.extend(node.agents);
    };
    let ids = [&node.thread.prompt].into_iter().chain(&node.thread.messages);
    let open = ids.filter_map(|id| (&self).act(id)).filter(|act| !act.done());
    let shape = |act: &Act| act.words.first().and_then(Value::as_str).unwrap_or("None").to_owned();
    open.map(|act| (act.id.to_string(), shape(act))).collect()
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
    state => Standing::of(state, thread.over, acts().find_map(|act| act.pause.as_ref())),
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
