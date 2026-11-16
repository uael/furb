//! The ear of the kits, which hosts the World part of each kit that a life plays: a process, in any
//! language, that answers the acts whose kinds the manifest of its kit names under `furb-acts`,
//! as [`kit`] finds it.
//!
//! The ear learns each kit from its fact enable, and starts its process at the first act of a kind
//! that the kit answers, never at the open, so a replay starts no process: the record answers each
//! act before the tip. It takes that act, says that it started, and speaks to the process on its
//! stdin and its stdout, one frame of JSON for each line, as LSP, DAP and MCP do: the frames of
//! the protocol of the daemon that a kit speaks, its hello, its attach, and the verbs peek, close,
//! fact, run, prompt and config, which the ear answers under its own name. Its welcome names the
//! folders of the life, and each verb past the protocol of the first kit, as `fact`, by which a
//! process says what it learned of its own accord, and `config`, by which it reads the configs and
//! the keys of its kit. Once the process attached, the ear
//! tells it each fact that it hears, as an event, the act that it takes among them, and the done
//! of that act, so a process ends the work of an act that a control ended.
//!
//! A kit draws nothing: its word draws its views through the life, so the ear refuses each verb of
//! a view. A new fact enable of a kit ends its process, and the next act starts the new one. A
//! process that ends before its life tells the operator once, with its exit and the last lines of
//! its stderr, and each act that it held is refused with the same reason. The end of the life ends
//! each process with each process that it started.

use std::{
  collections::{HashMap, VecDeque},
  env,
  ffi::OsStr,
  fs, io,
  path::{Path, PathBuf},
  process::{ExitStatus, Stdio},
};

mod uv;

use furb_wire::{
  error::ProtocolError,
  event::Event as Told,
  protocol::{Answer, Config, Folders, Frame, KITS, Kind, PROTOCOL, Peer, Request, Role, Verb},
};
use futures::{FutureExt, StreamExt, future::Shared, stream};
use serde_json::{Value, json};
use tokio::{
  io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
  process::{Child, ChildStderr, ChildStdin, ChildStdout, Command},
  sync::{mpsc, oneshot},
  time::{Duration, timeout},
};

use super::{Group, Shape, Unborn, config::Layers, interrupted, skills::Kit, spawn};
use crate::{
  ear::{Co, Ear, Next, ear, runtime, say},
  engine::ROOT,
  fact::Fact,
  skill::{Places, enabled_by},
  value::{Fault, Object},
  wire::{outward, parsed},
  world::skills::kit,
};
use uv::uv;

/// The variable of the environment of a process that gives the name of its kit.
const SKILL: &str = "FURB_SKILL";

/// The variable of the environment of a process that gives the life that it serves, by the path
/// of its record, which each frame of the ear names.
const LIFE: &str = "FURB_LIFE";

/// The variable of the environment that names the Python that runs a module of a kit.
const PYTHON: &str = "FURB_PYTHON";

/// The extensions of a module that bun runs, where a module of Python runs on [`python`] and any
/// other program runs as it stands.
const SCRIPTS: [&str; 6] = ["ts", "tsx", "mts", "js", "mjs", "cjs"];

/// How many of the last lines of its stderr the operator reads when a process ends before its
/// life, and how many characters of each line.
const TAIL: usize = 20;
const WIDTH: usize = 1000;

/// How long a process of a kit has to end its own work once the ear stopped it: the SDK of MCP
/// gives a server 2 seconds after its stdin closes, then 2 more after a SIGTERM.
const GRACE: Duration = Duration::from_secs(5);

/// How long the ear reads what stays of the stderr of a process that ended: a process that left
/// its group can hold the pipe open.
const DRAIN: Duration = Duration::from_secs(2);

/// The journal, which says the record again, so a fact that it says is of the past.
const JOURNAL: &str = "journal";

/// What a work of the ear came to: a line that the process of a kit wrote, or its end, told as the
/// text that the operator reads.
enum Worked {
  Line(String, String),
  Ended(String, String),
}

/// A kit that the life plays: its name, the program of its fact enable, and its World part, once
/// the ear found it.
struct Played {
  name: String,
  kit: Option<Kit>,
  running: Option<Running>,
}

/// The process of a kit that runs: what writes its stdin, the frames that wait for its attach, and
/// what ends it.
struct Running {
  stdin: mpsc::UnboundedSender<String>,
  waiting: Option<Vec<String>>,
  _stop: oneshot::Sender<()>,
}

impl Running {
  /// A frame for the process: written at once once it attached, else kept until it attaches.
  fn tell(&mut self, frame: &Frame) {
    let line = serde_json::to_string(frame).unwrap_or_default();
    match &mut self.waiting {
      Some(waiting) => waiting.push(line),
      None => drop(self.stdin.send(line)),
    }
  }

  /// The answer to one verb of the process.
  fn reply(&self, id: u64, answer: Result<Answer, String>) {
    let frame = match answer {
      Ok(answer) => Frame::Reply { id, answer },
      Err(why) => Frame::refused(Some(id), ProtocolError::Refused { why }),
    };
    drop(self.stdin.send(serde_json::to_string(&frame).unwrap_or_default()));
  }
}

/// The verbs that a life takes from a kit past those of the protocol [`KITS`], which its welcome
/// names: a new verb is a capability, never a new version.
const CAPABILITIES: [&str; 2] = ["fact", "config"];

/// The ear of the kits of a life whose record is `life`, whose folder is `here` and whose images
/// stand in `images`, with the skills of the user in `places`.
pub(crate) fn kits(
  life: String,
  here: PathBuf,
  images: Option<PathBuf>,
  places: Places,
) -> Box<dyn Ear> {
  ear(move |mut co: Co<Worked>| async move {
    let mut kits =
      Kits { life, here, images, places, played: Vec::new(), held: HashMap::new(), seq: 0 };
    co.keeps(Programs::new())?;
    for (name, program) in co.kept::<Programs>()?.iter() {
      kits.learn(name, program.as_deref());
    }
    loop {
      match co.next().await {
        // An act that an earlier life took and that no kit answers is another ear's to take again.
        Next::Resumed(a) if kits.answering(a.kind()).is_some() => interrupted(&mut co, &a).await?,
        Next::Heard(a) | Next::Resumed(a) => {
          if let Some(one) = enabled_by(&a) {
            let known = co.kept::<Programs>()?;
            known.retain(|(name, _)| *name != one.name);
            known.push((one.name.clone(), one.program.clone()));
            kits.learn(&one.name, one.program.as_deref());
          }
          kits.heard(&mut co, &a);
        }
        Next::Worked(Worked::Line(kit, said)) => kits.said(&mut co, &kit, &said).await,
        Next::Worked(Worked::Ended(kit, why)) => kits.ended(&mut co, &kit, why).await?,
      }
    }
  })
}

/// The program of each kit that the life enabled, by its name, in order.
type Programs = Vec<(String, Option<String>)>;

/// What the ear holds: the life, its folder, the places of the skills, each kit that the life
/// plays in the order of its enables, the kit of each act that a process holds, and how many facts
/// the ear told.
struct Kits {
  life: String,
  here: PathBuf,
  images: Option<PathBuf>,
  places: Places,
  played: Vec<Played>,
  held: HashMap<String, String>,
  seq: u64,
}

impl Kits {
  /// A kit of a fact enable, in the place of the one of its name, whose process ends.
  fn learn(&mut self, name: &str, program: Option<&str>) {
    let found = kit(name, program, &self.here, &self.places);
    let one = Played { name: name.to_owned(), kit: found, running: None };
    match self.played.iter_mut().find(|two| two.name == name) {
      Some(two) => *two = one,
      None => self.played.push(one),
    }
  }

  /// The place in the list of the first kit whose process answers acts of a kind.
  fn answering(&self, kind: &str) -> Option<usize> {
    self
      .played
      .iter()
      .position(|one| one.kit.as_ref().is_some_and(|kit| kit.acts.iter().any(|act| act == kind)))
  }

  /// A fact heard: an act that a kit answers is taken, and each process that attached hears it.
  fn heard(&mut self, co: &mut Co<Worked>, a: &Fact) {
    if a.question()
      && a.by() != JOURNAL
      && let Some(one) = self.answering(a.kind()).and_then(|at| self.played.get_mut(at))
    {
      say(co, Fact::says("started", a.about(), []));
      self.held.insert(a.about().to_owned(), one.name.clone());
      if one.running.is_none() {
        let running = Self::started(&self.life, co, one);
        one.running = Some(running);
      }
    }
    if a.kind() == "done" {
      self.held.remove(a.about());
    }
    self.seq += 1;
    let frame = Frame::Event {
      life: self.life.clone(),
      seq: self.seq,
      event: Told::Fact(outward(a.0.as_ref())),
    };
    for running in self.played.iter_mut().filter_map(|one| one.running.as_mut()) {
      running.tell(&frame);
    }
  }

  /// The process of a kit of a life, started, or its process that ends at once with why it did not
  /// start.
  fn started(life: &str, co: &mut Co<Worked>, played: &Played) -> Running {
    let Played { name, kit, .. } = played;
    let program = kit.as_ref().map(|one| one.program.clone()).unwrap_or_default();
    let (stdin, lines) = mpsc::unbounded_channel();
    let (stop, stopped) = oneshot::channel();
    let (life, name, stopped) = (life.to_owned(), name.clone(), stopped.shared());
    let ending = stopped.clone();
    let work = stream::once(async move {
      match spawned(&life, &name, &program).await {
        Ok((mut child, group)) => {
          runtime().spawn(written(child.stdin.take(), lines));
          ran(name, program, (child, group), ending)
        }
        Err(no) => {
          let why = format!("The process {} of the kit {name} {no}", program.display());
          stream::once(async move { Worked::Ended(name, why) }).boxed_local()
        }
      }
    });
    co.work(work.flatten().take_until(stopped));
    Running { stdin, waiting: Some(Vec::new()), _stop: stop }
  }

  /// The process of the kit at a place of the list, while it runs.
  fn running(&mut self, at: usize) -> Option<&mut Running> {
    self.played.get_mut(at).and_then(|one| one.running.as_mut())
  }

  /// One line that the process of a kit wrote, answered as a verb of the protocol.
  async fn said(&mut self, co: &mut Co<Worked>, name: &str, line: &str) {
    let Some(at) = self.played.iter().position(|one| one.name == name) else { return };
    let request = match serde_json::from_str::<Request>(line) {
      Ok(request) => request,
      Err(no) => {
        let why = format!("The kit {name} wrote a line that is no verb: {no}");
        if let Some(running) = self.running(at) {
          let frame = Frame::refused(None, ProtocolError::Refused { why });
          drop(running.stdin.send(serde_json::to_string(&frame).unwrap_or_default()));
        }
        return;
      }
    };
    let answer = self.answered(co, name, request.verb).await;
    if let Some(running) = self.running(at) {
      running.reply(request.id, answer);
      if running.waiting.is_some() && attached(line) {
        for line in running.waiting.take().unwrap_or_default() {
          drop(running.stdin.send(line));
        }
      }
    }
  }

  /// The welcome of the process of a kit: the kit as a peer that writes, the verbs that the life
  /// takes past the first protocol of a kit, and the folders of the life.
  fn welcome(&self, name: &str) -> Answer {
    let peer = Peer {
      operator: String::new(),
      role: Role::Write,
      kind: Kind::Skill,
      name: Some(name.to_owned()),
    };
    let shown = |one: &Path| one.display().to_string();
    let folders = Folders {
      project: shown(&self.here),
      config: shown(&self.places.config),
      images: self.images.as_deref().map(shown),
    };
    // A kit runs on the machine of its life, which names no other.
    Answer::Welcome {
      protocol: PROTOCOL,
      peer,
      machine: String::new(),
      build: env!("CARGO_PKG_VERSION").to_owned(),
      program: None,
      capabilities: CAPABILITIES.map(str::to_owned).to_vec(),
      folders: Some(folders),
      home: None,
    }
  }

  /// What a verb of the process of a kit comes to.
  async fn answered(&self, co: &mut Co<Worked>, name: &str, verb: Verb) -> Result<Answer, String> {
    let fault = |no: Fault| no.message();
    match verb {
      Verb::Hello(hello) if hello.kind == Kind::Skill => {
        if !(KITS..=PROTOCOL).contains(&hello.protocol) {
          return Err(format!("This life speaks the protocol {KITS} to {PROTOCOL} to a kit."));
        }
        Ok(self.welcome(name))
      }
      Verb::Attach { .. } => Ok(Answer::Done),
      Verb::Peek { act, .. } => {
        if self.held.contains_key(&act) {
          return Err(format!("No act named {act} is done."));
        }
        let got = co.call("peek", vec![Object::string(&act)], vec![]).await.map_err(fault)?;
        Ok(Answer::Value(outward(got.as_ref())))
      }
      // The ear owns each act that it gave a kit, so it says its done; any other act closes as
      // the verb close closes it.
      Verb::Close { act, value, .. } => {
        let value = parsed(&value).map_err(fault)?;
        if self.held.contains_key(&act) {
          say(co, Fact::says("done", &act, [value]));
        } else {
          co.call("close", vec![value], vec![("id", Object::string(&act))]).await.map_err(fault)?;
        }
        Ok(Answer::Done)
      }
      Verb::Fact { kind, about, words, .. } => {
        let words = words.iter().map(|one| parsed(one)).collect::<Result<Vec<_>, _>>();
        say(co, Fact::says(&kind, &about, words.map_err(fault)?));
        Ok(Answer::Done)
      }
      Verb::Run { word, on, .. } => {
        let on = vec![("on", Object::string(on.unwrap_or_else(|| ROOT.to_owned())))];
        let made = co.call("rung", vec![Object::string(word)], on).await.map_err(fault)?;
        Ok(Answer::Act(made.as_ref().as_str().unwrap_or_default().to_owned()))
      }
      Verb::Prompt { shape, message, to, on, .. } => {
        let mut with = vec![("message", Object::string(message))];
        with.extend(to.map(|to| ("to", Object::string(to))));
        with.push(("on", Object::string(on.unwrap_or_else(|| ROOT.to_owned()))));
        let made = co.call("prompt", vec![Shape::object(&shape)], with).await.map_err(fault)?;
        Ok(Answer::Act(made.as_ref().as_str().unwrap_or_default().to_owned()))
      }
      // A kit reads the configs as a window reads them, each key with its value, its layer and its
      // property, and writes none: the operator changes them, and the kit hears the fact of each
      // change.
      Verb::Config { config: Config::Get, .. } => {
        let layers = Layers::new(&self.here, Some(self.places.config.clone()));
        let settings = layers.get().map_err(|no| no.to_string())?;
        Ok(Answer::Values(settings.into_iter().map(|one| json!(one)).collect()))
      }
      Verb::ViewTree(_) | Verb::ViewEffect(_) => {
        Err("A kit draws its views through its word, and its process draws none.".to_owned())
      }
      other => Err(format!(
        "A kit says hello, attach, peek, close, fact, run, prompt and config get, and no {other:?}."
      )),
    }
  }

  /// The process of a kit ended before its life: each act that it held is refused with why, and
  /// the operator reads why once.
  async fn ended(&mut self, co: &mut Co<Worked>, name: &str, why: String) -> Result<(), Fault> {
    if let Some(one) = self.played.iter_mut().find(|one| one.name == name) {
      one.running = None;
    }
    let held: Vec<String> =
      self.held.iter().filter(|(_, kit)| *kit == name).map(|(act, _)| act.clone()).collect();
    for act in held {
      self.held.remove(&act);
      say(co, Fact::says("done", &act, [Fault::refused(why.clone()).object()]));
    }
    let to = [("to", Object::string("operator")), ("on", Object::string(ROOT))];
    co.call("prompt", vec![Object::none(), Object::string(why)], to.into()).await.map(drop)
  }
}

/// Whether a line of a process is its attach, after which it hears the facts of the life.
fn attached(line: &str) -> bool {
  serde_json::from_str::<Value>(line).is_ok_and(|one| one.get("verb") == Some(&json!("attach")))
}

/// The process of a kit, in the folder of its program, with the variables that name its kit and
/// its life, and its stdin and its stdout, which carry the frames: a module of Python whose header
/// names what it needs, as PEP 723 says, runs on [`uv`], which makes that environment; another
/// module of Python on [`python`]; a module of TypeScript or of JavaScript on bun; any other
/// program as it stands.
async fn spawned(life: &str, name: &str, program: &Path) -> Result<(Child, Group), Unstarted> {
  let runner: Option<(PathBuf, &[&str])> = match program.extension().and_then(OsStr::to_str) {
    Some(one) if SCRIPTS.contains(&one) => Some((PathBuf::from("bun"), &[])),
    Some("py") if headed(program) => Some((uv().await?, &["run", "--quiet", "--script"])),
    Some("py") => Some((python(), &[])),
    _ => None,
  };
  let mut command = match runner {
    Some((runner, before)) => {
      let mut command = Command::new(runner);
      command.args(before).arg(program);
      command
    }
    None => Command::new(program),
  };
  command
    .current_dir(program.parent().unwrap_or(Path::new(".")))
    .env(LIFE, life)
    .env(SKILL, name)
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .kill_on_drop(true);
  Ok(spawn(&mut command)?)
}

/// Why the process of a kit did not start, which follows its name.
#[derive(Debug, thiserror::Error)]
enum Unstarted {
  #[error("did not start: {0}")]
  Uv(#[from] uv::Error),
  #[error(transparent)]
  Unborn(#[from] Unborn),
}

/// Whether a module of Python holds a header of PEP 723, which opens on this line.
fn headed(program: &Path) -> bool {
  fs::read_to_string(program).is_ok_and(|text| text.lines().any(|line| line == "# /// script"))
}

/// Each line for the stdin of a process, written in order until the process or the ear goes.
async fn written(input: Option<ChildStdin>, mut lines: mpsc::UnboundedReceiver<String>) {
  let Some(mut input) = input else { return };
  while let Some(mut line) = lines.recv().await {
    line.push('\n');
    if input.write_all(line.as_bytes()).await.is_err() {
      return;
    }
  }
}

/// The lines that a process writes on its stdout, then its end, told with its exit and the last
/// lines of its stderr. A stop of the work ends the process with each process that it started, as
/// [`waited`] says.
fn ran(
  name: String,
  program: PathBuf,
  (mut child, group): (Child, Group),
  stopped: Shared<oneshot::Receiver<()>>,
) -> stream::LocalBoxStream<'static, Worked> {
  let stderr = runtime().spawn(tail(child.stderr.take()));
  let lines = child.stdout.take().map(|one| BufReader::new(one).lines());
  let said = stream::unfold(lines, |lines: Option<Lines<BufReader<ChildStdout>>>| async move {
    let mut lines = lines?;
    let line = lines.next_line().await.ok().flatten()?;
    Some((line, Some(lines)))
  });
  let line_of = name.clone();
  let said = said.map(move |line| Worked::Line(line_of.clone(), line));
  let waiting = runtime().spawn(waited(child, group, stopped));
  let end = stream::once(async move {
    let (status, slain) = waiting.await.unwrap_or_else(|no| (Err(io::Error::other(no)), None));
    let lines = timeout(DRAIN, stderr).await.ok().and_then(Result::ok).unwrap_or_default();
    let exit = status
      .map_or_else(|no| format!("no status that furb could read ({no})"), |one| one.to_string());
    let mut why = format!(
      "The process {} of the kit {name} ended with {exit} before its life. It starts again at the next act of the kit.{}",
      program.display(),
      slain.unwrap_or_default()
    );
    if !lines.is_empty() {
      why.push_str("\nThe last lines of its stderr:\n");
      why.push_str(&Vec::from(lines).join("\n"));
    }
    Worked::Ended(name, why)
  });
  said.chain(end).boxed_local()
}

/// The exit of the process of a kit, and why its group stays when it does. A stop of the ear closes
/// the stdin of the process, which then ends its own work, as the servers that the process
/// started in a session of their own, within [`GRACE`]; after it, the ear ends the process. The
/// group ends either way, with each process left in it. This runs on the runtime, so it goes on
/// past the end of the ear that stopped.
async fn waited(
  mut child: Child,
  mut group: Group,
  stopped: Shared<oneshot::Receiver<()>>,
) -> (io::Result<ExitStatus>, Option<String>) {
  let status = tokio::select! {
    status = child.wait() => status,
    _ = stopped => match timeout(GRACE, child.wait()).await {
      Ok(status) => status,
      Err(_) => child.kill().await.and(child.wait().await),
    },
  };
  (status, group.slay().err().map(|no| format!(" Its group stays: {no}.")))
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

/// The Python that runs a module of a kit with no header: the one that [`PYTHON`] names, which the
/// door of Python names as its own; else the one beside the program of this process, which the
/// environment that installed furb holds, so the module imports `furb.skill`; else the one on PATH.
fn python() -> PathBuf {
  let name = if cfg!(windows) { "python.exe" } else { "python3" };
  let named = env::var_os(PYTHON).filter(|one| !one.is_empty()).map(PathBuf::from);
  named.or_else(|| beside(name)).unwrap_or_else(|| PathBuf::from(name))
}

/// The program of a name in the folder of the program of this process.
fn beside(name: &str) -> Option<PathBuf> {
  let program = env::current_exe().and_then(fs::canonicalize).ok()?;
  Some(program.parent()?.join(name)).filter(|one| one.is_file())
}

#[cfg(test)]
#[path = "kits.test.rs"]
mod test;
