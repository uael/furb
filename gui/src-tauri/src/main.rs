//! furb, the desktop app, as a preview: it reads the records of furb. Its core links the crate and reads a record with
//! `furb::world::retold` on a thread of its own, so no word runs again, no model is asked and no command runs, and it
//! sends the acts of the record to its window; the window, the UI of gui/src, derives each view from the acts with the
//! model of a life that the TUI reads too.
//!
//! `furb-gui` opens on a picker of the projects and of their records. `--record RECORD` opens a record to read.
//! `--check` ends the app once its window drew the whole record, and prints the count of its chains, threads and acts.
//! `--view PATH` reads the record that `--record` names with no window, and writes it as the window takes it, which
//! the UI shows in a browser as its fixture.

mod model;
mod places;
mod record;

#[cfg(test)]
#[path = "main.test.rs"]
mod tests;

use std::{
  path::{Path, PathBuf},
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
  },
  time::Duration,
};

use record::Event;
use serde_json::{Value, json};
use tauri::{Emitter, Manager};

/// The chain that every record of furb stands on.
const ROOT: &str = "chain1";

/// How many of the last lines that the app said it keeps and sends the window.
const SAID: usize = 50;

/// The record as the app holds it: its acts, where it stands, how far the app read it, its reports, and how far the
/// window heard it.
#[derive(Default)]
struct Seen {
  now: model::Life,
  directory: String,
  place: String,
  /// Whether the facts are those of the log beside the record, and the engine that made the record.
  logged: bool,
  made: Option<String>,
  /// Whether the app read every fact of the record.
  read: bool,
  said: Vec<String>,
  /// Whether the app said a line after the last part that it sent the window.
  told: bool,
  /// What the app last sent the window of the measures, and whether it told the window that it read every fact.
  shown: Value,
  ended: bool,
  /// What the app opens, until the record is open.
  opening: Option<String>,
  open: bool,
  /// Which record of the app it is, and the count of changes of its acts that the app last sent the window.
  generation: u64,
  sent: u64,
}

impl Seen {
  /// One event of the reader, taken in.
  fn heard(&mut self, event: Event) {
    match event {
      Event::Opened { directory, logged, made } => {
        eprintln!("furb-gui: opened a record to read in {directory}");
        self.place = directory.rsplit(['/', '\\']).next().unwrap_or_default().to_owned();
        self.directory = directory;
        self.logged = logged;
        self.made = made;
        self.open = true;
        self.opening = None;
      }
      Event::Facts(facts) => {
        for fact in facts {
          self.now.add(&model::lean(fact));
        }
      }
      Event::Read => self.read = true,
      Event::Said(line) => {
        eprintln!("furb-gui: {line}");
        if !self.open {
          self.opening = None;
        }
        self.said.push(line);
        let over = self.said.len().saturating_sub(SAID);
        self.said.drain(..over);
        self.told = true;
      }
    }
  }

  /// What every part of an open record carries: where it stands, and how far the app read it.
  fn head(&self, whole: bool, from: u64) -> Value {
    json!({
      "state": "open",
      "whole": whole,
      "generation": self.generation,
      "from": from,
      "count": self.now.count,
      "len": self.now.len,
      "root": ROOT,
      "place": self.place,
      "directory": self.directory,
      "logged": self.logged,
      "made": self.made,
      "read": self.read,
    })
  }

  /// The acts that changed after a count, and the words of their rungs.
  fn changed(&self, since: u64) -> (Vec<Value>, Value) {
    let acts = self.now.since(since);
    let words: serde_json::Map<String, Value> = acts
      .iter()
      .filter_map(|act| act["id"].as_str())
      .filter_map(|id| self.now.words.get(id).map(|word| (id.to_owned(), json!(word))))
      .collect();
    (acts, Value::Object(words))
  }

  /// The record as the window takes it whole: what the app shows, and for an open record the acts that changed after
  /// a count, with the words of their rungs and the measures of the chains.
  fn life(&self, since: u64) -> Value {
    let mut life = match (&self.opening, self.open) {
      (_, true) => self.head(true, since),
      (Some(what), false) => json!({"state": "opening", "what": what}),
      (None, false) => json!({"state": "picker"}),
    };
    life["generation"] = json!(self.generation);
    life["said"] = json!(self.said);
    if self.open {
      let (acts, words) = self.changed(since);
      life["acts"] = json!(acts);
      life["words"] = words;
      life["measures"] = self.now.measures();
    }
    life
  }

  /// What changed after the last part that the app sent the window, or nothing when nothing changed: the acts that
  /// changed, with the words of their rungs, and the measures, the end of the reading and the last lines that the app
  /// said, each when it changed.
  fn frame(&mut self) -> Option<Value> {
    if !self.open {
      return std::mem::take(&mut self.told).then(|| self.life(0));
    }
    let (acts, words) = self.changed(self.sent);
    let mut changed = !acts.is_empty();
    let mut part = self.head(false, self.sent);
    part["acts"] = json!(acts);
    part["words"] = words;
    if self.sent != self.now.count {
      let measures = self.now.measures();
      if measures != self.shown {
        part["measures"] = measures.clone();
        self.shown = measures;
        changed = true;
      }
    }
    if self.read != self.ended {
      self.ended = self.read;
      changed = true;
    }
    if std::mem::take(&mut self.told) {
      part["said"] = json!(self.said);
      changed = true;
    }
    self.sent = self.now.count;
    changed.then_some(part)
  }
}

/// What the commands of the UI reach: what the app saw of the record, which record is the last that the app opened,
/// and the config directory of the user.
struct Held {
  seen: Arc<Mutex<Seen>>,
  generation: Arc<AtomicU64>,
  config: PathBuf,
}

/// Whether the app runs as a check: it ends once its window drew the whole record, or fails after a while.
static CHECK: AtomicBool = AtomicBool::new(false);

/// The record at a path, and a few words for the operator while it opens.
fn opened(path: &str) -> Result<(PathBuf, String), String> {
  let path = PathBuf::from(path);
  if !path.is_file() {
    return Err(format!("There is no record at {}", path.display()));
  }
  let name = path.file_stem().and_then(|one| one.to_str()).unwrap_or_default().to_owned();
  Ok((path, format!("the record {name}")))
}

/// A record to read, in place of the one that the app held: the app forgets what it saw, reads the record, and tells
/// the window what changed in it, at most once each 80 milliseconds, and only when something changed.
fn start(app: &tauri::AppHandle, held: &Held, path: PathBuf, what: String) {
  let generation = held.generation.fetch_add(1, Ordering::SeqCst) + 1;
  eprintln!("furb-gui: opening {what}");
  if let Ok(mut seen) = held.seen.lock() {
    *seen = Seen { opening: Some(what), generation, ..Seen::default() };
    let _ = app.emit("life", seen.life(0));
  }
  let events = record::read(path);
  let (shared, handle, current) = (held.seen.clone(), app.clone(), held.generation.clone());
  let spawned = std::thread::Builder::new().name("furb-view".to_owned()).spawn(move || {
    while let Ok(event) = events.recv() {
      if current.load(Ordering::SeqCst) != generation {
        return;
      }
      if let Ok(mut seen) = shared.lock() {
        seen.heard(event);
        while let Ok(more) = events.try_recv() {
          seen.heard(more);
        }
        if let Some(part) = seen.frame() {
          let _ = handle.emit("life", part);
        }
      }
      std::thread::sleep(Duration::from_millis(80));
    }
  });
  if let Err(no) = spawned {
    eprintln!("furb-gui: the view has no thread: {no}");
  }
}

#[tauri::command]
fn life(held: tauri::State<'_, Held>, since: u64) -> Value {
  held.seen.lock().map(|seen| seen.life(since)).unwrap_or(Value::Null)
}

#[tauri::command]
fn places(held: tauri::State<'_, Held>) -> Value {
  places::listed(&held.config)
}

#[tauri::command]
fn open(app: tauri::AppHandle, held: tauri::State<'_, Held>, path: String) -> Result<(), String> {
  let (path, what) = opened(&path)?;
  start(&app, &held, path, what);
  Ok(())
}

#[tauri::command]
fn close(app: tauri::AppHandle, held: tauri::State<'_, Held>) {
  let generation = held.generation.fetch_add(1, Ordering::SeqCst) + 1;
  if let Ok(mut seen) = held.seen.lock() {
    *seen = Seen { generation, ..Seen::default() };
    let _ = app.emit("life", seen.life(0));
  }
}

/// What the window drew of the whole record, which it tells once it took the last part: the chains, the threads and
/// the acts of the record. A check prints them and ends.
#[tauri::command]
fn drew(chains: usize, threads: usize, acts: usize) {
  if CHECK.load(Ordering::Relaxed) {
    println!("furb-gui: the window drew {chains} chains, {threads} threads and {acts} acts");
    std::process::exit(0);
  }
}

/// What the words of the command line ask for: the record to read, if any, and the view to write with no window.
fn asked(words: &[String]) -> Result<(Option<String>, Option<PathBuf>), String> {
  let mut record = None;
  let mut dump = None;
  let mut words = words.iter();
  while let Some(word) = words.next() {
    let mut value = || words.next().cloned().ok_or_else(|| format!("{word} needs a value."));
    match word.as_str() {
      "--check" => CHECK.store(true, Ordering::Relaxed),
      "--record" => record = Some(value()?),
      "--view" => dump = Some(value()?.into()),
      other => return Err(format!("{other} is no word of furb-gui.")),
    }
  }
  Ok((record, dump))
}

/// A record read with no window, and its fixture written: the record as the window takes it, with the places that the
/// picker lists.
fn dumped(record: &Path, path: &Path) -> Result<(), String> {
  let mut seen = Seen { opening: Some("the record".to_owned()), ..Seen::default() };
  for event in record::read(record.to_path_buf()) {
    if let Event::Said(why) = event {
      return Err(why);
    }
    seen.heard(event);
  }
  let mut fixture = seen.life(0);
  fixture["places"] = places::listed(&furb::extension::Places::here().config);
  let text = serde_json::to_string_pretty(&fixture).map_err(|no| no.to_string())?;
  std::fs::write(path, text + "\n").map_err(|no| no.to_string())
}

fn main() {
  let words: Vec<String> = std::env::args().skip(1).collect();
  let (record, dump) = match asked(&words) {
    Ok(asked) => asked,
    Err(why) => {
      eprintln!("{why}");
      std::process::exit(2);
    }
  };
  let first = match record.as_deref().map(opened).transpose() {
    Ok(first) => first,
    Err(why) => {
      eprintln!("furb-gui: {why}");
      std::process::exit(1);
    }
  };
  if let Some(path) = dump {
    let Some((record, _)) = first else {
      eprintln!("--view needs --record.");
      std::process::exit(2);
    };
    if let Err(why) = dumped(&record, &path) {
      eprintln!("{why}");
      std::process::exit(1);
    }
    return;
  }
  let config = furb::extension::Places::here().config;
  tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .setup(move |app| {
      if CHECK.load(Ordering::Relaxed) {
        std::thread::spawn(|| {
          std::thread::sleep(Duration::from_secs(600));
          eprintln!("furb-gui: the window drew no whole record in 600 seconds");
          std::process::exit(1);
        });
      }
      let held = Held { seen: Arc::default(), generation: Arc::default(), config };
      if let Some((path, what)) = first {
        start(app.handle(), &held, path, what);
      }
      app.manage(held);
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![life, places, open, close, drew])
    .run(tauri::generate_context!())
    .expect("the window of furb opens");
}
