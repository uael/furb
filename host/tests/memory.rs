//! The memory that a life holds as it grows: a life that keeps a record of its own draws numbers
//! and answers prompts whose model writes its reply in many small parts, and the heap of the
//! process is read at 10k, 40k and 160k facts, with the events and the bytes that a reader who
//! attaches at the start of the log reads. The test counts each byte that the process allocates, so
//! it runs in a process of its own, with no harness.

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
  life::{Answer, Opening},
  view::Phase,
  world::Told,
};
use furb_host::{Command, Event, Life, LifeHandle, Reply, Seq};
use futures::{Stream, StreamExt};

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

/// The sizes of a life, in facts, at which the test reads the heap, which `FURB_MEMORY_SIZES` may
/// name as numbers apart by commas.
const SIZES: [usize; 3] = [10_000, 40_000, 160_000];

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
  let answer = Answer(Arc::new(move |_, told: Told| {
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
    let kind = format!("{event:?}").split(['(', ' ']).next().unwrap_or_default().to_owned();
    let tally = kinds.entry(kind).or_default();
    (tally.0, tally.1) = (tally.0 + 1, tally.1 + json.len());
    if seq >= last {
      break;
    }
  }
  eprintln!("kinds {kinds:?}");
  let of = |kind: &str| kinds.get(kind).map_or(0, |tally| tally.0);
  (count, bytes, of("Text"), of("Act"), acts.len())
}

fn main() {
  let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build();
  let runtime = runtime.expect("the test has a runtime");
  runtime.block_on(the_heap_of_a_life_grows_with_its_facts_and_no_more());
}

/// The heap of a life at each size, and what a reader that attaches reads, printed as a table.
async fn the_heap_of_a_life_grows_with_its_facts_and_no_more() {
  let at = yard::Yard::new(format!("furb-host-memory-{}", std::process::id()));
  let base = LIVE.load(Ordering::Relaxed);
  std::fs::write(at.join("big.txt"), "y".repeat(FILE - 1) + "\n").expect("the file is written");
  let life = Life::spawn(opening(&at)).expect("the life has a thread");
  let mut events = Box::pin(life.events(0));
  let (mut facts, mut last) = (0, 0);
  println!("facts\theap MiB\tbytes/fact\tattach events\tattach MiB");
  let mut rows = Vec::new();
  let sizes: Vec<usize> = std::env::var("FURB_MEMORY_SIZES").map_or_else(
    |_| SIZES.to_vec(),
    |named| named.split(',').map(|one| one.trim().parse().expect("a size is a number")).collect(),
  );
  let started = Instant::now();
  for size in sizes {
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
    println!("{facts}\t{}\t{}\t{count}\t{}", heap >> 20, heap / facts, bytes >> 20);
    rows.push((facts, parts, events, acts));
  }
  life.send(Command::End).await.ok();
  for (facts, parts, events, acts) in rows {
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
