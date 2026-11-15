//! The daemon of a machine, as its windows reach it on the local transport: the holders of a life,
//! the grace after the last of them, a fault of one life, and the handover to a new build.

#![cfg(test)]
#![expect(
  clippy::disallowed_methods,
  reason = "the tests block on the runtime that tokio starts for them"
)]

use crate::{life::crossed, yard};

use std::net::TcpListener;
use yard::Yard;

use std::{
  cell::RefCell,
  collections::VecDeque,
  env,
  fs::{self, File},
  ops::Deref,
  path::{Path, PathBuf},
  process,
  sync::{
    Arc, Mutex,
    atomic::{AtomicU32, AtomicUsize, Ordering},
    mpsc::{Receiver, Sender, channel},
  },
  thread,
  time::{Duration, Instant, UNIX_EPOCH},
};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use furb::{
  Object, image,
  life::{Answer as Answered, Log, Opening},
  view::{self, Part, Phase, Standing},
  world::{SchemaError, crossings},
};
use furb_host::{
  DaemonError, Door, Event, GRACE, ProtocolError, STALL, STARTING, Settings, Tailnet,
  TransportError,
  access::{Access, Ports},
  protocol::{
    Answer, Axis, Card, Config, Dispose, Draft, Effect, Frame, Gesture, Hello, Kind, Listed, Mount,
    Node, PROTOCOL, Present, Project, Request, Role, Shape, Slot, Tree, Verb, ViewEffect,
    ViewEvent,
  },
  run,
  transport::{Accepted, Dialer, Identity, Local, LocalApi, Tailscale, Who},
  upgrade,
};
use futures::{SinkExt, StreamExt, future};
use serde_json::{Value, json};
use tokio::{task, task::JoinHandle, time, time::timeout};
use tokio_util::codec::{Framed, LinesCodec};

/// How long a test waits for a frame before it fails.
const PATIENCE: Duration = Duration::from_secs(60);

/// A grace short enough for a test, and the wait that passes it with room.
const SHORT: Duration = Duration::from_millis(300);
const PAST: Duration = Duration::from_millis(1500);

/// A folder of one test, empty, and short enough for the path of a socket. It goes at the end of
/// the test, and again once the thread of the test ends: the end of the runtime of the test ends
/// the daemon, and each life that it held and whose grace is short then writes its image
/// there, on a thread of its own, before it lets its lease go.
fn place() -> Place {
  static NEXT: AtomicU32 = AtomicU32::new(0);
  let n = NEXT.fetch_add(1, Ordering::Relaxed);
  Place(Yard::new(format!("fd{}-{n}", process::id())))
}

/// The folder of one test, which goes with it.
struct Place(Yard);

impl Deref for Place {
  type Target = Path;

  fn deref(&self) -> &Path {
    &self.0
  }
}

impl Drop for Place {
  fn drop(&mut self) {
    let entries = fs::read_dir(&*self.0).into_iter().flatten().flatten();
    // Each lease is open before the yard goes, since a lock holds the file and not its name.
    let leases = entries
      .map(|one| one.path())
      .filter(|one| one.extension().is_some_and(|ext| ext == "lock"))
      .filter_map(|one| File::open(one).ok())
      .collect();
    HELD.with(|held| held.0.borrow_mut().push((self.0.to_path_buf(), leases)));
  }
}

thread_local! {
  static HELD: Held = const { Held(RefCell::new(Vec::new())) };
}

/// The folder of each test of this thread, and the leases of its records, which go again when
/// the thread ends, once no life holds them.
struct Held(RefCell<Vec<(PathBuf, Vec<File>)>>);

impl Drop for Held {
  /// The folders go on a thread of their own: the harness joins the thread of each test before it
  /// starts the next one, so a wait here would hold every test of the suite.
  fn drop(&mut self) {
    let kept = env::var_os("FURB_KEEP_YARDS").is_some();
    for (folder, leases) in self.0.take() {
      thread::spawn(move || reaped(&folder, &leases, kept));
    }
  }
}

/// A folder gone once each lease of it is free. A write of a life that lives on makes its folder
/// again, and a life that the test asked may still open, so the folder goes at each look until
/// each lease is free, with a bound twice the time that a life of a short grace takes to end.
fn reaped(folder: &Path, leases: &[File], kept: bool) {
  let end = Instant::now() + 2 * PAST;
  loop {
    if !kept {
      drop(fs::remove_dir_all(folder));
    }
    let free = !leases.is_empty() && leases.iter().all(|lease| lease.try_lock().is_ok());
    if free || Instant::now() > end {
      break;
    }
    thread::sleep(Duration::from_millis(20));
  }
  if !kept {
    drop(fs::remove_dir_all(folder));
  }
}

/// The opening of each life of a daemon of a test: no skill, and a model that closes each
/// prompt with None.
fn opening(at: &Path) -> Opening {
  let asked = AtomicUsize::new(0);
  let answer = Answered(Arc::new(move |_, _| {
    asked.fetch_add(1, Ordering::SeqCst);
    let turn = Object::tuple([
      Object::string("assistant"),
      Object::string("close(None)"),
      Object::none(),
      Object::none(),
    ]);
    Box::pin(async move { Ok(turn) })
  }));
  Opening {
    config: Some(at.join("config").display().to_string()),
    roster: Some(vec!["claude-cli:opus".to_owned()].into()),
    // A stand reads the settings again, and a command line of this machine would answer it
    // in seconds.
    claude: Some(at.join("no-claude").display().to_string()),
    skills: Some(false),
    answer: Some(answer),
    ..Opening::default()
  }
}

/// The opening of a life whose claude command line says nothing and ends at once, so a stand
/// that reads the settings again does not ask the command line of the machine, which takes
/// seconds to answer where one is installed. Where no script runs, as on Windows, it is the plain
/// opening.
fn mute(at: &Path) -> Opening {
  let opening = opening(at);
  #[cfg(unix)]
  let opening = {
    use std::os::unix::fs::PermissionsExt;
    let program = at.join("claude");
    fs::write(&program, "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    Opening { claude: Some(program.display().to_string()), ..opening }
  };
  opening
}

/// A daemon that listens in a folder, served on a task of the test.
async fn started(at: &Path, grace: Duration) -> (Local, JoinHandle<Result<(), DaemonError>>) {
  started_with(at, grace, opening(at)).await
}

/// A daemon that listens in a folder with an opening of the test, served on a task of the test.
async fn started_with(
  at: &Path,
  grace: Duration,
  opening: Opening,
) -> (Local, JoinHandle<Result<(), DaemonError>>) {
  let local = Local::new(at.join("sock"));
  let listener = local.listen().await.unwrap();
  // The data folder of the test, so no session with no project of the machine stands in its list.
  let data = Some(at.join("data"));
  let settings =
    Settings { opening, grace, first: STARTING, stall: STALL, tailnet: None, door: None, data };
  let served = local.clone();
  (local, tokio::spawn(async move { run(&served, listener, settings).await }))
}

/// The identity of a test: the peers that dial in turn hold the roles of its list, so that a
/// window can read.
#[derive(Clone)]
struct Grants {
  local: Local,
  next: Arc<AtomicUsize>,
  roles: Arc<[Role]>,
}

impl Identity for Grants {
  type Stream = Accepted;

  async fn identify(&self, stream: &Accepted) -> Result<Who, TransportError> {
    let who = self.local.identify(stream).await?;
    let at = self.next.fetch_add(1, Ordering::SeqCst);
    Ok(Who { role: self.roles[at], ..who })
  }
}

/// The identity of a test whose peers that dial in turn are the operators of a list, each a writer.
#[derive(Clone)]
struct Operators {
  local: Local,
  next: Arc<AtomicUsize>,
  names: Arc<[&'static str]>,
}

impl Identity for Operators {
  type Stream = Accepted;

  async fn identify(&self, stream: &Accepted) -> Result<Who, TransportError> {
    let who = self.local.identify(stream).await?;
    let at = self.next.fetch_add(1, Ordering::SeqCst);
    let operator =
      self.names.get(at).unwrap_or_else(|| panic!("no operator for dial {at} of {:?}", self.names));
    Ok(Who { operator: (*operator).to_owned(), ..who })
  }
}

/// Who said each fact seen of a life about a chain, in the order of the record.
async fn seen_by(window: &mut Window, life: &str, about: &str) -> Vec<String> {
  let of = Some("seen".to_owned());
  let verb = Verb::Transcript { life: life.to_owned(), on: None, of };
  let Ok(Answer::Values(facts)) = window.ask(verb).await else { panic!("no transcript") };
  let about = facts.iter().filter(|one| one[1] == about);
  about.filter_map(|one| one[2].as_str().map(str::to_owned)).collect()
}

#[tokio::test]
async fn the_seen_of_each_operator_is_said_under_his_profile_and_a_reopen_keeps_each() {
  let at = place();
  let local = Local::new(at.join("sock"));
  let listener = local.listen().await.unwrap();
  let settings = Settings {
    opening: opening(&at),
    grace: SHORT,
    first: STARTING,
    stall: STALL,
    tailnet: None,
    door: None,
    data: None,
  };
  let operators: Arc<[&str]> = ["alice", "bob", "alice"].into();
  let identity = Operators { local: local.clone(), next: Arc::default(), names: operators };
  tokio::spawn(async move { run(&identity, listener, settings).await });
  let (mut alice, mut bob) = (Window::dialed(&local).await, Window::dialed(&local).await);
  let life = opened(&mut alice, &at, "a.jsonl").await;
  alice.attach(&life).await;
  bob.attach(&life).await;
  let seen = |about: &str| Verb::Seen { life: life.clone(), about: about.to_owned() };
  assert_eq!(alice.ask(seen("chain1")).await, Ok(Answer::Done));
  assert_eq!(
    seen_by(&mut bob, &life, "chain1").await,
    ["operator/alice"],
    "the seen of alice is said under her operator, and no seen of bob stands"
  );
  assert_eq!(bob.ask(seen("chain1")).await, Ok(Answer::Done));
  assert_eq!(seen_by(&mut alice, &life, "chain1").await, ["operator/alice", "operator/bob"]);
  for window in [&mut alice, &mut bob] {
    assert_eq!(window.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
  }
  time::sleep(PAST).await;
  assert!(!alice.lives(&life).await, "the life ended after its grace");
  let mut again = Window::dialed(&local).await;
  opened(&mut again, &at, "a.jsonl").await;
  assert_eq!(
    seen_by(&mut again, &life, "chain1").await,
    ["operator/alice", "operator/bob"],
    "a reopen keeps the seen of each operator apart"
  );
}

/// A daemon that gives each peer that dials, in turn, the role of a list.
async fn started_as(at: &Path, roles: &[Role]) -> Local {
  let local = Local::new(at.join("sock"));
  let listener = local.listen().await.unwrap();
  let settings = Settings {
    opening: opening(at),
    grace: GRACE,
    first: STARTING,
    stall: STALL,
    tailnet: None,
    door: None,
    data: None,
  };
  let identity = Grants { local: local.clone(), next: Arc::default(), roles: roles.into() };
  tokio::spawn(async move { run(&identity, listener, settings).await });
  local
}

/// A window of a test: its stream, the id of its next request, and each frame that it read while
/// it waited for another.
struct Window {
  lines: Framed<<Local as Dialer>::Stream, LinesCodec>,
  next: u64,
  pushed: VecDeque<Frame>,
  life: String,
}

impl Window {
  /// A window that dialed the daemon and said its hello.
  async fn dialed(local: &Local) -> Self {
    Self::dialed_as(local, Kind::Window, None).await
  }

  /// A peer of a kind that dialed the daemon and said its hello.
  async fn dialed_as(local: &Local, kind: Kind, name: Option<&str>) -> Self {
    let lines = Framed::new(local.dial().await.unwrap(), LinesCodec::new());
    let mut window = Self { lines, next: 0, pushed: VecDeque::new(), life: String::new() };
    let name = name.map(str::to_owned);
    let hello = Verb::Hello(Hello { protocol: PROTOCOL, kind, name, role: None });
    let welcome = window.ask(hello).await;
    assert!(matches!(welcome, Ok(Answer::Welcome { .. })), "{welcome:?}");
    window
  }

  /// The next frame of the daemon, or none at the end of the stream.
  async fn read(&mut self) -> Option<Frame> {
    let line = timeout(PATIENCE, self.lines.next()).await.expect("the daemon says a frame");
    line.map(|line| serde_json::from_str(&line.unwrap()).unwrap())
  }

  /// The answer of the daemon to a verb.
  async fn ask(&mut self, verb: Verb) -> Result<Answer, ProtocolError> {
    let id = self.next;
    self.next += 1;
    self.lines.send(serde_json::to_string(&Request { id, verb }).unwrap()).await.unwrap();
    loop {
      match self.read().await.expect("the daemon answers each request") {
        Frame::Reply { id: got, answer } if got == id => return Ok(answer),
        Frame::Refused { id: Some(got), error, .. } if got == id => return Err(error),
        frame => self.pushed.push_back(frame),
      }
    }
  }

  /// The first frame that the daemon pushed and that the test waits for. It keeps each frame
  /// that it reads before that one, for a later wait or a count of what the window heard.
  async fn until(&mut self, wanted: impl Fn(&Frame) -> bool) -> Frame {
    if let Some(at) = self.pushed.iter().position(&wanted) {
      return self.pushed.remove(at).unwrap();
    }
    loop {
      let frame = self.read().await.expect("the daemon pushes the frame");
      if wanted(&frame) {
        return frame;
      }
      self.pushed.push_back(frame);
    }
  }

  /// Whether the daemon closed the stream of the window.
  async fn closed(&mut self) -> bool {
    while self.read().await.is_some() {}
    true
  }

  /// A word run on the root of a life, and its rung once it is done.
  async fn ran(&mut self, life: &str, word: &str) -> String {
    let verb = Verb::Run { life: life.to_owned(), word: word.to_owned(), on: None };
    let Ok(Answer::Act(rung)) = self.ask(verb).await else { panic!("no rung") };
    // Each birth names its rungs anew, so a rung of an earlier birth that a window reads late
    // may hold the same name: the done of this one holds this word.
    let this = |frame: &Frame| {
      matches!(frame, Frame::Event { event: Event::Act(act), .. }
        if act.id == rung && matches!(act.phase, Phase::Done(_)) && act.word.as_deref() == Some(word))
    };
    self.until(this).await;
    rung
  }

  /// The last reply of a life, which took the last answer of a model.
  async fn replied(&mut self, life: &str) -> String {
    let of = Some("reply".to_owned());
    let replies = Verb::Transcript { life: life.to_owned(), on: None, of };
    let Ok(Answer::Values(replies)) = self.ask(replies).await else { panic!("no replies") };
    let reply = replies.last().and_then(|one| one[1].as_str().map(str::to_owned));
    reply.expect("a reply took the answer")
  }

  async fn attach(&mut self, life: &str) {
    assert_eq!(self.ask(Verb::Attach { life: life.to_owned(), from: 0 }).await, Ok(Answer::Done));
  }

  async fn listed(&mut self) -> Vec<Listed> {
    let Ok(Answer::Lives(lives)) = self.ask(Verb::List { config: None }).await else {
      panic!("no list")
    };
    lives
  }

  /// Whether a life lives in the daemon, held or in its grace.
  async fn lives(&mut self, life: &str) -> bool {
    self.listed().await.iter().any(|one| one.life == life && one.open)
  }

  /// Until the list of the daemon holds a life open no more: a life whose record holds no thread
  /// tells no card at its end.
  #[cfg(unix)]
  async fn gone(&mut self, life: &str) {
    let gone = async {
      while self.lives(life).await {
        time::sleep(Duration::from_millis(10)).await;
      }
    };
    timeout(PATIENCE, gone).await.expect("the life ends");
  }

  /// A thread of the operator on the root of a life, which its model closes: the card of the life,
  /// which the window hears from now on.
  async fn threaded(&mut self, life: &str) {
    cards(self).await;
    messaged(self, life, "say done").await;
    let of = |card: &Card| card.life == life && card.thread.standing == Standing::Resolved;
    card(self, of).await;
  }

  /// Waits until the daemon holds a record no more: its life ended, with its image, and gave the
  /// lease of the record back. A life that ends is not open already, and still holds its record,
  /// so a life outside the daemon opens on the record only after this. The daemon tells it as a
  /// card of the life that it holds no more, which a window hears once it asked the cards.
  async fn ended(&mut self, life: &str) {
    card(self, |card| card.life == life && !card.held).await;
  }

  /// Detaches from a life and waits until the daemon holds it no more, so a test leaves no life
  /// that runs past its end: the process of a test can end at once, and a life cut in its middle
  /// leaves its scratch directory in the cache of the machine.
  async fn left(&mut self, life: &str) {
    assert_eq!(self.ask(Verb::Detach { life: life.to_owned() }).await, Ok(Answer::Done));
    let gone = async {
      while self.listed().await.iter().any(|one| one.life == life && one.held) {
        time::sleep(Duration::from_millis(50)).await;
      }
    };
    timeout(PATIENCE, gone).await.expect("the life ends after its grace");
  }
}

/// Whether a frame is the act of a name, done.
fn done(id: &str) -> impl Fn(&Frame) -> bool + '_ {
  move |frame| {
    matches!(frame, Frame::Event { event: Event::Act(act), .. }
      if act.id == id && matches!(act.phase, Phase::Done(_)))
  }
}

/// The command that a rung of a door made, once its fact is pushed.
async fn bashed(window: &mut Window, rung: &str) -> String {
  let made = |frame: &Frame| matches!(frame, Frame::Event { event: Event::Fact(f), .. } if f[0] == "bash" && f[2] == rung);
  let Frame::Event { event: Event::Fact(fact), .. } = window.until(made).await else {
    panic!("no fact")
  };
  fact[1].as_str().expect("the name of an act is a text").to_owned()
}

/// Whether a frame says that windows of a count of operators are attached.
fn present(count: usize) -> impl Fn(&Frame) -> bool {
  move |frame| matches!(frame, Frame::Presence { operators, .. } if operators.len() == count)
}

/// A life opened on a new record in a folder.
async fn opened(window: &mut Window, at: &Path, name: &str) -> String {
  open(window, at, name, None).await
}

/// A life opened on a new record in a folder, which the open attaches to the window from a place
/// of its log when the test names one.
async fn open(window: &mut Window, at: &Path, name: &str, attach: Option<u64>) -> String {
  let life = at.join(name).display().to_string();
  let cwd = Some(at.display().to_string());
  let verb = Verb::Open {
    life: life.clone(),
    cwd,
    actor: None,
    roster: None,
    skills: None,
    config: None,
    claude: None,
    answers: false,
    attach,
  };
  assert_eq!(window.ask(verb).await, Ok(Answer::Done));
  life
}

#[tokio::test]
async fn a_daemon_whose_last_window_left_ends_once_its_grace_passed() {
  let at = place();
  let (local, daemon) = started(&at, SHORT).await;
  let mut window = Window::dialed(&local).await;
  let life = opened(&mut window, &at, "a.jsonl").await;
  window.attach(&life).await;
  drop(window);
  let ended = timeout(Duration::from_secs(20), daemon).await;
  assert!(matches!(ended, Ok(Ok(Ok(())))), "the daemon ends with no window: {ended:?}");
  assert!(local.dial().await.is_err(), "no daemon listens after its grace");
}

#[tokio::test]
async fn a_daemon_that_no_peer_dials_ends_once_its_start_passed() {
  let at = place();
  let local = Local::new(at.join("sock"));
  let listener = local.listen().await.unwrap();
  let (opening, grace, first) = (opening(&at), GRACE, SHORT);
  let settings =
    Settings { opening, grace, first, stall: STALL, tailnet: None, door: None, data: None };
  let ended = timeout(PATIENCE, run(&local, listener, settings)).await;
  assert!(matches!(ended, Ok(Ok(()))), "the daemon ends with no peer: {ended:?}");
  assert!(local.dial().await.is_err(), "no daemon listens after its start");
}

#[tokio::test]
async fn two_windows_on_one_life_both_stay_and_hear_it() {
  let at = place();
  let (local, _daemon) = started(&at, GRACE).await;
  let (mut first, mut second) = (Window::dialed(&local).await, Window::dialed(&local).await);
  let life = opened(&mut first, &at, "a.jsonl").await;
  first.attach(&life).await;
  second.attach(&life).await;
  let rung = second.ran(&life, "x = 1").await;
  first.until(done(&rung)).await;
  let rung = first.ran(&life, "y = 2").await;
  second.until(done(&rung)).await;
  let Ok(Answer::Presence(operators)) = first.ask(Verb::Presence { life }).await else { panic!() };
  assert_eq!(operators.len(), 1, "two windows of one operator are one operator");
}

#[tokio::test]
async fn an_image_of_a_window_stands_in_the_gallery_of_the_life_and_a_message_names_it() {
  let at = place();
  let (local, _daemon) = started(&at, GRACE).await;
  let mut window = Window::dialed(&local).await;
  let life = opened(&mut window, &at, "a.jsonl").await;
  let (name, data) = (
    "image-1.png".to_owned(),
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR4nGP4takZAATZAjDdIHqMAAAAAElFTkSuQmCC"
      .to_owned(),
  );
  let named = window.ask(Verb::Image { life: life.clone(), name, data }).await;
  let file = "9e485a66b9b8e7bee8eec3a548388240c61b2d9854bd1a1dc1aa5de904b8ae17.png";
  assert_eq!(named, Ok(Answer::Value(json!(format!("![image-1.png](furb-image://{file})")))));
  assert!(at.join("a.jsonl.images").join(file).is_file());
  let data = "bm8gaW1hZ2U=".to_owned();
  let refused = window.ask(Verb::Image { life, name: "image-2.png".to_owned(), data }).await;
  assert!(matches!(refused, Err(ProtocolError::Refused { .. })), "{refused:?}");
}

#[tokio::test]
async fn the_detach_of_one_window_leaves_the_other() {
  let at = place();
  let (local, _daemon) = started(&at, SHORT).await;
  let (mut first, mut second) = (Window::dialed(&local).await, Window::dialed(&local).await);
  let life = opened(&mut first, &at, "a.jsonl").await;
  first.attach(&life).await;
  second.attach(&life).await;
  assert_eq!(first.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
  second.until(present(1)).await;
  time::sleep(PAST).await;
  assert!(second.lives(&life).await);
  second.ran(&life, "x = 1").await;
  let refused = first.ask(Verb::Detach { life: life.clone() }).await;
  assert_eq!(refused, Err(ProtocolError::Unattached { life }));
}

#[tokio::test]
async fn a_life_ends_after_the_grace_of_its_last_holder_and_not_while_a_pin_holds_it() {
  let at = place();
  let (local, _daemon) = started(&at, SHORT).await;
  let mut window = Window::dialed(&local).await;
  let life = opened(&mut window, &at, "a.jsonl").await;
  let pin = || Verb::Pin { life: life.clone(), holder: "job".to_owned() };
  assert_eq!(window.ask(pin()).await, Ok(Answer::Done));
  time::sleep(PAST).await;
  assert!(window.lives(&life).await);
  let unpin = Verb::Unpin { life: life.clone(), holder: "job".to_owned() };
  assert_eq!(window.ask(unpin).await, Ok(Answer::Done));
  time::sleep(PAST).await;
  assert!(!window.lives(&life).await);
  let refused = window.ask(pin()).await;
  assert!(matches!(refused, Err(ProtocolError::Closed { .. })), "{refused:?}");
}

#[tokio::test]
async fn a_window_ends_its_life_now_whatever_pins_hold_it_and_never_one_that_another_shows() {
  let at = place();
  let (local, _daemon) = started(&at, GRACE).await;
  let (mut first, mut second) = (Window::dialed(&local).await, Window::dialed(&local).await);
  let life = opened(&mut first, &at, "a.jsonl").await;
  first.attach(&life).await;
  second.attach(&life).await;
  let end = || Verb::End { life: life.clone(), cancel: false };
  assert_eq!(first.ask(end()).await, Err(ProtocolError::Shown { life: life.clone() }));
  assert_eq!(second.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
  let pin = Verb::Pin { life: life.clone(), holder: "background".to_owned() };
  assert_eq!(first.ask(pin).await, Ok(Answer::Done));
  assert_eq!(first.ask(end()).await, Ok(Answer::Done));
  first.until(|frame| matches!(frame, Frame::Event { event: Event::Ended(None), .. })).await;
  assert!(!first.lives(&life).await, "the life ends before its grace");
}

#[tokio::test]
async fn an_end_is_a_pause_whose_next_life_takes_its_open_work_and_a_kill_cancels_it_first() {
  for (cancel, after) in [(false, view::State::Done), (true, view::State::Cancelled)] {
    let at = place();
    let (local, _daemon) = started(&at, GRACE).await;
    let mut window = Window::dialed(&local).await;
    let life = open(&mut window, &at, "a.jsonl", Some(0)).await;
    let word = "await wait(2, __name__)".to_owned();
    let run = Verb::Run { life: life.clone(), word, on: None };
    assert!(matches!(window.ask(run).await, Ok(Answer::Act(_))));
    let waits = |frame: &Frame| matches!(frame, Frame::Event { event: Event::Act(act), .. } if act.kind == view::Kind::Wait);
    let Frame::Event { event: Event::Act(wait), .. } = window.until(waits).await else { panic!() };
    let end = Verb::End { life: life.clone(), cancel };
    assert_eq!(window.ask(end).await, Ok(Answer::Done));
    window.until(|frame| matches!(frame, Frame::Event { event: Event::Ended(None), .. })).await;
    open(&mut window, &at, "a.jsonl", None).await;
    let ended = Verb::Ended { life, act: wait.id.clone() };
    let Ok(Answer::Acts(acts)) = window.ask(ended).await else { panic!("no end of the wait") };
    let states: Vec<view::State> = acts.iter().map(|act| act.state).collect();
    assert_eq!(states, [after], "an end with cancel {cancel}");
  }
}

#[tokio::test]
async fn the_list_and_the_cards_name_the_pins_so_a_life_in_its_grace_is_no_life_of_the_background()
{
  let at = place();
  let (local, _daemon) = started(&at, GRACE).await;
  let mut window = Window::dialed(&local).await;
  let life = opened(&mut window, &at, "a.jsonl").await;
  window.threaded(&life).await;
  let pins =
    |listed: Vec<Listed>| listed.into_iter().find(|one| one.life == life).map(|one| one.pins);
  assert_eq!(pins(window.listed().await), Some(Vec::new()));
  let pin = Verb::Pin { life: life.clone(), holder: "background".to_owned() };
  assert_eq!(window.ask(pin).await, Ok(Answer::Done));
  assert_eq!(pins(window.listed().await), Some(vec!["background".to_owned()]));
  card(&mut window, |card| card.life == life && card.pins == ["background"]).await;
  let unpin = Verb::Unpin { life: life.clone(), holder: "background".to_owned() };
  assert_eq!(window.ask(unpin).await, Ok(Answer::Done));
  card(&mut window, |card| card.life == life && card.pins.is_empty()).await;
}

#[tokio::test]
async fn the_presence_names_each_profile_once_and_the_draft_that_one_of_its_windows_writes() {
  let at = place();
  let (local, _daemon) = started(&at, GRACE).await;
  let (mut first, mut second) = (Window::dialed(&local).await, Window::dialed(&local).await);
  let life = opened(&mut first, &at, "a.jsonl").await;
  first.attach(&life).await;
  second.attach(&life).await;
  let draft = Some(Draft { on: Some("prompt3".to_owned()) });
  let writes = Verb::Writes { life: life.clone(), draft: draft.clone() };
  assert_eq!(second.ask(writes).await, Ok(Answer::Done));
  let writing = |any: bool| {
    move |frame: &Frame| {
      matches!(frame, Frame::Presence { operators, .. }
        if operators.iter().any(|one| one.writes.is_some()) == any)
    }
  };
  let Frame::Presence { operators, .. } = first.until(writing(true)).await else { panic!() };
  let operator = operators[0].operator.clone();
  assert_eq!(operators, [Present { operator, role: Role::Write, writes: draft, windows: 2 }]);
  let cleared = Verb::Writes { life: life.clone(), draft: None };
  assert_eq!(second.ask(cleared).await, Ok(Answer::Done));
  first.until(writing(false)).await;
  let stranger = Verb::Writes { life: life.clone(), draft: None };
  let mut third = Window::dialed(&local).await;
  assert_eq!(third.ask(stranger).await, Err(ProtocolError::Unattached { life }));
}

/// A folder `main` that the list of projects names, with a worktree `tree`, and the record
/// `two.jsonl` in the sessions of the worktree, last written at the second 1000000 of the epoch:
/// the folder of the sessions of `main`, and the one of `tree`.
fn worktree(at: &Path) -> (PathBuf, PathBuf) {
  let main = at.join("main");
  let sessions = main.join(".furb").join("sessions");
  let tree = at.join("tree");
  let marks = main.join(".git").join("worktrees").join("tree");
  for folder in [&sessions, &tree.join(".furb").join("sessions"), &marks] {
    fs::create_dir_all(folder).unwrap();
  }
  fs::write(marks.join("gitdir"), format!("{}\n", tree.join(".git").display())).unwrap();
  fs::write(tree.join(".git"), "gitdir: elsewhere\n").unwrap();
  let tree = dunce::canonicalize(&tree).unwrap().join(".furb").join("sessions");
  fs::write(tree.join("two.jsonl"), "").unwrap();
  let then = UNIX_EPOCH + Duration::from_secs(1_000_000);
  File::options().write(true).open(tree.join("two.jsonl")).unwrap().set_modified(then).unwrap();
  fs::write(tree.join("two.changes.jsonl"), "").unwrap();
  let list = json!({ "workspaces": [{ "directory": main.display().to_string() }] });
  fs::create_dir_all(at.join("config")).unwrap();
  fs::write(at.join("config").join("workspaces.json"), list.to_string()).unwrap();
  (sessions, tree)
}

#[tokio::test]
async fn the_list_gives_each_record_of_the_sessions_of_each_project_and_whether_a_life_holds_it() {
  let at = place();
  let (sessions, tree) = worktree(&at);
  let (local, _daemon) = started(&at, GRACE).await;
  let mut window = Window::dialed(&local).await;
  let one = opened(&mut window, &sessions, "one.jsonl").await;
  let alone = opened(&mut window, &at, "a.jsonl").await;
  let project = Some(at.join("main").display().to_string());
  let mut wanted = vec![
    Listed {
      life: one,
      project: project.clone(),
      used: None,
      held: true,
      open: true,
      watched: false,
      pins: Vec::new(),
      name: None,
    },
    Listed {
      life: alone,
      project: None,
      used: None,
      held: true,
      open: true,
      watched: false,
      pins: Vec::new(),
      name: None,
    },
    Listed {
      life: tree.join("two.jsonl").display().to_string(),
      project,
      used: Some(1_000_000),
      held: false,
      open: false,
      watched: false,
      pins: Vec::new(),
      name: None,
    },
  ];
  wanted.sort_by(|a, b| a.life.cmp(&b.life));
  let mut listed = window.listed().await;
  // A life writes its record as it goes, at a second that the test does not set.
  listed.iter_mut().filter(|one| one.held).for_each(|one| one.used = None);
  assert_eq!(listed, wanted);
  let verbatim = |one: &Listed| one.life.starts_with(r"\\?\");
  assert!(!listed.iter().any(verbatim), "a worktree is listed under the path that made it");
}

#[tokio::test]
async fn the_projects_are_those_of_the_list_with_their_sessions_and_their_last_use() {
  let at = place();
  let (sessions, _) = worktree(&at);
  fs::write(sessions.join("one.jsonl"), "").unwrap();
  let then = UNIX_EPOCH + Duration::from_secs(2_000_000);
  File::options().write(true).open(sessions.join("one.jsonl")).unwrap().set_modified(then).unwrap();
  let (local, _daemon) = started(&at, GRACE).await;
  let mut window = Window::dialed(&local).await;
  let folder = at.join("main").display().to_string();
  let used = Some(2_000_000);
  let projects = window.ask(Verb::Projects { config: None }).await;
  let alone = at.join("data").join("sessions").display().to_string();
  let wanted = vec![
    Project { folder, sessions: 2, used, detached: false },
    Project { folder: alone, sessions: 0, used: None, detached: true },
  ];
  assert_eq!(projects, Ok(Answer::Projects(wanted)));
}

#[tokio::test]
async fn a_session_with_no_project_opens_in_a_folder_of_its_own_and_stands_in_no_project() {
  let at = place();
  let (local, _daemon) = started(&at, GRACE).await;
  let mut window = Window::dialed(&local).await;
  let alone = at.join("data").join("sessions");
  let folder = alone.join("plan-the-week");
  let record = folder.join(".furb").join("sessions").join("one.jsonl");
  let life = record.display().to_string();
  let verb = Verb::Open {
    life: life.clone(),
    cwd: None,
    actor: None,
    roster: None,
    skills: None,
    config: None,
    claude: None,
    answers: false,
    attach: None,
  };
  assert_eq!(window.ask(verb).await, Ok(Answer::Done));
  let cwd = window.ask(Verb::Cwd { life: life.clone(), on: None }).await;
  assert_eq!(cwd, Ok(Answer::Value(json!(folder.display().to_string()))));
  let listed = window.listed().await;
  assert_eq!(
    listed.iter().map(|one| (&*one.life, one.project.as_deref())).collect::<Vec<_>>(),
    [(&*life, None)]
  );
  let projects = window.ask(Verb::Projects { config: None }).await;
  let Ok(Answer::Projects(projects)) = projects else { panic!("{projects:?}") };
  let wanted = Project {
    folder: alone.display().to_string(),
    sessions: 1,
    used: projects[0].used,
    detached: true,
  };
  assert_eq!(projects, [wanted]);
}

#[cfg(unix)]
#[tokio::test]
async fn the_models_are_those_that_the_catalog_offers_with_no_life() {
  use std::os::unix::fs::PermissionsExt;

  use furb_host::protocol::Model;
  let at = place();
  let (local, _daemon) = started(&at, GRACE).await;
  let mut window = Window::dialed(&local).await;
  // A claude command line that says nothing offers the models of its own catalog.
  let claude = at.join("claude");
  fs::write(&claude, "#!/bin/sh\n").unwrap();
  fs::set_permissions(&claude, fs::Permissions::from_mode(0o755)).unwrap();
  let claude = Some(claude.display().to_string());
  let models = Verb::Models { cwd: Some(at.display().to_string()), config: None, claude };
  let Ok(Answer::Models(models)) = window.ask(models).await else { panic!("no models") };
  let efforts = ["low", "medium", "high", "xhigh", "max"].map(str::to_owned).to_vec();
  let effort = Some("high".to_owned());
  let opus =
    Model { name: "claude-cli:opus".to_owned(), efforts, effort, window: 1_000_000, images: true };
  assert!(models.contains(&opus), "{models:?}");
  assert_eq!(window.listed().await, Vec::new(), "the daemon holds no life to list them");
}

#[tokio::test]
async fn the_files_of_a_folder_hold_its_dot_files_and_leave_out_what_git_ignores_and_the_records() {
  use furb_host::protocol::Files;
  let at = place();
  let (local, _daemon) = started(&at, GRACE).await;
  let mut window = Window::dialed(&local).await;
  let folder = at.join("project");
  for (path, text) in [
    ("a.txt", ""),
    (".env", ""),
    (".gitignore", "target/\n"),
    ("sub/b.rs", ""),
    ("target/x.o", ""),
    (".git/HEAD", ""),
    (".furb/sessions/a.jsonl", ""),
    (".furb/config.json", "{}"),
  ] {
    let file = folder.join(path);
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(file, text).unwrap();
  }
  let files = Verb::Files { cwd: folder.display().to_string() };
  let paths =
    [".env", ".furb/config.json", ".gitignore", "a.txt", "sub/b.rs"].map(str::to_owned).to_vec();
  assert_eq!(window.ask(files).await, Ok(Answer::Files(Files { paths, more: false })));
  assert_eq!(window.listed().await, Vec::new(), "the daemon holds no life to walk them");
}

#[tokio::test]
async fn the_list_says_whether_a_window_holds_each_life_and_a_pin_is_no_window() {
  let at = place();
  let (local, _daemon) = started(&at, GRACE).await;
  let mut window = Window::dialed(&local).await;
  let mut other = Window::dialed(&local).await;
  let life = opened(&mut window, &at, "a.jsonl").await;
  let watched = |listed: Vec<Listed>| listed.iter().any(|one| one.life == life && one.watched);
  assert!(!watched(window.listed().await), "an open that attaches nothing is held by no window");
  let pin = Verb::Pin { life: life.clone(), holder: "skill".to_owned() };
  assert_eq!(window.ask(pin).await, Ok(Answer::Done));
  assert!(!watched(window.listed().await), "a pin holds the life and no window holds it");
  other.attach(&life).await;
  assert!(watched(window.listed().await), "a window of another connection holds the life");
  assert_eq!(other.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
  assert!(!watched(window.listed().await), "a window that detached holds nothing");
}

#[tokio::test]
async fn a_life_that_its_open_attaches_has_a_holder_before_a_grace_of_none_passes() {
  let at = place();
  let (local, _daemon) = started(&at, Duration::ZERO).await;
  let mut window = Window::dialed(&local).await;
  let life = at.join("a.jsonl").display().to_string();
  let open = Verb::Open {
    life: life.clone(),
    cwd: Some(at.display().to_string()),
    actor: None,
    roster: None,
    skills: None,
    config: None,
    claude: None,
    answers: false,
    attach: Some(0),
  };
  assert_eq!(window.ask(open).await, Ok(Answer::Done));
  window.until(present(1)).await;
  time::sleep(PAST).await;
  assert!(window.lives(&life).await);
  window.ran(&life, "x = 1").await;
  assert_eq!(window.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
  time::sleep(PAST).await;
  assert!(!window.lives(&life).await);
}

/// The frames of a window in the order that it read them, up to the first that the test waits for.
#[cfg(unix)]
async fn read_until(window: &mut Window, read: &mut Vec<Frame>, wanted: impl Fn(&Frame) -> bool) {
  loop {
    let frame = match window.pushed.pop_front() {
      Some(frame) => frame,
      None => window.read().await.expect("the daemon pushes the frame"),
    };
    let found = wanted(&frame);
    read.push(frame);
    if found {
      return;
    }
  }
}

/// Holds what a window read of an open and of the life after it: each stage once and in order,
/// numbers that never go back, a last frame that is whole, and no frame of the open after the
/// first event of the life.
#[cfg(unix)]
fn told_whole(read: &[Frame]) {
  use furb::life::Stage;
  let event = read.iter().position(|one| matches!(one, Frame::Event { .. })).unwrap_or(read.len());
  let opening = |some: &[Frame]| -> Vec<(Stage, u64, u64)> {
    let frames = some.iter().filter_map(|one| match one {
      Frame::Opening { stage, done, of, .. } => Some((*stage, *done, *of)),
      _ => None,
    });
    frames.collect()
  };
  let frames = opening(&read[..event]);
  let mut stages: Vec<Stage> = frames.iter().map(|one| one.0).collect();
  stages.dedup();
  // The record did not change after the stage find, so no stage reads it again.
  let all = [Stage::Recall, Stage::Find, Stage::Seed, Stage::Boot, Stage::Log];
  assert_eq!(stages, all, "each stage comes once, in order: {frames:?}");
  let back = |pair: &[(Stage, u64, u64)]| {
    pair[0].0 == pair[1].0 && (pair[1].1 < pair[0].1 || pair[1].2 < pair[0].2)
  };
  assert!(!frames.windows(2).any(back), "no number of a stage goes back: {frames:?}");
  assert!(frames.iter().all(|one| one.1 <= one.2), "no work done passes the work known");
  let (stage, done, of) = *frames.last().unwrap();
  assert!(stage == Stage::Log && of > 0 && done == of, "the last frame is whole: {frames:?}");
  let after = opening(&read[event..]);
  assert!(after.is_empty(), "no frame of the open comes after the first event: {after:?}");
}

#[cfg(unix)]
#[tokio::test]
async fn a_window_of_a_life_that_opens_from_its_image_hears_its_stages_then_its_events() {
  use furb::life::Stage;
  let at = place();
  let (local, _daemon) = started(&at, SHORT).await;
  let mut window = Window::dialed(&local).await;
  let life = opened(&mut window, &at, "a.jsonl").await;
  window.attach(&life).await;
  window.threaded(&life).await;
  window.ran(&life, "x = 41 + 1").await;
  assert_eq!(window.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
  window.ended(&life).await;
  window.pushed.clear();
  let open = Verb::Open {
    life: life.clone(),
    cwd: Some(at.display().to_string()),
    actor: None,
    roster: None,
    skills: None,
    config: None,
    claude: None,
    answers: false,
    attach: Some(0),
  };
  assert_eq!(window.ask(open).await, Ok(Answer::Done));
  let mut read = Vec::new();
  read_until(&mut window, &mut read, |frame| matches!(frame, Frame::Event { .. })).await;
  let mut stages: Vec<Stage> = read
    .iter()
    .filter_map(|one| match one {
      Frame::Opening { stage, .. } => Some(*stage),
      _ => None,
    })
    .collect();
  stages.dedup();
  // The life reads no entry of its record, so it tells no recall, no read and no boot.
  assert_eq!(stages, [Stage::Find, Stage::Seed, Stage::Log], "{read:?}");
}

/// A claude command line of a test that says nothing until a file is there, and that file. The
/// catalog waits for a command line that it never saw, so the open of a life on it stands at the
/// catalog, after the scan of its record, until the test makes the file.
#[cfg(unix)]
fn waiting(at: &Path) -> (String, PathBuf) {
  use std::os::unix::fs::PermissionsExt;
  let (program, go) = (at.join("claude"), at.join("go"));
  let waits = format!("#!/bin/sh\nuntil [ -e '{}' ]; do sleep 0.02; done\n", go.display());
  fs::write(&program, waits).unwrap();
  fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
  (program.display().to_string(), go)
}

#[cfg(unix)]
#[tokio::test]
async fn each_window_of_a_life_that_opens_hears_how_far_the_open_stands_then_its_events() {
  use furb::life::Stage;
  let at = place();
  let (local, _daemon) = started(&at, SHORT).await;
  let mut first = Window::dialed(&local).await;
  let life = opened(&mut first, &at, "a.jsonl").await;
  first.attach(&life).await;
  first.threaded(&life).await;
  first.ran(&life, "x = 41 + 1").await;
  assert_eq!(first.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
  first.ended(&life).await;
  // A crossing in the record, and no image, make the open go by the record, through each stage
  // but the crossing, which a life of this build made already.
  crossed(Opening { record: Some(life.clone()), ..opening(&at) }).await;
  let crossing = Opening { record: Some(life.clone()), keeps: Some(true), ..opening(&at) };
  let crosses = move || drop(crossing.boot::<String>([]).expect("the life that crosses"));
  task::spawn_blocking(crosses).await.expect("the life that crosses ends");
  fs::remove_file(format!("{life}.image")).ok();
  first.pushed.clear();
  let (claude, go) = waiting(&at);
  let open = Verb::Open {
    life: life.clone(),
    cwd: Some(at.display().to_string()),
    actor: None,
    roster: None,
    skills: None,
    config: None,
    claude: Some(claude),
    answers: false,
    attach: Some(0),
  };
  assert_eq!(first.ask(open).await, Ok(Answer::Done));
  let found = |frame: &Frame| matches!(frame, Frame::Opening { stage: Stage::Find, done, of, .. } if done == of);
  let event = |frame: &Frame| matches!(frame, Frame::Event { .. });
  let (mut early, mut late) = (Vec::new(), Vec::new());
  read_until(&mut first, &mut early, found).await;
  let mut second = Window::dialed(&local).await;
  second.attach(&life).await;
  read_until(&mut second, &mut late, found).await;
  // A verb that a window says while the life opens waits for the life.
  let inspect = Verb::Inspect { life: life.clone(), name: "x".to_owned(), on: None };
  let asked = Request { id: first.next, verb: inspect.clone() };
  first.lines.send(serde_json::to_string(&asked).unwrap()).await.unwrap();
  assert!(
    !early.iter().chain(&late).any(event),
    "the open stands at the catalog, and no window heard an event of the life"
  );
  File::create(&go).unwrap();
  let value = json!(42);
  let answer = Answer::Inspected { kind: "int".to_owned(), repr: "42".to_owned(), value };
  let replied = |frame: &Frame| *frame == Frame::Reply { id: asked.id, answer: answer.clone() };
  read_until(&mut first, &mut early, replied).await;
  read_until(&mut first, &mut early, event).await;
  read_until(&mut second, &mut late, event).await;
  assert_eq!(second.ask(inspect).await, Ok(answer.clone()));
  late.extend(second.pushed.drain(..));
  told_whole(&early);
  told_whole(&late);
}

#[cfg(unix)]
#[tokio::test]
async fn a_verb_that_waited_for_an_open_that_failed_hears_why() {
  use std::io::Write as _;

  use furb::life::Stage;
  let at = place();
  let (local, _daemon) = started(&at, SHORT).await;
  let mut window = Window::dialed(&local).await;
  let life = opened(&mut window, &at, "a.jsonl").await;
  window.attach(&life).await;
  window.threaded(&life).await;
  window.ran(&life, "x = 1").await;
  assert_eq!(window.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
  window.ended(&life).await;
  crossed(Opening { record: Some(life.clone()), ..opening(&at) }).await;
  let (claude, go) = waiting(&at);
  let open = Verb::Open {
    life: life.clone(),
    cwd: Some(at.display().to_string()),
    actor: None,
    roster: None,
    skills: None,
    config: None,
    claude: Some(claude),
    answers: false,
    attach: Some(0),
  };
  assert_eq!(window.ask(open).await, Ok(Answer::Done));
  let found = |frame: &Frame| matches!(frame, Frame::Opening { stage: Stage::Find, done, of, .. } if done == of);
  read_until(&mut window, &mut Vec::new(), found).await;
  // The record takes a line that is no entry while the open stands at the catalog, so the open
  // reads the record again after the verb waits for it, and fails.
  let at_byte = fs::metadata(&life).unwrap().len();
  fs::OpenOptions::new().append(true).open(&life).unwrap().write_all(b"no entry\n").unwrap();
  let inspect = Verb::Inspect { life: life.clone(), name: "x".to_owned(), on: None };
  let asked = Request { id: window.next, verb: inspect };
  window.lines.send(serde_json::to_string(&asked).unwrap()).await.unwrap();
  File::create(&go).unwrap();
  let refused =
    |frame: &Frame| matches!(frame, Frame::Refused { id: Some(id), .. } if *id == asked.id);
  let mut read = Vec::new();
  read_until(&mut window, &mut read, refused).await;
  let Some(Frame::Refused { why, .. }) = read.pop() else { unreachable!("the wait ends on it") };
  assert_eq!(why, format!("Refused: Invalid record entry at byte {at_byte} in {life}."));
}

#[cfg(unix)]
#[tokio::test]
async fn a_verb_said_after_an_open_failed_hears_why_until_the_next_open() {
  let at = place();
  let (local, _daemon) = started(&at, SHORT).await;
  let mut window = Window::dialed(&local).await;
  cards(&mut window).await;
  let life = at.join("a.jsonl").display().to_string();
  // The record holds a line that is no entry, so the open fails after its answer.
  fs::write(&life, "no entry\n").unwrap();
  open(&mut window, &at, "a.jsonl", None).await;
  // The daemon holds the life no more before the verb comes, as for a window that said it after
  // the answer to its open.
  window.gone(&life).await;
  let inspect = Verb::Inspect { life: life.clone(), name: "x".to_owned(), on: None };
  let why = format!("Refused: Invalid record entry at byte 0 in {life}.");
  assert_eq!(window.ask(inspect).await, Err(ProtocolError::Refused { why }));
  fs::remove_file(&life).unwrap();
  open(&mut window, &at, "a.jsonl", None).await;
  let pending = window.ask(Verb::Pending { life }).await;
  assert!(pending.is_ok(), "{pending:?}");
}

#[cfg(unix)]
#[tokio::test]
async fn an_open_of_a_record_that_another_process_holds_is_refused_at_once_with_the_reason() {
  let at = place();
  let (local, _daemon) = started(&at, SHORT).await;
  let mut window = Window::dialed(&local).await;
  let life = at.join("a.jsonl").display().to_string();
  let lease = File::create(format!("{life}.lock")).unwrap();
  lease.try_lock().unwrap();
  let verb = Verb::Open {
    life: life.clone(),
    cwd: Some(at.display().to_string()),
    actor: None,
    roster: None,
    skills: None,
    config: None,
    claude: None,
    answers: false,
    attach: None,
  };
  let why = format!("Refused: Another process owns {life}.");
  assert_eq!(window.ask(verb).await, Err(ProtocolError::Refused { why }));
  drop(lease);
  open(&mut window, &at, "a.jsonl", None).await;
}

#[tokio::test]
async fn a_life_that_ended_opens_again_at_the_tip_of_its_record() {
  let at = place();
  let (local, _daemon) = started(&at, SHORT).await;
  let mut window = Window::dialed(&local).await;
  let life = opened(&mut window, &at, "a.jsonl").await;
  window.attach(&life).await;
  window.ran(&life, "x = 41 + 1").await;
  assert_eq!(window.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
  time::sleep(PAST).await;
  assert!(!window.lives(&life).await);
  opened(&mut window, &at, "a.jsonl").await;
  let inspect = Verb::Inspect { life, name: "x".to_owned(), on: None };
  let Ok(Answer::Inspected { value, .. }) = window.ask(inspect).await else { panic!() };
  assert_eq!(value, json!(42));
}

#[tokio::test]
async fn a_window_of_a_life_born_again_hears_each_fact_of_the_life_once() {
  let at = place();
  let (local, _daemon) = started(&at, SHORT).await;
  let mut window = Window::dialed(&local).await;
  let life = opened(&mut window, &at, "a.jsonl").await;
  window.attach(&life).await;
  let first = window.ran(&life, "x = 41 + 1").await;
  assert_eq!(window.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
  time::sleep(PAST).await;
  assert!(!window.lives(&life).await);
  // A second birth opens on the image of the first, and ends with an image of its own.
  let mut between = Window::dialed(&local).await;
  opened(&mut between, &at, "a.jsonl").await;
  between.attach(&life).await;
  between.ran(&life, "z = 0").await;
  assert_eq!(between.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
  time::sleep(PAST).await;
  assert!(!between.lives(&life).await);
  let mut later = Window::dialed(&local).await;
  opened(&mut later, &at, "a.jsonl").await;
  later.attach(&life).await;
  later.ran(&life, "y = 1").await;
  let facts_of = |window: &Window| -> Vec<Value> {
    let facts = window.pushed.iter().filter_map(|frame| match frame {
      // The extensions teach when they load, which a birth may do after the rung that the test
      // runs, so the test counts none of their facts.
      Frame::Event { event: Event::Fact(fact), .. }
        if fact[0] != "keep" && fact[2] != "extensions" =>
      {
        Some(fact.clone())
      }
      _ => None,
    });
    facts.collect()
  };
  let (once, twice, facts) = (facts_of(&window), facts_of(&between), facts_of(&later));
  let about = |kind: &str| facts.iter().filter(|one| one[0] == kind && one[1] == first).count();
  let mine: Vec<&Value> = facts.iter().filter(|one| one[1] == first).collect();
  assert_eq!((about("rung"), about("done")), (1, 1), "{mine:?}");
  // The second and the third births do alike, so each adds as many occurrences of each fact of
  // the first: a fact that an earlier birth restates told again would add more, and a fact that
  // repeats an earlier one, dropped, fewer.
  let times = |facts: &[Value], one: &Value| facts.iter().filter(|other| *other == one).count();
  let added = |later: &[Value], earlier: &[Value], one: &Value| {
    i64::try_from(times(later, one)).unwrap() - i64::try_from(times(earlier, one)).unwrap()
  };
  let unlike = |one: &&Value| added(&facts, &twice, one) != added(&twice, &once, one);
  let again: Vec<&Value> = once.iter().filter(unlike).collect();
  assert!(again.is_empty(), "the births add these facts unlike: {again:?}");
}

#[tokio::test]
async fn a_fault_of_one_life_leaves_the_daemon_and_the_other_lives() {
  let at = place();
  let (local, daemon) = started(&at, GRACE).await;
  let mut window = Window::dialed(&local).await;
  let good = opened(&mut window, &at, "a.jsonl").await;
  fs::create_dir_all(at.join("bad.jsonl")).unwrap();
  let bad = opened(&mut window, &at, "bad.jsonl").await;
  match window.ask(Verb::Attach { life: bad.clone(), from: 0 }).await {
    Ok(Answer::Done) => {
      let ended = |frame: &Frame| matches!(frame, Frame::Event { life, event: Event::Ended(Some(_)), .. } if *life == bad);
      window.until(ended).await;
    }
    refused => assert_eq!(refused, Err(ProtocolError::Closed { life: bad.clone() })),
  }
  window.attach(&good).await;
  window.ran(&good, "x = 1").await;
  assert!(window.lives(&good).await);
  assert!(!daemon.is_finished());
}

/// A record whose replay drifts at its first cd, which moved to a place that the word of the life
/// does not ask for.
fn drifted(life: &str) -> Vec<u8> {
  let text = fs::read_to_string(life).unwrap();
  let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
  let moved = lines.iter().position(|line| line.starts_with(r#"[["cd","cd1","#)).unwrap();
  let mut entry: Value = serde_json::from_str(&lines[moved]).unwrap();
  let place = entry[0][4].as_str().expect("the cd names its place").to_owned();
  entry[0][4] = Value::from(format!("{place}-elsewhere"));
  lines[moved] = entry.to_string();
  fs::write(life, lines.join("\n") + "\n").unwrap();
  fs::write(format!("{life}.log"), "{\"entries\": 0}\n").unwrap();
  fs::read(life).unwrap()
}

/// The name and the threads that an index of a record holds.
fn threads(index: &Value) -> (Value, Vec<Value>) {
  let all = index["threads"].as_array().cloned().unwrap_or_default();
  (index["name"].clone(), all.into_iter().map(|thread| thread["thread"].clone()).collect())
}

/// The name and the threads of the index of a record at a path, once it names a tip of that many
/// bytes, as the daemon writes it from a fold of the record.
async fn folded(index: &str, bytes: usize) -> (Value, Vec<Value>) {
  let start = Instant::now();
  loop {
    let read: Option<Value> =
      fs::read(index).ok().and_then(|one| serde_json::from_slice(&one).ok());
    if let Some(read) = read.filter(|one| one["tip"]["bytes"] == bytes) {
      return threads(&read);
    }
    assert!(start.elapsed() < PATIENCE, "the daemon makes no index of the record at rest");
    time::sleep(SHORT).await;
  }
}

/// A window that attaches a life after a fault ended it hears its events and its end, with why.
#[tokio::test]
async fn a_window_that_attaches_a_life_that_a_fault_ended_hears_why() {
  let at = place();
  let (local, _daemon) = started_with(&at, SHORT, opening(&at)).await;
  let mut window = Window::dialed(&local).await;
  cards(&mut window).await;
  let before = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/replay/speaker.jsonl");
  fs::copy(before, at.join("before.jsonl")).unwrap();
  let life = open(&mut window, &at, "before.jsonl", None).await;
  window.ended(&life).await;
  assert_eq!(window.ask(Verb::Attach { life: life.clone(), from: 0 }).await, Ok(Answer::Done));
  let why = SchemaError::BeforeCut { of: "record" }.to_string();
  let ended = |frame: &Frame| matches!(frame, Frame::Event { life: one, event: Event::Ended(Some(no)), .. } if *one == life && *no == why);
  window.until(ended).await;
}

/// A record from before the stable cut opens in no life: its window hears why, reads its threads,
/// read only, until it detaches the life, and hears why for each other verb.
#[tokio::test]
async fn a_record_from_before_the_stable_cut_shows_its_threads_to_its_window_until_it_detaches_it()
{
  let at = place();
  let (local, _daemon) = started_with(&at, SHORT, opening(&at)).await;
  let mut window = Window::dialed(&local).await;
  let before = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/replay/speaker.jsonl");
  fs::copy(before, at.join("before.jsonl")).unwrap();
  let life = open(&mut window, &at, "before.jsonl", Some(0)).await;
  let why = SchemaError::BeforeCut { of: "record" }.to_string();
  let ended = |frame: &Frame| matches!(frame, Frame::Event { life: one, event: Event::Ended(Some(no)), .. } if *one == life && *no == why);
  window.until(ended).await;
  let page = || Verb::Thread {
    life: life.clone(),
    thread: "prompt1".to_owned(),
    before: None,
    limit: 10,
    at: None,
  };
  let read = window.ask(page()).await;
  assert!(matches!(&read, Ok(Answer::Page(one)) if one.exchanges.len() == 1), "{read:?}");
  let refused = window.ask(Verb::Standing { life: life.clone() }).await;
  assert_eq!(refused, Err(ProtocolError::Refused { why: why.clone() }));
  assert_eq!(window.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
  assert_eq!(window.ask(page()).await, Err(ProtocolError::Refused { why }), "a detach lets it go");
}

/// The card of a record from before the stable cut says why no life opens it, before a window
/// opens it, and stands with the resolved threads: its thread only reads.
#[tokio::test]
async fn the_card_of_a_record_from_before_the_stable_cut_says_why_it_only_reads() {
  let at = place();
  let (_, sessions) = project(&at);
  let before = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/replay/speaker.jsonl");
  fs::copy(before, sessions.join("before.jsonl")).unwrap();
  let (local, _daemon) = started(&at, SHORT).await;
  let mut window = Window::dialed(&local).await;
  cards(&mut window).await;
  let told = card(&mut window, |card| card.life.ends_with("before.jsonl")).await;
  let why = SchemaError::BeforeCut { of: "record" }.to_string();
  assert_eq!(told.refused, Some(why));
  assert_eq!(told.place("anyone").0, Part::Resolved);
}

#[tokio::test]
async fn a_drift_at_the_open_of_a_life_tells_its_window_why_and_leaves_the_daemon() {
  let at = place();
  let lines = Arc::new(Mutex::new(Vec::<String>::new()));
  let kept = Arc::clone(&lines);
  let log = Log(Arc::new(move |line: &str| kept.lock().unwrap().push(line.to_owned())));
  let (local, daemon) = started_with(&at, SHORT, Opening { log: Some(log), ..opening(&at) }).await;
  let mut window = Window::dialed(&local).await;
  let life = opened(&mut window, &at, "a.jsonl").await;
  window.attach(&life).await;
  window.threaded(&life).await;
  window.ran(&life, "x = 1").await;
  assert_eq!(window.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
  time::sleep(PAST).await;
  assert!(!window.lives(&life).await);
  window.ended(&life).await;
  let held = drifted(&life);
  let index = format!("{life}.threads.json");
  let indexed = fs::read(&index).unwrap();
  let mut later = Window::dialed(&local).await;
  cards(&mut later).await;
  let drift = |why: &str| why.starts_with("Drift: ") && why.contains(" drifts: ");
  let verb = Verb::Open {
    life: life.clone(),
    cwd: Some(at.display().to_string()),
    actor: None,
    roster: None,
    skills: None,
    config: None,
    claude: None,
    answers: false,
    attach: Some(0),
  };
  assert_eq!(later.ask(verb).await, Ok(Answer::Done));
  let ended = |frame: &Frame| matches!(frame, Frame::Event { life: one, event: Event::Ended(Some(why)), .. } if *one == life && drift(why));
  later.until(ended).await;
  let said = format!("furb host: the life of {life} ended: ");
  let told = |line: &String| line.strip_prefix(&said).is_some_and(drift);
  let start = Instant::now();
  while !lines.lock().unwrap().iter().any(told) {
    assert!(start.elapsed() < PATIENCE, "{:?}", lines.lock().unwrap());
    time::sleep(SHORT).await;
  }
  assert_eq!(fs::read(&life).unwrap(), held, "a life that drifts keeps nothing in its record");
  // A life that did not open writes no index. The record changed, so the daemon makes the index
  // of the record at rest again from a fold of the record once the life ended: it names the tip
  // of the record and holds the threads that the life before told, and no fold of a part of it.
  later.ended(&life).await;
  let before = threads(&serde_json::from_slice(&indexed).unwrap());
  assert!(!before.1.is_empty(), "the life before told a thread");
  assert_eq!(folded(&index, held.len()).await, before, "a life that did not open writes no index");
  let other = opened(&mut later, &at, "b.jsonl").await;
  later.attach(&other).await;
  later.ran(&other, "y = 1").await;
  assert!(!daemon.is_finished());
}

#[tokio::test]
async fn the_log_of_the_daemon_says_each_end_of_a_life_with_its_fault() {
  let at = place();
  let lines = Arc::new(Mutex::new(Vec::<String>::new()));
  let kept = Arc::clone(&lines);
  let log = Log(Arc::new(move |line: &str| kept.lock().unwrap().push(line.to_owned())));
  let (local, _daemon) = started_with(&at, SHORT, Opening { log: Some(log), ..opening(&at) }).await;
  let mut window = Window::dialed(&local).await;
  let life = opened(&mut window, &at, "a.jsonl").await;
  let said = format!("furb host: the life of {life} ended: with no fault");
  let start = Instant::now();
  while !lines.lock().unwrap().contains(&said) {
    assert!(start.elapsed() < PATIENCE, "{:?}", lines.lock().unwrap());
    time::sleep(SHORT).await;
  }
}

#[tokio::test]
async fn a_handover_keeps_each_life_its_pins_and_each_window() {
  let at = place();
  let (local, old) = started(&at, SHORT).await;
  let mut window = Window::dialed(&local).await;
  let life = opened(&mut window, &at, "a.jsonl").await;
  window.attach(&life).await;
  let pin = Verb::Pin { life: life.clone(), holder: "job".to_owned() };
  assert_eq!(window.ask(pin).await, Ok(Answer::Done));
  window.ran(&life, "x = 41 + 1").await;
  let settings = Settings {
    opening: opening(&at),
    grace: SHORT,
    first: STARTING,
    stall: STALL,
    tailnet: None,
    door: None,
    data: None,
  };
  let next = local.clone();
  let mut new = tokio::spawn(async move { upgrade(&next, settings).await });
  assert!(window.closed().await);
  assert!(matches!(old.await, Ok(Ok(()))));
  let record = Path::new(&life);
  assert_eq!(crossings(record).unwrap(), Vec::<usize>::new(), "a handover writes no fact made");
  assert!(image::agrees(record), "a handover ends the life with its image");
  listening(&local, &mut new).await;
  let mut window = Window::dialed(&local).await;
  window.attach(&life).await;
  let inspect = Verb::Inspect { life: life.clone(), name: "x".to_owned(), on: None };
  let Ok(Answer::Inspected { value, .. }) = window.ask(inspect).await else { panic!() };
  assert_eq!(value, json!(42));
  window.ran(&life, "y = x").await;
  assert_eq!(window.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
  time::sleep(PAST).await;
  assert!(window.lives(&life).await);
  assert!(!new.is_finished());
}

/// The new daemon of a handover, once it listens: the old one ends before the new one binds. A new
/// daemon that ends before it listens fails the test with its error.
async fn listening(local: &Local, new: &mut JoinHandle<Result<(), DaemonError>>) {
  let start = Instant::now();
  while local.dial().await.is_err() {
    assert!(!new.is_finished(), "the new daemon ended before it listened: {:?}", new.await);
    assert!(start.elapsed() < PATIENCE, "the new daemon listens");
    time::sleep(SHORT).await;
  }
}

/// The opening of a test whose model blocks the thread of its life: it tells the test that it
/// began, then waits until the test drops the sender of the gate.
fn blocking(at: &Path, log: Arc<Mutex<Vec<String>>>) -> (Opening, Sender<()>, Receiver<()>) {
  let (hold, gate) = channel::<()>();
  let (began, entered) = channel::<()>();
  let (gate, began) = (Mutex::new(gate), Mutex::new(began));
  let inner = opening(at).answer.expect("the opening of a test has a model");
  let answer = Answered(Arc::new(move |kind, message| {
    began.lock().unwrap().send(()).ok();
    gate.lock().unwrap().recv().ok();
    (inner.0)(kind, message)
  }));
  let tell = Log(Arc::new(move |line: &str| log.lock().unwrap().push(line.to_owned())));
  (Opening { answer: Some(answer), log: Some(tell), ..opening(at) }, hold, entered)
}

#[tokio::test]
async fn a_handover_waits_for_each_life_to_end_and_the_life_goes_on() {
  let at = place();
  let log = Arc::new(Mutex::new(Vec::new()));
  let (blocked, hold, entered) = blocking(&at, Arc::clone(&log));
  let local = Local::new(at.join("sock"));
  let listener = local.listen().await.unwrap();
  let settings = Settings {
    opening: blocked,
    grace: GRACE,
    first: STARTING,
    stall: STALL,
    tailnet: None,
    door: None,
    data: None,
  };
  let served = local.clone();
  let old = tokio::spawn(async move { run(&served, listener, settings).await });
  let mut window = Window::dialed(&local).await;
  let life = opened(&mut window, &at, "a.jsonl").await;
  window.attach(&life).await;
  window.ran(&life, "x = 41 + 1").await;
  let word = "prompt(None, 'go', on=chain('side'))".to_owned();
  let verb = Verb::Run { life: life.clone(), word, on: None };
  assert!(matches!(window.ask(verb).await, Ok(Answer::Act(_))));
  let began = task::spawn_blocking(move || entered.recv_timeout(PATIENCE));
  assert!(began.await.unwrap().is_ok(), "the model of the life blocks its thread");
  let settings = Settings {
    opening: opening(&at),
    grace: GRACE,
    first: STARTING,
    stall: STALL,
    tailnet: None,
    door: None,
    data: None,
  };
  let next = local.clone();
  let mut new = tokio::spawn(async move { upgrade(&next, settings).await });
  time::sleep(PAST).await;
  assert!(!old.is_finished(), "the handover waits for a life that has not ended");
  let seen = log.lock().unwrap().join("\n");
  assert!(
    seen.contains("the handover waits for each life to end") && seen.contains(&life),
    "the log says which lives the handover waits for: {seen}"
  );
  drop(hold);
  let gone = timeout(PATIENCE, old).await.expect("the handover ends once each life ended");
  assert!(matches!(gone, Ok(Ok(()))));
  listening(&local, &mut new).await;
  let mut window = Window::dialed(&local).await;
  window.attach(&life).await;
  window.ran(&life, "y = x").await;
  let inspect = Verb::Inspect { life: life.clone(), name: "y".to_owned(), on: None };
  let Ok(Answer::Inspected { value, .. }) = window.ask(inspect).await else { panic!() };
  assert_eq!(value, json!(42), "the new daemon holds the life whole, and a word runs on it");
  assert!(!new.is_finished());
}

/// A view mounted in the footer by a window, once the skill that serves it attached the life.
async fn mounted(window: &mut Window, verb: &Verb) {
  let start = Instant::now();
  while window.ask(verb.clone()).await != Ok(Answer::Done) {
    assert!(start.elapsed() < PATIENCE, "no skill serves {verb:?}");
    time::sleep(SHORT).await;
  }
}

/// Whether a frame is a tree whose first run says a text.
fn shows(text: &str) -> impl Fn(&Frame) -> bool + '_ {
  move |frame| {
    matches!(frame, Frame::ViewTree(Tree { tree: Node { shape: Shape::Text { runs }, .. }, .. })
      if runs.first().is_some_and(|run| run.text == text))
  }
}

/// The text that the view of a kit shows.
fn said(text: &str) -> String {
  format!(r#"view = {{"id": "todo", "node": "text", "runs": [{{"text": "{text}"}}]}}"#)
}

/// The view of the kit `todo` that a window mounts in the footer of a life.
fn viewing(life: &str, viewer: &str) -> Verb {
  let (viewer, life, skill) = (viewer.to_owned(), life.to_owned(), "todo".to_owned());
  Verb::ViewMount(Mount { viewer, life, slot: Slot::Footer, skill })
}

/// The identity of a test that counts the peers that dialed the daemon.
#[derive(Clone)]
struct Counted {
  local: Local,
  dialed: Arc<AtomicUsize>,
}

impl Identity for Counted {
  type Stream = Accepted;

  async fn identify(&self, stream: &Accepted) -> Result<Who, TransportError> {
    self.dialed.fetch_add(1, Ordering::SeqCst);
    self.local.identify(stream).await
  }
}

/// A daemon on the identity that `identity` makes from its transport, and a window that holds a
/// life of `a.jsonl` whose record holds the kit `todo` of a session, which shows `first`.
async fn kitted<I>(at: &Path, identity: impl FnOnce(Local) -> I) -> (Local, Window, String)
where
  I: Identity<Stream = Accepted> + Clone + Send + Sync + 'static,
{
  fs::create_dir_all(at.join(".furb")).unwrap();
  fs::write(at.join(".furb").join("config.json"), r#"{"skills": {"ouroboros": true}}"#).unwrap();
  let local = Local::new(at.join("sock"));
  let listener = local.listen().await.unwrap();
  let opening = Opening { skills: None, ..opening(at) };
  let settings = Settings {
    opening,
    grace: SHORT,
    first: STARTING,
    stall: STALL,
    tailnet: None,
    door: None,
    data: None,
  };
  let served = identity(local.clone());
  tokio::spawn(async move { run(&served, listener, settings).await });
  let mut window = Window::dialed(&local).await;
  let life = opened(&mut window, at, "a.jsonl").await;
  window.attach(&life).await;
  let word = window.ran(&life, &said("first")).await;
  window.ran(&life, &format!("extend('todo', '{word}', session=True, view='view')")).await;
  (local, window, life)
}

#[tokio::test]
async fn a_kit_of_a_session_gives_its_view_to_each_window_in_each_life_of_its_record() {
  let at = place();
  let (local, mut window, life) = kitted(&at, |local| local).await;
  let todo = vec!["todo".to_owned()];
  window.until(|frame| matches!(frame, Frame::Served { skills, .. } if *skills == todo)).await;
  mounted(&mut window, &viewing(&life, "v1")).await;
  window.until(shows("first")).await;
  window.ran(&life, &said("second")).await;
  window.until(shows("second")).await;
  assert_eq!(window.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
  time::sleep(PAST).await;
  assert!(!window.lives(&life).await);
  let mut later = Window::dialed(&local).await;
  opened(&mut later, &at, "a.jsonl").await;
  later.attach(&life).await;
  mounted(&mut later, &viewing(&life, "v2")).await;
  later.until(shows("second")).await;
  assert!(!at.join(".furb").join("skills").exists(), "the kit writes no file");
}

#[tokio::test]
async fn a_kit_of_a_session_opens_no_connection_to_the_socket_of_its_own_daemon() {
  let at = place();
  let dialed = Arc::new(AtomicUsize::new(0));
  let counted = Arc::clone(&dialed);
  let (_, mut window, life) = kitted(&at, |local| Counted { local, dialed: counted }).await;
  mounted(&mut window, &viewing(&life, "v1")).await;
  window.until(shows("first")).await;
  assert_eq!(dialed.load(Ordering::SeqCst), 1, "only the window dialed the daemon");
}

/// The view of a skill that a window mounts in the footer of a life.
fn mount(life: &str, viewer: &str) -> Verb {
  let (viewer, life, skill) = (viewer.to_owned(), life.to_owned(), "k".to_owned());
  Verb::ViewMount(Mount { viewer, life, slot: Slot::Footer, skill })
}

/// A tree with a button, which only a window that writes may press.
fn tree(viewer: &str) -> Verb {
  let button =
    Node { id: "go".to_owned(), shape: Shape::Button { label: "go".to_owned(), inert: false } };
  let shape = Shape::Box { axis: Axis::Row, gap: 0, border: false, children: vec![button] };
  let tree = Node { id: "root".to_owned(), shape };
  Verb::ViewTree(Tree { viewer: viewer.to_owned(), slot: Slot::Footer, version: 1, tree })
}

/// Whether a frame is the tree of a viewer, and whether its button is inert.
fn inert(frame: &Frame) -> Option<bool> {
  let Frame::ViewTree(Tree { tree: Node { shape: Shape::Box { children, .. }, .. }, .. }) = frame
  else {
    return None;
  };
  let Some(Node { shape: Shape::Button { inert, .. }, .. }) = children.first() else {
    return None;
  };
  Some(*inert)
}

fn pressed(viewer: &str) -> Verb {
  let (viewer, node) = (viewer.to_owned(), "go".to_owned());
  Verb::ViewEvent(ViewEvent { viewer, node, kind: Gesture::Press, value: None })
}

#[tokio::test]
async fn a_skill_serves_a_tree_to_a_writer_and_a_reader_and_hears_the_writer_alone() {
  let at = place();
  let local = started_as(&at, &[Role::Write, Role::Write, Role::Read]).await;
  let mut skill = Window::dialed_as(&local, Kind::Skill, Some("k")).await;
  let (mut writer, mut reader) = (Window::dialed(&local).await, Window::dialed(&local).await);
  let life = opened(&mut writer, &at, "a.jsonl").await;
  for peer in [&mut skill, &mut writer, &mut reader] {
    peer.attach(&life).await;
  }
  for (window, viewer) in [(&mut writer, "w"), (&mut reader, "r")] {
    assert_eq!(window.ask(mount(&life, viewer)).await, Ok(Answer::Done));
    skill.until(|frame| matches!(frame, Frame::ViewMount(one) if one.viewer == viewer)).await;
    assert_eq!(skill.ask(tree(viewer)).await, Ok(Answer::Done));
  }
  assert_eq!(inert(&writer.until(|frame| inert(frame).is_some()).await), Some(false));
  assert_eq!(inert(&reader.until(|frame| inert(frame).is_some()).await), Some(true));
  let notice = Effect::Notice { text: "done".to_owned() };
  let effect = Verb::ViewEffect(ViewEffect { viewer: "w".to_owned(), effect: notice });
  assert_eq!(skill.ask(effect).await, Ok(Answer::Done));
  writer.until(|frame| matches!(frame, Frame::ViewEffect(_))).await;
  assert!(writer.pushed.iter().all(|frame| inert(frame).is_none()));
  let refused = reader.ask(pressed("r")).await;
  assert_eq!(refused, Err(ProtocolError::Role { needs: Role::Write, has: Role::Read }));
  assert_eq!(writer.ask(pressed("w")).await, Ok(Answer::Done));
  let heard = skill.until(|frame| matches!(frame, Frame::ViewEvent(_))).await;
  assert_eq!(
    heard,
    Frame::ViewEvent(ViewEvent {
      viewer: "w".to_owned(),
      node: "go".to_owned(),
      kind: Gesture::Press,
      value: None
    })
  );
}

#[tokio::test]
async fn each_frame_that_a_kit_of_each_version_reads_stands_in_what_the_daemon_tells_it() {
  let at = place();
  let local = started_as(&at, &[Role::Write, Role::Write]).await;
  let mut skill = Window::dialed_as(&local, Kind::Skill, Some("k")).await;
  let mut writer = Window::dialed(&local).await;
  let life = opened(&mut writer, &at, "a.jsonl").await;
  skill.attach(&life).await;
  writer.attach(&life).await;
  writer.ran(&life, "x = 1").await;
  assert_eq!(writer.ask(mount(&life, "w")).await, Ok(Answer::Done));
  let mut told = vec![skill.until(|frame| matches!(frame, Frame::ViewMount(_))).await];
  assert_eq!(skill.ask(tree("w")).await, Ok(Answer::Done));
  writer.until(|frame| matches!(frame, Frame::ViewTree(_))).await;
  assert_eq!(writer.ask(pressed("w")).await, Ok(Answer::Done));
  told.push(skill.until(|frame| matches!(frame, Frame::ViewEvent(_))).await);
  assert_eq!(writer.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
  told.push(skill.until(|frame| matches!(frame, Frame::ViewDispose(_))).await);
  told.extend(skill.pushed.drain(..));
  let told: Vec<Value> = told.iter().map(|frame| serde_json::to_value(frame).unwrap()).collect();
  let kits = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/kits");
  for heard in fs::read_dir(&kits).unwrap().flatten().map(|one| one.path()) {
    if !heard.to_string_lossy().ends_with(".heard.jsonl") {
      continue;
    }
    for line in fs::read_to_string(&heard).unwrap().lines() {
      let read: Value = serde_json::from_str(line).unwrap();
      let stands =
        told.iter().any(|frame| frame["frame"] == read["frame"] && holds("", frame, &read));
      assert!(stands, "{}: no frame holds {line}", heard.display());
    }
  }
}

/// The keys whose text names a variant, which a kit matches on as it stands.
const TAGS: [&str; 4] = ["frame", "event", "slot", "kind"];

/// Whether a value that the daemon told holds each key that a kit reads, under a key, with a value
/// of the same type, and the same text where the key names a variant.
fn holds(key: &str, told: &Value, read: &Value) -> bool {
  match (told, read) {
    (Value::Object(told), Value::Object(read)) => {
      read.iter().all(|(key, one)| told.get(key).is_some_and(|two| holds(key, two, one)))
    }
    (Value::Array(_), Value::Array(_))
    | (Value::Number(_), Value::Number(_))
    | (Value::Bool(_), Value::Bool(_))
    | (Value::Null, Value::Null) => true,
    (Value::String(told), Value::String(read)) => told == read || !TAGS.contains(&key),
    _ => false,
  }
}

#[tokio::test]
async fn a_mount_needs_a_skill_and_a_holder_and_a_view_ends_with_its_peer() {
  let at = place();
  let local = started_as(&at, &[Role::Write, Role::Write]).await;
  let (mut writer, mut skill) =
    (Window::dialed(&local).await, Window::dialed_as(&local, Kind::Skill, Some("k")).await);
  let life = opened(&mut writer, &at, "a.jsonl").await;
  writer.attach(&life).await;
  let noskill = ProtocolError::NoSkill { life: life.clone(), skill: "k".to_owned() };
  assert_eq!(writer.ask(mount(&life, "v")).await, Err(noskill));
  skill.attach(&life).await;
  assert_eq!(writer.ask(mount(&life, "v")).await, Ok(Answer::Done));
  let taken = writer.ask(mount(&life, "v")).await;
  assert_eq!(taken, Err(ProtocolError::Mounted { viewer: "v".to_owned() }));
  let stranger = skill
    .ask(Verb::ViewEvent(ViewEvent {
      viewer: "v".to_owned(),
      node: "go".to_owned(),
      kind: Gesture::Press,
      value: None,
    }))
    .await;
  assert!(matches!(stranger, Err(ProtocolError::Speaker { .. })), "{stranger:?}");
  drop(skill);
  let ended = Frame::ViewDispose(Dispose { viewer: "v".to_owned() });
  assert_eq!(writer.until(|frame| matches!(frame, Frame::ViewDispose(_))).await, ended);
  let gone = writer.ask(pressed("v")).await;
  assert_eq!(gone, Err(ProtocolError::Unmounted { viewer: "v".to_owned() }));
}

#[tokio::test]
async fn a_skill_says_each_act_under_its_name_and_a_window_under_its_operator() {
  let at = place();
  let local = served(&at).await;
  let mut writer = Window::dialed(&local).await;
  let mut skill = Window::dialed_as(&local, Kind::Skill, Some("k")).await;
  let life = opened(&mut writer, &at, "a.jsonl").await;
  writer.attach(&life).await;
  skill.attach(&life).await;
  let to = Some("operator".to_owned());
  let message = "from k".to_owned();
  let asked = Verb::Prompt { life: life.clone(), shape: "None".to_owned(), message, to, on: None };
  let Ok(Answer::Act(asked)) = skill.ask(asked).await else { panic!("no prompt") };
  let said = skill.ran(&life, "x = 1").await;
  let mine = writer.ran(&life, "y = 1").await;
  let mut speakers = Vec::new();
  for id in [&asked, &said, &mine] {
    let get = Verb::Get { life: life.clone(), act: id.clone() };
    let Ok(Answer::Value(fact)) = writer.ask(get).await else { panic!("no act {id}") };
    speakers.push(fact[2].clone());
  }
  let Ok(Answer::Presence(operators)) = writer.ask(Verb::Presence { life: life.clone() }).await
  else {
    panic!("no presence")
  };
  let operator = operators.first().map(|one| one.operator.clone());
  let member = json!(format!("operator/{}", operator.expect("the window is present")));
  assert_eq!(speakers, [json!("skill/k"), json!("skill/k"), member]);
  let y = Verb::Inspect { life: life.clone(), name: "x".to_owned(), on: None };
  assert!(matches!(skill.ask(y).await, Ok(Answer::Inspected { .. })), "the word of a skill ran");
}

#[tokio::test]
async fn a_skill_answers_an_act_that_its_word_took_with_a_close() {
  let at = place();
  let local = served(&at).await;
  let mut writer = Window::dialed(&local).await;
  let mut skill = Window::dialed_as(&local, Kind::Skill, Some("k")).await;
  let life = opened(&mut writer, &at, "a.jsonl").await;
  writer.attach(&life).await;
  skill.attach(&life).await;
  let word = "def taken(id):\n  yield 'started', id\n  yield from idle(id)\nadded = act('k.add', '', controlled(taken), 'milk')";
  writer.ran(&life, word).await;
  let asked = |frame: &Frame| matches!(frame, Frame::Event { event: Event::Fact(fact), .. } if fact[0] == "k.add");
  let Frame::Event { event: Event::Fact(fact), .. } = skill.until(asked).await else {
    panic!("no act")
  };
  assert_eq!(fact[4], json!("milk"), "the skill hears the words of the act");
  let act = fact[1].as_str().expect("the act has a name").to_owned();
  let close = Verb::Close { life: life.clone(), act, value: "\"added milk\"".to_owned() };
  assert_eq!(skill.ask(close).await, Ok(Answer::Done));
  writer.ran(&life, "got = await added").await;
  let got = Verb::Inspect { life: life.clone(), name: "got".to_owned(), on: None };
  let Ok(Answer::Inspected { value, .. }) = writer.ask(got).await else { panic!("no value") };
  assert_eq!(value, json!("added milk"), "the word awaits the value of the close");
}

#[tokio::test]
async fn the_roster_of_a_life_names_the_user_of_the_daemon_as_a_member_of_the_operator() {
  let at = place();
  let (local, _daemon) = started(&at, GRACE).await;
  let mut window = Window::dialed(&local).await;
  let life = opened(&mut window, &at, "a.jsonl").await;
  window.attach(&life).await;
  let Ok(Answer::Presence(operators)) = window.ask(Verb::Presence { life: life.clone() }).await
  else {
    panic!("no presence")
  };
  let operator = operators.first().map(|one| one.operator.clone());
  let operator = operator.expect("the window is present");
  let word = format!(
    "members = [x[1] for x in standing()[0] if x[0] == OPERATOR][0]\nasks = offered(standing()[0], 'operator/{operator}') is not None"
  );
  window.ran(&life, &word).await;
  let read = |name: &str| Verb::Inspect { life: life.clone(), name: name.to_owned(), on: None };
  let Ok(Answer::Inspected { value: members, .. }) = window.ask(read("members")).await else {
    panic!("no members")
  };
  assert_eq!(members, json!([operator]), "the operator of the roster lists the user");
  let Ok(Answer::Inspected { value: asks, .. }) = window.ask(read("asks")).await else {
    panic!("no asks")
  };
  assert_eq!(asks, json!(true), "a prompt can ask that member by name");
}

/// The facts of the stands of a life, which grow with each stand that it asks or hears.
async fn stands(window: &mut Window, life: &str) -> usize {
  let of = Some("stand".to_owned());
  let verb = Verb::Transcript { life: life.to_owned(), on: None, of };
  let Ok(Answer::Values(facts)) = window.ask(verb).await else { panic!("no stands") };
  facts.len()
}

/// The members of the operator in the roster of a life, read again until they are `until`, or
/// as they stand when the patience of the test runs out.
async fn members(window: &mut Window, life: &str, until: &Value) -> Value {
  let word = "members = [x[1] for x in standing()[0] if x[0] == OPERATOR][0]";
  let read = Verb::Inspect { life: life.to_owned(), name: "members".to_owned(), on: None };
  let mut found = Value::Null;
  for _ in 0..100 {
    window.ran(life, word).await;
    let Ok(Answer::Inspected { value, .. }) = window.ask(read.clone()).await else {
      panic!("no members")
    };
    found = value;
    if found == *until {
      break;
    }
    time::sleep(Duration::from_millis(50)).await;
  }
  found
}

#[tokio::test]
async fn the_roster_names_the_access_list_and_a_change_of_it_tells_each_life_to_stand() {
  let at = place();
  let list = |operators: &[&str]| -> Access {
    operators.iter().map(|one| ((*one).to_owned(), Role::Write)).collect()
  };
  let api = LocalApi::Port { port: 1, token: "none".to_owned() };
  let identity = Tailscale::new(api, list(&["ann", "bob"]));
  let tailnet = Some(Tailnet { identity: identity.clone(), ports: Ports::default() });
  let local = Local::new(at.join("sock"));
  let listener = local.listen().await.unwrap();
  let settings = Settings {
    opening: mute(&at),
    grace: GRACE,
    first: STARTING,
    stall: STALL,
    tailnet,
    door: None,
    data: None,
  };
  let served = local.clone();
  let _daemon = tokio::spawn(async move { run(&served, listener, settings).await });
  let mut window = Window::dialed(&local).await;
  let life = opened(&mut window, &at, "a.jsonl").await;
  window.attach(&life).await;
  let Ok(Answer::Presence(operators)) = window.ask(Verb::Presence { life: life.clone() }).await
  else {
    panic!("no presence")
  };
  let operator = operators.first().map(|one| one.operator.clone());
  let operator = operator.expect("the window is present");
  let before = json!([operator, "ann", "bob"]);
  assert_eq!(members(&mut window, &life, &before).await, before, "the list gives the members");
  let stood = stands(&mut window, &life).await;
  identity.replace(list(&["ann", "bob", "cat"]));
  for _ in 0..100 {
    if stands(&mut window, &life).await > stood {
      break;
    }
    time::sleep(Duration::from_millis(50)).await;
  }
  assert!(stands(&mut window, &life).await > stood, "a change of the list tells the life to stand");
  let after = json!([operator, "ann", "bob", "cat"]);
  assert_eq!(
    members(&mut window, &life, &after).await,
    after,
    "the life that lives names the third operator after its stand"
  );
}

#[tokio::test]
async fn a_second_daemon_of_the_machine_refuses_to_start() {
  let at = place();
  let (local, _daemon) = started(&at, GRACE).await;
  let refused = local.listen().await;
  assert!(matches!(refused, Err(TransportError::Taken { .. })));
}

#[tokio::test]
async fn a_window_that_answers_the_models_of_its_life_gives_each_turn() {
  let at = place();
  let (local, _daemon) = started(&at, GRACE).await;
  let mut window = Window::dialed(&local).await;
  let life = at.join("a.jsonl").display().to_string();
  let roster = Some(vec!["claude-cli:sonnet".to_owned()]);
  let cwd = Some(at.display().to_string());
  let open = Verb::Open {
    life: life.clone(),
    cwd,
    actor: None,
    roster,
    skills: None,
    config: None,
    claude: None,
    answers: true,
    attach: None,
  };
  assert_eq!(window.ask(open).await, Ok(Answer::Done));
  window.attach(&life).await;
  let prompt = Verb::Prompt {
    life: life.clone(),
    shape: "str".to_owned(),
    message: "Say ok.".to_owned(),
    to: None,
    on: None,
  };
  let Ok(Answer::Act(id)) = window.ask(prompt).await else { panic!("no prompt") };
  let Frame::Ask { ask, request, .. } = window.until(|one| matches!(one, Frame::Ask { .. })).await
  else {
    panic!("no ask")
  };
  assert!(request["actor"].as_str().is_some_and(|one| one.contains("sonnet")), "{request}");
  let wrote = |ask| Verb::Wrote {
    life: life.clone(),
    ask,
    text: "close(".to_owned(),
    thinking: "So.".to_owned(),
  };
  assert_eq!(window.ask(wrote(ask)).await, Ok(Answer::Done));
  let thought = |one: &Frame| matches!(one, Frame::Event { event: Event::Text { thinking, .. }, .. } if thinking == "So.");
  window.until(thought).await;
  let turn = Some(json!(["assistant", "close('ok')", [500_000, 10, 0, 0, 0.0], null]));
  let answered = Verb::Answered { life: life.clone(), ask, turn, why: None };
  assert_eq!(window.ask(answered.clone()).await, Ok(Answer::Done));
  let Frame::Event { event: Event::Act(act), .. } = window.until(done(&id)).await else {
    panic!("no act")
  };
  assert_eq!(act.value(), Some(&json!("ok")));
  let filled = Verb::Filled { life: life.clone(), act: window.replied(&life).await };
  let share = window.ask(filled).await;
  let half = |one: &Value| one.as_f64().is_some_and(|share| share > 0.0 && share <= 1.0);
  assert!(matches!(&share, Ok(Answer::Value(one)) if half(one)), "{share:?}");
  let again = window.ask(answered).await;
  assert_eq!(again, Err(ProtocolError::Unasked { life: life.clone(), ask }));
  let other = opened(&mut window, &at, "b.jsonl").await;
  let answered = Verb::Answered { life: other.clone(), ask, turn: None, why: None };
  assert_eq!(window.ask(answered).await, Err(ProtocolError::Answerless { life: other }));
}

#[tokio::test]
async fn a_peer_that_opens_with_answers_answers_in_the_place_of_one_that_is_gone() {
  let at = place();
  let (local, _daemon) = started(&at, GRACE).await;
  let life = at.join("a.jsonl").display().to_string();
  let open = Verb::Open {
    life: life.clone(),
    cwd: Some(at.display().to_string()),
    actor: None,
    roster: Some(vec!["claude-cli:sonnet".to_owned()]),
    skills: None,
    config: None,
    claude: None,
    answers: true,
    attach: Some(0),
  };
  let mut first = Window::dialed(&local).await;
  assert_eq!(first.ask(open.clone()).await, Ok(Answer::Done));
  drop(first);
  let mut window = Window::dialed(&local).await;
  let mut taken = window.ask(open.clone()).await;
  while taken.is_err() {
    time::sleep(SHORT).await;
    taken = window.ask(open.clone()).await;
  }
  assert_eq!(taken, Ok(Answer::Done), "the peer that answered is gone");
  let prompt = Verb::Prompt {
    life: life.clone(),
    shape: "str".to_owned(),
    message: "Say ok.".to_owned(),
    to: None,
    on: None,
  };
  let Ok(Answer::Act(id)) = window.ask(prompt).await else { panic!("no prompt") };
  let Frame::Ask { ask, .. } = window.until(|one| matches!(one, Frame::Ask { .. })).await else {
    panic!("the new peer is asked")
  };
  let turn = Some(json!(["assistant", "close('ok')", [10, 10, 0, 0, 0.0], null]));
  let answered = Verb::Answered { life: life.clone(), ask, turn, why: None };
  assert_eq!(window.ask(answered).await, Ok(Answer::Done));
  let Frame::Event { event: Event::Act(act), .. } = window.until(done(&id)).await else {
    panic!("no act")
  };
  assert_eq!(act.value(), Some(&json!("ok")));
}

#[tokio::test]
async fn a_second_writer_cannot_answer_the_request_of_the_peer_that_answers_the_models() {
  let at = place();
  let (local, _daemon) = started(&at, GRACE).await;
  let (mut window, mut other) = (Window::dialed(&local).await, Window::dialed(&local).await);
  let life = at.join("a.jsonl").display().to_string();
  let open = Verb::Open {
    life: life.clone(),
    cwd: Some(at.display().to_string()),
    actor: None,
    roster: Some(vec!["claude-cli:sonnet".to_owned()]),
    skills: None,
    config: None,
    claude: None,
    answers: true,
    attach: None,
  };
  assert_eq!(window.ask(open.clone()).await, Ok(Answer::Done));
  window.attach(&life).await;
  let answering = Err(ProtocolError::Answering { life: life.clone() });
  assert_eq!(other.ask(open).await, answering, "the peer that answers lives");
  other.attach(&life).await;
  let prompt = Verb::Prompt {
    life: life.clone(),
    shape: "str".to_owned(),
    message: "Say ok.".to_owned(),
    to: None,
    on: None,
  };
  let Ok(Answer::Act(id)) = other.ask(prompt).await else { panic!("no prompt") };
  let Frame::Ask { ask, .. } = window.until(|one| matches!(one, Frame::Ask { .. })).await else {
    panic!("no ask")
  };
  let foreign = Err(ProtocolError::Foreign { life: life.clone(), ask });
  let wrote = Verb::Wrote {
    life: life.clone(),
    ask,
    text: "close('taken')".to_owned(),
    thinking: String::new(),
  };
  assert_eq!(other.ask(wrote).await, foreign);
  let turn = Some(json!(["assistant", "close('taken')", [10, 10, 0, 0, 0.0], null]));
  let taken = Verb::Answered { life: life.clone(), ask, turn, why: None };
  assert_eq!(other.ask(taken).await, foreign);
  let turn = Some(json!(["assistant", "close('ok')", [10, 10, 0, 0, 0.0], null]));
  let answered = Verb::Answered { life: life.clone(), ask, turn, why: None };
  assert_eq!(window.ask(answered).await, Ok(Answer::Done));
  let Frame::Event { event: Event::Act(act), .. } = window.until(done(&id)).await else {
    panic!("no act")
  };
  assert_eq!(act.value(), Some(&json!("ok")));
}

#[tokio::test]
async fn a_command_that_a_window_feeds_reads_each_line_that_it_writes() {
  let at = place();
  let (local, _daemon) = started(&at, SHORT).await;
  let mut window = Window::dialed(&local).await;
  let life = opened(&mut window, &at, "a.jsonl").await;
  window.attach(&life).await;
  let run = |door: Door| Verb::Run { life: life.clone(), word: door.word(), on: None };
  let text = |one: &str| one.to_owned();
  let given = Door::Bash { command: text("cat"), stdin: Some(text("given\n")) };
  let Ok(Answer::Act(rung)) = window.ask(run(given)).await else { panic!("no command") };
  let given = bashed(&mut window, &rung).await;
  let Frame::Event { event: Event::Act(act), .. } = window.until(done(&given)).await else {
    panic!("no act")
  };
  assert!(act.value().is_some_and(|exit| exit.to_string().contains("given")), "{act:?}");
  let fed = Door::Bash { command: text("cat"), stdin: None };
  let Ok(Answer::Act(rung)) = window.ask(run(fed)).await else { panic!("no command") };
  let fed = bashed(&mut window, &rung).await;
  for content in ["fed\n", ""] {
    let write = Door::Write { path: format!("{fed}/stdin"), content: text(content) };
    assert!(matches!(window.ask(run(write)).await, Ok(Answer::Act(_))));
  }
  let Frame::Event { event: Event::Act(act), .. } = window.until(done(&fed)).await else {
    panic!("no act")
  };
  assert!(act.value().is_some_and(|exit| exit.to_string().contains("fed")), "{act:?}");
  window.left(&life).await;
}

/// The bound of a stall in a test: a gap between two facts of a word that waits 0.05 s in a loop
/// stays far below it on a loaded machine.
const QUIET: Duration = Duration::from_millis(1500);

/// A daemon of a test whose watch tells a stall after the bound given.
async fn watched(at: &Path, opening: Opening, stall: Duration) -> Local {
  let local = Local::new(at.join("sock"));
  let listener = local.listen().await.unwrap();
  let settings = Settings {
    opening,
    grace: SHORT,
    first: STARTING,
    stall,
    tailnet: None,
    door: None,
    data: None,
  };
  let served = local.clone();
  tokio::spawn(async move { run(&served, listener, settings).await });
  local
}

/// Whether a frame tells that a life stalled.
fn stall(frame: &Frame) -> bool {
  matches!(frame, Frame::Stalled { .. })
}

/// Whether a frame is the fact of a prompt of None, the message of a stall to the root.
fn message(frame: &Frame) -> bool {
  matches!(frame, Frame::Event { event: Event::Fact(fact), .. }
    if fact[0] == "prompt" && fact[3] == "chain1" && fact[4] == "None")
}

impl Window {
  /// Each frame that the daemon pushed, up to the first that the test waits for.
  async fn heard_until(&mut self, wanted: impl Fn(&Frame) -> bool) -> Vec<Frame> {
    let mut heard: Vec<Frame> = self.pushed.drain(..).collect();
    while !heard.last().is_some_and(&wanted) {
      heard.push(self.read().await.expect("the daemon pushes the frame"));
    }
    heard
  }

  /// Each frame that the daemon pushed for a while.
  async fn during(&mut self, span: Duration) -> Vec<Frame> {
    let mut heard: Vec<Frame> = self.pushed.drain(..).collect();
    let end = time::Instant::now() + span;
    while let Ok(Some(line)) = time::timeout_at(end, self.lines.next()).await {
      heard.push(serde_json::from_str(&line.unwrap()).unwrap());
    }
    heard
  }
}

#[tokio::test]
async fn a_life_that_holds_an_open_act_and_says_no_fact_is_told_once_to_its_windows_and_root() {
  let at = place();
  let local = watched(&at, opening(&at), QUIET).await;
  let mut window = Window::dialed(&local).await;
  let life = opened(&mut window, &at, "a.jsonl").await;
  window.attach(&life).await;
  let flows = "for _ in range(60):\n  await wait(0.05, __name__)";
  let verb = Verb::Run { life: life.clone(), word: flows.to_owned(), on: None };
  let Ok(Answer::Act(rung)) = window.ask(verb).await else { panic!("no rung") };
  let heard = window.heard_until(done(&rung)).await;
  assert!(!heard.iter().any(stall), "a run whose facts flow for twice the bound never stalls");
  let asked = r#"await bash("sleep 20", timeout=60)"#;
  let verb = Verb::Run { life: life.clone(), word: asked.to_owned(), on: None };
  let began = Instant::now();
  let Ok(Answer::Act(_)) = window.ask(verb).await else { panic!("no rung") };
  let Frame::Stalled { acts, notice, .. } = window.until(stall).await else { unreachable!() };
  assert!(began.elapsed() >= QUIET, "the daemon tells a stall after its bound alone");
  assert!(acts.iter().any(|one| one.kind == "run" && one.chain == "chain1"), "{acts:?}");
  assert!(notice.contains("1.5 seconds") && notice.contains(" on chain1"), "{notice}");
  let Frame::Event { event: Event::Fact(sent), .. } = window.until(message).await else {
    unreachable!()
  };
  assert_eq!(sent[5], notice.as_str(), "the root takes the notice as its message");
  let heard = window.during(QUIET * 3).await;
  assert!(!heard.iter().any(stall), "the watch tells one stall once");
  assert!(!heard.iter().any(message), "the watch sends one message for one stall");
  let verb = Verb::Run { life: life.clone(), word: "x = 1".to_owned(), on: None };
  let Ok(Answer::Act(rung)) = window.ask(verb).await else { panic!("no rung") };
  let heard = window.heard_until(stall).await;
  assert!(heard.iter().any(done(&rung)), "a new stall comes after a new fact");
  window.until(message).await;
  let prompts = Verb::Transcript { life: life.clone(), on: None, of: Some("prompt".to_owned()) };
  let Ok(Answer::Values(prompts)) = window.ask(prompts).await else { panic!("no prompts") };
  let messages = prompts.iter().filter(|one| one[4] == "None").count();
  assert_eq!(messages, 2, "a new fact and a new stall send a new message: {prompts:?}");
  window.left(&life).await;
}

/// The rung of a word that a window ran on the root, as the daemon answers it once its life folds
/// it done.
async fn finished(window: &mut Window, life: &str, word: &str) -> view::Act {
  let verb = Verb::Run { life: life.to_owned(), word: word.to_owned(), on: None };
  let Ok(Answer::Act(rung)) = window.ask(verb).await else { panic!("no rung") };
  awaited(window, life, &rung).await
}

/// An act of a life as the daemon answers the verb `ended` of it.
async fn awaited(window: &mut Window, life: &str, act: &str) -> view::Act {
  let verb = Verb::Ended { life: life.to_owned(), act: act.to_owned() };
  let answer = timeout(PATIENCE, window.ask(verb)).await.expect("the end comes");
  let Ok(Answer::Acts(mut acts)) = answer else { panic!("no act: {answer:?}") };
  assert_eq!(acts.len(), 1, "{acts:?}");
  acts.remove(0)
}

#[tokio::test]
async fn the_end_of_a_word_that_ran_comes_as_its_life_folds_it() {
  let at = place();
  let (local, _daemon) = started(&at, PATIENCE).await;
  let mut window = Window::dialed(&local).await;
  let life = opened(&mut window, &at, "a.jsonl").await;
  let ran = finished(&mut window, &life, "x = 41 + 1").await;
  assert_eq!((ran.word.as_deref(), ran.run), (Some("x = 41 + 1"), Some(view::RunState::Done)));
  let raised = finished(&mut window, &life, "raise ValueError('no forty-two')").await;
  let failed = view::RunState::Failed("ValueError: no forty-two".to_owned());
  assert_eq!(raised.run, Some(failed));
  let waited = finished(&mut window, &life, "await wait(0.5, __name__)").await;
  assert_eq!(waited.run, Some(view::RunState::Done), "the end comes when the word ends");
  let refused = window.ask(Verb::Ended { life, act: "rung999".to_owned() }).await;
  assert!(matches!(refused, Err(ProtocolError::Refused { .. })), "{refused:?}");
}

#[tokio::test]
async fn a_window_that_dials_again_hears_the_end_of_a_word_that_ended_while_it_was_away() {
  let at = place();
  let (local, _daemon) = started(&at, PATIENCE).await;
  let mut window = Window::dialed(&local).await;
  let life = opened(&mut window, &at, "a.jsonl").await;
  let word = "await wait(1, __name__)\nx = 1";
  let verb = Verb::Run { life: life.clone(), word: word.to_owned(), on: None };
  let Ok(Answer::Act(rung)) = window.ask(verb).await else { panic!("no rung") };
  let asked = Verb::Ended { life: life.clone(), act: rung.clone() };
  let request = serde_json::to_string(&Request { id: 99, verb: asked }).unwrap();
  window.lines.send(request).await.unwrap();
  drop(window);
  time::sleep(PAST).await;
  let mut again = Window::dialed(&local).await;
  let ran = awaited(&mut again, &life, &rung).await;
  assert!(matches!(ran.phase, Phase::Done(_)), "{ran:?}");
  assert_eq!((ran.word.as_deref(), ran.run), (Some(word), Some(view::RunState::Done)));
}

#[tokio::test]
async fn a_reopen_tells_no_stall_of_an_act_that_the_life_before_its_crossing_left_open() {
  let at = place();
  let local = watched(&at, opening(&at), QUIET).await;
  let mut window = Window::dialed(&local).await;
  let life = opened(&mut window, &at, "a.jsonl").await;
  window.attach(&life).await;
  window.threaded(&life).await;
  let waits = "await wait(3600, __name__)".to_owned();
  let verb = Verb::Run { life: life.clone(), word: waits, on: None };
  let Ok(Answer::Act(rung)) = window.ask(verb).await else { panic!("no rung") };
  assert_eq!(window.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
  window.ended(&life).await;
  // The life ended with its image, which holds the word that waits. A crossing at the tip of the
  // record makes the next life open by the record, across that crossing.
  crossed(Opening { record: Some(life.clone()), ..opening(&at) }).await;
  opened(&mut window, &at, "a.jsonl").await;
  window.attach(&life).await;
  let stalls: Vec<Frame> = window.during(QUIET * 3).await.into_iter().filter(stall).collect();
  assert!(stalls.is_empty(), "{rung} and its run are acts of the life before: {stalls:?}");
}

#[tokio::test]
async fn a_rung_that_waits_with_no_reply_is_told_to_its_chain_after_the_bound() {
  let at = place();
  let local = watched(&at, opening(&at), QUIET).await;
  let mut window = Window::dialed(&local).await;
  let life = opened(&mut window, &at, "a.jsonl").await;
  window.attach(&life).await;
  let waits = "side = chain(\"side\")\nlong = bash(\"for i in $(seq 100); do echo $i; sleep 0.1; done\", \
               on=side)\nrung(\"x = 1\", long)";
  let verb = Verb::Run { life: life.clone(), word: waits.to_owned(), on: None };
  let began = Instant::now();
  let Ok(Answer::Act(_)) = window.ask(verb).await else { panic!("no rung") };
  // A command that writes nothing for the bound, as one that a loaded machine starts late, also
  // stalls the life, with a notice that names the command alone, beside the stall of the chain.
  let rung = |frame: &Frame| matches!(frame, Frame::Stalled { acts, .. } if acts.iter().any(|one| one.kind == "rung" && one.chain == "chain1"));
  let Frame::Stalled { acts, notice, .. } = window.until(rung).await else { unreachable!() };
  assert!(began.elapsed() >= QUIET, "the daemon tells a stall after its bound alone");
  assert!(!acts.iter().any(|one| one.kind == "run"), "no run is open: {acts:?}");
  let told = |frame: &Frame| {
    message(frame)
      && matches!(frame, Frame::Event { event: Event::Fact(sent), .. } if sent[5] == notice.as_str())
  };
  window.until(told).await;
  window.left(&life).await;
}

#[tokio::test]
async fn a_chain_stuck_beside_a_chain_that_works_is_told_alone_and_takes_the_notice() {
  let at = place();
  let local = watched(&at, opening(&at), QUIET).await;
  let mut window = Window::dialed(&local).await;
  let life = opened(&mut window, &at, "a.jsonl").await;
  window.attach(&life).await;
  let word = "long = bash(\"for i in $(seq 100); do echo $i; sleep 0.1; done\")\nside = chain(\"side\")\n\
              rung(\"x = 1\", long, on=side)\nfor _ in range(80):\n  await wait(0.05, __name__)";
  let verb = Verb::Run { life: life.clone(), word: word.to_owned(), on: None };
  let Ok(Answer::Act(rung)) = window.ask(verb).await else { panic!("no rung") };
  let heard = window.heard_until(stall).await;
  assert!(!heard.iter().any(done(&rung)), "the stall comes while the root works");
  let Some(Frame::Stalled { acts, notice, .. }) = heard.last() else { unreachable!() };
  let side = acts.first().map(|one| one.chain.clone()).expect("the stall names an act");
  assert!(side != "chain1" && acts.iter().all(|one| one.chain == side), "{acts:?}");
  assert!(acts.iter().any(|one| one.kind == "rung"), "{acts:?}");
  let taken = |frame: &Frame| {
    matches!(frame, Frame::Event { event: Event::Fact(fact), .. }
      if fact[0] == "prompt" && fact[3] == side.as_str() && fact[4] == "None")
  };
  let Frame::Event { event: Event::Fact(sent), .. } = window.until(taken).await else {
    unreachable!()
  };
  assert_eq!(sent[5], notice.as_str(), "the stuck chain takes the notice as its message");
  window.left(&life).await;
}

#[tokio::test]
async fn work_that_rests_or_waits_for_the_operator_never_stalls_and_stalls_once_he_answered() {
  let at = place();
  let local = watched(&at, opening(&at), QUIET).await;
  let mut window = Window::dialed(&local).await;
  let life = opened(&mut window, &at, "a.jsonl").await;
  window.attach(&life).await;
  let rests = window.during(QUIET * 2).await;
  assert!(!rests.iter().any(stall), "a life that rests tells no stall");
  // Two rungs and the run of the root await a prompt to the operator, one of the rungs is paused,
  // and the root then waits on a command. No act runs while he does not answer: the first command
  // of a process waits for the PATH of the login shell of the user with no fact, which a loaded
  // machine gives in more than the bound.
  let word = "q = prompt(str, \"q\", to=OPERATOR)\n\
              stopped = rung(\"x = 2\", q, on=chain(\"stopped\"))\npause(stopped)\n\
              rung(\"x = 1\", q, on=chain(\"held\"))\nawait q\nawait bash(\"sleep 20\", timeout=60)";
  let verb = Verb::Run { life: life.clone(), word: word.to_owned(), on: None };
  let Ok(Answer::Act(_)) = window.ask(verb).await else { panic!("no rung") };
  let asked = |frame: &Frame| matches!(frame, Frame::Event { event: Event::Fact(fact), .. } if fact[0] == "prompt" && fact[5] == "q");
  let mut heard = window.heard_until(asked).await;
  let Some(Frame::Event { event: Event::Fact(q), .. }) = heard.last().cloned() else {
    unreachable!()
  };
  let waits = window.during(QUIET * 2).await;
  assert!(!waits.iter().any(stall), "work that waits for the operator never stalls: {waits:?}");
  heard.extend(waits);
  let stopped = heard.iter().find_map(|frame| match frame {
    Frame::Event { event: Event::Fact(fact), .. } if fact[0] == "rung" && fact[4] == "x = 2" => {
      fact[3].as_str().map(str::to_owned)
    }
    _ => None,
  });
  let stopped = stopped.expect("the paused rung is told");
  let act = q[1].as_str().unwrap().to_owned();
  let said = Verb::Answer { life: life.clone(), act, line: "a".to_owned() };
  assert_eq!(window.ask(said).await, Ok(Answer::Done));
  let answered = Instant::now();
  let Frame::Stalled { acts, .. } = window.until(stall).await else { unreachable!() };
  assert!(answered.elapsed() >= QUIET, "the daemon tells a stall after its bound alone");
  assert!(acts.iter().any(|one| one.kind == "run" && one.chain == "chain1"), "{acts:?}");
  assert!(!acts.iter().any(|one| one.chain == stopped), "a paused rung waits on purpose: {acts:?}");
  window.left(&life).await;
}

/// The bound of a stall in the test of a command that keeps writing: far above any pause between
/// two lines of a shell on a loaded machine.
const SLOW: Duration = Duration::from_secs(4);

#[tokio::test]
async fn a_command_that_keeps_writing_for_longer_than_the_bound_never_stalls() {
  let at = place();
  let local = watched(&at, opening(&at), SLOW).await;
  let mut window = Window::dialed(&local).await;
  let life = opened(&mut window, &at, "a.jsonl").await;
  window.attach(&life).await;
  let ran = |word: &str| Verb::Run { life: life.clone(), word: word.to_owned(), on: None };
  let ended = |id: &'static str| move |frame: &Frame| matches!(frame, Frame::Event { event: Event::Fact(fact), .. } if fact[0] == "done" && fact[1] == id);
  // The first command of a process waits for the PATH of the login shell of the user, which a
  // loaded machine gives in seconds, with no fact: the command that writes runs after it.
  let Ok(Answer::Act(_)) = window.ask(ran("await bash(\"true\")")).await else { panic!("no rung") };
  window.heard_until(ended("bash1")).await;
  // The command writes a line each tenth of a second for more than the bound.
  let word = "await bash(\"for i in $(seq 50); do echo $i; sleep 0.1; done\", timeout=60)";
  let Ok(Answer::Act(_)) = window.ask(ran(word)).await else { panic!("no rung") };
  let began = Instant::now();
  let heard = window.heard_until(ended("bash2")).await;
  assert!(began.elapsed() >= SLOW, "the command wrote for longer than the bound");
  let stalls: Vec<&Frame> = heard.iter().filter(|frame| stall(frame)).collect();
  assert!(stalls.is_empty(), "a command that keeps writing never stalls: {stalls:?}");
  window.left(&life).await;
}

#[tokio::test]
async fn a_reply_that_writes_nothing_for_the_bound_stalls_its_chain_and_what_it_writes_is_progress()
{
  let at = place();
  // The model of the chain `side` writes a word each tenth of a second for twice the bound, then
  // nothing; the root closes each prompt at once.
  let wrote = Arc::new(Mutex::new(Instant::now()));
  let (last, root) = (Arc::clone(&wrote), opening(&at).answer.expect("a model"));
  let answer = Answered(Arc::new(move |request: Value, told| {
    if request["chain"] == "chain1" {
      return (root.0)(request, told);
    }
    let last = Arc::clone(&last);
    Box::pin(async move {
      for _ in 0..30 {
        told("a", "");
        *last.lock().unwrap() = Instant::now();
        time::sleep(QUIET / 15).await;
      }
      future::pending().await
    })
  }));
  let local = watched(&at, Opening { answer: Some(answer), ..opening(&at) }, QUIET).await;
  let mut window = Window::dialed(&local).await;
  let life = opened(&mut window, &at, "a.jsonl").await;
  window.attach(&life).await;
  let word = "prompt(None, 'go', on=chain('side'))".to_owned();
  let Ok(Answer::Act(_)) = window.ask(Verb::Run { life: life.clone(), word, on: None }).await
  else {
    panic!("no rung")
  };
  let began = Instant::now();
  let Frame::Stalled { acts, .. } = window.until(stall).await else { unreachable!() };
  assert!(began.elapsed() >= QUIET * 2, "what a model writes is progress");
  assert!(
    wrote.lock().unwrap().elapsed() >= QUIET,
    "the reply stalls after the bound with no word"
  );
  let side = acts.iter().find(|one| one.kind == "reply").map(|one| one.chain.clone());
  let side = side.expect("the stall names the reply");
  assert!(side != "chain1" && acts.iter().all(|one| one.chain == side), "{acts:?}");
  let taken = |frame: &Frame| {
    matches!(frame, Frame::Event { event: Event::Fact(fact), .. }
      if fact[0] == "prompt" && fact[3] == side.as_str() && fact[4] == "None")
  };
  let heard = window.heard_until(taken).await;
  let Some(Frame::Event { event: Event::Fact(sent), .. }) = heard.last() else { unreachable!() };
  let chain =
    |frame: &Frame| matches!(frame, Frame::Stalled { notice, .. } if sent[5] == notice.as_str());
  assert!(heard.iter().any(chain), "the chain of the reply takes the notice of its own stall");
  window.left(&life).await;
}

/// A daemon of a test whose lives end after a short grace.
async fn served(at: &Path) -> Local {
  let local = Local::new(at.join("sock"));
  let listener = local.listen().await.unwrap();
  let settings = Settings {
    opening: opening(at),
    grace: SHORT,
    first: STARTING,
    stall: STALL,
    tailnet: None,
    door: None,
    data: None,
  };
  let served = local.clone();
  tokio::spawn(async move { run(&served, listener, settings).await });
  local
}

/// The port of the browser callback of a sign-in, which is one for the machine, held by one test of
/// any run of the tests on the machine at a time: each run of the other trees takes the same lock
/// of a file, so no other test binds the port between the claim of a test and the bind of its
/// sign-in.
async fn port() -> File {
  let lock = File::create(env::temp_dir().join("furb-sign-in-port.lock")).unwrap();
  let start = Instant::now();
  while lock.try_lock().is_err() {
    assert!(start.elapsed() < PATIENCE, "another run of the tests holds the port of a sign-in");
    time::sleep(SHORT).await;
  }
  lock
}

/// A folder of configs whose credential of codex ended, so that no account of this machine serves
/// and no request goes out.
fn signed_out(at: &Path) {
  let folder = at.join("config").join("auth");
  fs::create_dir_all(&folder).unwrap();
  let ended = r#"{"access":"","refresh":null,"expires":0,"account":"none"}"#;
  fs::write(folder.join("codex.json"), ended).unwrap();
}

/// The request that turns codex on, said with no wait for its answer, and its id.
async fn signing(window: &mut Window) -> u64 {
  let id = window.next;
  window.next += 1;
  let key = ["providers", "codex"].map(str::to_owned).to_vec();
  let config = Config::Set { key, value: Some(json!(true)), layer: None };
  let verb = Verb::Config { life: window.life.clone(), config };
  window.lines.send(serde_json::to_string(&Request { id, verb }).unwrap()).await.unwrap();
  id
}

/// The port of the browser callback of a sign-in, once no process holds it: another run of the
/// tests on this machine may hold it for a while, and a sign-in that finds it busy fails.
async fn claimed() -> TcpListener {
  let start = Instant::now();
  loop {
    match TcpListener::bind(("127.0.0.1", 1455)) {
      Ok(port) => return port,
      Err(no) => assert!(start.elapsed() < PATIENCE, "no process lets go of the port: {no}"),
    }
    time::sleep(SHORT).await;
  }
}

fn signed_in(state: &'static str) -> impl Fn(&Frame) -> bool {
  move |frame| matches!(frame, Frame::SignIn { state: got, .. } if got == state)
}

#[tokio::test]
async fn a_sign_in_of_codex_that_fails_is_pushed_to_the_window_and_refuses_its_config() {
  let _port = port().await;
  let at = place();
  signed_out(&at);
  let (local, _daemon) = started(&at, GRACE).await;
  let mut window = Window::dialed(&local).await;
  window.life = opened(&mut window, &at, "a.jsonl").await;
  let life = window.life.clone();
  window.attach(&life).await;
  let taken = claimed().await;
  let id = signing(&mut window).await;
  let Frame::SignIn { url, message, .. } = window.until(signed_in("failed")).await else {
    unreachable!()
  };
  assert_eq!(url, None);
  assert!(message.is_some_and(|why| !why.is_empty()), "the frame tells why it failed");
  let answer =
    window.until(|frame| matches!(frame, Frame::Refused { id: Some(got), .. } if *got == id));
  assert!(matches!(answer.await, Frame::Refused { error: ProtocolError::Refused { .. }, .. }));
  drop(taken);
}

#[tokio::test]
async fn a_sign_in_of_codex_that_the_operator_cancels_is_pushed_and_refuses_its_config() {
  let _port = port().await;
  let at = place();
  signed_out(&at);
  let (local, _daemon) = started(&at, GRACE).await;
  let mut window = Window::dialed(&local).await;
  window.life = opened(&mut window, &at, "a.jsonl").await;
  let life = window.life.clone();
  window.attach(&life).await;
  drop(claimed().await);
  let id = signing(&mut window).await;
  let Frame::SignIn { url, .. } = window.until(signed_in("waiting")).await else { unreachable!() };
  assert!(url.is_some_and(|one| one.starts_with("https://")), "the frame holds the address");
  let cancel = Verb::CancelSignIn;
  assert_eq!(window.ask(cancel).await, Ok(Answer::Done));
  window.until(signed_in("canceled")).await;
  let answer =
    window.until(|frame| matches!(frame, Frame::Refused { id: Some(got), .. } if *got == id));
  assert!(matches!(answer.await, Frame::Refused { error: ProtocolError::Refused { .. }, .. }));
}

/// A folder of configs whose credential of codex serves and whose list of the models of its
/// account is new, so a sign-in connects at once and no request goes out.
fn signed_in_already(at: &Path) {
  let folder = at.join("config").join("auth");
  fs::create_dir_all(&folder).unwrap();
  // The account of a credential is a claim of its access token, which is no signed token here.
  let claims = r#"{"https://api.openai.com/auth":{"chatgpt_account_id":"acct"}}"#;
  let access = format!("e30.{}.sig", URL_SAFE_NO_PAD.encode(claims));
  let far = u64::MAX / 2;
  let held = json!({ "access": access, "refresh": "r", "expires": far, "account": "acct" });
  fs::write(folder.join("codex.json"), held.to_string()).unwrap();
  let now = UNIX_EPOCH.elapsed().unwrap().as_millis();
  let model = json!({ "slug": "gpt-test", "context_window": 200_000, "visibility": "list" });
  let listed = json!({ "account": "acct", "fetched": now, "models": [model] });
  fs::write(folder.join("codex.models.json"), listed.to_string()).unwrap();
}

#[tokio::test]
async fn a_sign_in_of_the_machine_turns_codex_on_for_its_user_and_each_project_offers_its_models() {
  let at = place();
  signed_in_already(&at);
  let (local, _daemon) = started(&at, GRACE).await;
  let mut window = Window::dialed(&local).await;
  let models = Verb::Models { cwd: None, config: None, claude: None };
  let codex = |answer: Result<Answer, ProtocolError>| match answer {
    Ok(Answer::Models(models)) => models.iter().any(|one| one.name == "codex:gpt-test"),
    other => panic!("no models: {other:?}"),
  };
  assert!(!codex(window.ask(models.clone()).await), "codex is off before the sign-in");
  let sign_in = Verb::SignIn { cwd: None, device: false };
  assert!(codex(window.ask(sign_in).await), "the answer gives the models of the account");
  window.until(signed_in("connected")).await;
  let user = fs::read_to_string(at.join("config").join("config.json")).unwrap();
  let user: Value = serde_json::from_str(&user).unwrap();
  assert_eq!(user["providers"]["codex"], json!(true), "the user config turns codex on");
  assert!(codex(window.ask(models).await), "the catalog offers the models of the account");
}

/// The welcome of a daemon to a new window: the name of its machine and its build.
async fn welcome(local: &Local) -> (String, String) {
  let mut lines = Framed::new(local.dial().await.unwrap(), LinesCodec::new());
  let hello = Hello { protocol: PROTOCOL, kind: Kind::Window, name: None, role: None };
  let request = Request { id: 0, verb: Verb::Hello(hello) };
  lines.send(serde_json::to_string(&request).unwrap()).await.unwrap();
  let line = timeout(PATIENCE, lines.next()).await.unwrap().unwrap().unwrap();
  match serde_json::from_str(&line).unwrap() {
    Frame::Reply { answer: Answer::Welcome { machine, build, .. }, .. } => (machine, build),
    other => panic!("no welcome: {other:?}"),
  }
}

#[tokio::test]
async fn the_welcome_names_the_machine_and_its_build() {
  let at = place();
  let (local, _daemon) = started(&at, GRACE).await;
  let named = (furb_host::machine(), furb_host::BUILD.to_owned());
  assert_eq!(welcome(&local).await, named);
}

/// One project in a folder, which the list of the daemon names, and the folder of its sessions.
fn project(at: &Path) -> (String, PathBuf) {
  let main = at.join("main");
  let sessions = main.join(".furb").join("sessions");
  fs::create_dir_all(&sessions).unwrap();
  let list = json!({ "workspaces": [{ "directory": main.display().to_string() }] });
  fs::create_dir_all(at.join("config")).unwrap();
  fs::write(at.join("config").join("workspaces.json"), list.to_string()).unwrap();
  (main.display().to_string(), sessions)
}

/// A life opened on a new record of a folder and attached by its open: a life with no holder
/// ends at once under a grace of none, so an attach after the open can meet a life that ended.
async fn held(window: &mut Window, at: &Path, name: &str) -> String {
  open(window, at, name, Some(0)).await
}

/// The cards of the machine, which the window hears from then on.
async fn cards(window: &mut Window) -> Vec<Card> {
  match window.ask(Verb::Cards).await {
    Ok(Answer::Cards(cards)) => cards,
    other => panic!("no cards: {other:?}"),
  }
}

/// The first card that a frame of the cards tells and that the test waits for.
async fn card(window: &mut Window, wanted: impl Fn(&Card) -> bool) -> Card {
  let told =
    |frame: &Frame| matches!(frame, Frame::Cards { changed, .. } if changed.iter().any(&wanted));
  match window.until(told).await {
    Frame::Cards { changed, .. } => changed.into_iter().find(wanted).unwrap(),
    other => panic!("no cards: {other:?}"),
  }
}

/// A message of the operator sent to the root of a life: its prompt.
async fn messaged(window: &mut Window, life: &str, message: &str) -> String {
  let (shape, message) = ("None".to_owned(), message.to_owned());
  let verb = Verb::Prompt { life: life.to_owned(), shape, message, to: None, on: None };
  match window.ask(verb).await {
    Ok(Answer::Act(prompt)) => prompt,
    other => panic!("no prompt: {other:?}"),
  }
}

#[tokio::test]
async fn the_cards_tell_each_thread_of_the_operator_as_its_life_changes_it() {
  let at = place();
  let (main, sessions) = project(&at);
  let (local, _daemon) = started(&at, GRACE).await;
  let mut window = Window::dialed(&local).await;
  assert_eq!(cards(&mut window).await, Vec::new());
  let life = held(&mut window, &sessions, "one.jsonl").await;
  let prompt = messaged(&mut window, &life, "ground the branch\nand say what moved").await;
  let resolved = |card: &Card| card.thread.standing == Standing::Resolved;
  let told = card(&mut window, resolved).await;
  let operator = told.viewers.first().cloned().expect("the window holds the life");
  let thread = view::Card {
    prompt: prompt.clone(),
    title: "ground the branch".to_owned(),
    standing: Standing::Resolved,
    model: Some("claude-cli:opus".to_owned()),
    agents: 0,
    readers: Vec::new(),
  };
  let wanted = Card {
    life: life.clone(),
    session: None,
    project: Some(main),
    refused: None,
    held: true,
    viewers: vec![operator.clone()],
    pins: Vec::new(),
    since: told.since,
    thread,
  };
  assert_eq!(told, wanted);
  assert!(told.thread.unread(&operator), "he did not read the end of the thread");
  assert_eq!(told.place(&operator).0, Part::Resting);
  // What he read is told as a change of the card, and a later peer gets the card as it stands.
  let seen = Verb::Seen { life: life.clone(), about: prompt };
  assert_eq!(window.ask(seen).await, Ok(Answer::Done));
  // The window kept each card that came before the resolved one, as one of the thread while it
  // worked, which a member who wrote its last message reads already: the wait is for the end read.
  let read = card(&mut window, |card| resolved(card) && card.thread.read_by(&operator)).await;
  assert_eq!(read.place(&operator).0, Part::Resolved);
  let mut late = Window::dialed(&local).await;
  assert_eq!(cards(&mut late).await, vec![read]);
}

#[tokio::test]
async fn the_history_gives_what_the_window_sent_in_the_project_its_record_first_and_newest_first() {
  let at = place();
  let (_, sessions) = project(&at);
  let (local, _daemon) = started(&at, SHORT).await;
  let mut window = Window::dialed(&local).await;
  cards(&mut window).await;
  let told = |life: &str, title: &str| {
    let (life, title) = (life.to_owned(), title.to_owned());
    move |card: &Card| card.life == life && card.thread.title == title
  };
  let one = held(&mut window, &sessions, "one.jsonl").await;
  messaged(&mut window, &one, "say what moved").await;
  messaged(&mut window, &one, "fix the build").await;
  card(&mut window, told(&one, "fix the build")).await;
  // The first life ends, so what was sent to it comes from the index beside its record.
  assert_eq!(window.ask(Verb::Detach { life: one.clone() }).await, Ok(Answer::Done));
  time::sleep(PAST).await;
  assert!(!window.lives(&one).await);
  let two = held(&mut window, &sessions, "two.jsonl").await;
  messaged(&mut window, &two, "fix the build").await;
  let verb = Verb::Run { life: two.clone(), word: "print('moved')".to_owned(), on: None };
  assert!(matches!(window.ask(verb).await, Ok(Answer::Act(_))));
  messaged(&mut window, &two, "ground the branch").await;
  let last = card(&mut window, told(&two, "ground the branch")).await;
  let operator = format!("operator/{}", last.viewers.first().expect("the window holds the life"));
  let sent =
    |text: &str, python: bool| view::Sent { by: operator.clone(), text: text.to_owned(), python };
  let wanted = vec![
    sent("ground the branch", false),
    sent("print('moved')", true),
    sent("fix the build", false),
    sent("say what moved", false),
  ];
  assert_eq!(window.ask(Verb::History { life: two }).await, Ok(Answer::History(wanted)));
}

#[tokio::test]
async fn a_history_asked_right_after_a_send_holds_that_send() {
  let at = place();
  let (_, sessions) = project(&at);
  let (local, _daemon) = started(&at, GRACE).await;
  let mut window = Window::dialed(&local).await;
  let life = held(&mut window, &sessions, "one.jsonl").await;
  for text in ["say what moved", "fix the build", "ground the branch"] {
    messaged(&mut window, &life, text).await;
    let Ok(Answer::History(sent)) = window.ask(Verb::History { life: life.clone() }).await else {
      panic!("no history")
    };
    assert_eq!(sent.first().map(|one| one.text.as_str()), Some(text));
  }
}

#[tokio::test]
async fn the_skills_of_a_record_are_those_that_its_root_read_else_those_of_its_project() {
  let at = place();
  let (main, sessions) = project(&at);
  let folder = Path::new(&main).join(".furb").join("skills").join("probe");
  fs::create_dir_all(&folder).unwrap();
  let manifest = "---\nname: probe\ndescription: Probes the window\n---\nRead the frames.\n";
  fs::write(folder.join("SKILL.md"), manifest).unwrap();
  let (local, _daemon) = started(&at, SHORT).await;
  let mut window = Window::dialed(&local).await;
  cards(&mut window).await;
  let one = held(&mut window, &sessions, "one.jsonl").await;
  // A life whose root read no index of the skills offers none.
  assert_eq!(window.ask(Verb::Skills { life: one.clone() }).await, Ok(Answer::Skills(Vec::new())));
  let read = Verb::Run { life: one.clone(), word: "read('skills://')".to_owned(), on: None };
  assert!(matches!(window.ask(read).await, Ok(Answer::Act(_))));
  let probe = view::Skill {
    name: "probe".to_owned(),
    description: "Probes the window".to_owned(),
    ..view::Skill::default()
  };
  let skills = read_skills(&mut window, &one).await;
  assert!(skills.contains(&probe), "{skills:?}");
  let mut names: Vec<&str> = skills.iter().map(|one| one.name.as_str()).collect();
  names.sort_unstable();
  assert_eq!(names, skills.iter().map(|one| one.name.as_str()).collect::<Vec<_>>());
  // The first life ends, and a new session of the project offers the skills that its index keeps.
  assert_eq!(window.ask(Verb::Detach { life: one.clone() }).await, Ok(Answer::Done));
  time::sleep(PAST).await;
  assert!(!window.lives(&one).await);
  let two = held(&mut window, &sessions, "two.jsonl").await;
  assert_eq!(window.ask(Verb::Skills { life: two }).await, Ok(Answer::Skills(skills)));
}

/// The skills of a life once its root read them, which its cards tell a moment after the read.
async fn read_skills(window: &mut Window, life: &str) -> Vec<view::Skill> {
  let end = Instant::now() + PATIENCE;
  while Instant::now() < end {
    match window.ask(Verb::Skills { life: life.to_owned() }).await {
      Ok(Answer::Skills(skills)) if !skills.is_empty() => return skills,
      Ok(Answer::Skills(_)) => time::sleep(Duration::from_millis(20)).await,
      other => panic!("no skills: {other:?}"),
    }
  }
  panic!("the life never told the skills that its root read")
}

#[cfg(unix)]
#[tokio::test]
async fn a_record_that_a_link_names_has_one_key_in_the_daemon() {
  use std::os::unix::fs::symlink;
  let at = place();
  let (main, sessions) = project(&at);
  // The list names the project through a link, and the window names the record by its real path.
  let link = at.join("link");
  symlink(&main, &link).unwrap();
  let list = json!({ "workspaces": [{ "directory": link.display().to_string() }] });
  fs::write(at.join("config").join("workspaces.json"), list.to_string()).unwrap();
  let (local, _daemon) = started(&at, GRACE).await;
  let mut window = Window::dialed(&local).await;
  cards(&mut window).await;
  let life = held(&mut window, &sessions, "one.jsonl").await;
  messaged(&mut window, &life, "ground the branch").await;
  card(&mut window, |card| card.thread.standing == Standing::Resolved).await;
  let lives: Vec<String> = window.listed().await.into_iter().map(|one| one.life).collect();
  assert_eq!(lives, vec![life.clone()]);
  // A verb that names the record through the link reaches the same life.
  let through = link.join(".furb").join("sessions").join("one.jsonl").display().to_string();
  assert_eq!(
    window.ask(Verb::Seen { life: through, about: "chain1".to_owned() }).await,
    Ok(Answer::Done)
  );
  assert_eq!(seen_by(&mut window, &life, "chain1").await.len(), 1);
}

#[cfg(unix)]
#[tokio::test]
async fn a_new_record_that_a_link_names_keeps_one_key_once_the_daemon_made_its_folder() {
  use std::os::unix::fs::symlink;
  let at = place();
  let (main, _) = project(&at);
  let link = at.join("link");
  symlink(&main, &link).unwrap();
  // No folder holds the sessions of the project yet when the window names a new record through the
  // link, and the daemon makes it at the open.
  let fresh = link.join("fresh").join("sessions");
  let (local, _daemon) = started(&at, GRACE).await;
  let mut window = Window::dialed(&local).await;
  cards(&mut window).await;
  let life = held(&mut window, &fresh, "one.jsonl").await;
  messaged(&mut window, &life, "ground the branch").await;
  card(&mut window, |card| card.thread.standing == Standing::Resolved).await;
  let real = Path::new(&main).join("fresh").join("sessions").join("one.jsonl");
  let lives: Vec<String> = window.listed().await.into_iter().map(|one| one.life).collect();
  assert_eq!(lives, vec![real.display().to_string()]);
  // The record stands now, and each of its names reaches the one life.
  for name in [&life, &real.display().to_string()] {
    let seen = Verb::Seen { life: name.clone(), about: "chain1".to_owned() };
    assert_eq!(window.ask(seen).await, Ok(Answer::Done));
  }
}

/// The opening of the lives of a test whose model asks the operator at each prompt, so a thread
/// waits for him, and closes its prompt once he answers.
fn asking(at: &Path) -> Opening {
  let answer = Answered(Arc::new(|_, _| {
    let word = "await prompt(str, 'Which key sends the queue?', to=OPERATOR)\nclose(None)";
    let turn = ["assistant", word].map(Object::string).into_iter();
    Box::pin(async move { Ok(Object::tuple(turn.chain([Object::none(), Object::none()]))) })
  }));
  Opening { answer: Some(answer), ..opening(at) }
}

#[tokio::test]
async fn a_resolve_closes_each_act_that_keeps_a_thread_open_and_a_message_opens_it_again() {
  let at = place();
  let (_, sessions) = project(&at);
  let (local, _daemon) = started_with(&at, GRACE, asking(&at)).await;
  let mut window = Window::dialed(&local).await;
  cards(&mut window).await;
  let life = held(&mut window, &sessions, "one.jsonl").await;
  let prompt = messaged(&mut window, &life, "ground the branch").await;
  let stands = |prompt: &str, standing: Standing| {
    let prompt = prompt.to_owned();
    move |card: &Card| card.thread.prompt == prompt && card.thread.standing == standing
  };
  let asks = Standing::Asks { of: None };
  card(&mut window, stands(&prompt, asks.clone())).await;
  let resolve = Verb::Resolve { life: life.clone(), thread: prompt.clone() };
  assert_eq!(window.ask(resolve.clone()).await, Ok(Answer::Done));
  card(&mut window, stands(&prompt, Standing::Resolved)).await;
  // A message opens the thread again as a question, and a resolve closes that message with a text.
  let (shape, message) = ("str".to_owned(), "and say what moved".to_owned());
  let on = Some(prompt.clone());
  let said = Verb::Prompt { life: life.clone(), shape, message, to: None, on };
  let Ok(Answer::Act(again)) = window.ask(said).await else { panic!("no message") };
  card(&mut window, stands(&prompt, asks)).await;
  assert_eq!(window.ask(resolve).await, Ok(Answer::Done));
  card(&mut window, stands(&prompt, Standing::Resolved)).await;
  let peek = Verb::Peek { life: life.clone(), act: again };
  assert_eq!(window.ask(peek).await, Ok(Answer::Value(json!("resolved"))));
  // A thread whose prompt takes no text waits for its model.
  let (shape, message) = ("int".to_owned(), "count the regressions".to_owned());
  let said = Verb::Prompt { life: life.clone(), shape, message, to: None, on: None };
  let Ok(Answer::Act(count)) = window.ask(said).await else { panic!("no prompt") };
  card(&mut window, stands(&count, Standing::Asks { of: None })).await;
  // A message after it that takes a text stays open too: a resolve closes all or nothing.
  let (shape, message) = ("str".to_owned(), "and name each".to_owned());
  let on = Some(count.clone());
  let said = Verb::Prompt { life: life.clone(), shape, message, to: None, on };
  let Ok(Answer::Act(named)) = window.ask(said).await else { panic!("no message") };
  let refused = window.ask(Verb::Resolve { life: life.clone(), thread: count.clone() }).await;
  let why = format!("{count} takes a value of the shape int, so only its model resolves it.");
  assert_eq!(refused, Err(ProtocolError::Refused { why }));
  let why = format!("No act named {named} is done.");
  let peek = Verb::Peek { life, act: named };
  assert_eq!(window.ask(peek).await, Err(ProtocolError::Refused { why }));
}

/// The lines of the block of each exchange of a page, as their numbers and their texts.
fn numbered(page: &view::Page) -> Vec<(usize, Vec<(usize, String)>)> {
  let lines =
    |block: &view::Block| block.lines.iter().map(|one| (one.number, one.text.clone())).collect();
  page
    .exchanges
    .iter()
    .map(|one| (one.at, one.block.as_ref().map(lines).unwrap_or_default()))
    .collect()
}

#[tokio::test]
async fn a_thread_gives_its_end_by_page_and_tells_each_change_to_the_peer_that_reads_it() {
  let at = place();
  let (_, sessions) = project(&at);
  let (local, _daemon) = started(&at, GRACE).await;
  let mut window = Window::dialed(&local).await;
  cards(&mut window).await;
  let life = held(&mut window, &sessions, "one.jsonl").await;
  let prompt = messaged(&mut window, &life, "ground the branch").await;
  card(&mut window, |card| card.thread.standing == Standing::Resolved).await;
  let read = |before, limit| Verb::Thread {
    life: life.clone(),
    thread: prompt.clone(),
    before,
    limit,
    at: None,
  };
  let Ok(Answer::Page(page)) = window.ask(read(None, 10)).await else { panic!("no page") };
  assert_eq!((page.above, page.exchanges[0].text.as_str()), (0, "ground the branch"));
  assert_eq!(numbered(&page), [(0, vec![(1, "close(None)".to_owned())])]);
  // A message on the thread is told to the peer that reads its end, with its block numbered on.
  let (shape, message) = ("None".to_owned(), "and say what moved".to_owned());
  let on = Some(prompt.clone());
  let said = Verb::Prompt { life: life.clone(), shape, message, to: None, on };
  assert!(matches!(window.ask(said).await, Ok(Answer::Act(_))));
  let answered = |frame: &Frame| matches!(frame, Frame::Thread { page, .. } if numbered(page).contains(&(1, vec![(2, "close(None)".to_owned())])));
  let Frame::Thread { thread, .. } = window.until(answered).await else { unreachable!() };
  assert_eq!(thread, prompt);
  // A page of one exchange at the end leaves the first above it, and an older page gives it.
  let Ok(Answer::Page(tip)) = window.ask(read(None, 1)).await else { panic!("no page") };
  assert_eq!((tip.above, numbered(&tip)), (1, vec![(1, vec![(2, "close(None)".to_owned())])]));
  let Ok(Answer::Page(older)) = window.ask(read(Some(1), 1)).await else { panic!("no page") };
  assert_eq!((older.above, numbered(&older)), (0, vec![(0, vec![(1, "close(None)".to_owned())])]));
}

#[tokio::test]
async fn the_cards_of_a_record_stay_the_same_through_each_birth_across_a_crossing_live_and_at_rest()
{
  let at = place();
  let (_, sessions) = project(&at);
  let (local, _daemon) = started(&at, Duration::ZERO).await;
  let mut window = Window::dialed(&local).await;
  cards(&mut window).await;
  let life = held(&mut window, &sessions, "one.jsonl").await;
  let prompt = messaged(&mut window, &life, "ground the branch").await;
  card(&mut window, |card| card.thread.standing == Standing::Resolved).await;
  assert_eq!(window.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
  let rests = card(&mut window, |card| !card.held).await;
  let same = |card: &Card| Card { since: 0, held: false, viewers: Vec::new(), ..card.clone() };
  // A crossing at the tip of the record, then a birth across it, which says each act of its
  // carry again: the thread that he started stays his.
  crossed(Opening { record: Some(life.clone()), ..opening(&at) }).await;
  held(&mut window, &sessions, "one.jsonl").await;
  window.ran(&life, "x = 1").await;
  // The cards that the life holds once it is born, which the index that its end writes keeps, and
  // a fold of the record with no index.
  assert_eq!(window.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
  let ended = card(&mut window, |card| !card.held && card.thread.prompt == prompt).await;
  assert_eq!(same(&ended), same(&rests));
  let index = format!("{life}.threads.json");
  let kept: Value = serde_json::from_slice(&fs::read(&index).unwrap()).unwrap();
  let prompts: Vec<_> =
    kept["threads"].as_array().unwrap().iter().map(|one| &one["prompt"]).collect();
  assert_eq!(prompts, [&json!(prompt)]);
  fs::remove_file(&index).unwrap();
  let mut later = Window::dialed(&local).await;
  cards(&mut later).await;
  let folded = card(&mut later, |card| card.thread.prompt == prompt).await;
  assert_eq!(same(&folded), same(&rests));
}

/// A life that opens on a record whose index names an older tip keeps the cards of that index in
/// the list until it tells its own: a record only grows, so each thread of the index stands in it.
/// So a window that dials while the life opens, as after a handover, sees each thread it saw.
#[cfg(unix)]
#[tokio::test]
async fn a_life_that_opens_keeps_the_cards_of_an_older_index_until_it_tells_its_own() {
  use std::os::unix::fs::PermissionsExt;
  let at = place();
  let (_, sessions) = project(&at);
  let (local, _daemon) = started(&at, Duration::ZERO).await;
  let mut window = Window::dialed(&local).await;
  cards(&mut window).await;
  let life = held(&mut window, &sessions, "one.jsonl").await;
  messaged(&mut window, &life, "ground the branch").await;
  card(&mut window, |card| card.thread.standing == Standing::Resolved).await;
  assert_eq!(window.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
  let rests = card(&mut window, |card| !card.held).await;
  let index = format!("{life}.threads.json");
  let mut kept: Value = serde_json::from_slice(&fs::read(&index).unwrap()).unwrap();
  kept["tip"]["bytes"] = json!(1);
  fs::write(&index, kept.to_string()).unwrap();
  // The open waits at the catalog of its models, after the read of its record, until the test
  // makes the file `go`: its claude command line waits for it.
  let (waits, go) = (at.join("waits"), at.join("go"));
  let script = format!("#!/bin/sh\nuntil [ -e '{}' ]; do sleep 0.02; done\n", go.display());
  fs::write(&waits, script).unwrap();
  fs::set_permissions(&waits, fs::Permissions::from_mode(0o755)).unwrap();
  let verb = Verb::Open {
    life: life.clone(),
    cwd: Some(sessions.display().to_string()),
    actor: None,
    roster: None,
    skills: None,
    config: None,
    claude: Some(waits.display().to_string()),
    answers: false,
    attach: Some(0),
  };
  let opens = tokio::spawn(async move { window.ask(verb).await });
  let mut later = Window::dialed(&local).await;
  let open = async {
    while !later.listed().await.iter().any(|one| one.life == life && one.held) {
      time::sleep(Duration::from_millis(10)).await;
    }
  };
  timeout(PATIENCE, open).await.expect("the life opens");
  let told: Vec<Card> = cards(&mut later)
    .await
    .into_iter()
    .map(|one| Card { held: false, thread: one.thread.at_rest(), ..one })
    .collect();
  assert_eq!(told, vec![rests]);
  fs::write(&go, "").unwrap();
  assert_eq!(opens.await.unwrap(), Ok(Answer::Done));
}

#[tokio::test]
async fn the_cards_of_a_record_at_rest_come_from_its_index_and_never_from_a_fold_in_the_list() {
  let at = place();
  let (_, sessions) = project(&at);
  let (local, _daemon) = started(&at, Duration::ZERO).await;
  let mut window = Window::dialed(&local).await;
  assert_eq!(cards(&mut window).await, Vec::new());
  let life = held(&mut window, &sessions, "one.jsonl").await;
  messaged(&mut window, &life, "ground the branch").await;
  let resolved = |card: &Card| card.thread.standing == Standing::Resolved;
  let lived = card(&mut window, resolved).await;
  // The life ends at the detach, writes its index, and its card stays with no life.
  assert_eq!(window.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
  let rests = card(&mut window, |card| !card.held).await;
  assert_eq!(rests, Card { held: false, viewers: Vec::new(), ..lived });
  // A list of a record at rest reads its index: nothing retold the record, which would leave the
  // file of what it tells beside it.
  let (index, retold) = (format!("{life}.threads.json"), format!("{life}.told"));
  let mut second = Window::dialed(&local).await;
  assert_eq!(cards(&mut second).await, vec![rests.clone()]);
  assert!(!Path::new(&retold).exists(), "the list read the past of a record");
  // An index that names an older tip of its record gives its cards, since a record only grows,
  // until the daemon made the index again from the record: no card comes and goes.
  let mut kept: Value = serde_json::from_slice(&fs::read(&index).unwrap()).unwrap();
  kept["tip"]["bytes"] = json!(1);
  fs::write(&index, kept.to_string()).unwrap();
  let mut third = Window::dialed(&local).await;
  assert_eq!(cards(&mut third).await, vec![rests.clone()]);
  let tip = || serde_json::from_slice::<Value>(&fs::read(&index).unwrap()).unwrap()["tip"].clone();
  let made = async {
    while tip() == kept["tip"] {
      time::sleep(Duration::from_millis(10)).await;
    }
  };
  timeout(PATIENCE, made).await.expect("the daemon makes the index again");
  let mut fourth = Window::dialed(&local).await;
  let folded = cards(&mut fourth).await.into_iter().map(|one| Card { since: 0, ..one });
  assert_eq!(folded.collect::<Vec<_>>(), vec![Card { since: 0, ..rests }]);
}

#[tokio::test]
async fn the_daemon_tells_how_far_the_index_of_a_record_stands_before_its_cards() {
  let at = place();
  let (_, sessions) = project(&at);
  let (local, _daemon) = started(&at, Duration::ZERO).await;
  let mut window = Window::dialed(&local).await;
  assert_eq!(cards(&mut window).await, Vec::new());
  let life = held(&mut window, &sessions, "one.jsonl").await;
  messaged(&mut window, &life, "ground the branch").await;
  let lived = card(&mut window, |card| card.thread.standing == Standing::Resolved).await;
  assert_eq!(window.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
  card(&mut window, |card| !card.held).await;
  fs::remove_file(format!("{life}.threads.json")).unwrap();
  let mut second = Window::dialed(&local).await;
  assert_eq!(cards(&mut second).await, Vec::new());
  card(&mut second, |card| card.thread.prompt == lived.thread.prompt).await;
  let steps: Vec<(u64, u64)> = second
    .pushed
    .iter()
    .filter_map(|frame| match frame {
      Frame::Indexing { life: of_life, done, of } if *of_life == life => Some((*done, *of)),
      _ => None,
    })
    .collect();
  assert!(steps.len() >= 2, "{steps:?}");
  assert!(steps.windows(2).all(|two| two[0].0 <= two[1].0), "{steps:?}");
  assert!(steps.iter().all(|(_, of)| *of == 1000), "{steps:?}");
  assert_eq!(steps.last(), Some(&(1000, 1000)));
}

#[tokio::test]
async fn the_changes_of_a_checkout_give_its_branch_its_files_with_their_counts_and_a_diff() {
  use furb_host::protocol::{Change, Changed, Changes};
  let at = place();
  let (local, _daemon) = started(&at, GRACE).await;
  let mut window = Window::dialed(&local).await;
  let folder = at.join("project");
  fs::create_dir_all(&folder).unwrap();
  let git = |args: &[&str]| {
    let done = process::Command::new("git").arg("-C").arg(&folder).args(args).output();
    assert!(done.unwrap().status.success(), "git {args:?}");
  };
  git(&["init", "-q", "-b", "trunk"]);
  fs::write(folder.join("a.txt"), "one\ntwo\n").unwrap();
  git(&["add", "a.txt"]);
  git(&["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", "a"]);
  fs::write(folder.join("a.txt"), "one\n2\nthree\n").unwrap();
  fs::write(folder.join("b.txt"), "new\n").unwrap();
  let cwd = folder.display().to_string();
  let asked = window.ask(Verb::Changes { cwd: cwd.clone(), path: Some("b.txt".to_owned()) }).await;
  let Ok(Answer::Changes(Changes { branch, files, diff })) = asked else { panic!("{asked:?}") };
  let file =
    |path: &str, added, removed, state| Changed { path: path.to_owned(), added, removed, state };
  assert_eq!(branch.as_deref(), Some("trunk"));
  assert_eq!(
    files,
    [file("a.txt", 2, 1, Change::Modified), file("b.txt", 1, 0, Change::Untracked)]
  );
  assert!(diff.is_some_and(|one| one.contains("+new")), "the diff adds the new file whole");
  let elsewhere = at.join("nowhere");
  fs::create_dir_all(&elsewhere).unwrap();
  let none = Verb::Changes { cwd: elsewhere.display().to_string(), path: None };
  assert_eq!(window.ask(none).await, Ok(Answer::Changes(Changes::default())));
}

#[tokio::test]
async fn the_changes_name_a_branch_with_no_commit_and_none_when_detached_and_count_a_rename_and_a_binary()
 {
  use furb_host::protocol::{Change, Changed, Changes};
  let at = place();
  let (local, _daemon) = started(&at, GRACE).await;
  let mut window = Window::dialed(&local).await;
  let folder = at.join("project");
  fs::create_dir_all(folder.join(".furb/sessions")).unwrap();
  let git = |args: &[&str]| {
    let done = process::Command::new("git").arg("-C").arg(&folder).args(args).output();
    assert!(done.unwrap().status.success(), "git {args:?}");
  };
  let cwd = folder.display().to_string();
  let mut changes = async || {
    let asked = window.ask(Verb::Changes { cwd: cwd.clone(), path: None }).await;
    let Ok(Answer::Changes(changes)) = asked else { panic!("{asked:?}") };
    changes
  };
  // A checkout with no commit names the branch that its first commit makes. A record of a
  // session is no change of the project.
  git(&["init", "-q", "-b", "trunk"]);
  fs::write(folder.join("old.rs"), "fn a() {}\nfn b() {}\n").unwrap();
  fs::write(folder.join("logo.png"), [0u8, 1, 2, 3]).unwrap();
  fs::write(folder.join(".furb/sessions/a.jsonl"), "[]\n").unwrap();
  let Changes { branch, files, .. } = changes().await;
  assert_eq!(branch.as_deref(), Some("trunk"));
  let paths: Vec<&str> = files.iter().map(|one| one.path.as_str()).collect();
  assert_eq!(paths, ["logo.png", "old.rs"]);
  fs::write(folder.join("gone.rs"), "fn gone() {}\n").unwrap();
  git(&["add", "old.rs", "logo.png", "gone.rs"]);
  git(&["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", "a"]);
  let remote = at.join("remote.git");
  let bare = process::Command::new("git").args(["init", "-q", "--bare"]).arg(&remote).output();
  assert!(bare.unwrap().status.success());
  git(&["remote", "add", "origin", &remote.display().to_string()]);
  git(&["push", "-q", "-u", "origin", "trunk"]);
  // A rename counts under its new path, and a binary file counts no line.
  git(&["mv", "old.rs", "new.rs"]);
  fs::write(folder.join("new.rs"), "fn a() {}\nfn b() {}\nfn c() {}\n").unwrap();
  git(&["add", "new.rs"]);
  fs::write(folder.join("logo.png"), [9u8, 8, 7, 0]).unwrap();
  git(&["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-am", "b"]);
  // A branch ahead of its upstream names itself alone.
  assert_eq!(changes().await.branch.as_deref(), Some("trunk"));
  git(&["checkout", "-q", "--detach"]);
  git(&["mv", "new.rs", "newer.rs"]);
  fs::write(folder.join("newer.rs"), "fn a() {}\nfn b() {}\nfn c() {}\nfn d() {}\n").unwrap();
  git(&["add", "newer.rs"]);
  fs::write(folder.join("logo.png"), [5u8, 0, 5]).unwrap();
  fs::remove_file(folder.join("gone.rs")).unwrap();
  let Changes { branch, files, .. } = changes().await;
  let file =
    |path: &str, added, removed, state| Changed { path: path.to_owned(), added, removed, state };
  assert_eq!(branch, None);
  assert_eq!(
    files,
    [
      file("gone.rs", 0, 1, Change::Deleted),
      file("logo.png", 0, 0, Change::Modified),
      file("newer.rs", 1, 0, Change::Renamed)
    ]
  );
}
