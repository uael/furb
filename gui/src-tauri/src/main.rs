//! furb, the desktop app: the debugger of a living program. Its core links the crate and runs a life on a thread of
//! its own; its window shows the UI of gui/src, which hears the view of the life and sends the commands of the
//! operator.
//!
//! `furb-gui` opens the demo, whose models answer from a script. `--record PATH` opens a record to read, and `--live
//! DIRECTORY` a life on this machine. `--view PATH` runs the demo with no window until its question waits, and writes
//! the view of that moment, which the UI shows in a browser as its fixture.

mod demo;
mod life;
mod model;
mod source;
mod view;

use std::{
  path::PathBuf,
  sync::{Arc, Mutex},
  time::{Duration, Instant},
};

use life::{Command, Event, Life, Source};
use serde_json::Value;
use tauri::{Emitter, Manager};

/// The life as the app holds it: what it said, the cursor in time, and the view of it that the UI draws.
#[derive(Default)]
struct Seen {
  facts: Vec<Value>,
  cursor: Option<usize>,
  place: String,
  works: bool,
  /// What each model writes now, by its rung, and the reports of the life that no model reads.
  writing: std::collections::HashMap<String, String>,
  said: Vec<String>,
}

impl Seen {
  /// One event of the life, taken in.
  fn heard(&mut self, event: Event) {
    match event {
      Event::Opened { directory, works, .. } => {
        let how = if works { "a life that works" } else { "a record to read" };
        eprintln!("furb-gui: opened {how} in {directory}");
        self.place = directory.rsplit(['/', '\\']).next().unwrap_or_default().to_owned();
        self.works = works;
      }
      Event::Facts(facts) => {
        for fact in &facts {
          if fact[0] == "ready" {
            self.writing.remove(fact[1].as_str().unwrap_or_default());
          }
        }
        self.facts.extend(facts);
      }
      Event::Writing { rung, text } => self.writing.entry(rung).or_default().push_str(&text),
      Event::Said(line) => {
        eprintln!("furb-gui: {line}");
        self.said.push(line);
      }
    }
  }

  /// The view of the life at the cursor.
  fn view(&self) -> Value {
    let at = self.cursor.unwrap_or(self.facts.len()).min(self.facts.len());
    let mut life = model::Life::of(&self.facts, at);
    life.len = self.facts.len();
    let mut view = view::view(&life, &self.place, self.works && self.cursor.is_none());
    view["writing"] = serde_json::json!(self.writing);
    view["said"] = serde_json::json!(self.said);
    view
  }
}

/// What the commands of the UI reach: the life and what the app saw of it.
struct Held {
  seen: Arc<Mutex<Seen>>,
  commands: futures::channel::mpsc::UnboundedSender<Command>,
}

#[tauri::command]
fn view(held: tauri::State<'_, Held>) -> Value {
  held.seen.lock().map(|seen| seen.view()).unwrap_or(Value::Null)
}

#[tauri::command]
fn send(held: tauri::State<'_, Held>, command: Command) {
  let _ = held.commands.unbounded_send(command);
}

/// Whether the app runs as a check: it ends once its window drew the threads of the life, or fails after a while.
static CHECK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// What the window drew, which it tells at each view it draws: the threads and the chains of that view.
#[tauri::command]
fn drew(threads: usize, chains: usize) {
  if CHECK.load(std::sync::atomic::Ordering::Relaxed) && threads > 0 {
    println!("furb-gui: the window drew {threads} threads of {chains} chains");
    std::process::exit(0);
  }
}

#[tauri::command]
fn travel(app: tauri::AppHandle, held: tauri::State<'_, Held>, at: Option<usize>) {
  if let Ok(mut seen) = held.seen.lock() {
    seen.cursor = at;
    let _ = app.emit("view", seen.view());
  }
}

/// What the words of the command line ask for: the life, and the view to write with no window.
fn asked(words: &[String]) -> Result<(Source, Option<PathBuf>), String> {
  let mut source = Source::Demo { pace: Duration::from_millis(900) };
  let mut dump = None;
  let mut words = words.iter();
  while let Some(word) = words.next() {
    let mut value = || words.next().cloned().ok_or_else(|| format!("{word} needs a value."));
    match word.as_str() {
      "--demo" => {}
      "--check" => CHECK.store(true, std::sync::atomic::Ordering::Relaxed),
      "--record" => source = Source::Record(value()?.into()),
      "--live" => source = Source::Live { directory: value()?.into(), record: None },
      "--view" => {
        dump = Some(value()?.into());
        source = Source::Demo { pace: Duration::from_millis(250) };
      }
      other => return Err(format!("{other} is no word of furb-gui.")),
    }
  }
  Ok((source, dump))
}

/// The demo run with no window until its question waits and the other workers settle, and its view written.
fn dumped(source: Source, path: &PathBuf) -> Result<(), String> {
  let (life, events) = Life::open(source, || {});
  let mut seen = Seen::default();
  let end = Instant::now() + Duration::from_secs(60);
  let mut asked: Option<Instant> = None;
  while Instant::now() < end {
    if let Ok(event) = events.recv_timeout(Duration::from_millis(50)) {
      seen.heard(event);
    }
    let owed = seen.facts.iter().any(|one| one[0] == "started" && one[2] == "console");
    if owed && asked.is_none() {
      asked = Some(Instant::now());
    }
    if asked.is_some_and(|at| at.elapsed() > Duration::from_secs(4)) {
      break;
    }
  }
  drop(life);
  let text = serde_json::to_string_pretty(&seen.view()).map_err(|no| no.to_string())?;
  std::fs::write(path, text + "\n").map_err(|no| no.to_string())
}

fn main() {
  let words: Vec<String> = std::env::args().skip(1).collect();
  let (source, dump) = match asked(&words) {
    Ok(asked) => asked,
    Err(why) => {
      eprintln!("{why}");
      std::process::exit(2);
    }
  };
  if let Some(path) = dump {
    if let Err(why) = dumped(source, &path) {
      eprintln!("{why}");
      std::process::exit(1);
    }
    return;
  }
  tauri::Builder::default()
    .setup(move |app| {
      if CHECK.load(std::sync::atomic::Ordering::Relaxed) {
        std::thread::spawn(|| {
          std::thread::sleep(Duration::from_secs(45));
          eprintln!("furb-gui: the window drew no thread in 45 seconds");
          std::process::exit(1);
        });
      }
      let handle = app.handle().clone();
      let woke = handle.clone();
      let (life, events) = Life::open(source, move || {
        let _ = woke.emit("woke", ());
      });
      let seen = Arc::new(Mutex::new(Seen::default()));
      let shared = seen.clone();
      std::thread::Builder::new().name("furb-view".to_owned()).spawn(move || {
        while let Ok(event) = events.recv() {
          let Ok(mut seen) = shared.lock() else { return };
          seen.heard(event);
          while let Ok(more) = events.try_recv() {
            seen.heard(more);
          }
          let _ = handle.emit("view", seen.view());
        }
      })?;
      app.manage(Held { seen, commands: life.commands() });
      std::mem::forget(life);
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![view, send, travel, drew])
    .run(tauri::generate_context!())
    .expect("the window of furb opens");
}
