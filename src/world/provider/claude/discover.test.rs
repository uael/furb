use std::{
  env, fs, process,
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering::Relaxed},
  },
  thread,
};

use super::{fallback, forget, store, stored};

const NOW: u64 = 1_000_000;

/// The cache is one file that each life reads, changes and writes whole, so a store that read the
/// file before a refresh removed it must not write it back after: the discovery that the refresh
/// forgot would answer the next models, and a stand would ask the command line nothing. The file is
/// the test's own, so no other process of the machine writes it.
#[test]
fn a_refresh_leaves_no_discovery_that_a_store_of_another_program_wrote_back() {
  let file = env::temp_dir().join(format!("furb-discover-{}.json", process::id()));
  let file = Arc::new(file);
  let stop = Arc::new(AtomicBool::new(false));
  let others: Vec<_> = (0..3)
    .map(|n| {
      let (stop, file) = (Arc::clone(&stop), Arc::clone(&file));
      thread::spawn(move || {
        while !stop.load(Relaxed) {
          store(&file, &format!("other{n}\0program\0{n}\0{n}"), &fallback(), NOW);
        }
      })
    })
    .collect();
  let mut kept = None;
  for round in 0..1000 {
    let stamp = format!("mine\0program\0{round}\0{round}");
    store(&file, &stamp, &fallback(), NOW);
    forget(&file);
    if stored(&file, &stamp, NOW).is_some() {
      kept = Some(round);
      break;
    }
  }
  stop.store(true, Relaxed);
  for one in others {
    one.join().expect("a store ends");
  }
  drop(fs::remove_file(&*file));
  assert_eq!(kept, None, "a discovery that a refresh forgot came back");
}
