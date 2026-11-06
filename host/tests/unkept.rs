//! A life whose record takes no fact for a time, past the tail of facts that a checkpoint bounds:
//! it holds its rebirth and refuses its end until the record takes them again. The test fills the
//! record with the limit of the size of a file of the process, which only a system of Unix gives,
//! so it runs in a process of its own, with no harness.

#![cfg_attr(
  unix,
  expect(
    clippy::disallowed_methods,
    reason = "the test blocks on the runtime that it starts for itself"
  )
)]
#![cfg_attr(unix, expect(clippy::expect_used, reason = "a test with no harness fails by a panic"))]

#[cfg(unix)]
#[path = "../../tests/common/yard.rs"]
mod yard;

fn main() {
  #[cfg(unix)]
  {
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build();
    let runtime = runtime.expect("the test has a runtime");
    runtime.block_on(unix::a_life_whose_record_takes_no_fact_holds_its_rebirth_and_its_end());
  }
}

#[cfg(unix)]
mod unix {
  use std::{
    path::Path,
    sync::{
      Arc,
      atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
  };

  use furb::{
    Object,
    life::{Answer, Opening},
  };
  use furb_host::{Command, Event, Life, LifeError, LifeHandle, Reply, Seq, view::Phase};
  use futures::{Stream, StreamExt};
  use rustix::process::{Resource, getrlimit, setrlimit};

  use super::yard;

  /// The events of a life, each under its place in the log.
  type Logged = (Seq, Arc<Event>);

  /// How many times the word of the test draws a number: more than enough facts to pass the tail
  /// after which a life is born again.
  const DRAWS: usize = 2800;

  /// A life that keeps its record, with no extension, whose model closes each request with None.
  fn opening(at: &Path) -> Opening {
    let asked = AtomicUsize::new(0);
    let answer = Answer(Arc::new(move |_, _| {
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
      directory: at.display().to_string(),
      record: Some(at.join("record.jsonl").display().to_string()),
      config: Some(at.join("config").display().to_string()),
      roster: Some(vec!["claude-cli:opus".to_owned()]),
      extensions: Some(false),
      answer: Some(answer),
      ..Opening::default()
    }
  }

  /// The events of a stream, up to the first that the test waits for. A bound of time fails a
  /// test that hangs.
  async fn until(
    events: &mut (impl Stream<Item = Logged> + Unpin),
    wanted: impl Fn(&Event) -> bool,
  ) -> Vec<Logged> {
    let mut got = Vec::new();
    loop {
      let next = tokio::time::timeout(Duration::from_secs(120), events.next()).await;
      let Ok(Some(logged)) = next else {
        let last: Vec<String> =
          got.iter().rev().take(20).map(|one| format!("{:.300}", format!("{one:?}"))).collect();
        panic!("the life told no event in time, after {}", last.join("\n"))
      };
      let found = wanted(&logged.1);
      got.push(logged);
      if found {
        return got;
      }
    }
  }

  /// A word that runs on the root as a turn of the operator, and the events until it is done.
  async fn ran(
    life: &LifeHandle,
    events: &mut (impl Stream<Item = Logged> + Unpin),
    word: &str,
  ) -> Vec<Logged> {
    let made = life.send(Command::Rung { word: word.to_owned(), on: None }).await;
    let Ok(Reply::Act(rung)) = made else { panic!("{word} did not run: {made:?}") };
    until(events, |event| {
      matches!(event, Event::Act(act) if act.id == rung && matches!(act.phase, Phase::Done(_)))
    })
    .await
  }

  /// The largest size of a file that the process writes, as a limit of the system: a write past
  /// it fails, as on a full disk.
  fn limited(bytes: u64) {
    let mut now = getrlimit(Resource::Fsize);
    now.current = Some(now.maximum.map_or(bytes, |most| bytes.min(most)));
    setrlimit(Resource::Fsize, now).expect("the limit of the size of a file is set");
  }

  /// The lines of the record of the life, in order.
  fn lines(at: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(at.join("record.jsonl")).expect("the record reads");
    text.lines().map(str::to_owned).collect()
  }

  #[expect(unsafe_code, reason = "no crate of the lock sets a signal to be ignored with no unsafe")]
  pub(super) async fn a_life_whose_record_takes_no_fact_holds_its_rebirth_and_its_end() {
    let at = yard::Yard::new(format!("furb-host-unkept-{}", std::process::id()));
    let life = Life::spawn(opening(&at)).expect("the life has a thread");
    let mut events = Box::pin(life.events(0));
    let mut log = ran(&life, &mut events, "n = 1").await;

    // A write past the limit raises the signal SIGXFSZ, which ends the process unless it is
    // ignored: the write then fails, as on a full disk.
    // SAFETY: the test sets the action of one signal, and no handler of its own.
    unsafe { libc::signal(libc::SIGXFSZ, libc::SIG_IGN) };
    let size = std::fs::metadata(at.join("record.jsonl")).expect("the record stands").len();
    limited(size);
    log.extend(ran(&life, &mut events, &format!("for i in range({DRAWS}):\n  chance()")).await);
    let refused = life.send(Command::End).await;
    let Err(LifeError::Refused(why)) = refused else {
      panic!("the life ended while its record did not hold its facts: {refused:?}")
    };
    assert!(why.contains("does not hold"), "{why}");
    while let Ok(Some(logged)) =
      tokio::time::timeout(Duration::from_millis(500), events.next()).await
    {
      log.push(logged);
    }
    let rebirths = log.iter().filter(|one| matches!(*one.1, Event::Reborn(_))).count();
    assert_eq!(rebirths, 0, "no rebirth while the record does not hold each fact");
    let held = std::fs::metadata(at.join("record.jsonl")).expect("the record stands").len();
    assert_eq!(held, size, "the record took no fact while the limit stood");

    limited(libc::RLIM_INFINITY);
    until(&mut events, |event| matches!(event, Event::Reborn(None))).await;
    assert!(matches!(life.send(Command::End).await, Ok(Reply::Done)));
    until(&mut events, |event| matches!(event, Event::Ended(None))).await;

    let kept = lines(&at);
    let draws: Vec<usize> = kept
      .iter()
      .enumerate()
      .filter(|(_, line)| line.starts_with("[[\"done\",\"chance"))
      .map(|(at, _)| at)
      .collect();
    assert_eq!(draws.len(), DRAWS, "the record holds each fact that waited");
    let checkpoints: Vec<usize> = kept
      .iter()
      .enumerate()
      .filter(|(_, line)| line.starts_with("[[\"checkpoint\""))
      .map(|(at, _)| at)
      .collect();
    assert_eq!(checkpoints.len(), 2, "the checkpoint of the rebirth, then the one of the end");
    assert!(draws.last() < checkpoints.first(), "the checkpoint of the rebirth follows each fact");
    let again = Life::spawn(opening(&at)).expect("the life has a thread");
    let n = again.send(Command::Inspect { name: "n".to_owned(), on: None }).await;
    assert!(matches!(n, Ok(Reply::Inspected { ref repr, .. }) if repr == "1"), "{n:?}");
    assert!(matches!(again.send(Command::End).await, Ok(Reply::Done)));
  }
}
