//! The ear of the kits, which hosts the World part of each kit that a life plays: a process, in any
//! language, that answers the acts whose kinds the manifest of its kit names under `furb-acts`,
//! as [`kit`] finds it.
//!
//! The ear learns each kit from its fact enable, and starts its process at the first act of a kind
//! that the kit answers, never at the open, so a replay starts no process: the record answers each
//! act before the tip. It takes that act, says that it started, and speaks to the process on its
//! stdin and its stdout, one frame of JSON for each line, as LSP, DAP and MCP do: the frames of
//! the protocol of the daemon that a kit speaks, its hello, its attach, and the verbs peek, close,
//! run and prompt, which the ear answers under its own name. Once the process attached, the ear
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
  fs,
  path::{Path, PathBuf},
  process::Stdio,
};

use furb_wire::{
  error::ProtocolError,
  event::Event as Told,
  protocol::{Answer, Frame, KITS, Kind, PROTOCOL, Peer, Request, Role, Verb},
};
use futures::{StreamExt, stream};
use serde_json::{Value, json};
use tokio::{
  io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
  process::{Child, ChildStderr, ChildStdin, ChildStdout, Command},
  sync::{mpsc, oneshot},
  time::{Duration, timeout},
};

use super::{Group, Shape, interrupted, skills::Kit, spawn};
use crate::{
  ear::{Co, Ear, Next, ear, runtime, say},
  engine::ROOT,
  fact::Fact,
  skill::{Places, enabled_by},
  value::{Fault, Object},
  wire::{outward, parsed},
  world::skills::kit,
};

/// The variable of the environment of a process that gives the name of its kit.
const SKILL: &str = "FURB_SKILL";

/// The variable of the environment of a process that gives the life that it serves, by the path
/// of its record, which each frame of the ear names.
const LIFE: &str = "FURB_LIFE";

/// The extensions of a module that bun runs, where a module of Python runs on [`python`] and any
/// other program runs as it stands.
const SCRIPTS: [&str; 6] = ["ts", "tsx", "mts", "js", "mjs", "cjs"];

/// How many of the last lines of its stderr the operator reads when a process ends before its
/// life, and how many characters of each line.
const TAIL: usize = 20;
const WIDTH: usize = 1000;

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

/// The ear of the kits of a life whose record is `life` and whose folder is `here`, with the
/// skills of the user in `places`.
pub(crate) fn kits(life: String, here: PathBuf, places: Places) -> Box<dyn Ear> {
  ear(move |mut co: Co<Worked>| async move {
    let mut kits = Kits { life, here, places, played: Vec::new(), held: HashMap::new(), seq: 0 };
    let kept = co.keeps(Vec::<(String, Option<String>)>::new())?;
    for (name, program) in kept.borrow().iter() {
      kits.learn(name, program.as_deref());
    }
    loop {
      match co.next().await {
        Next::Resumed(a) => interrupted(&mut co, &a).await?,
        Next::Heard(a) => {
          if let Some(one) = enabled_by(&a) {
            let mut known = kept.borrow_mut();
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

/// What the ear holds: the life, its folder, the places of the skills, each kit that the life
/// plays in the order of its enables, the kit of each act that a process holds, and how many facts
/// the ear told.
struct Kits {
  life: String,
  here: PathBuf,
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

  /// A fact heard: an act that a kit answers is taken, and each process that attached hears it.
  fn heard(&mut self, co: &mut Co<Worked>, a: &Fact) {
    if a.question() && a.by() != JOURNAL {
      let at = self.played.iter().position(|one| {
        one.kit.as_ref().is_some_and(|kit| kit.acts.iter().any(|kind| kind == a.kind()))
      });
      if let Some(at) = at {
        say(co, Fact::says("started", a.about(), []));
        self.held.insert(a.about().to_owned(), self.played[at].name.clone());
        if self.played[at].running.is_none() {
          self.played[at].running = Some(self.started(co, at));
        }
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

  /// The process of the kit at a place of the list, started, or its process that ends at once
  /// with why it did not start.
  fn started(&self, co: &mut Co<Worked>, at: usize) -> Running {
    let Played { name, kit, .. } = &self.played[at];
    let program = kit.as_ref().map(|one| one.program.clone()).unwrap_or_default();
    let (stdin, lines) = mpsc::unbounded_channel();
    let (stop, stopped) = oneshot::channel();
    let ran = self.spawned(name, &program).map(|(mut child, group)| {
      runtime().spawn(written(child.stdin.take(), lines));
      ran(name.clone(), program.clone(), child, group)
    });
    let work = ran.unwrap_or_else(|no| {
      let why = format!("The process {} of the kit {name} did not start: {no}", program.display());
      stream::once(async move { Worked::Ended(String::new(), why) }).boxed_local()
    });
    let named = name.clone();
    co.work(work.take_until(stopped).map(move |worked| match worked {
      Worked::Ended(_, why) => Worked::Ended(named.clone(), why),
      line @ Worked::Line(..) => line,
    }));
    Running { stdin, waiting: Some(Vec::new()), _stop: stop }
  }

  /// The process of a kit, in the folder of its program, with the variables that name its kit and
  /// its life, and its stdin and its stdout, which carry the frames.
  fn spawned(&self, name: &str, program: &Path) -> Result<(Child, Group), super::Unborn> {
    let runner = match program.extension().and_then(OsStr::to_str) {
      Some(one) if SCRIPTS.contains(&one) => Some(PathBuf::from("bun")),
      Some("py") => Some(python()),
      _ => None,
    };
    let mut command = match runner {
      Some(runner) => {
        let mut command = Command::new(runner);
        command.arg(program);
        command
      }
      None => Command::new(program),
    };
    command
      .current_dir(program.parent().unwrap_or(Path::new(".")))
      .env(LIFE, &self.life)
      .env(SKILL, name)
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .stderr(Stdio::piped())
      .kill_on_drop(true);
    spawn(&mut command)
  }

  /// One line that the process of a kit wrote, answered as a verb of the protocol.
  async fn said(&mut self, co: &mut Co<Worked>, name: &str, line: &str) {
    let Some(at) = self.played.iter().position(|one| one.name == name) else { return };
    let request = match serde_json::from_str::<Request>(line) {
      Ok(request) => request,
      Err(no) => {
        let why = format!("The kit {name} wrote a line that is no verb: {no}");
        if let Some(running) = &self.played[at].running {
          let frame = Frame::refused(None, ProtocolError::Refused { why });
          drop(running.stdin.send(serde_json::to_string(&frame).unwrap_or_default()));
        }
        return;
      }
    };
    let answer = self.answered(co, name, request.verb).await;
    if let Some(running) = self.played[at].running.as_mut() {
      running.reply(request.id, answer);
      if running.waiting.is_some() && attached(line) {
        for line in running.waiting.take().unwrap_or_default() {
          drop(running.stdin.send(line));
        }
      }
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
        let peer = Peer {
          operator: String::new(),
          role: Role::Write,
          kind: Kind::Skill,
          name: Some(name.to_owned()),
        };
        // A kit runs on the machine of its life, which names no other.
        let build = env!("CARGO_PKG_VERSION").to_owned();
        Ok(Answer::Welcome { protocol: PROTOCOL, peer, machine: String::new(), build })
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
      Verb::ViewTree(_) | Verb::ViewEffect(_) => {
        Err("A kit draws its views through its word, and its process draws none.".to_owned())
      }
      other => {
        Err(format!("A kit says hello, attach, peek, close, run and prompt, and no {other:?}."))
      }
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
  serde_json::from_str::<Value>(line).is_ok_and(|one| one["verb"] == json!("attach"))
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
/// lines of its stderr. The group of the process ends with the stream, so a stop of the work ends
/// the process with each process that it started.
fn ran(
  name: String,
  program: PathBuf,
  mut child: Child,
  group: Group,
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
  let end = stream::once(async move {
    let mut group = group;
    let status = child.wait().await;
    let slain = group.slay().err().map(|no| format!(" Its group stays: {no}."));
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

/// The Python that runs a module of a kit: the one beside the program of this process, which the
/// environment that installed furb holds, so the module imports `furb.skill`; else the one on PATH.
fn python() -> PathBuf {
  let name = if cfg!(windows) { "python.exe" } else { "python3" };
  let beside = env::current_exe()
    .and_then(fs::canonicalize)
    .ok()
    .and_then(|program| Some(program.parent()?.join(name)).filter(|one| one.is_file()));
  beside.unwrap_or_else(|| PathBuf::from(name))
}

#[cfg(test)]
#[path = "kits.test.rs"]
mod test;
