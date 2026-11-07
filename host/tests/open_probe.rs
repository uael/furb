//! The time that a life takes to open on a copy of a real record, as a person measures it: the
//! moment from the spawn of the life to its first reply to the standing, or to its end when its
//! open is refused. A probe, which a person runs on a copy of a record with a line of facts:
//!
//! ```sh
//! FURB_PROBE_RECORD=<copy> FURB_PROBE_DIRECTORY=<directory of the life> \
//!   cargo test -p furb-host --test open_probe -- --ignored --nocapture
//! ```
//!
//! The life keeps its record, so the probe writes to the copy and never to a record of a person.

#![expect(
  clippy::disallowed_methods,
  reason = "the test blocks on the runtime that tokio starts for it"
)]

use std::env;
use std::process;
use std::time::Instant;

use furb::life::Opening;
use furb_host::{Command, Life};

#[tokio::test]
#[ignore = "a probe: FURB_PROBE_RECORD names a copy of a record, FURB_PROBE_DIRECTORY its folder"]
async fn the_time_to_open_a_life_on_a_record() {
  let record = env::var("FURB_PROBE_RECORD").expect("FURB_PROBE_RECORD names a record");
  let directory = env::var("FURB_PROBE_DIRECTORY").expect("FURB_PROBE_DIRECTORY names a folder");
  let config = env::temp_dir().join(format!("furb-open-probe-{}", process::id()));
  let opening = Opening {
    directory,
    record: Some(record),
    config: Some(config.display().to_string()),
    extensions: Some(false),
    ..Opening::default()
  };
  let started = Instant::now();
  let life = Life::spawn(opening).expect("the life has a thread");
  let standing = life.send(Command::Standing).await;
  eprintln!("the life answered in {:?}: {}", started.elapsed(), standing.is_ok());
  life.send(Command::End).await.ok();
}
