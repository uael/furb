//! What a client sends: where its input sends, each message that waits to be sent, and each
//! request that waits for its answer, with what it asked.
//!
//! Each message waits in one queue, and only one request of the queue is asked at a time, so each
//! message goes after the one before it, and a message that goes to the thread that the message
//! before it made knows that thread when it goes.

use furb_wire::protocol::{Frame, Hello, Kind, PROTOCOL, Request, Role, Verb};
use serde_json::Value;

use crate::{
  Client, Command, Key, Life, Link, Pasted, Shown,
  opens::{Ask, Hold},
};

/// The place in the log of a life from which the client hears its events: past each event.
const NO_EVENT: u64 = u64::MAX;

/// The id of the first request of a client after the greeting, whose two requests take the ids
/// before it at each connection.
pub(crate) const FIRST: u64 = 2;

/// The first requests of each connection to a daemon, under the ids that no other request of a
/// client takes: the hello of a peer of a kind, with the role that it asks, none for the one of its
/// login, and the ask of the cards.
pub fn greeting(kind: Kind, role: Option<Role>) -> [Request; 2] {
  let hello = Hello { protocol: PROTOCOL, kind, name: None, role };
  [Request { id: 0, verb: Verb::Hello(hello) }, Request { id: 1, verb: Verb::Cards }]
}

/// Where the input sends: the open thread, a new thread of a session on a machine, or a new
/// session on a machine in the project of a folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
  Thread(Key),
  Session(usize, String),
  New(usize, String),
}

impl Target {
  /// The target as a view that borrows it.
  pub fn aim(&self) -> Aim<'_> {
    match self {
      Self::Thread(key) => Aim::Thread(key),
      Self::Session(at, life) => Aim::Session(*at, life),
      Self::New(at, folder) => Aim::New(*at, folder),
    }
  }

  /// The machine that the target stands on.
  pub fn machine(&self) -> usize {
    self.aim().machine()
  }
}

/// Where the input sends, as a view that borrows the thread, the life or the folder that it names,
/// so a read of the target copies none of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aim<'a> {
  Thread(&'a Key),
  Session(usize, &'a str),
  New(usize, &'a str),
}

impl<'a> Aim<'a> {
  /// The life that the target sends to: none for a new session.
  pub fn life(self) -> Option<&'a str> {
    match self {
      Self::Thread(key) => Some(&key.1),
      Self::Session(_, life) => Some(life),
      Self::New(..) => None,
    }
  }

  /// The machine that the target stands on.
  pub fn machine(self) -> usize {
    match self {
      Self::Thread(key) => key.0,
      Self::Session(at, _) | Self::New(at, _) => at,
    }
  }

  /// The target that the view borrows, owned, to keep.
  pub fn owned(self) -> Target {
    match self {
      Self::Thread(key) => Target::Thread(key.clone()),
      Self::Session(at, life) => Target::Session(at, life.to_owned()),
      Self::New(at, folder) => Target::New(at, folder.to_owned()),
    }
  }
}

/// An actor that the operator chose for a target of the input: its machine, the record of its life,
/// none for a new session, which opens on it, and whether the client asked the life to take it,
/// as a word of the operator on its root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chosen {
  pub at: usize,
  pub life: Option<String>,
  pub actor: String,
  pub asked: bool,
}

impl Chosen {
  /// The actor chosen for a target.
  pub fn of(target: Aim<'_>, actor: String) -> Self {
    Self { at: target.machine(), life: target.life().map(str::to_owned), actor, asked: false }
  }

  /// Whether the actor stands for a target.
  pub fn aims(&self, target: Aim<'_>) -> bool {
    self.at == target.machine() && self.life.as_deref() == target.life()
  }
}

/// A message that waits to be sent: its machine, its record, its thread, which is none for a new
/// thread and empty for the thread that the message before it makes, its text, whether it is a
/// word of python that runs, and the images that its text marks. It goes once the daemon answered
/// the open of its life, after each of its images.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outgoing {
  pub at: usize,
  pub life: String,
  pub thread: Option<String>,
  pub text: String,
  pub python: bool,
  pub images: Vec<Pasted>,
}

/// What a request of the client asked of a daemon: the open of a life, a message, a control of a
/// thread, as a stop or a resolve, the take back of a message with its text, the answer to a
/// question, what the operator saw, the records of the machine, the models of its catalog, its
/// projects, the end of the rung of a word by its life and its name, the history of the input, or
/// a list that the completion of the input needs: the skills of a life, or the files of a folder,
/// or the changes of the checkout of a folder, with the diff of a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Asked {
  Open,
  Say,
  Control,
  Back(String),
  Answer,
  Seen,
  List,
  Models,
  Projects,
  Ended(String, String),
  History(Target),
  Skills(String),
  Files(String),
  Changes(String, Option<String>),
  /// A verb of a view: its mount, a gesture on one of its nodes, or its end.
  View,
  /// What the operator did to a session that runs: its pause, its kill or its pin.
  Running,
  /// The turns of the chain of a thread, for the kinds of what its answer of a count of tokens
  /// read.
  Parts(Key, u64),
  /// The moments of a thread, for the bar of its time.
  Moments(Key),
  /// The chains of the life of a thread, under the bar of its time.
  Tracks(Key),
  /// The settings of a session that the operator reads.
  Settings,
  /// The skills of a session, with how each plays, that the operator reads.
  Plays,
  /// The follow-ups of a thread, by its life and its prompt.
  Queued(String, String),
  /// A change of a setting of a session.
  Setting,
  /// The sign-in of Codex of the user of a machine, which the machine answers once it ends.
  SignIn,
  /// Where the operator writes, which each other window of the life shows.
  Writes,
  /// A name of the code of a thread that the pointer is on, by the thread and the name.
  Inspect(Key, String),
  /// An image that the open thread shows.
  Picture(Shown),
  /// The whole of what a command wrote, which the open thread shows.
  Whole(Command),
  /// The turns of the chain of a thread, for its transcript.
  Transcript(Key),
  /// The page of the conversation of a thread about lines, by its prompt, which the review shows.
  Talk(String),
}

impl Asked {
  /// Whether the client alone reads the answer to the ask: the moments and the chains of the bar
  /// of the time, the follow-ups of a thread, a name of its code, and an image that it shows.
  pub fn read_here(&self) -> bool {
    matches!(
      self,
      Self::Moments(_)
        | Self::Tracks(_)
        | Self::Queued(..)
        | Self::Inspect(..)
        | Self::Picture(_)
        | Self::Whole(_)
        | Self::Transcript(_)
        | Self::Talk(_)
    )
  }
}

impl Client {
  /// What a machine answered to an ask whose answer the client alone reads.
  pub fn answered(&mut self, at: usize, asked: Asked, heard: Option<Frame>) {
    match asked {
      Asked::Moments(key) => self.timed(&key, heard),
      Asked::Tracks(key) => self.tracked(&key, heard),
      Asked::Queued(life, thread) => self.followed(at, (life, thread), heard),
      Asked::Inspect(key, name) => self.told_inspected((&key, &name), heard),
      Asked::Picture(shown) => self.pictured(&shown, heard),
      Asked::Whole(command) => self.told_whole(command, heard),
      Asked::Transcript(key) => self.transcribe(&key, heard),
      Asked::Talk(prompt) => self.told_talk(&prompt, heard),
      _ => {}
    }
  }

  /// A verb asked of a machine, as the request to send, under an id that no other request of the
  /// client took: none while the machine does not list its cards, since a machine that the client
  /// dials again takes no verb, which would go nowhere, and none for a verb that writes where the
  /// login only reads, which the daemon would refuse.
  pub fn request(&mut self, at: usize, verb: Verb) -> Option<Request> {
    let listed = self.machines.get(at).is_some_and(|told| told.link == Link::Listed);
    let refused = verb.needs() == Role::Write && self.only_reads(at);
    (listed && !refused).then(|| {
      let id = self.next;
      self.next += 1;
      Request { id, verb }
    })
  }

  /// Each ask of a list that is due, with its machine and what it asks: a verb of a view, a
  /// control of a session, the changes of a folder, the turns of the box of the context, the
  /// moments of the bar of the time of the open thread, the sign-in of Codex, the name that the
  /// pointer is on, and,
  /// while the client lists everything that the operator can go to, the files and the skills of
  /// the target of the input.
  pub fn asks_due(&mut self, listing: bool) -> Vec<(usize, Verb, Asked)> {
    self.bar_due();
    let mut due = self.views_due();
    due.extend(self.controls_due());
    due.extend(self.changes_due());
    due.extend(self.parts_due());
    due.extend(self.transcript_due());
    due.extend(self.moments_due());
    due.extend(self.tracks_due());
    due.extend(self.settings_due());
    due.extend(self.sign_in_due());
    due.extend(self.inspect_due());
    due.extend(self.pictures_due());
    due.extend(self.wholes_due());
    due.extend(self.talks_due());
    due.extend(self.follows_due());
    due.extend(self.writes_due());
    if listing {
      due.extend(self.lists_due());
    }
    due
  }

  /// The first message of the queue of the input, once no message waits for its answer and the
  /// daemon answered the open of its life, each of its images first. The daemon holds it until
  /// that life stands.
  pub fn due(&self) -> Option<(usize, Verb)> {
    if self.asked.values().any(|asked| *asked == Asked::Say) {
      return None;
    }
    let next = self.outgoing.front().filter(|next| self.stands(next.at, &next.life))?;
    let life = next.life.clone();
    if let Some(image) = next.images.iter().find(|one| one.named.is_none()) {
      let (name, data) = (image.name.clone(), image.data.clone());
      return Some((next.at, Verb::Image { life, name, data }));
    }
    let message = next.said();
    // A message waits for its answer, on a thread as on a new one: it is a question.
    let verb = if next.python {
      // A word on a thread is a turn of the operator on it, which the thread shows.
      let on = next.thread.clone().filter(|thread| !thread.is_empty());
      Verb::Run { life, word: message, on }
    } else {
      Verb::Prompt { life, shape: "str".to_owned(), message, to: None, on: next.thread.clone() }
    };
    Some((next.at, verb))
  }

  /// The wake of the thread of the first message of the queue, while a pause holds that thread: a
  /// message wakes a paused thread. It goes right after the message, so the model reads the message
  /// at the ask that the wake makes.
  pub fn wake_due(&self) -> Option<Verb> {
    let next = self.outgoing.front()?;
    let thread = next.thread.clone()?;
    self.wake(&(next.at, next.life.clone(), thread)).map(|(_, wake)| wake)
  }

  /// The open of each life that the client holds and did not ask yet, with its machine: the daemon
  /// opens it, in its folder when no life holds its record, and attaches the client past each
  /// event, since the client reads the cards and the pages of the threads, and no event. Where the
  /// login only reads, the client attaches to the life that the daemon holds, and opens none.
  pub fn opens_due(&self) -> Vec<(Life, Verb)> {
    let due = self.holds.iter().filter(|(_, hold)| hold.ask == Ask::Due);
    let open = |(key, hold): (&Life, &Hold)| {
      if self.only_reads(key.0) {
        return (key.clone(), Verb::Attach { life: key.1.clone(), from: NO_EVENT });
      }
      let open = Verb::Open {
        life: key.1.clone(),
        cwd: hold.cwd.clone(),
        actor: hold.actor.clone(),
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

  /// The ask of the models of each machine that lists its cards and gave none yet, for a life in
  /// the folder of the client on this machine: the launcher shows the model and the effort that a
  /// new session takes before any session stands.
  pub fn models_due(&self) -> Vec<(usize, Verb)> {
    let asking =
      |at: usize| self.asked.iter().any(|(key, one)| key.0 == at && *one == Asked::Models);
    let due = self
      .machines
      .iter()
      .enumerate()
      .filter(|(at, told)| told.link == Link::Listed && told.models.is_none() && !asking(*at));
    let ask = |(at, _)| {
      let cwd = (at == 0).then(|| self.folder.clone());
      (at, Verb::Models { cwd, config: None, claude: None })
    };
    due.map(ask).collect()
  }

  /// The word that gives the root of a life that stands the actor that the operator chose for it,
  /// once the daemon answered the open of its life.
  pub fn chosen_due(&mut self) -> Option<(usize, Verb)> {
    let chosen = self.chosen.as_ref().filter(|one| !one.asked)?;
    let life = chosen.life.clone().filter(|life| self.stands(chosen.at, life))?;
    let word = format!("actor = {}", Value::String(chosen.actor.clone()));
    let at = chosen.at;
    self.chosen.as_mut()?.asked = true;
    Some((at, Verb::Run { life, word, on: None }))
  }
}
