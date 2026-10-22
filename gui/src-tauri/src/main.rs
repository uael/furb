//! furb, the desktop app: the debugger of a living program. Its core links the crate and runs a life on a thread of
//! its own, and sends the acts of the life to its window; the window, the UI of gui/src, derives each view from the
//! acts with the model of a life that the TUI reads too, and sends the commands of the operator.
//!
//! `furb-gui` opens on a picker of the projects and of their records. `--live DIRECTORY` opens a new life in a
//! project, `--continue RECORD` the life of a record of a project, `--record RECORD` a record to read, which the crate
//! reads with `furb::world::retold`, so no word runs again, no model is asked and no command runs, and `--demo` the
//! demo, whose models answer from a script. `--check` ends the app once its window drew the life, or the whole record,
//! and prints the count of its chains, threads and acts. `--view PATH` runs the demo, or reads the record that
//! `--record` names, with no window until it rests, and writes the life as the window takes it, which the UI shows in a
//! browser as its fixture.

mod demo;
mod life;
mod model;
mod places;

#[cfg(test)]
#[path = "main.test.rs"]
mod tests;

use std::{
  collections::HashMap,
  path::{Path, PathBuf},
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
  },
  time::{Duration, Instant},
};

use life::{Break, Command, Event, Life, Source};
use serde_json::{Value, json};
use tauri::{Emitter, Manager};

/// How many of the last lines that the app said it keeps and sends the window.
const SAID: usize = 50;

/// What a model writes for a rung while its reply runs: its text and its thought.
#[derive(Debug, Default, Clone, PartialEq, serde::Serialize)]
struct Writing {
  text: String,
  thinking: String,
}

/// The life as the app holds it: its acts, what it is, how far the app read it, what its models write now, the
/// breakpoints, its reports, and how far the window heard it.
#[derive(Default)]
struct Seen {
  now: model::Life,
  root: String,
  directory: String,
  place: String,
  works: bool,
  /// Whether the facts are those of a whole log, and the engine that made a record to read.
  logged: bool,
  made: Option<String>,
  /// Whether the app read every fact of a record, which a life that works holds from its opening.
  read: bool,
  writing: HashMap<String, Writing>,
  breaks: Vec<Break>,
  said: Vec<String>,
  /// Whether the app said a line, and whether what the models write or the breakpoints changed, after the last part
  /// that it sent the window.
  told: bool,
  wrote: bool,
  /// What the app last sent the window of the measures and of the prompts that the operator owes, and whether it told
  /// the window that it read every fact.
  shown: Value,
  owed: Vec<String>,
  ended: bool,
  /// What the app opens, until the life is open.
  opening: Option<String>,
  open: bool,
  /// Which life of the app it is, and the count of changes of its acts that the app last sent the window.
  generation: u64,
  sent: u64,
}

impl Seen {
  /// One event of the life, taken in.
  fn heard(&mut self, event: Event) {
    match event {
      Event::Opened { root, directory, works, logged, made } => {
        let how = if works { "a life that works" } else { "a record to read" };
        eprintln!("furb-gui: opened {how} in {directory}");
        self.place = directory.rsplit(['/', '\\']).next().unwrap_or_default().to_owned();
        self.works = works;
        self.root = root;
        self.directory = directory;
        self.logged = logged;
        self.made = made;
        self.read = works;
        self.open = true;
        self.opening = None;
      }
      Event::Facts(facts) => {
        for fact in facts {
          if fact[0] == "ready"
            && self.writing.remove(fact[1].as_str().unwrap_or_default()).is_some()
          {
            self.wrote = true;
          }
          self.now.add(&model::lean(fact));
        }
      }
      Event::Writing { rung, text, thinking } => {
        let held = self.writing.entry(rung).or_default();
        held.text.push_str(&text);
        held.thinking.push_str(&thinking);
        self.wrote = true;
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

  /// What every part of an open life carries: what it is, where it stands, how far the app read it, the prompts that
  /// the operator owes, and the breakpoints.
  fn head(&self, whole: bool, from: u64) -> Value {
    json!({
      "state": "open",
      "whole": whole,
      "generation": self.generation,
      "from": from,
      "count": self.now.count,
      "len": self.now.len,
      "root": self.root,
      "place": self.place,
      "directory": self.directory,
      "works": self.works,
      "logged": self.logged,
      "made": self.made,
      "read": self.read,
      "owed": self.now.owed(),
      "breaks": self.breaks,
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

  /// The life as the window takes it whole: what the app shows, and for an open life the acts that changed after a
  /// count, with the words of their rungs, the measures of the chains, and what the models write now.
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
      life["streams"] = json!(self.writing);
    }
    life
  }

  /// What changed after the last part that the app sent the window, or nothing when nothing changed: the acts that
  /// changed, with the words of their rungs, and the measures, what the models write, the end of the reading and the
  /// last lines that the app said, each when it changed.
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
    if std::mem::take(&mut self.wrote) {
      part["streams"] = json!(self.writing);
      changed = true;
    }
    let owed = self.now.owed();
    if owed != self.owed {
      self.owed = owed;
      changed = true;
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

  /// The bindings that a chain told, as the watch of the window shows them.
  fn watch(&self, chain: &str) -> Value {
    let bindings = self.now.bindings(chain);
    let listed = bindings
      .iter()
      .map(|one| json!({"name": one.name, "value": one.value, "at": one.at, "act": one.act}));
    Value::Array(listed.collect())
  }
}

/// What the operator picks to open: a project, on one of its records or on a new one; a record to read; or the demo.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "lowercase")]
enum Pick {
  Project { directory: String, record: Option<String> },
  Record(String),
  Demo,
}

/// What the commands of the UI reach: what the app saw of the life, the life, which life is the last that the app
/// opened, and the config directory of the user.
struct Held {
  seen: Arc<Mutex<Seen>>,
  life: Mutex<Option<Life>>,
  generation: Arc<AtomicU64>,
  config: PathBuf,
}

/// Whether the app runs as a check: it ends once its window drew the life, or fails after a while.
static CHECK: AtomicBool = AtomicBool::new(false);

/// The source of the life that a pick opens, and a few words for the operator while it opens. A project gets a new
/// record when the pick names none, and the app keeps it among the projects it opened, but in a check.
fn opened(config: &Path, pick: Pick) -> Result<(Source, String), String> {
  match pick {
    Pick::Demo => Ok((Source::Demo { pace: Duration::from_millis(900) }, "the demo".to_owned())),
    Pick::Record(path) => {
      let path = PathBuf::from(path);
      if !path.is_file() {
        return Err(format!("There is no record at {}", path.display()));
      }
      let name = path.file_stem().and_then(|one| one.to_str()).unwrap_or_default().to_owned();
      Ok((Source::Record(path), format!("the record {name}")))
    }
    Pick::Project { directory, record } => {
      let directory = PathBuf::from(directory);
      if !directory.is_dir() {
        return Err(format!("There is no folder at {}", directory.display()));
      }
      let record = match record {
        Some(one) => PathBuf::from(one),
        None => places::fresh(&directory)?,
      };
      if !CHECK.load(Ordering::Relaxed) {
        places::remember(config, &directory)?;
      }
      let name = directory.file_name().and_then(|one| one.to_str()).unwrap_or_default().to_owned();
      Ok((Source::Live { directory, record }, format!("a life in {name}")))
    }
  }
}

/// A new life, in place of the one that the app held: the app forgets what it saw, opens the life, and tells the
/// window what changed in it, at most once each 80 milliseconds, and only when something changed.
fn start(app: &tauri::AppHandle, held: &Held, source: Source, what: String) {
  let generation = held.generation.fetch_add(1, Ordering::SeqCst) + 1;
  if let Ok(mut life) = held.life.lock() {
    life.take();
  }
  eprintln!("furb-gui: opening {what}");
  if let Ok(mut seen) = held.seen.lock() {
    *seen = Seen { opening: Some(what), generation, ..Seen::default() };
    let _ = app.emit("life", seen.life(0));
  }
  let woke = app.clone();
  let (life, events) = Life::open(source, move || {
    let _ = woke.emit("woke", ());
  });
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
  if let Ok(mut place) = held.life.lock() {
    *place = Some(life);
  }
}

#[tauri::command]
fn life(held: tauri::State<'_, Held>, since: u64) -> Value {
  held.seen.lock().map(|seen| seen.life(since)).unwrap_or(Value::Null)
}

#[tauri::command]
fn watch(held: tauri::State<'_, Held>, chain: String) -> Value {
  held.seen.lock().map(|seen| seen.watch(&chain)).unwrap_or(Value::Null)
}

#[tauri::command]
fn places(held: tauri::State<'_, Held>) -> Value {
  places::listed(&held.config)
}

#[tauri::command]
fn open(app: tauri::AppHandle, held: tauri::State<'_, Held>, pick: Pick) -> Result<(), String> {
  let (source, what) = opened(&held.config, pick)?;
  start(&app, &held, source, what);
  Ok(())
}

#[tauri::command]
fn close(app: tauri::AppHandle, held: tauri::State<'_, Held>) {
  let generation = held.generation.fetch_add(1, Ordering::SeqCst) + 1;
  if let Ok(mut life) = held.life.lock() {
    life.take();
  }
  if let Ok(mut seen) = held.seen.lock() {
    *seen = Seen { generation, ..Seen::default() };
    let _ = app.emit("life", seen.life(0));
  }
}

#[tauri::command]
fn send(app: tauri::AppHandle, held: tauri::State<'_, Held>, command: Command) {
  if let Command::Breaks(breaks) = &command
    && let Ok(mut seen) = held.seen.lock()
  {
    seen.breaks.clone_from(breaks);
    let _ = app.emit("life", seen.life(seen.sent));
  }
  if let Ok(life) = held.life.lock()
    && let Some(life) = life.as_ref()
  {
    let _ = life.commands().unbounded_send(command);
  }
}

/// What the window drew, which it tells once it took the last part of a record, or each part of a life that works once
/// a chain stands: the chains, the threads and the acts. A check prints them and ends.
#[tauri::command]
fn drew(chains: usize, threads: usize, acts: usize) {
  if CHECK.load(Ordering::Relaxed) {
    println!("furb-gui: the window drew {chains} chains, {threads} threads and {acts} acts");
    std::process::exit(0);
  }
}

/// What the words of the command line ask for: what to open, if anything, and the view to write with no window.
fn asked(words: &[String]) -> Result<(Option<Pick>, Option<PathBuf>), String> {
  let mut pick = None;
  let mut dump = None;
  let mut words = words.iter();
  while let Some(word) = words.next() {
    let mut value = || words.next().cloned().ok_or_else(|| format!("{word} needs a value."));
    match word.as_str() {
      "--demo" => pick = Some(Pick::Demo),
      "--check" => CHECK.store(true, Ordering::Relaxed),
      "--record" => pick = Some(Pick::Record(value()?)),
      "--live" => pick = Some(Pick::Project { directory: value()?, record: None }),
      "--continue" => {
        let record = value()?;
        let directory = places::project_of(Path::new(&record)).display().to_string();
        pick = Some(Pick::Project { directory, record: Some(record) });
      }
      "--view" => dump = Some(value()?.into()),
      other => return Err(format!("{other} is no word of furb-gui.")),
    }
  }
  Ok((pick, dump))
}

/// A life run with no window until it rests, and its fixture written: the life as the window takes it, with the watch
/// of each chain, and for a record the places that the picker lists. The demo rests once a question waits for the
/// operator, and no model writes and no command runs for a second; a record rests once the app read all of it.
fn dumped(source: Source, path: &Path) -> Result<(), String> {
  let demo = matches!(source, Source::Demo { .. });
  let (life, events) = Life::open(source, || {});
  let mut seen = Seen { opening: Some("the life".to_owned()), ..Seen::default() };
  let end = Instant::now() + Duration::from_secs(if demo { 90 } else { 900 });
  let mut rests: Option<Instant> = None;
  while Instant::now() < end {
    if let Ok(event) = events.recv_timeout(Duration::from_millis(50)) {
      seen.heard(event);
      while let Ok(more) = events.try_recv() {
        seen.heard(more);
      }
      rests = None;
    }
    if !seen.open && seen.opening.is_none() && !seen.said.is_empty() {
      return Err(seen.said.join("\n"));
    }
    if !demo && seen.read {
      break;
    }
    let settled = demo && !seen.now.owed().is_empty() && !seen.now.busy();
    rests = if settled { rests.or_else(|| Some(Instant::now())) } else { None };
    if rests.is_some_and(|at| at.elapsed() > Duration::from_secs(1)) {
      break;
    }
  }
  drop(life);
  let mut fixture = seen.life(0);
  let watch: serde_json::Map<String, Value> =
    seen.now.chains().into_iter().map(|chain| (chain.clone(), seen.watch(&chain))).collect();
  fixture["watch"] = Value::Object(watch);
  if !demo {
    fixture["places"] = places::listed(&furb::extension::Places::here().config);
  }
  let text = serde_json::to_string_pretty(&fixture).map_err(|no| no.to_string())?;
  std::fs::write(path, text + "\n").map_err(|no| no.to_string())
}

fn main() {
  let words: Vec<String> = std::env::args().skip(1).collect();
  let (pick, dump) = match asked(&words) {
    Ok(asked) => asked,
    Err(why) => {
      eprintln!("{why}");
      std::process::exit(2);
    }
  };
  if let Some(path) = dump {
    let source = match pick {
      Some(Pick::Record(record)) => Source::Record(record.into()),
      _ => Source::Demo { pace: Duration::from_millis(250) },
    };
    if let Err(why) = dumped(source, &path) {
      eprintln!("{why}");
      std::process::exit(1);
    }
    return;
  }
  let config = furb::extension::Places::here().config;
  let first = match pick.map(|one| opened(&config, one)).transpose() {
    Ok(first) => first,
    Err(why) => {
      eprintln!("furb-gui: {why}");
      std::process::exit(1);
    }
  };
  tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .setup(move |app| {
      if CHECK.load(Ordering::Relaxed) {
        std::thread::spawn(|| {
          std::thread::sleep(Duration::from_secs(600));
          eprintln!("furb-gui: the window drew no life in 600 seconds");
          std::process::exit(1);
        });
      }
      let held =
        Held { seen: Arc::default(), life: Mutex::new(None), generation: Arc::default(), config };
      if let Some((source, what)) = first {
        start(app.handle(), &held, source, what);
      }
      app.manage(held);
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![life, watch, places, open, close, send, drew])
    .run(tauri::generate_context!())
    .expect("the window of furb opens");
}
