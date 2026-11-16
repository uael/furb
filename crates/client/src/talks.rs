//! The conversations of the threads about lines of a file, which the review shows under those
//! lines as a review of a pull request shows its comments: the page of each, which the client asks
//! of its machine once, and again each time its agent changes. Such a page names the exchange
//! before which it ends, so the machine follows no change of it, and the thread that the operator
//! reads stays the one that his machine tells him of.

use furb_wire::{
  Agent, Exchange,
  protocol::{Answer, Frame, Verb},
};

use crate::{Asked, Client, PAGE};

/// The conversation of a thread about lines: the machine and the life of the thread, its agent as
/// the open thread holds it now and as it stood when the client last asked its page, whether a
/// page is on its way, and the exchanges of the last page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Talk {
  at: usize,
  life: String,
  now: Agent,
  read: Option<Agent>,
  asked: bool,
  pub exchanges: Option<Vec<Exchange>>,
}

impl Client {
  /// The exchanges of the conversation of a thread about lines, once its machine gave them.
  pub fn talk(&self, prompt: &str) -> Option<&[Exchange]> {
    self.talks.iter().find(|one| one.now.card.prompt == prompt)?.exchanges.as_deref()
  }

  /// The threads about lines whose conversations the review shows, as agents of a thread of a life
  /// of a machine: the client forgets each other one.
  pub fn show_talks(&mut self, at: usize, life: &str, shown: &[Agent]) {
    self.talks.retain(|one| {
      one.at == at
        && one.life == life
        && shown.iter().any(|agent| agent.card.prompt == one.now.card.prompt)
    });
    for agent in shown {
      match self.talks.iter_mut().find(|one| one.now.card.prompt == agent.card.prompt) {
        Some(talk) => talk.now = agent.clone(),
        None => self.talks.push(Talk {
          at,
          life: life.to_owned(),
          now: agent.clone(),
          read: None,
          asked: false,
          exchanges: None,
        }),
      }
    }
  }

  /// The page of each conversation that changed since the client read it, and that it did not ask.
  pub(crate) fn talks_due(&mut self) -> Vec<(usize, Verb, Asked)> {
    let mut due = Vec::new();
    for talk in
      self.talks.iter_mut().filter(|one| !one.asked && one.read.as_ref() != Some(&one.now))
    {
      (talk.asked, talk.read) = (true, Some(talk.now.clone()));
      let thread = talk.now.card.prompt.clone();
      let (life, at) = (talk.life.clone(), talk.at);
      let verb = Verb::Thread {
        life,
        thread: thread.clone(),
        before: Some(usize::MAX),
        limit: PAGE,
        at: None,
      };
      due.push((at, verb, Asked::Talk(thread)));
    }
    due
  }

  /// What a machine told of the page of a conversation. A refusal keeps the exchanges before it.
  pub(crate) fn told_talk(&mut self, prompt: &str, heard: Option<Frame>) {
    let Some(talk) = self.talks.iter_mut().find(|one| one.now.card.prompt == prompt) else {
      return;
    };
    talk.asked = false;
    if let Some(Frame::Reply { answer: Answer::Page(page), .. }) = heard {
      talk.exchanges = Some(page.exchanges);
    }
  }
}
