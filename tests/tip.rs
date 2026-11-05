//! The state of a life at the tip of a record, as text, to compare two builds on the same record:
//! each chain with its transcript and its turns, each act that a transcript names with its outcome,
//! and each module. A probe that a person runs on a copy of a real record, which opens it to read
//! only:
//!
//! ```sh
//! FURB_PROBE_RECORD=<copy> FURB_PROBE_OUT=<file> FURB_PROBE_DIRECTORY=<directory of the life> \
//!   cargo test -p furb --test tip -- --ignored
//! ```
//!
//! `FURB_PROBE_CONFIG` names the config directory, and `FURB_PROBE_EXTENSIONS` set to anything runs
//! the extensions that it turns on.

use std::{fs, path::Path};

use furb::life::Opening;

/// Each chain with its transcript and its turns, each act that a transcript names with its
/// outcome, and each module.
const STATE: &str = "(lambda chains: (lambda names: '\\n'.join([*[repr(('transcript', c, transcript(c))) for c in chains], \
  *[repr(('turns', c, turns(on=c))) for c in chains], *[repr(('peek', n, get(n), peek(n, '<open>'))) for n in names], \
  *[repr(('module', c, sorted((k, re.sub(r' at 0x[0-9a-f]+', '', repr(v))) for k, v in module(c).items()))) \
  for c in chains]]))(sorted({w for c in chains for x in transcript(c) for w in x[1:3] if isinstance(w, str) and get(w)}, \
  key=lambda n: (re.sub(r'\\d+$', '', n), int(re.sub(r'^\\D+', '', n) or 0)))))([f'chain{n}' for n in range(1, 999) \
  if get(f'chain{n}')])";

#[test]
#[ignore = "a probe: FURB_PROBE_RECORD names a record, FURB_PROBE_OUT the file of its state"]
fn the_state_at_the_tip() {
  let record = std::env::var("FURB_PROBE_RECORD").unwrap();
  let out = std::env::var("FURB_PROBE_OUT").unwrap();
  let directory = std::env::var("FURB_PROBE_DIRECTORY").unwrap();
  let config = Path::new(&out).with_extension("config");
  fs::create_dir_all(&config).unwrap();
  let opening = Opening {
    directory,
    record: Some(record),
    keeps: Some(false),
    inspecting: Some(true),
    extensions: std::env::var("FURB_PROBE_EXTENSIONS").map_or(Some(false), |_| None),
    config: Some(
      std::env::var("FURB_PROBE_CONFIG").unwrap_or_else(|_| config.display().to_string()),
    ),
    ..Opening::default()
  };
  let started = std::time::Instant::now();
  let mut engine = match opening.boot::<String>([]) {
    Ok(one) => one.0,
    Err(why) => {
      fs::write(&out, format!("refused: {why}\n")).unwrap();
      panic!("refused: {why:?}");
    }
  };
  eprintln!("opened in {:?}", started.elapsed());
  let state = engine.word(STATE, vec![]).unwrap();
  fs::write(&out, format!("{}\n", state.as_ref().as_str().unwrap())).unwrap();
}
