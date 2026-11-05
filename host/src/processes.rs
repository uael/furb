//! The process of each skill that a life plays (D15 of #984). A skill whose manifest names a
//! program under `metadata` as `furb-process` gets a process of its own while a life plays it:
//! the daemon starts it at the tip of the open, for each skill that the record enabled, then at
//! each new fact enable, from the program that the fact names, which the folder that the chain
//! played the skill from gives, and ends it when the life ends or when a new fact enable of the
//! same skill reloads it. The process speaks the protocol as a window does: it dials the daemon at
//! the folder that [`HOST`] gives, says its hello as a skill under the name that [`SKILL`] gives,
//! and attaches the life that [`LIFE`] gives.
//!
//! A replay starts no process before the tip, since the record answers each act that a process
//! said. Code plays only by a fact enable, never because a folder holds a skill.

use std::{
  collections::HashMap,
  ffi::OsStr,
  io,
  path::{Path, PathBuf},
  pin::pin,
  process::Stdio,
};

use furb::{
  extension::{
    ENABLE, Places,
    skills::{process, program as program_of},
  },
  life::{Log, Opening},
};
use futures::StreamExt;
use serde_json::Value;
use tokio::process::{Child, Command as Program};

use crate::{
  event::Event,
  life::{Command, LifeHandle, Reply},
};

/// The variable of the environment of a process that gives the folder where the daemon listens.
pub const HOST: &str = "FURB_HOST";

/// The variable of the environment of a process that gives the life that it serves, by the path of
/// its record.
pub const LIFE: &str = "FURB_LIFE";

/// The variable of the environment of a process that gives the name of its skill.
pub const SKILL: &str = "FURB_SKILL";

/// The extensions of a module that bun runs, where any other program runs as it stands.
const SCRIPTS: [&str; 6] = ["ts", "tsx", "mts", "js", "mjs", "cjs"];

/// A life whose skills get their processes: its handle, its record, its folder, where the daemon
/// listens, where the skills of the user stand, and the log of the daemon.
pub(crate) struct Played {
  pub(crate) handle: LifeHandle,
  pub(crate) life: String,
  pub(crate) cwd: PathBuf,
  pub(crate) host: PathBuf,
  pub(crate) places: Places,
  pub(crate) log: Option<Log>,
}

/// Where the skills of the user stand for the lives of an opening: its config folder, when it
/// gives one.
pub(crate) fn places(opening: &Opening) -> Places {
  let here = Places::here();
  match &opening.config {
    Some(config) => Places { config: PathBuf::from(config), ..here },
    None => here,
  }
}

impl Played {
  /// The process of each skill that the life plays, from the tip of its open until its end.
  pub(crate) async fn run(self) {
    let enables = Command::Transcript { on: None, of: Some(ENABLE.to_owned()) };
    let Ok(Reply::Values(tip)) = self.handle.send(enables).await else { return };
    let mut running: HashMap<String, Child> = HashMap::new();
    let mut named: Vec<Enabled> = Vec::new();
    for one in tip.iter().filter_map(enabled) {
      match named.iter_mut().find(|two| two.name == one.name) {
        Some(two) => *two = one,
        None => named.push(one),
      }
    }
    for one in named {
      self.start(one, &mut running).await;
    }
    // The log says again each fact enable of the tip before the facts said after it.
    let mut replayed = tip.len();
    let mut events = pin!(self.handle.events(0));
    while let Some((_, event)) = events.next().await {
      match &*event {
        Event::Fact(fact) => match enabled(fact) {
          Some(_) if replayed > 0 => replayed -= 1,
          Some(one) => self.start(one, &mut running).await,
          None => {}
        },
        Event::Ended(_) => break,
        _ => {}
      }
    }
    for (_, mut child) in running {
      child.kill().await.ok();
    }
  }

  /// The process of a skill started, once its earlier process ended, when its fact enable names
  /// one: the program of the folder that the life played the skill from, or, for a fact of a build
  /// before the program, the program that the folder of the life and its config find by name.
  async fn start(&self, one: Enabled<'_>, running: &mut HashMap<String, Child>) {
    let Enabled { name, program } = one;
    if let Some(mut child) = running.remove(name) {
      child.kill().await.ok();
    }
    let program = match program {
      Some(named) => program_of(named),
      None => process(name, &self.cwd, &self.places),
    };
    let Some(program) = program else { return };
    match self.spawned(name, &program) {
      Ok(child) => {
        running.insert(name.to_owned(), child);
      }
      Err(no) => self.report(&format!(
        "the process {} of the skill {name} did not start: {no}",
        program.display()
      )),
    }
  }

  /// The process of a skill, in the folder of its program, with the variables that say where it
  /// dials and what it serves.
  fn spawned(&self, name: &str, program: &Path) -> io::Result<Child> {
    let script =
      program.extension().and_then(OsStr::to_str).is_some_and(|one| SCRIPTS.contains(&one));
    let mut command = if script {
      let mut bun = Program::new("bun");
      bun.arg(program);
      bun
    } else {
      Program::new(program)
    };
    command
      .current_dir(program.parent().unwrap_or(Path::new(".")))
      .env(HOST, &self.host)
      .env(LIFE, &self.life)
      .env(SKILL, name)
      .stdin(Stdio::null())
      .stdout(Stdio::null())
      .kill_on_drop(true)
      .spawn()
  }

  /// A line of the daemon, in its log, else on stderr.
  fn report(&self, line: &str) {
    match &self.log {
      Some(log) => (log.0)(line),
      None => eprintln!("{line}"),
    }
  }
}

/// What a fact enable says of the process of its skill: the name of the skill, and the program
/// that the fact names, empty when the folder of the skill names none, and none for a fact of a
/// build before the program.
struct Enabled<'a> {
  name: &'a str,
  program: Option<&'a str>,
}

/// What a fact enable carries of the process of its skill, as JSON reads the fact: its kind, its
/// about and its speaker, then the name, the word, the life word, the root, the file, the digest
/// and the program.
fn enabled(fact: &Value) -> Option<Enabled<'_>> {
  let words = fact.as_array()?;
  (words.first()?.as_str()? == ENABLE).then_some(())?;
  let name = words.get(3)?.as_str()?;
  Some(Enabled { name, program: words.get(9).and_then(Value::as_str) })
}
