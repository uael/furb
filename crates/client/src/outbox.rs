//! What a client sends: where its input sends, each message that waits to be sent, and each
//! request that waits for its answer, with what it asked.
//!
//! Each message waits in one queue, and only one request of the queue is asked at a time, so each
//! message goes after the one before it, and a message that goes to the thread that the message
//! before it made knows that thread when it goes.

use std::time::Duration;

use furb_wire::protocol::{Hello, Kind, PROTOCOL, Request, Verb};

use crate::{
  Client, Key, Life, Link, Pending, Thread,
  opens::{Ask, Hold},
};

/// The place in the log of a life from which the client hears its events: past each event.
const NO_EVENT: u64 = u64::MAX;

/// The id of the first request of a client after the greeting, whose two requests take the ids
/// before it at each connection.
pub(crate) const FIRST: u64 = 2;

/// The first requests of each connection to a daemon, under the ids that no other request of a
/// client takes: the hello of a peer of a kind, and the ask of the cards.
pub fn greeting(kind: Kind) -> [Request; 2] {
  let hello = Hello { protocol: PROTOCOL, kind, name: None, role: None };
  [Request { id: 0, verb: Verb::Hello(hello) }, Request { id: 1, verb: Verb::Cards }]
}

/// Where the input sends: the open thread, a new thread of a session on a machine, or a new
/// session on a machine in the project of the folder of the client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
  Thread(Key),
  Session(usize, String),
  New(usize),
}

/// A message that waits to be sent: its machine, its record, its thread, which is none for a new
/// thread and empty for the thread that the message before it makes, its text, and whether it is
/// a word of python that runs. It goes once the daemon answered the open of its life.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outgoing {
  pub at: usize,
  pub life: String,
  pub thread: Option<String>,
  pub text: String,
  pub python: bool,
}

/// What a request of the client asked of a daemon: the open of a life, a message, a control of a
/// thread, as a stop or a resolve, the take back of a message with its text, the answer to a
/// question, what the operator saw, the records of the machine, the end of the rung of a word, or
/// the history of the input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Asked {
  Open,
  Say,
  Control,
  Back(String),
  Answer,
  Seen,
  List,
  Ended,
  History(Target),
}

impl Client {
  /// A verb asked of a machine, as the request to send, under an id that no other request of the
  /// client took: none while the machine does not list its cards, since a machine that the client
  /// dials again takes no verb, which would go nowhere.
  pub fn request(&mut self, at: usize, verb: Verb) -> Option<Request> {
    let listed = self.machines.get(at).is_some_and(|told| told.link == Link::Listed);
    listed.then(|| {
      let id = self.next;
      self.next += 1;
      Request { id, verb }
    })
  }

  /// A message of the operator sent: it waits in the queue, the client holds its life, opened in
  /// the folder `cwd` when no life holds its record, and the thread that it goes to shows it at
  /// once. A message to no thread starts one, which the client gives back, and a message to a
  /// thread shows in `open`, the thread that the operator reads. A word of python shows in no
  /// thread. A machine that the client dials again, or a life that a fault ended, takes nothing,
  /// and the client says why.
  #[expect(clippy::too_many_arguments, reason = "a message takes where its life opens, and when")]
  pub fn send(
    &mut self,
    sent: Outgoing,
    cwd: Option<String>,
    since: Duration,
    open: Option<&mut Thread>,
  ) -> Result<Option<Thread>, String> {
    if let Some(away) = self.away(sent.at) {
      return Err(away);
    }
    if let Some(why) = self.fault_of(sent.at, &sent.life) {
      return Err(format!("Nothing went: {why}"));
    }
    self.hold((sent.at, sent.life.clone()), cwd, since);
    let pending = Pending { act: None, text: sent.text.clone() };
    let made = match (&sent.thread, sent.python, open) {
      (_, true, _) | (Some(_), false, None) => None,
      (None, false, _) => {
        let key = (sent.at, sent.life.clone(), String::new());
        let pending = vec![pending];
        Some(Thread { key, above: Some(0), started: true, pending, ..Thread::default() })
      }
      (Some(_), false, Some(open)) => {
        open.pending.push(pending);
        None
      }
    };
    self.outgoing.push_back(sent);
    Ok(made)
  }

  /// The first message of the queue of the input, once no message waits for its answer and the
  /// daemon answered the open of its life. The daemon holds it until that life stands.
  pub fn due(&self) -> Option<(usize, Verb)> {
    if self.asked.values().any(|asked| *asked == Asked::Say) {
      return None;
    }
    let next = self.outgoing.front().filter(|next| self.stands(next.at, &next.life))?;
    let life = next.life.clone();
    let message = next.text.clone();
    let verb = match &next.thread {
      _ if next.python => Verb::Run { life, word: message, on: None },
      None => Verb::Prompt { life, shape: "str".to_owned(), message, to: None, on: None },
      Some(thread) => {
        let card = self.card(&(next.at, life.clone(), thread.clone()));
        let shape = card.map_or("None", |card| card.thread.reply()).to_owned();
        Verb::Prompt { life, shape, message, to: None, on: Some(thread.clone()) }
      }
    };
    Some((next.at, verb))
  }

  /// The open of each life that the client holds and did not ask yet, with its machine: the daemon
  /// opens it, in its folder when no life holds its record, and attaches the client past each
  /// event, since the client reads the cards and the pages of the threads, and no event.
  pub fn opens_due(&self) -> Vec<(Life, Verb)> {
    let due = self.holds.iter().filter(|(_, hold)| hold.ask == Ask::Due);
    let open = |(key, hold): (&Life, &Hold)| {
      let open = Verb::Open {
        life: key.1.clone(),
        cwd: hold.cwd.clone(),
        actor: None,
        roster: None,
        skills: None,
        config: None,
        claude: None,
        answers: false,
        attach: Some(NO_EVENT),
      };
      (key.clone(), open)
    };
    due.map(open).collect()
  }
}
