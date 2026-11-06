//! The process of each skill that a life plays (D15 of #984). A skill whose manifest names a
//! program under `metadata` as `furb-process` gets a process of its own while a life plays it:
//! the daemon starts it at the tip of the open, for each skill that the record enabled, then at
//! each new fact enable, from the program that the fact names, which the folder that the chain
//! played the skill from gives, and ends it when the life ends or when a new fact enable of the
//! same skill reloads it. The process speaks the protocol as a window does: it dials the daemon at
//! the folder that [`HOST`] gives, says its hello as a skill under the name that [`SKILL`] gives,
//! and attaches the life that [`LIFE`] gives.
//!
//! A process says each act under the name that [`speaker`] gives its skill. A process that ends
//! before its life tells the operator once, under that name, with its exit and the last lines of
//! its stderr, and starts again at the next fact enable of its skill. The end of the life and a
//! reload kill the process with each process that it started, its group on Unix, and so does the
//! drop of its watch, so no process of a skill outlives the daemon.
//!
//! A replay starts no process before the tip, since the record answers each act that a process
//! said. Code plays only by a fact enable, never because a folder holds a skill.

use std::{
  collections::{HashMap, VecDeque},
  ffi::OsStr,
  io,
  path::{Path, PathBuf},
  pin::pin,
  process::{ExitStatus, Stdio},
  time::Duration,
};

use furb::{
  engine::OPERATOR,
  extension::{
    ENABLE, Places,
    skills::{process, program as program_of},
  },
  life::{Log, Opening},
};
use futures::{StreamExt, future};
use serde_json::Value;
use tokio::{
  io::{AsyncBufReadExt, BufReader},
  process::{Child, ChildStderr, Command as Program},
  sync::oneshot,
  task::JoinHandle,
  time::timeout,
};

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

/// How many of the last lines of its stderr the operator reads when a process ends before its life,
/// and how many characters of each line.
const TAIL: usize = 20;
const WIDTH: usize = 1000;

/// How long the watch of a process that ended reads what stays of its stderr, once its group is
/// killed: a process that left the group can hold the pipe open.
const DRAIN: Duration = Duration::from_secs(2);

/// The name under which the process of a skill speaks in a life. The slash keeps it apart from the
/// name of each act, which a replay reads as an act, as the operator speaks under `operator`.
pub(crate) fn speaker(skill: &str) -> String {
  format!("skill/{skill}")
}

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
    let mut running: HashMap<String, Running> = HashMap::new();
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
    future::join_all(running.into_values().map(Running::ended)).await;
  }

  /// The process of a skill started, once its earlier process ended, when its fact enable names
  /// one: the program of the folder that the life played the skill from, or, for a fact of a build
  /// before the program, the program that the folder of the life and its config find by name.
  async fn start(&self, one: Enabled<'_>, running: &mut HashMap<String, Running>) {
    let Enabled { name, program } = one;
    if let Some(old) = running.remove(name) {
      old.ended().await;
    }
    let program = match program {
      Some(named) => program_of(named),
      None => process(name, &self.cwd, &self.places),
    };
    let Some(program) = program else { return };
    match self.spawned(name, &program) {
      Ok(child) => {
        running.insert(name.to_owned(), self.watch(name, &program, child));
      }
      Err(no) => said(
        self.log.as_ref(),
        &format!("the process {} of the skill {name} did not start: {no}", program.display()),
      ),
    }
  }

  /// The watch of a process of a skill that started, on a task of its own.
  fn watch(&self, name: &str, program: &Path, child: Child) -> Running {
    let (end, ends) = oneshot::channel();
    let teller = Teller {
      handle: self.handle.clone(),
      skill: name.to_owned(),
      program: program.display().to_string(),
      log: self.log.clone(),
    };
    let watch = tokio::spawn(watched(Process::of(child), ends, teller));
    Running { end, watch }
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
      .stderr(Stdio::piped())
      .kill_on_drop(true);
    #[cfg(unix)]
    command.process_group(0);
    command.spawn()
  }
}

/// A line of the daemon, in its log, else on stderr.
fn said(log: Option<&Log>, line: &str) {
  match log {
    Some(log) => (log.0)(line),
    None => eprintln!("{line}"),
  }
}

/// A process of a skill that runs, as the life holds it: what asks its end, and its watch.
struct Running {
  end: oneshot::Sender<()>,
  watch: JoinHandle<()>,
}

impl Running {
  /// The process ended for sure, with each process of its group, once its watch is over.
  async fn ended(self) {
    // A watch that is over, after a process that ended by itself, hears no end and needs none.
    self.end.send(()).ok();
    // A watch that panicked dropped its process, and the drop of its group killed the group.
    self.watch.await.ok();
  }
}

/// A process of a skill: the child, its group, and the reader of its stderr, which keeps its last
/// lines.
struct Process {
  child: Child,
  group: Group,
  stderr: JoinHandle<VecDeque<String>>,
}

impl Process {
  fn of(mut child: Child) -> Self {
    let group = Group(child.id());
    let stderr = tokio::spawn(tail(child.stderr.take()));
    Process { child, group, stderr }
  }
}

/// The group of a process, which holds each process that it started unless one left it. A kill
/// ends them all, once, and so does a drop, so that no process of a skill outlives the daemon even
/// when its runtime ends with no word. On a system other than Unix, the group is the process alone,
/// which `kill_on_drop` ends.
struct Group(Option<u32>);

impl Group {
  fn kill(&mut self) {
    let pid = self.0.take();
    #[cfg(unix)]
    if let Some(pid) = pid.and_then(|one| i32::try_from(one).ok()) {
      // SAFETY: killpg reads no memory. A group that is gone gives ESRCH, and nothing stays to end.
      unsafe { libc::killpg(pid, libc::SIGKILL) };
    }
    #[cfg(not(unix))]
    let _: Option<u32> = pid;
  }
}

impl Drop for Group {
  fn drop(&mut self) {
    self.kill();
  }
}

/// The last lines that a process wrote on its stderr, until the pipe closes or fails.
async fn tail(stderr: Option<ChildStderr>) -> VecDeque<String> {
  let mut lines = VecDeque::new();
  let Some(stderr) = stderr else { return lines };
  let mut reader = BufReader::new(stderr);
  let mut line = Vec::new();
  // A pipe that fails ends the tail as a pipe that closes does: the lines read so far stay.
  while reader.read_until(b'\n', &mut line).await.is_ok_and(|read| read > 0) {
    if lines.len() == TAIL {
      lines.pop_front();
    }
    lines.push_back(String::from_utf8_lossy(&line).trim_end().chars().take(WIDTH).collect());
    line.clear();
  }
  lines
}

/// What tells the operator that the process of a skill ended before its life.
struct Teller {
  handle: LifeHandle,
  skill: String,
  program: String,
  log: Option<Log>,
}

/// The watch of a process until its end. An end that the life asks kills the process and its
/// group, and tells nothing. An end of its own kills what stays of its group, and tells the
/// operator once, under the name of its skill, with its exit and the last lines of its stderr.
async fn watched(mut process: Process, ends: oneshot::Receiver<()>, teller: Teller) {
  let status = tokio::select! {
    biased;
    _ = ends => {
      process.group.kill();
      if let Err(no) = process.child.kill().await {
        said(teller.log.as_ref(), &format!("the process {} stays: {no}", teller.program));
      }
      process.stderr.abort();
      return;
    }
    status = process.child.wait() => status,
  };
  process.group.kill();
  let reader = process.stderr.abort_handle();
  let lines = timeout(DRAIN, process.stderr).await;
  reader.abort();
  let lines = lines.ok().and_then(Result::ok).unwrap_or_default();
  teller.told(&status, lines).await;
}

impl Teller {
  /// The operator told, by a message under the name of the skill, that its process ended.
  async fn told(&self, status: &io::Result<ExitStatus>, lines: VecDeque<String>) {
    let Teller { handle, skill, program, log } = self;
    let exit = match status {
      Ok(status) => status.to_string(),
      Err(no) => format!("no status that the daemon could read ({no})"),
    };
    let mut message = format!(
      "The process {program} of the skill {skill} ended with {exit} before its life. It starts \
       again at the next enable of the skill."
    );
    if !lines.is_empty() {
      message.push_str("\nThe last lines of its stderr:\n");
      message.push_str(&Vec::from(lines).join("\n"));
    }
    let prompt = Command::Prompt {
      shape: "None".to_owned(),
      message,
      to: Some(OPERATOR.to_owned()),
      on: None,
    };
    if let Err(no) = handle.send(prompt.by(speaker(skill))).await {
      said(log.as_ref(), &format!("the end of the process {program} is not told: {no}"));
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
