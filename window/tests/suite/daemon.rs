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
  GRACE, STALL, Settings,
  protocol::{Answer, Card, Frame, Hello, Kind, PROTOCOL, Request, Verb},
  run,
  transport::{Dialer, Listener, Local},
  upgrade,
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
  /// The folder where the daemon listens, and its transport.
  host: PathBuf,
  local: Local,
  /// The transport that listens, until the daemon is served on it.
  listener: Option<Listener>,
  runtime: Handle,
  /// The main checkout of each project of the machine.
  projects: Vec<PathBuf>,
  /// The claude command line of each life that a window opens: none, or one that holds the open.
  claude: PathBuf,
}

impl Daemon {
  /// A daemon that listens and answers no peer until it is served: a window that dials it stays in
  /// its dial for as long as the test needs.
  pub(crate) fn bound(app: &TestApp) -> Self {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    // A short name, since the path of a socket has a bound.
    let yard = Yard::new(format!("fw{}-{n}", process::id()));
    let host = yard.join("host");
    let local = Local::new(&host);
    let runtime = app.runtime();
    let listener = runtime.block_on(local.listen()).expect("the folder of the test is free");
    let claude = yard.join("no-claude");
    Self { yard, host, local, listener: Some(listener), runtime, projects: Vec::new(), claude }
  }

  /// The daemon of the test ended with the app whose runtime served it, and a new one on the
  /// records that the first one held, on the runtime of another app. It listens in a folder of its
  /// own, since the end of the first one can take its socket with it at any moment.
  pub(crate) fn again(&mut self, app: &TestApp) {
    self.host = self.yard.join("host-again");
    self.local = Local::new(&self.host);
    self.runtime = app.runtime();
    let listener =
      self.runtime.block_on(self.local.listen()).expect("the folder of the test is free");
    self.listener = Some(listener);
  }

  /// The daemon of the test ended with the app whose runtime served it, and a new one on the
  /// records that the first one held, on the runtime of another app, in the same folder, where a
  /// window that lost the first one dials it again. The end of the first one took its socket.
  #[cfg(unix)]
  pub(crate) fn back(&mut self, app: &TestApp) {
    self.runtime = app.runtime();
    let listener =
      self.runtime.block_on(self.local.listen()).expect("the daemon before took its socket");
    self.listener = Some(listener);
  }

  /// The opens of each life that a window asks held at the catalog of its models, after the read of
  /// its record, until the test makes the file that this gives: their claude command line waits
  /// for it, and the catalog waits for a command line that it never saw. A peer of the test opens
  /// each life on no command line, so it never waits.
  #[cfg(unix)]
  pub(crate) fn holding(&mut self) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let (program, go) = (self.yard.join("claude"), self.yard.join("go"));
    let waits = format!("#!/bin/sh\nuntil [ -e '{}' ]; do sleep 0.02; done\n", go.display());
    fs::write(&program, waits).unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    self.claude = program;
    go
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
    let (local, settings) = (self.local.clone(), self.settings());
    drop(self.runtime.spawn(async move { run(&local, listener, settings).await }));
  }

  /// A daemon of a new build in the place of the daemon that runs, on the runtime of another app,
  /// as `furb host upgrade` puts it: the daemon that runs gives its lives and closes each window.
  pub(crate) fn upgraded(&mut self, app: &TestApp) {
    self.runtime = app.runtime();
    let (local, settings) = (self.local.clone(), self.settings());
    // The grace of the machine, so the daemon that took the lives waits for the window that dials
    // it again, as a daemon of `furb host upgrade` does.
    let settings = Settings { grace: GRACE, ..settings };
    drop(self.runtime.spawn(async move { upgrade(&local, settings).await }));
  }

  /// Whether a window holds a life of the daemon that listens now: no while no daemon answers, or
  /// while the one that answers stops.
  pub(crate) fn watched(&self) -> bool {
    let asked = async {
      let mut lines = Framed::new(self.local.dial().await.ok()?, LinesCodec::new());
      let hello = Hello { protocol: PROTOCOL, kind: Kind::Window, name: None, role: None };
      for (id, verb) in [(0, Verb::Hello(hello)), (1, Verb::List { config: None })] {
        lines.send(serde_json::to_string(&Request { id, verb }).ok()?).await.ok()?;
        loop {
          match serde_json::from_str(&lines.next().await?.ok()?).ok()? {
            Frame::Reply { id: 1, answer: Answer::Lives(records) } => {
              return Some(records.iter().any(|one| one.watched));
            }
            Frame::Reply { id: got, .. } if got == id => break,
            Frame::Refused { .. } => return None,
            _ => {}
          }
        }
      }
      None
    };
    self.runtime.block_on(async { timeout(PATIENCE, asked).await }).ok().flatten() == Some(true)
  }

  /// The settings of each daemon of the test.
  fn settings(&self) -> Settings {
    Settings {
      opening: scripted(&self.yard, &self.claude),
      grace: Duration::ZERO,
      stall: STALL,
      tailnet: None,
      door: None,
      host: None,
    }
  }

  /// A file of the folder of the test.
  pub(crate) fn file(&self, name: &str) -> PathBuf {
    self.yard.join(name)
  }

  /// The machine of the daemon under a name, as a window dials it. The command that serves it is
  /// no program, so a window that dials it while no daemon listens starts nothing, and dials it
  /// again.
  pub(crate) fn machine(&self, name: &str) -> Machine {
    let served = Command::new(self.yard.join("no-furb"));
    Machine { name: name.to_owned(), folder: self.host.clone(), served }
  }

  /// A peer of the daemon, which hears each change of its cards.
  pub(crate) fn peer(&self) -> Peer {
    let dialed = self.runtime.block_on(self.local.dial()).expect("the daemon listens");
    let mut peer = Peer {
      lines: Framed::new(dialed, LinesCodec::new()),
      next: 0,
      runtime: self.runtime.clone(),
      claude: self.yard.join("no-claude").display().to_string(),
    };
    let hello = Hello { protocol: PROTOCOL, kind: Kind::Window, name: None, role: None };
    assert!(matches!(peer.ask(Verb::Hello(hello)), Answer::Welcome { .. }));
    assert!(matches!(peer.ask(Verb::Cards), Answer::Cards(_)));
    peer
  }
}

/// A peer of a daemon of a test: its stream, the id of its next request, and the claude command
/// line of each life that it opens.
pub(crate) struct Peer {
  lines: Framed<<Local as Dialer>::Stream, LinesCodec>,
  next: u64,
  runtime: Handle,
  claude: String,
}

impl Peer {
  fn read(&mut self) -> Frame {
    let next = async { timeout(PATIENCE, self.lines.next()).await };
    let line = self.runtime.block_on(next).expect("the daemon says a frame");
    serde_json::from_str(&line.expect("the daemon holds the stream").unwrap()).unwrap()
  }

  /// The answer of the daemon to a verb.
  pub(crate) fn ask(&mut self, verb: Verb) -> Answer {
    self.tried(verb).unwrap_or_else(|why| panic!("the daemon refused: {why}"))
  }

  /// The answer of the daemon to a verb, or why it refused it.
  pub(crate) fn tried(&mut self, verb: Verb) -> Result<Answer, String> {
    let id = self.next;
    self.next += 1;
    let line = serde_json::to_string(&Request { id, verb }).unwrap();
    self.runtime.block_on(self.lines.send(line)).unwrap();
    loop {
      match self.read() {
        Frame::Reply { id: got, answer } if got == id => return Ok(answer),
        Frame::Refused { id: Some(got), why, .. } if got == id => return Err(why),
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
      claude: Some(self.claude.clone()),
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
    let stands = if message.contains(WORKS) || message.contains(THINKS) {
      "thinks"
    } else if message.contains(LONG) {
      "runs"
    } else if message.contains(ASKS) || QUESTIONS.iter().any(|one| message.contains(one.0)) {
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

/// The words of a message of a test that make the model ask the operator, each with the word that
/// asks him: a choice of a Literal, a truth, a whole number, two questions at once, values to
/// tick, and a notice, which asks nothing. Each word closes its thread with what it was told.
pub(crate) const CHOOSES: &str = "(the model asks a choice)";
pub(crate) const CONFIRMS: &str = "(the model asks yes or no)";
pub(crate) const COUNTS: &str = "(the model asks a number)";
pub(crate) const TWICE: &str = "(the model asks twice)";
pub(crate) const TICKS: &str = "(the model asks ticks)";
pub(crate) const TELLS: &str = "(the model tells)";
const QUESTIONS: [(&str, &str); 5] = [
  (
    CHOOSES,
    "side = await prompt(Literal['right', 'left'], 'Where does the list of cards stand?', to=OPERATOR)\nclose(side)",
  ),
  (CONFIRMS, "sure = await prompt(bool, 'Is the bar orange?', to=OPERATOR)\nclose(str(sure))"),
  (
    COUNTS,
    "rows = await prompt(int, 'How many rows does the form take?', to=OPERATOR)\nclose(str(rows))",
  ),
  (
    TWICE,
    "one = prompt(bool, 'Does the head count the questions?', to=OPERATOR)\ntwo = prompt(str, 'What does the head say?', to=OPERATOR)\nclose(f'{await one} {await two}')",
  ),
  (
    TICKS,
    "keys = await prompt(list[Literal['up', 'down', 'tab']], 'Which keys does the form take?', to=OPERATOR)\nclose(' '.join(keys))",
  ),
];

/// What the model of a test says once it read the refusal of a question.
pub(crate) const READ: &str = "The model read the refusal.";

/// The words of a message of a test that make the model think and write a word that it never
/// ends, write a long word that waits, answer with prose, or write a word that fails once.
pub(crate) const THINKS: &str = "(the model thinks)";
pub(crate) const LONG: &str = "(the model writes a long word)";
pub(crate) const ANSWERS: &str = "(the model answers)";
pub(crate) const FAILS: &str = "(the model fails)";

/// The long word of a model of a test, which waits ten minutes on its chain at its end: a wait
/// needs an act, and a chain is over only at its end.
const WORD: &str = "birth = 'async def boot(): return await seed()'
word = birth.split('async def')[1]
cost = len(word)
plan = 'read the boot, then the seed, then the turns of each chain, then say where the time of a resume goes'
stages = ['read', 'boot', 'seed', 'turns']
for stage in stages:
  cost = cost + len(stage)
total = cost * 2
await wait(600, chain)";

/// The prose of a model of a test, which closes its thread.
const PROSE: &str = "found = 2
close('The open takes **329 s**. Two stages hold most of it: 181 s to send each entry to the ears of the host, and 104 s in the boot.\\n\\n- `seed` reads each chain once\\n- the image skips the boot\\n\\n```python\\nopen = boot + seed\\n```\\n\\nSee [the trace](https://github.com/uael/furb/issues/988).')";

/// The opening of each life of the daemon of a test: the config of the test, no skill, a claude
/// command line of the test, and a model that answers from the message that it reads. It closes
/// the prompt of a message, asks the operator, or never answers.
fn scripted(at: &Path, claude: &Path) -> Opening {
  let answer = Answered(Arc::new(|request, told| {
    let request = request.to_string();
    Box::pin(async move {
      if request.contains(WORKS) {
        future::pending::<()>().await;
      }
      if request.contains(THINKS) {
        told(
          "",
          "I read the stages from the frames and sum them, so the open shows where its time goes. ",
        );
        told("", "Then the seed");
        told("cost = tokens(word)\nproof = await bash('pytest -q test/outside", "");
        future::pending::<()>().await;
      }
      let asks = "await prompt(str, 'Which key sends the queue?', to=OPERATOR)\nclose(None)";
      let fails =
        "x = 1\nraise ValueError('a maximum of 4 blocks with cache_control may be provided')";
      let question = QUESTIONS.iter().find(|one| request.contains(one.0)).map(|one| one.1);
      let read = format!("close({READ:?})");
      let told = "await prompt(None, 'The bench ran.', to=OPERATOR)\nclose(None)";
      let word = if request.contains("refused the question") {
        &read
      } else if let Some(question) = question {
        question
      } else if request.contains(TELLS) {
        told
      } else if request.contains(ASKS) {
        asks
      } else if request.contains(LONG) {
        WORD
      } else if request.contains(ANSWERS) {
        PROSE
      } else if request.contains(FAILS) && !request.contains("ValueError") {
        fails
      } else {
        "close(None)"
      };
      let turn = ["assistant", word].map(Object::string).into_iter();
      Ok(Object::tuple(turn.chain([Object::none(), Object::none()])))
    })
  }));
  Opening {
    config: Some(at.join("config").display().to_string()),
    roster: Some(vec!["claude-cli:opus".to_owned()].into()),
    claude: Some(claude.display().to_string()),
    skills: Some(false),
    answer: Some(answer),
    ..Opening::default()
  }
}
