use std::{
  env, fs,
  path::Path,
  process,
  sync::{Arc, Barrier},
  thread,
};

#[cfg(unix)]
use super::{super::test::Yard, LASTS, Program, RETRY, cached, models_at, stamp, store};
use super::{Found, Told, fallback, forget, stored, stored_by};
#[cfg(unix)]
use crate::testing::{on_reactor, pause};
#[cfg(unix)]
use std::time::Duration;

const NOW: u64 = 1_000_000;

/// The rounds of the race. Each round is one step that a barrier counts, so a load of the machine
/// never changes how many times the race runs.
const ROUNDS: usize = 50;

/// The cache is one file that each life reads, changes and writes whole, so a store that read the
/// file before a refresh removed it must not write it back after: the discovery that the refresh
/// forgot would answer the next models, and a stand would ask the command line nothing. In each
/// round the stores of three other programs start together with the refresh, and the round ends
/// when each of them ended, so the test waits on no time. The file is the test's own, so no other
/// process of the machine writes it.
#[test]
fn a_refresh_leaves_no_discovery_that_a_store_of_another_program_wrote_back() {
  let file = Arc::new(env::temp_dir().join(format!("furb-discover-{}.json", process::id())));
  let (begin, end) = (Arc::new(Barrier::new(4)), Arc::new(Barrier::new(4)));
  let others: Vec<_> = (0..3)
    .map(|n| {
      let (file, begin, end) = (Arc::clone(&file), Arc::clone(&begin), Arc::clone(&end));
      thread::spawn(move || {
        for _ in 0..ROUNDS {
          begin.wait();
          unsynced(&file, &format!("other{n}\0program\0{n}\0{n}"), &fallback());
          end.wait();
        }
      })
    })
    .collect();
  let mut kept = Vec::new();
  for round in 0..ROUNDS {
    let stamp = format!("mine\0program\0{round}\0{round}");
    unsynced(&file, &stamp, &fallback());
    begin.wait();
    forget(&file, "mine\0");
    end.wait();
    if stored(&file, &stamp).is_some() {
      kept.push(round);
    }
  }
  for one in others {
    one.join().expect("a store ends");
  }
  drop(fs::remove_file(&*file));
  assert_eq!(kept, Vec::<usize>::new(), "a discovery that a refresh forgot came back");
}

/// A store of the cache with no sync to the disk.
fn unsynced(file: &Path, stamp: &str, found: &[Found]) {
  let told = Told { at: NOW, found: found.to_vec(), asked: None };
  stored_by(file, stamp, &told, |file, bytes| {
    fs::write(file, bytes).expect("the cache is written");
  });
}

/// The aliases that furb gives of a program at a second.
#[cfg(unix)]
fn aliases(program: &Program, at: u64) -> Option<Vec<String>> {
  on_reactor(models_at(program, at)).map(|found| found.into_iter().map(|one| one.alias).collect())
}

/// Whether something came true before a limit that a loaded machine cannot cross.
#[cfg(unix)]
fn came(true_now: impl Fn() -> bool) -> bool {
  for _ in 0..3000 {
    if true_now() {
      return true;
    }
    pause(Duration::from_millis(20));
  }
  false
}

#[cfg(unix)]
#[test]
fn a_discovery_that_is_old_gives_its_models_with_no_wait_and_the_command_line_is_asked_behind_it() {
  let yard = Yard::new("old");
  let named = |models: &str| fs::write(yard.at.join("models"), models).expect("the models");
  let asked =
    || fs::read_to_string(yard.at.join("discoveries")).unwrap_or_default().lines().count();
  named("opus 100 low\n");
  let program = Program::new(yard.at.join("claude"), Vec::new());
  let opus = Some(vec!["opus".to_owned()]);
  assert_eq!(aliases(&program, NOW), opus, "the models wait for the first answer");
  named("opus 100 low\nfable 100 low\n");
  // The command line answers no request from here on, so models that came did not wait for it.
  fs::write(yard.at.join("slow"), "").expect("the cue of the fake");
  assert_eq!(aliases(&program, NOW + LASTS - 1), opus);
  assert_eq!(asked(), 1, "a discovery that is young asks nothing");
  let later = NOW + 2 * LASTS;
  assert_eq!(
    aliases(&program, later),
    opus,
    "what furb holds answers while the command line does not"
  );
  assert_eq!(aliases(&program, later), opus);
  fs::remove_file(yard.at.join("slow")).expect("the cue goes");
  let both = Some(vec!["opus".to_owned(), "fable".to_owned()]);
  assert!(came(|| aliases(&program, later) == both), "the answer of the ask behind never showed");
  assert_eq!(asked(), 2, "one ask behind answered each read of the old discovery");
  let said =
    |alias: &str| Found { alias: alias.to_owned(), window: 100, efforts: vec!["low".to_owned()] };
  assert_eq!(
    stored(&cached().expect("the cache of the tests"), &stamp(&program)),
    Some(Told { at: later, found: vec![said("opus"), said("fable")], asked: None }),
    "the cache holds what the ask behind heard"
  );
}

#[cfg(unix)]
#[test]
fn a_command_line_that_left_an_ask_with_no_answer_is_not_waited_for_by_a_later_process() {
  let yard = Yard::new("silent");
  let asked =
    || fs::read_to_string(yard.at.join("discoveries")).unwrap_or_default().lines().count();
  let program = Program::new(yard.at.join("claude"), Vec::new());
  // What a process that waited for the command line with no answer left in the cache. This process
  // holds nothing of the program in memory, as a process that starts.
  let left = Told { asked: Some(NOW), ..Told::default() };
  store(&cached().expect("the cache of the tests"), &stamp(&program), &left);
  fs::write(yard.at.join("slow"), "").expect("the cue of the fake");
  assert_eq!(aliases(&program, NOW + RETRY - 1), None);
  assert_eq!(asked(), 0, "an ask that got no answer holds the next one off");
  assert_eq!(aliases(&program, NOW + RETRY), None, "the models did not wait for the command line");
  fs::write(yard.at.join("models"), "opus 100 low\n").expect("the models");
  fs::remove_file(yard.at.join("slow")).expect("the cue goes");
  let opus = Some(vec!["opus".to_owned()]);
  assert!(
    came(|| aliases(&program, NOW + RETRY) == opus),
    "the answer of the ask behind never showed"
  );
  assert_eq!(asked(), 1);
}
