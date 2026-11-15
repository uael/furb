//! What each machine told a client: its login, its cards and the records that it lists, and the
//! order of the cards of every machine in one list.

use furb_wire::{
  Part,
  protocol::{Answer, Card, Frame, Listed},
};
use serde::{Deserialize, Serialize};

use crate::Client;

/// What a client hears of a machine: a frame of its daemon, or why the connection ended.
pub type Heard = Result<Frame, String>;

/// A card among those of each machine: its machine, its record and its prompt.
pub type Key = (usize, String, String);

/// Where the connection to a machine stands: the client dials it, its daemon gave its cards, or
/// the connection ended, with why.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Link {
  #[default]
  Dialing,
  Listed,
  Lost(String),
}

/// What one machine told the client, which a client may keep for its next first frame: the name
/// of the machine, the login of the operator there, its cards, and the second at which it last
/// told them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Told {
  pub name: String,
  #[serde(default)]
  pub login: String,
  #[serde(default)]
  pub heard: u64,
  #[serde(default)]
  pub cards: Vec<Card>,
  #[serde(skip)]
  pub link: Link,
  /// The records of the machine, as it last listed them.
  #[serde(skip)]
  pub lives: Vec<Listed>,
}

/// What a frame changed of what a machine told: its cards or its login, nothing that the list
/// shows, or the connection, which ended, with why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Listing {
  Changed,
  Same,
  Lost(String),
}

impl Client {
  /// What one frame of a machine, or the end of its connection, changed of what the machine told,
  /// at the second `now`. A machine whose connection ended hears nothing until its daemon welcomes
  /// the client again, so the first cause of a loss stays.
  pub fn listed(&mut self, at: usize, heard: Heard, now: u64) -> Listing {
    let told = &mut self.machines[at];
    let welcome = matches!(heard, Ok(Frame::Reply { answer: Answer::Welcome { .. }, .. }));
    if matches!(told.link, Link::Lost(_)) && !welcome {
      return Listing::Same;
    }
    match heard {
      Ok(Frame::Reply { answer: Answer::Welcome { peer, .. }, .. }) => {
        (told.login, told.link) = (peer.operator, Link::Dialing);
      }
      Ok(Frame::Reply { answer: Answer::Cards(cards), .. }) => {
        (told.cards, told.heard, told.link) = (cards, now, Link::Listed);
      }
      Ok(Frame::Cards { life, changed, gone }) => {
        let stays = |one: &Card| {
          let told = |prompt: &String| *prompt == one.thread.prompt;
          one.life != life
            || !(gone.iter().any(told) || changed.iter().any(|new| told(&new.thread.prompt)))
        };
        told.cards.retain(stays);
        told.cards.extend(changed);
        told.heard = now;
      }
      Ok(Frame::Refused { why, .. }) | Err(why) => {
        told.link = Link::Lost(why.clone());
        return Listing::Lost(why);
      }
      // A frame that the list does not show.
      Ok(_) => return Listing::Same,
    }
    for (_, key, card) in self.settled.iter_mut().filter(|one| one.1.0 == at) {
      let told = |one: &&Card| one.life == key.1 && one.thread.prompt == key.2;
      if let Some(now) = self.machines[at].cards.iter().rfind(told) {
        card.clone_from(now);
      }
    }
    Listing::Changed
  }

  /// The card of a thread, as its machine last told it.
  pub fn card(&self, key: &Key) -> Option<&Card> {
    let told = |one: &&Card| one.life == key.1 && one.thread.prompt == key.2;
    self.machines.get(key.0)?.cards.iter().find(told)
  }

  /// The list settled: each card of each machine at its place, and the card of a thread that the
  /// client started, which its machine did not tell yet. A card ranks by its part, then by its
  /// title, then by its machine.
  pub fn settle(&mut self, untold: Option<&(usize, Card)>) {
    let told = self.machines.iter().enumerate();
    let told = told.flat_map(|(at, told)| told.cards.iter().map(move |card| (at, card)));
    let cards = told
      .chain(untold.map(|(at, card)| (*at, card)))
      .map(|(at, card)| (card.place(&self.machines[at].login), &card.thread.title, at, card));
    let mut cards: Vec<_> = cards.collect();
    cards.sort_by(|one, other| (&one.0, one.1, one.2).cmp(&(&other.0, other.1, other.2)));
    let keyed = |((part, ..), _, at, card): ((Part, u8, i64), _, usize, &Card)| {
      (part, (at, card.life.clone(), card.thread.prompt.clone()), card.clone())
    };
    self.settled = cards.into_iter().map(keyed).collect();
  }
}
