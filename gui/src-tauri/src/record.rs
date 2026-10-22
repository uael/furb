//! The record that the app reads, on a thread of its own, with no life: no word runs again, no model is asked, no
//! command runs, and a record of any engine opens. The crate reads it with `furb::world::retold`, the one reader of a
//! record that the TUI reads too, and the app hears its facts as events, in the order of the log.

use std::{path::PathBuf, sync::mpsc, thread};

use furb::wire;
use serde_json::Value;

use crate::places;

/// How many facts the app hears in one event.
const PART: usize = 4000;

/// What the reader tells the app.
#[derive(Debug, Clone)]
pub enum Event {
  /// The record is open: the directory of its project, whether its facts are those of its log, and the engine that
  /// made it, as the record names it.
  Opened { directory: String, logged: bool, made: Option<String> },
  /// The next facts of the record, in order.
  Facts(Vec<Value>),
  /// The app heard every fact of the record.
  Read,
  /// A report for the operator: why the record did not open.
  Said(String),
}

/// A record read on a thread of its own, and what the reader tells.
pub fn read(path: PathBuf) -> mpsc::Receiver<Event> {
  let (events, told) = mpsc::channel();
  let spawned = thread::Builder::new().name("furb-record".to_owned()).spawn(move || {
    if let Err(no) = retold(&path, &events) {
      let _ = events.send(Event::Said(no));
    }
  });
  if let Err(no) = spawned {
    eprintln!("furb-gui: the record has no thread: {no}");
  }
  told
}

/// The facts of a record, told to the app in parts, then the end of them.
fn retold(path: &std::path::Path, events: &mpsc::Sender<Event>) -> Result<(), String> {
  let directory = places::project_of(path).display().to_string();
  let furb::world::Retold { made, logged, facts } =
    furb::world::retold(path).map_err(|no| no.to_string())?;
  let _ = events.send(Event::Opened { directory, logged, made });
  for part in facts.chunks(PART) {
    let _ = events.send(Event::Facts(part.iter().map(|one| wire::outward(one.as_ref())).collect()));
  }
  let _ = events.send(Event::Read);
  Ok(())
}
