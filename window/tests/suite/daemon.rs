//! The daemon of a test, and a peer that drives its lives as a window with an input will.

#![cfg(test)]
#![expect(clippy::disallowed_methods, reason = "a test blocks on the runtime of its app")]

use std::{
  fs::{self, File},
  future,
  path::{Path, PathBuf},
  process,
  sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
  },
  time::{Duration, SystemTime},
};

use furb::{
  Object,
  life::{Answer as Answered, Opening},
};
use furb_host::{
  STALL, Settings,
  protocol::{Answer, Card, Frame, Hello, Kind, PROTOCOL, Request, Verb},
  run,
  transport::{Dialer, Listener, Local},
};
use furb_window::Machine;
use futures::{SinkExt, StreamExt};
use opentui::testing::TestApp;
use serde_json::json;
use tokio::{process::Command, runtime::Handle, time::timeout};
use tokio_util::codec::{Framed, LinesCodec};

use crate::yard::Yard;

/// How long a test waits for a daemon, which a loaded machine cannot pass.
pub(crate) const PATIENCE: Duration = Duration::from_secs(60);

/// A daemon in a folder of one test, which goes with it. The runtime of the app of the test serves
/// it, so it ends with that app.
pub(crate) struct Daemon {
  yard: Yard,
  local: Local,
  /// The transport that listens, until the daemon is served on it.
  listener: Option<Listener>,
  runtime: Handle,
  /// The main checkout of each project of the machine.
  projects: Vec<PathBuf>,
}

impl Daemon {
  /// A daemon that listens and answers no peer until it is served: a window that dials it stays in
  /// its dial for as long as the test needs.
  pub(crate) fn bound(app: &TestApp) -> Self {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    // A short name, since the path of a socket has a bound.
    let yard = Yard::new(format!("fw{}-{n}", process::id()));
    let local = Local::new(yard.join("host"));
    let runtime = app.runtime();
    let listener = runtime.block_on(local.listen()).expect("the folder of the test is free");
    Self { yard, local, listener: Some(listener), runtime, projects: Vec::new() }
  }

  /// A daemon that took the dial of a window, read its hello and its ask of the cards, and is gone
  /// before it answered: the window reads the end of the stream.
  pub(crate) fn lost(&mut self) {
    let mut listener = self.listener.take().expect("the daemon is not served");
    let asked = async {
      let mut lines = Framed::new(listener.accept().await.unwrap(), LinesCodec::new());
      (lines.next().await, lines.next().await)
    };
    let asked = self.runtime.block_on(async { timeout(PATIENCE, asked).await });
    assert!(matches!(asked, Ok((Some(Ok(_)), Some(Ok(_))))), "a window dials the daemon");
  }

  /// A project of the machine, which the daemon lists, and the folder of its sessions.
  pub(crate) fn project(&mut self, name: &str) -> PathBuf {
    let main = self.yard.join(name);
    let sessions = main.join(".furb").join("sessions");
    fs::create_dir_all(&sessions).unwrap();
    self.projects.push(main);
    let listed: Vec<_> =
      self.projects.iter().map(|one| json!({ "directory": one.display().to_string() })).collect();
    fs::create_dir_all(self.yard.join("config")).unwrap();
    let list = json!({ "workspaces": listed }).to_string();
    fs::write(self.yard.join("config").join("workspaces.json"), list).unwrap();
    sessions
  }

  /// The daemon served, on the runtime of the app. A life with no holder ends at once.
  pub(crate) fn served(&mut self) {
    let listener = self.listener.take().expect("a daemon is served once");
    let settings = Settings {
      opening: scripted(&self.yard),
      grace: Duration::ZERO,
      stall: STALL,
      tailnet: None,
      door: None,
      host: None,
      heap: None,
    };
    let local = self.local.clone();
    drop(self.runtime.spawn(async move { run(&local, listener, settings).await }));
  }

  /// A file of the folder of the test.
  pub(crate) fn file(&self, name: &str) -> PathBuf {
    self.yard.join(name)
  }

  /// The machine of the daemon under a name, as a window dials it. Its daemon listens, so the
  /// window never runs the command that serves it.
  pub(crate) fn machine(&self, name: &str) -> Machine {
    Machine { name: name.to_owned(), folder: self.yard.join("host"), served: Command::new("furb") }
  }

  /// A peer of the daemon, which hears each change of its cards.
  pub(crate) fn peer(&self) -> Peer {
    let dialed = self.runtime.block_on(self.local.dial()).expect("the daemon listens");
    let mut peer = Peer {
      lines: Framed::new(dialed, LinesCodec::new()),
      next: 0,
      runtime: self.runtime.clone(),
    };
    let hello = Hello { protocol: PROTOCOL, kind: Kind::Window, name: None, role: None };
    assert!(matches!(peer.ask(Verb::Hello(hello)), Answer::Welcome { .. }));
    assert!(matches!(peer.ask(Verb::Cards), Answer::Cards(_)));
    peer
  }
}

/// A peer of a daemon of a test: its stream, and the id of its next request.
pub(crate) struct Peer {
  lines: Framed<<Local as Dialer>::Stream, LinesCodec>,
  next: u64,
  runtime: Handle,
}

impl Peer {
  fn read(&mut self) -> Frame {
    let next = async { timeout(PATIENCE, self.lines.next()).await };
    let line = self.runtime.block_on(next).expect("the daemon says a frame");
    serde_json::from_str(&line.expect("the daemon holds the stream").unwrap()).unwrap()
  }

  /// The answer of the daemon to a verb.
  pub(crate) fn ask(&mut self, verb: Verb) -> Answer {
    let id = self.next;
    self.next += 1;
    let line = serde_json::to_string(&Request { id, verb }).unwrap();
    self.runtime.block_on(self.lines.send(line)).unwrap();
    loop {
      match self.read() {
        Frame::Reply { id: got, answer } if got == id => return answer,
        Frame::Refused { id: Some(got), why, .. } if got == id => {
          panic!("the daemon refused: {why}")
        }
        _ => {}
      }
    }
  }

  /// The first card that the daemon tells from now on and that the test waits for.
  pub(crate) fn card(&mut self, wanted: impl Fn(&Card) -> bool) -> Card {
    loop {
      if let Frame::Cards { changed, .. } = self.read()
        && let Some(card) = changed.into_iter().find(&wanted)
      {
        return card;
      }
    }
  }

  /// A life opened on a record of the sessions of a project, which the peer holds as a view, or
  /// with a pin and no view. The open attaches the peer, since a life with no holder ends at once.
  pub(crate) fn opened(&mut self, record: &Path, view: bool) -> String {
    let life = record.display().to_string();
    let cwd = record.ancestors().nth(3).map(|main| main.display().to_string());
    let open = Verb::Open {
      life: life.clone(),
      cwd,
      actor: None,
      roster: None,
      skills: None,
      config: None,
      claude: None,
      answers: false,
      attach: Some(0),
    };
    assert_eq!(self.ask(open), Answer::Done);
    if !view {
      let pin = Verb::Pin { life: life.clone(), holder: "test".to_owned() };
      assert_eq!(self.ask(pin), Answer::Done);
      assert_eq!(self.ask(Verb::Detach { life: life.clone() }), Answer::Done);
    }
    life
  }

  /// A message of the operator on the root of a life, which starts a thread: its prompt. The
  /// thread stands once the model of the test answered, or works for ever when the model does.
  pub(crate) fn said(&mut self, life: &str, message: &str) -> String {
    let verb = Verb::Prompt {
      life: life.to_owned(),
      shape: "None".to_owned(),
      message: message.to_owned(),
      to: None,
      on: None,
    };
    let Answer::Act(prompt) = self.ask(verb) else { panic!("no prompt") };
    let stands = if message.contains(WORKS) {
      "thinks"
    } else if message.contains(ASKS) {
      "asks"
    } else {
      "resolved"
    };
    self.card(|card| card.thread.prompt == prompt && json!(card.thread.standing)["is"] == stands);
    prompt
  }

  /// A session of one thread on a new record, left at rest: the peer said its message, did what
  /// the test asks of the thread, and let the life go.
  pub(crate) fn rested(&mut self, record: &Path, message: &str, hold: Option<Hold>) -> String {
    let life = self.opened(record, true);
    let act = self.said(&life, message);
    match hold {
      Some(Hold::Seen) => self.ask(Verb::Seen { life: life.clone(), about: act }),
      Some(Hold::Paused) => self.ask(Verb::Pause { life: life.clone(), act, why: None }),
      None => Answer::Done,
    };
    self.left(&life);
    life
  }

  /// The peer lets a life go, which ends once it wrote the index of its threads.
  pub(crate) fn left(&mut self, life: &str) {
    assert_eq!(self.ask(Verb::Detach { life: life.to_owned() }), Answer::Done);
    self.card(|card| card.life == life && !card.held);
  }
}

/// What the operator did with a thread before he left its session: he read it, or he paused it.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Hold {
  Seen,
  Paused,
}

/// A record at rest as the daemon finds it when no life wrote the index of its threads: it last
/// changed a count of seconds before a second.
pub(crate) fn aged(life: &str, at: u64, before: u64) {
  fs::remove_file(format!("{life}.threads.json")).unwrap();
  let changed = SystemTime::UNIX_EPOCH + Duration::from_secs(at - before);
  File::options().write(true).open(life).unwrap().set_modified(changed).unwrap();
}

/// The words of a message of a test that make the model of the test work for ever, and ask the
/// operator a question.
pub(crate) const WORKS: &str = "(the model works on it)";
pub(crate) const ASKS: &str = "(the model asks)";

/// The opening of each life of the daemon of a test: the config of the test, no skill, and a model
/// that answers from the message that it reads, with no command line of this machine. It closes
/// the prompt of a message, asks the operator, or never answers.
fn scripted(at: &Path) -> Opening {
  let answer = Answered(Arc::new(|request, _| {
    let request = request.to_string();
    Box::pin(async move {
      if request.contains(WORKS) {
        future::pending::<()>().await;
      }
      let asks = "await prompt(str, 'Which key sends the queue?', to=OPERATOR)\nclose(None)";
      let word = if request.contains(ASKS) { asks } else { "close(None)" };
      let turn = ["assistant", word].map(Object::string).into_iter();
      Ok(Object::tuple(turn.chain([Object::none(), Object::none()])))
    })
  }));
  Opening {
    config: Some(at.join("config").display().to_string()),
    roster: Some(vec!["claude-cli:opus".to_owned()].into()),
    claude: Some(at.join("no-claude").display().to_string()),
    skills: Some(false),
    answer: Some(answer),
    ..Opening::default()
  }
}
