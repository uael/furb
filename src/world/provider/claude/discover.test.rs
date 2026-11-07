use std::{
  env, fs,
  path::Path,
  process,
  sync::{Arc, Barrier},
  thread,
};

use super::{Found, entry, fallback, forget, stored, stored_by};

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
    forget(&file);
    end.wait();
    if stored(&file, &stamp, NOW).is_some() {
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
  stored_by(file, stamp, entry(found, NOW), |file, bytes| {
    fs::write(file, bytes).expect("the cache is written");
  });
}
