//! The memory that a life holds as it grows: a life that keeps a record of its own draws numbers
//! and answers prompts whose model writes its reply in many small parts, and the heap of the
//! process is read at the sizes of `SMALL`, with the events and the bytes that a reader who
//! attaches at the start of the log reads. With the argument `--ignored` it is the measure of the
//! bench, at the sizes of `LARGE`, with the budget `BUDGET` of its heap. The test counts each byte
//! that the process allocates, so it runs in a process of its own, with no harness, and its
//! argument takes the place of an ignored test.

#![expect(
  clippy::disallowed_methods,
  reason = "the test blocks on the runtime that it starts for itself"
)]
#![expect(clippy::expect_used, reason = "a test with no harness fails by a panic")]

#[path = "../../tests/common/yard.rs"]
mod yard;

use std::{
  alloc::{GlobalAlloc, Layout, System},
  collections::{BTreeMap, HashSet},
  path::Path,
  sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
  },
  time::{Duration, Instant},
};

use furb::{
  Object,
  life::{Answer as Answered, Opening},
  view::Phase,
  world::Told,
};
use furb_host::{
  Command, Event, GRACE, Life, LifeHandle, Reply, STALL, Seq, Settings,
  protocol::{Answer, Frame, Hello, Kind, PROTOCOL, Request, Verb},
  run,
  transport::{Dialer, Local},
};
use futures::{SinkExt, Stream, StreamExt};
use tokio_util::codec::{Framed, LinesCodec};

/// The events of a life, each under its place in the log.
type Logged = (Seq, Arc<Event>);

/// The allocator of the system, which counts the bytes that the process holds.
struct Counted;

/// The bytes that the process holds on its heap.
static LIVE: AtomicUsize = AtomicUsize::new(0);

#[expect(
  unsafe_code,
  reason = "a count of the heap wraps the allocator of the system, whose trait is unsafe"
)]
// SAFETY: each call goes to the allocator of the system with its own arguments, and only counts
// the bytes.
unsafe impl GlobalAlloc for Counted {
  unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
    LIVE.fetch_add(layout.size(), Ordering::Relaxed);
    // SAFETY: the caller keeps the contract of alloc, which is that of the system.
    unsafe { System.alloc(layout) }
  }

  unsafe fn dealloc(&self, at: *mut u8, layout: Layout) {
    LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
    // SAFETY: at came from the allocator of the system with this layout.
    unsafe { System.dealloc(at, layout) }
  }
}

#[global_allocator]
static COUNTED: Counted = Counted;

/// The sizes of a life, in facts, at which the suite reads the heap: two sizes that run in a few
/// seconds, which show what a reader reads and how the heap grows from one to the other.
/// `FURB_MEMORY_SIZES` may name others, as numbers apart by commas.
const SMALL: [usize; 2] = [2_000, 6_000];

/// The sizes of the measure of the bench, which the argument `--ignored` runs.
const LARGE: [usize; 3] = [10_000, 40_000, 160_000];

/// The most heap that a life of the last large size may hold, in MiB: the budget of the bench.
const BUDGET: usize = 1024;

/// How many events a reader may stand behind the life and still hear each of them, as the life
/// says.
const BEHIND: usize = 256;

/// The most bytes that the test lets the process hold, past which it stops with what it measured:
/// a life that grows with no bound would take the memory of the machine.
const MOST: usize = 3 << 30;

/// How many numbers one word of the operator draws.
const DRAWS: usize = 500;

/// How many parts the model writes for each reply: the command line of a model streams a part for
/// each few tokens, so a reply of a thousand words comes in about a thousand parts.
const PARTS: usize = 1_000;

/// How many times one word of the operator reads a file, and the size of the file: each read tells
/// its text to the chain, as a real life reads its sources and the output of its commands.
const READS: usize = 10;
const FILE: usize = 8_000;

/// A life that keeps its record, with no extension, whose model writes each reply in parts and
/// closes it with a word.
fn opening(at: &Path) -> Opening {
  let answer = Answered(Arc::new(move |_, told: Told| {
    let part = "a word ".to_owned();
    for _ in 0..PARTS {
      told(&part, "");
    }
    let turn = Object::tuple([
      Object::string("assistant"),
      Object::string("close('ok')"),
      Object::none(),
      Object::none(),
    ]);
    Box::pin(async move { Ok(turn) })
  }));
  Opening {
    directory: at.display().to_string(),
    record: Some(at.join("record.jsonl").display().to_string()),
    config: Some(at.join("config").display().to_string()),
    roster: Some(vec!["claude-cli:opus".to_owned()]),
    skills: Some(false),
    answer: Some(answer),
    ..Opening::default()
  }
}

/// The events of a stream up to the first that the test waits for, with the facts among them.
async fn until(
  events: &mut (impl Stream<Item = Logged> + Unpin),
  wanted: impl Fn(&Event) -> bool,
) -> (Seq, usize) {
  let mut facts = 0;
  loop {
    let next = tokio::time::timeout(Duration::from_secs(300), events.next()).await;
    let Ok(Some((seq, event))) = next else { panic!("the life told no event in time") };
    facts += usize::from(matches!(&*event, Event::Fact(_)));
    if wanted(&event) {
      return (seq, facts);
    }
  }
}

/// Whether an event is the act of a name, done.
fn done(id: String) -> impl Fn(&Event) -> bool {
  move |event| matches!(event, Event::Act(act) if act.id == id && matches!(act.phase, Phase::Done(_)))
}

/// The act that a command made.
async fn made(life: &LifeHandle, command: Command) -> String {
  match life.send(command).await {
    Ok(Reply::Act(id)) => id,
    other => panic!("the command made no act: {other:?}"),
  }
}

/// What a reader that attaches at the start of the log reads, up to a place: its events, their
/// bytes as JSON, its parts that a model wrote, its events of acts, and the acts that they name.
async fn attached(life: &LifeHandle, last: Seq) -> (usize, usize, usize, usize, usize) {
  let mut events = Box::pin(life.events(0));
  let (mut count, mut bytes) = (0, 0);
  let mut kinds: BTreeMap<String, (usize, usize)> = BTreeMap::new();
  let mut acts = HashSet::new();
  while let Some((seq, event)) = events.next().await {
    if let Event::Act(act) = &*event {
      acts.insert(act.id.clone());
    }
    count += 1;
    let json = serde_json::to_vec(&*event).expect("an event is JSON");
    bytes += json.len();
    let kind = match &*event {
      Event::Fact(fact) if fact[0] == "keep" => {
        format!("keep {}", fact[3][0][0].as_str().unwrap_or("?"))
      }
      Event::Fact(fact) => format!("fact {}", fact[0].as_str().unwrap_or("?")),
      _ => format!("{event:?}").split(['(', ' ']).next().unwrap_or_default().to_owned(),
    };
    let tally = kinds.entry(kind).or_default();
    (tally.0, tally.1) = (tally.0 + 1, tally.1 + json.len());
    if seq >= last {
      break;
    }
  }
  eprintln!("kinds {kinds:?}");
  let mut heavy: Vec<_> =
    kinds.iter().filter(|one| one.0.starts_with("fact") || one.0.starts_with("keep")).collect();
  heavy.sort_by_key(|one| std::cmp::Reverse(one.1.1));
  eprintln!("heaviest facts {:?}", &heavy[..heavy.len().min(8)]);
  let facts: usize = heavy.iter().map(|one| one.1.0).sum();
  let of =
    |kind: &str| if kind == "Fact" { facts } else { kinds.get(kind).map_or(0, |tally| tally.0) };
  (count, bytes, of("Text"), of("Act"), acts.len())
}

/// The footprint of this process, as the footprint tool of the system reads it.
fn footprint() -> String {
  let id = std::process::id().to_string();
  let read = std::process::Command::new("footprint").arg(id).output();
  let text = read.map(|one| String::from_utf8_lossy(&one.stdout).into_owned()).unwrap_or_default();
  let line = text.lines().find(|one| one.contains("Footprint:")).unwrap_or_default();
  line.split("Footprint: ").nth(1).and_then(|one| one.split(" (").next()).unwrap_or("?").to_owned()
}

fn main() {
  let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build();
  let runtime = runtime.expect("the test has a runtime");
  let large = std::env::args().any(|one| one == "--ignored");
  let sizes: Vec<usize> = std::env::var("FURB_MEMORY_SIZES").map_or_else(
    |_| if large { LARGE.to_vec() } else { SMALL.to_vec() },
    |named| named.split(',').map(|one| one.trim().parse().expect("a size is a number")).collect(),
  );
  runtime.block_on(the_heap_of_a_life_grows_with_its_facts_and_no_more(&sizes, large));
  let window = if large { (40_000, 16 << 20, 3) } else { (10_000, 6 << 20, 1) };
  runtime.block_on(a_window_that_reads_slowly_holds_no_copy_of_the_log(window));
}

/// The heap of a life at each size, and what a reader that attaches reads, printed as a table.
async fn the_heap_of_a_life_grows_with_its_facts_and_no_more(sizes: &[usize], large: bool) {
  let at = yard::Yard::new(format!("furb-host-memory-{}", std::process::id()));
  let base = LIVE.load(Ordering::Relaxed);
  std::fs::write(at.join("big.txt"), "y".repeat(FILE - 1) + "\n").expect("the file is written");
  let life = Life::spawn(opening(&at)).expect("the life has a thread");
  let mut events = Box::pin(life.events(0));
  let (mut facts, mut last) = (0, 0);
  println!("facts\theap MiB\tbytes/fact\tattach events\tattach MiB");
  let mut rows = Vec::new();
  let started = Instant::now();
  let mut biggest = (0, 0);
  for &size in sizes {
    while facts < size {
      let heap = LIVE.load(Ordering::Relaxed);
      assert!(heap < MOST, "the process holds {heap} bytes at {facts} facts, past {MOST}");
      eprintln!("{:?}\t{facts} facts\t{} MB", started.elapsed(), heap >> 20);
      let word = format!(
        "for i in range({DRAWS}):\n  chance()\nfor i in range({READS}):\n  read('big.txt')"
      );
      let rung = made(&life, Command::Rung { word, on: None }).await;
      let (seq, more) = until(&mut events, done(rung)).await;
      let prompt = Command::Prompt {
        shape: "str".to_owned(),
        message: "Say ok.".to_owned(),
        to: None,
        on: None,
      };
      let asked = made(&life, prompt).await;
      let (end, after) = until(&mut events, done(asked)).await;
      (facts, last) = (facts + more + after, seq.max(end));
    }
    tokio::time::sleep(Duration::from_millis(200)).await;
    let heap = LIVE.load(Ordering::Relaxed).saturating_sub(base);
    let (count, bytes, parts, events, acts) = attached(&life, last).await;
    println!(
      "{facts}\t{}\t{}\t{count}\t{}\t{}",
      heap >> 20,
      heap / facts,
      bytes >> 20,
      footprint()
    );
    rows.push((facts, parts, events, acts));
    biggest = (facts, heap);
  }
  life.send(Command::End).await.ok();
  checked(&rows, biggest, large);
}

/// What each size showed: what a reader that attaches read and, for the bench, the budget of the
/// heap of the last size, as its facts and its bytes.
fn checked(rows: &[(usize, usize, usize, usize)], last: (usize, usize), large: bool) {
  if large {
    let (facts, heap) = last;
    assert!(
      heap >> 20 <= BUDGET,
      "a life of {facts} facts holds {} MiB of heap, past the budget of {BUDGET} MiB",
      heap >> 20
    );
  }
  for &(facts, parts, events, acts) in rows {
    assert!(
      parts <= PARTS,
      "at {facts} facts, a reader who attaches reads {parts} parts that a model wrote, past the {PARTS} of the last reply: the log keeps each part of each reply that is done"
    );
    assert!(
      events <= acts + BEHIND,
      "at {facts} facts, a reader who attaches reads {events} events of {acts} acts: the log keeps each state of each act"
    );
  }
}

/// A peer of the daemon: its stream and the id of its next request.
struct Peer {
  lines: Framed<<Local as Dialer>::Stream, LinesCodec>,
  next: u64,
}

impl Peer {
  /// A window that dialed the daemon and said its hello.
  async fn dialed(local: &Local) -> Self {
    let lines = Framed::new(local.dial().await.expect("the daemon listens"), LinesCodec::new());
    let mut peer = Self { lines, next: 0 };
    let hello =
      Verb::Hello(Hello { protocol: PROTOCOL, kind: Kind::Window, name: None, role: None });
    let welcome = peer.ask(hello).await;
    assert!(matches!(welcome, Answer::Welcome { .. }), "{welcome:?}");
    peer
  }

  /// A request said, whose answer the peer reads later or never.
  async fn said(&mut self, verb: Verb) -> u64 {
    let id = self.next;
    self.next += 1;
    let line = serde_json::to_string(&Request { id, verb }).expect("a request is JSON");
    self.lines.send(line).await.expect("the daemon hears");
    id
  }

  /// The next frame of the daemon.
  async fn read(&mut self) -> Frame {
    let line = tokio::time::timeout(Duration::from_secs(300), self.lines.next()).await;
    let line = line.expect("the daemon says a frame in time").expect("the daemon is up");
    serde_json::from_str(&line.expect("a line")).expect("a frame is JSON")
  }

  /// The answer of the daemon to a verb, past each frame that it pushed before it.
  async fn ask(&mut self, verb: Verb) -> Answer {
    let id = self.said(verb).await;
    loop {
      match self.read().await {
        Frame::Reply { id: got, answer } if got == id => return answer,
        Frame::Refused { id: Some(got), error, .. } if got == id => panic!("refused: {error:?}"),
        _ => {}
      }
    }
  }

  /// The facts that the daemon pushed until an act is done.
  async fn until_done(&mut self, id: &str) -> usize {
    let mut facts = 0;
    loop {
      match self.read().await {
        Frame::Event { event: Event::Fact(_), .. } => facts += 1,
        Frame::Event { event: Event::Act(act), .. }
          if act.id == id && matches!(act.phase, Phase::Done(_)) =>
        {
          return facts;
        }
        _ => {}
      }
    }
  }
}

/// A daemon whose life told many facts, and a window that attaches at the start of its log and
/// reads nothing: the daemon pushes it the events of the life no faster than it reads them, so it
/// holds no copy of the log for it.
async fn a_window_that_reads_slowly_holds_no_copy_of_the_log(window: (usize, usize, u64)) {
  let (attached, most, wait) = window;
  let at = yard::Yard::new(format!("furb-host-slow-{}", std::process::id()));
  std::fs::write(at.join("big.txt"), "y".repeat(FILE - 1) + "\n").expect("the file is written");
  let local = Local::new(at.join("sock"));
  let listener = local.listen().await.expect("the daemon listens");
  let settings = Settings {
    opening: opening(&at),
    grace: GRACE,
    stall: STALL,
    tailnet: None,
    door: None,
    host: None,
  };
  let served = local.clone();
  tokio::spawn(async move { run(&served, listener, settings).await });
  let mut driver = Peer::dialed(&local).await;
  let life = at.join("slow.jsonl").display().to_string();
  let open = Verb::Open {
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
  assert_eq!(driver.ask(open).await, Answer::Done);
  assert_eq!(driver.ask(Verb::Attach { life: life.clone(), from: 0 }).await, Answer::Done);
  let mut facts = 0;
  while facts < attached {
    let word =
      format!("for i in range({DRAWS}):\n  chance()\nfor i in range({READS}):\n  read('big.txt')");
    let Answer::Act(rung) = driver.ask(Verb::Run { life: life.clone(), word, on: None }).await
    else {
      panic!("the word made no rung")
    };
    facts += driver.until_done(&rung).await;
  }
  tokio::time::sleep(Duration::from_millis(500)).await;
  let before = LIVE.load(Ordering::Relaxed);
  let mut slow = Peer::dialed(&local).await;
  slow.said(Verb::Attach { life: life.clone(), from: 0 }).await;
  tokio::time::sleep(Duration::from_secs(wait)).await;
  let taken = LIVE.load(Ordering::Relaxed).saturating_sub(before);
  println!("a slow window on a life of {facts} facts: the daemon took {} KiB", taken >> 10);
  assert!(
    taken < most,
    "a window that reads nothing on a life of {facts} facts took {taken} bytes of the daemon, past {most}: the daemon copies the log for it"
  );
  drop(slow);
}
