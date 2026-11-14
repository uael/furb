//! The heap of a daemon past its share: the life that said the most facts since it opened writes
//! its checkpoint and is born again, the heap falls under the share, and the other life goes on
//! untouched. The test counts each byte that the process allocates, so a child of the suite runs it
//! alone, as `apart` says.

#![expect(
  clippy::disallowed_methods,
  reason = "the test blocks on the runtime that it starts for itself"
)]
#![expect(clippy::expect_used, reason = "a helper of the test fails it by a panic")]

use crate::{apart, yard};

use std::{
  fs, process,
  sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
  },
  time::Duration,
};

use furb::{
  Object,
  life::{Answer as Answered, Opening},
  view::Phase,
};
use furb_host::{
  Counted, Event, Heap, STALL, Settings,
  protocol::{Answer, Frame, Hello, Kind, PROTOCOL, Request, Verb},
  run,
  transport::{Dialer, Local},
};
use futures::{SinkExt, StreamExt};
use serde_json::json;
use std::path::Path;
use tokio::{runtime::Builder, time};
use tokio_util::codec::{Framed, LinesCodec};

/// The bound of the heap of the daemon of the test, which reads its heap past a floor.
const BOUND: usize = 1 << 20;

/// The bytes of the process under which the daemon reads no heap: none reads until the test sets
/// it, so the lives grow first.
static FLOOR: AtomicUsize = AtomicUsize::new(usize::MAX);

/// The heap that the daemon of the test reads: what the process holds past the floor.
fn heap() -> usize {
  Counted::live().saturating_sub(FLOOR.load(Ordering::Relaxed))
}

/// How long a life with no holder lives in the daemon of the test.
const GRACE: Duration = Duration::from_secs(1);

#[test]
#[ignore = "a child of the suite runs it alone"]
fn alone() {
  if !apart::alone("heap::alone") {
    return;
  }
  let runtime = Builder::new_current_thread().enable_all().build();
  let runtime = runtime.expect("the test has a runtime");
  runtime.block_on(the_heaviest_life_is_born_again_when_the_heap_passes_its_share());
}

/// A life that keeps its record, with no skill, whose model closes each request.
fn opening(at: &Path) -> Opening {
  let answer = Answered(Arc::new(move |_, _| {
    let turn = Object::tuple([
      Object::string("assistant"),
      Object::string("close('ok')"),
      Object::none(),
      Object::none(),
    ]);
    Box::pin(async move { Ok(turn) })
  }));
  Opening {
    claude: Some(at.join("no-claude").display().to_string()),
    directory: at.display().to_string(),
    config: Some(at.join("config").display().to_string()),
    roster: Some(vec!["claude-cli:opus".to_owned()].into()),
    skills: Some(false),
    answer: Some(answer),
    ..Opening::default()
  }
}

/// A window of the daemon: its stream, the id of its next request, and each event that it heard,
/// under the life that told it.
struct Peer {
  lines: Framed<<Local as Dialer>::Stream, LinesCodec>,
  next: u64,
  heard: Vec<(String, Event)>,
}

impl Peer {
  /// A window that dialed the daemon and said its hello.
  async fn dialed(local: &Local) -> Self {
    let lines = Framed::new(local.dial().await.expect("the daemon listens"), LinesCodec::new());
    let mut peer = Self { lines, next: 0, heard: Vec::new() };
    let hello =
      Verb::Hello(Hello { protocol: PROTOCOL, kind: Kind::Window, name: None, role: None });
    let welcome = peer.ask(hello).await;
    assert!(matches!(welcome, Answer::Welcome { .. }), "{welcome:?}");
    peer
  }

  /// The next frame of the daemon, with each event kept.
  async fn read(&mut self) -> Frame {
    let line = time::timeout(Duration::from_secs(120), self.lines.next()).await;
    let line = line.expect("the daemon says a frame in time").expect("the daemon is up");
    let frame: Frame = serde_json::from_str(&line.expect("a line")).expect("a frame is JSON");
    // The peer keeps only what the test waits for, since the heap of the process counts it.
    if let Frame::Event { life, event, .. } = &frame
      && match event {
        Event::Act(act) => matches!(act.phase, Phase::Done(_)),
        Event::Reborn(_) | Event::Ended(_) => true,
        _ => false,
      }
    {
      self.heard.push((life.clone(), event.clone()));
    }
    frame
  }

  /// The answer of the daemon to a verb, past each frame that it pushed before it.
  async fn ask(&mut self, verb: Verb) -> Answer {
    let id = self.next;
    self.next += 1;
    let line = serde_json::to_string(&Request { id, verb }).expect("a request is JSON");
    self.lines.send(line).await.expect("the daemon hears");
    loop {
      match self.read().await {
        Frame::Reply { id: got, answer } if got == id => return answer,
        Frame::Refused { id: Some(got), error, .. } if got == id => panic!("refused: {error:?}"),
        _ => {}
      }
    }
  }

  /// Whether the peer heard an event of a life that holds a condition.
  fn has(&self, life: &str, wanted: impl Fn(&Event) -> bool) -> bool {
    self.heard.iter().any(|(one, event)| one == life && wanted(event))
  }

  /// The frames of the daemon read until the peer heard an event of a life that holds a condition.
  async fn until(&mut self, life: &str, wanted: impl Fn(&Event) -> bool) {
    while !self.has(life, &wanted) {
      self.read().await;
    }
  }

  /// A word run on a life, until its rung is done.
  async fn ran(&mut self, life: &str, word: &str) {
    let run = Verb::Run { life: life.to_owned(), word: word.to_owned(), on: None };
    let Answer::Act(rung) = self.ask(run).await else { panic!("the word made no rung") };
    let done = |event: &Event| matches!(event, Event::Act(act) if act.id == rung && matches!(act.phase, Phase::Done(_)));
    self.until(life, done).await;
  }

  /// The value that a name of the root of a life binds.
  async fn bound(&mut self, life: &str, name: &str) -> serde_json::Value {
    let inspect = Verb::Inspect { life: life.to_owned(), name: name.to_owned(), on: None };
    match self.ask(inspect).await {
      Answer::Inspected { value, .. } => value,
      other => panic!("{name} is not bound on {life}: {other:?}"),
    }
  }

  /// A life opened in a folder and attached from the start of its log.
  async fn opened(&mut self, life: &str, cwd: &Path) {
    let open = Verb::Open {
      life: life.to_owned(),
      cwd: Some(cwd.display().to_string()),
      actor: None,
      roster: None,
      skills: None,
      config: None,
      claude: None,
      answers: false,
      attach: Some(0),
    };
    assert_eq!(self.ask(open).await, Answer::Done);
  }
}

/// Whether an event is a rebirth.
fn reborn(event: &Event) -> bool {
  matches!(event, Event::Reborn(_))
}

/// A daemon with two lives and a small bound of its heap: once the heap passes its share, the life
/// that said the most facts is born again with each name of its module, the heap falls under the
/// share, the other life is not born again, and a later open of each reaches the same names.
async fn the_heaviest_life_is_born_again_when_the_heap_passes_its_share() {
  let at = yard::Yard::new(format!("furb-host-heap-{}", process::id()));
  fs::write(at.join("big.txt"), "y".repeat(7_999) + "\n").expect("the file is written");
  let local = served(&at).await;
  let mut peer = Peer::dialed(&local).await;
  let big = at.join("big.jsonl").display().to_string();
  let small = at.join("small.jsonl").display().to_string();
  peer.opened(&big, &at).await;
  peer.opened(&small, &at).await;
  peer.ran(&small, "kept = 1").await;
  peer.ran(&big, "kept = 7\nside = chain('side')").await;
  time::sleep(Duration::from_millis(500)).await;
  let grow = "for i in range(500):\n  chance()\nfor i in range(10):\n  read('big.txt')";
  for _ in 0..4 {
    peer.ran(&big, grow).await;
  }
  assert!(!peer.has(&big, reborn), "the big life was born again before the test");
  time::sleep(Duration::from_millis(500)).await;
  let before = Counted::live();
  FLOOR.store(before - BOUND, Ordering::Relaxed);
  peer.until(&big, reborn).await;
  // A few weighs of the daemon past the rebirth, in which it asks no other life once the heap
  // fell under its share.
  time::sleep(Duration::from_secs(3)).await;
  let after = Counted::live();
  println!(
    "the heap of the daemon: {} KiB before the rebirth, {} KiB after",
    before >> 10,
    after >> 10
  );
  assert!(
    heap() * 100 < BOUND * furb_host::SHARE,
    "the heap fell under its share: {} KiB before the rebirth, {} KiB after",
    before >> 10,
    after >> 10
  );
  assert!(!peer.has(&small, reborn), "the small life was born again");
  FLOOR.store(usize::MAX, Ordering::Relaxed);
  assert_eq!(peer.bound(&big, "kept").await, json!(7), "the big life keeps its module");
  peer.ran(&big, "kept = kept + 1").await;
  assert_eq!(peer.bound(&big, "kept").await, json!(8), "the big life goes on");
  assert_eq!(peer.bound(&small, "kept").await, json!(1), "the small life goes on");
  peer.reopened(&at, &big, &small).await;
}

/// A daemon of the test served in a folder, under the bound of its heap, and its transport.
async fn served(at: &Path) -> Local {
  let local = Local::new(at.join("sock"));
  let listener = local.listen().await.expect("the daemon listens");
  let settings = Settings {
    opening: opening(at),
    grace: GRACE,
    stall: STALL,
    tailnet: None,
    door: None,
    host: None,
    heap: Some(Heap { bound: BOUND, live: heap }),
  };
  let served = local.clone();
  tokio::spawn(async move { run(&served, listener, settings).await });
  local
}

impl Peer {
  /// Both lives let go until each ended, then opened again: each reaches the names of its tip.
  async fn reopened(&mut self, at: &Path, big: &str, small: &str) {
    let peer = self;
    for life in [big, small] {
      assert_eq!(peer.ask(Verb::Detach { life: life.to_owned() }).await, Answer::Done);
    }
    // A window that attached would hold the life, so the test asks the list of the daemon until it
    // holds neither life: each wrote its checkpoint and ended after its grace.
    for _ in 0..600 {
      let Answer::Lives(listed) = peer.ask(Verb::List { config: None }).await else {
        panic!("the daemon gave no list")
      };
      if !listed.iter().any(|one| one.held && (one.life == big || one.life == small)) {
        break;
      }
      time::sleep(Duration::from_millis(100)).await;
    }
    peer.opened(big, at).await;
    peer.opened(small, at).await;
    assert_eq!(
      peer.bound(big, "kept").await,
      json!(8),
      "a later open of the big life reaches its tip"
    );
    assert!(
      !peer.bound(big, "side").await.is_null(),
      "a later open of the big life holds its chain"
    );
    assert_eq!(
      peer.bound(small, "kept").await,
      json!(1),
      "a later open of the small life reaches its tip"
    );
  }
}
