//! Commands: each runs in a shell of the machine, in the working directory of its chain, says what
//! it writes as it writes it, and ends at its exit, at its timeout, or at the done that a control
//! said of it first.

mod group;
mod kept;
mod shell;

use std::{
  collections::{HashMap, HashSet},
  future,
  path::PathBuf,
  pin::pin,
  process::{Command, Stdio},
  time::Duration,
};

use futures::{
  FutureExt, StreamExt,
  channel::mpsc::{TrySendError, UnboundedSender, unbounded},
  future::FusedFuture,
  stream,
};
use tokio::{
  io::AsyncWriteExt,
  process::ChildStdin,
  task::JoinHandle,
  time::{Instant, sleep, sleep_until},
};
use unsync::{oneshot, spsc};

pub(crate) use self::group::{Group, grouped};
#[cfg(test)]
pub(super) use self::kept::whole;
pub use self::shell::{program, shell};
use self::{
  kept::{HEAD, Kept, Stream, TAIL, Unread, read},
  shell::{reaped, spawned},
};
use super::{Scratch, here};
use crate::{
  ear::{Co, Ear, Next, hearing, say},
  engine::left,
  fact::{Fact, named},
  value::{Exit, Fault, Given, Object, Text},
};

/// The ear of commands: it takes a command, says what it writes, answers a write of a door of it,
/// feeds it what a write of its stdin gives, and says it done with its exit.
///
/// A command that the record shows started and not done was cut by the end of the life that ran it:
/// the ear runs it no more, and when a wake puts it to the ear, the ear takes it and closes it with
/// a refusal that says it was interrupted, so the model sees that outcome and runs it again if it
/// wants, and its doors keep what it wrote before. It takes no write.
///
/// A stream keeps at most `HEAD` bytes at its start and `TAIL` bytes at its end: the ear reads the
/// pipe to its end, says the first bytes as they come and then a line that says the rest is cut,
/// and its exit gives the first bytes, a line that counts the bytes cut, and the last bytes.
///
/// A command runs with the PATH of the login shell of the user, read once, ahead of the PATH of the
/// process, so it finds the programs that the profile of the user adds. A command runs with
/// `TMPDIR` set to the scratch directory of the life, and `TMP` and `TEMP` too on Windows, which
/// the ear makes first, so a command finds there the files of `scratch://`. A command ends with
/// every process it started, which its group holds. A command that a control ended first, which the
/// engine says done, is ended so and says nothing more.
pub fn bash(scratch: Scratch, record: &[Object]) -> Box<dyn Ear> {
  let mut commands =
    Commands { scratch, running: HashMap::new(), closed: HashSet::new(), cut: crashed(record) };
  hearing(&["bash", "write"], move |mut co: Co<Said>| async move {
    loop {
      match co.next().await {
        Next::Heard(a) => commands.heard(&mut co, &a).await?,
        Next::Worked(one) => commands.worked(&mut co, one).await?,
      }
    }
  })
}

/// What the ear of the commands holds: the scratch directory of the life, each command that runs,
/// the commands whose stdin a write closed, and each command that a crash cut, so that a later act
/// that only shares its name, as after a drift, is no command of the record.
struct Commands {
  scratch: Scratch,
  running: HashMap<String, Running>,
  closed: HashSet<String>,
  cut: HashMap<String, Held>,
}

impl Commands {
  /// What a command says while it runs, which a control that ended it first silences.
  async fn worked(&mut self, co: &mut Co<Said>, said: Said) -> Result<(), Fault> {
    let saying = match said {
      Said::Out { about, text, stream } if self.running.contains_key(&about) => {
        Fact::says("out", &about, [Object::string(text), Object::string(stream.to_string())])
      }
      Said::Exit { about, exit } if self.running.remove(&about).is_some() => {
        Fact::says("done", &about, [exit.object()])
      }
      // The machine would not start it, so the ear closes it with why, as a prompt that the
      // operator cannot answer is closed, and the chain is told why.
      Said::Failed { about, fault } if self.running.remove(&about).is_some() => {
        return closing(co, &about, fault).await;
      }
      _ => return Ok(()),
    };
    say(co, saying);
    Ok(())
  }

  /// What a fact that the ear hears asks of it.
  async fn heard(&mut self, co: &mut Co<Said>, a: &Fact) -> Result<(), Fault> {
    let about = a.about().to_owned();
    match a.kind() {
      "bash" if a.question() && self.cut.get(&about).is_some_and(|it| *it == held(a)) => {
        self.cut.remove(&about);
        interrupted(co, a, &about).await
      }
      "bash" if a.question() => self.started(co, a, about).await,
      "write" if a.question() => self.wrote(co, a, &about).await,
      "done" => {
        self.running.remove(&about);
        self.closed.remove(&about);
        self.cut.remove(&about);
        Ok(())
      }
      _ => Ok(()),
    }
  }

  /// A command that starts: its stdin and its stop kept, and its streams worked.
  async fn started(&mut self, co: &mut Co<Said>, a: &Fact, about: String) -> Result<(), Fault> {
    say(co, Fact::says("started", &about, []));
    match begun(co, a, &self.scratch).await {
      Ok(mut begun) => {
        let (stdin, fed_in) = spsc::unbounded();
        let (stop, stopped) = oneshot::channel();
        let mut one = Running { stdin, _stop: stop };
        // A stdin of text goes in whole at the start, and then it closes.
        if let Some(text) = begun.given.take().filter(|text| !text.is_empty()) {
          one.feed(Some(text))?;
          one.feed(None)?;
        }
        co.work(command(begun, fed_in, stopped));
        self.running.insert(about, one);
        Ok(())
      }
      // The machine would not start it, so the ear closes it with why, as a prompt that the
      // operator cannot answer is closed, and the chain is told why.
      Err(fault) => closing(co, &about, fault).await,
    }
  }

  /// A write of a door of a command: the stdin of a command whose stdin is None takes it while the
  /// command runs and until a write of nothing closes it, and every other door takes no write.
  async fn wrote(&mut self, co: &mut Co<Said>, a: &Fact, about: &str) -> Result<(), Fault> {
    let Some(at) = door(a) else { return Ok(()) };
    let act = co.call("get", vec![Object::string(&at.command)], vec![]).await?;
    let Some(act) = Fact::of(act.as_ref()) else { return Ok(()) };
    let over = co.call("over", vec![Object::string(&at.command)], vec![]).await?;
    let open = at.stdin
      && over.as_ref().as_bool() != Some(true)
      && act.word(2).is_some_and(|one| one.type_name() == "NoneType")
      && !self.closed.contains(&at.command)
      && !self.cut.contains_key(&at.command);
    let answer = if open {
      match self.written(&at.command, &at.text.content) {
        Ok(()) => at.text.object(),
        Err(fault) => fault.object(),
      }
    } else {
      Fault::refused(format!("{} takes no write", at.text.path)).object()
    };
    say(co, Fact::says("done", about, [answer]));
    Ok(())
  }

  /// What a write of the stdin of a command gives it: the text, or nothing for an empty text, which
  /// closes the stdin.
  fn written(&mut self, command: &str, content: &str) -> Result<(), Fault> {
    let text = (!content.is_empty()).then(|| content.to_owned());
    if text.is_none() {
      self.closed.insert(command.to_owned());
    }
    self.running.get_mut(command).map_or(Ok(()), |one| one.feed(text))
  }
}

/// The line of a command, as its act gives it.
fn command_line(a: &Fact) -> String {
  a.word(1).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default()
}

/// The chain and the line of a command, which say whether an act is the command of the record under
/// its name.
#[derive(PartialEq)]
struct Held {
  on: String,
  line: String,
}

fn held(a: &Fact) -> Held {
  Held { on: a.on().to_owned(), line: command_line(a) }
}

/// Each command that the record shows started and not done, which a crash cut, with its chain and
/// its line.
fn crashed(record: &[Object]) -> HashMap<String, Held> {
  let left: HashSet<String> = left(record).into_iter().collect();
  record
    .iter()
    .filter_map(|one| Fact::of(*one.as_ref().items()?.first()?))
    .filter(|a| a.kind() == "bash" && a.question() && left.contains(a.about()))
    .map(|a| (a.about().to_owned(), held(&a)))
    .collect()
}

/// A command that a crash cut, started and closed with a refusal that says it was interrupted.
async fn interrupted(co: &mut Co<Said>, a: &Fact, about: &str) -> Result<(), Fault> {
  say(co, Fact::says("started", about, []));
  let line = command_line(a);
  let why =
    Fault::refused(format!("{line:?} was interrupted: the life that ran it ended before it did"));
  closing(co, about, why).await
}

/// An act closed with a fault, as the engine closes a prompt that nobody can answer.
async fn closing(co: &mut Co<Said>, about: &str, fault: Fault) -> Result<(), Fault> {
  co.call("close", vec![fault.object()], vec![("id", Object::string(about))]).await?;
  Ok(())
}

/// A write of a door of a command: the text, the command, and whether the door is its stdin.
pub(super) struct Door {
  text: Text,
  pub(super) command: String,
  stdin: bool,
}

/// The write of a door of a command, when the act is one: the stdin takes a write, the stdout and
/// the stderr do not.
pub(super) fn door(a: &Fact) -> Option<Door> {
  let text = a.word(1).and_then(Text::of)?;
  let (command, stream) = text.path.split_once('/')?;
  let (command, stdin, out) =
    (command.to_owned(), stream == "stdin", matches!(stream, "stdout" | "stderr"));
  (named(&command, "bash") && (stdin || out)).then_some(Door { text, command, stdin })
}

/// One command that runs, as the ear holds it: the door to its stdin, and its stop, which ends it
/// when it drops.
struct Running {
  stdin: spsc::Sender<Option<String>>,
  _stop: oneshot::Sender<()>,
}

impl Running {
  /// One text into its stdin, or nothing to close it. A command that reads no more refuses a text.
  /// A stdin that reads no more is closed already.
  fn feed(&mut self, text: Option<String>) -> Result<(), Fault> {
    let closing = text.is_none();
    match self.stdin.try_send(text) {
      Err(_) if !closing => Err(Fault::refused("the command reads no more")),
      _ => Ok(()),
    }
  }
}

/// What a command says: a text that one of its streams wrote, its exit, or why the machine would
/// not start it.
enum Said {
  Out { about: String, text: String, stream: Stream },
  Exit { about: String, exit: Exit },
  Failed { about: String, fault: Fault },
}

/// A command ready to start: its act, its shell with its words, how it is shown, its directory, its
/// timeout, and the text that its stdin is given, or nothing when its stdin stays open for what is
/// fed to it.
struct Begun {
  about: String,
  command: Command,
  shown: String,
  dir: PathBuf,
  timeout: Option<Duration>,
  given: Option<String>,
}

/// A command begun: its shell made ready in the directory of its chain, with the scratch directory
/// as its `TMPDIR`.
async fn begun(co: &mut Co<Said>, a: &Fact, scratch: &Scratch) -> Result<Begun, Fault> {
  let about = a.about().to_owned();
  let on = a.on().to_owned();
  let line = command_line(a);
  let shown = format!("{line:?}");
  // A stdin of text goes in whole and closes, None stays open for what is fed, and any other word,
  // as the flag of a record made before the stdin was a text, gives a stdin that is empty and
  // closed.
  let given = match a.word(2) {
    Some(one) if one.type_name() == "NoneType" => None,
    one => Some(one.and_then(|it| it.as_str()).unwrap_or_default().to_owned()),
  };
  let timeout = a.word(3).and_then(|one| one.seconds());
  // A timeout past what the machine counts runs to the end of the command, as no timeout does.
  let timeout = timeout.and_then(|seconds| Duration::try_from_secs_f64(seconds).ok());
  let dir = here(co, &on).await?;
  let asked = vec![Object::string("merged"), Object::string(on), Object::string(about.clone())];
  let merged = co.call("ask", asked, vec![]).await?.as_ref().as_bool().unwrap_or_default();
  let mut command = Command::new(shell());
  command
    .arg("-c")
    .arg(if merged { format!("exec 2>&1\n{line}") } else { line })
    .current_dir(&dir)
    .stdin(if given.as_deref() == Some("") { Stdio::null() } else { Stdio::piped() })
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
  let tmp = scratch.made()?;
  command.env("TMPDIR", tmp);
  if cfg!(windows) {
    command.env("TMP", tmp).env("TEMP", tmp);
  }
  Ok(Begun { about, command, shown, dir, timeout, given })
}

/// What a command says as it runs: each text its streams write, then its exit; or nothing more once
/// it stops.
fn command(
  begun: Begun,
  fed: spsc::Receiver<Option<String>>,
  stopped: oneshot::Receiver<()>,
) -> impl futures::Stream<Item = Said> {
  let (says, said) = unbounded();
  let ran = ran(begun, fed, stopped, says);
  // The command says through the channel alone, so its exit comes after all it wrote.
  stream::select(said, stream::once(ran).filter_map(future::ready))
}

/// A command run to its end: what its streams write is said as it comes, what is fed goes into its
/// stdin, and its exit is said at its end; or the command is ended at its stop, with every process
/// of it.
///
/// A command that outlives its timeout is ended with every process of it, and exits with no code.
/// Its exit comes once its shell exited and its streams closed, or once its shell exited and DRAIN
/// passed with a stream open, as when a process that the command started holds a stream: then the
/// exit gives what its streams wrote until then, and that process is left to run, with no reader on
/// its stream. The shell is reaped as it exits, and its group keeps its number for as long as a
/// process of it lives, so an end signals no group that another took, but in the moment after the
/// last process of the group ended and before that end.
///
/// What the channel cannot take, because its stream is going, comes back for that stream to give.
/// What ended the wait on a command before its exit.
enum Cut {
  Stopped,
  Unread(Unread),
  Unslain(std::io::Error),
  Unwaited(std::io::Error),
}

/// How a command exited: the code of its shell, and whether its timeout ended it.
struct Exited {
  status: Option<i32>,
  late: bool,
}

/// Where a command stands: it runs; its shell exited and its streams have DRAIN to give what they
/// hold, which is the one timeout that follows the exit; or it is done.
enum Phase {
  Running,
  Draining { status: Option<i32>, until: Instant },
  Done(Option<i32>),
}

/// The wait on a command through its three phases. A stop ends it at once, a fault of a stream or a
/// signal that cannot reach the group ends it with that fault, and the timeout of the command ends
/// its processes at any time.
async fn watch(
  shell: JoinHandle<std::io::Result<Option<i32>>>,
  streams: impl Future<Output = Result<(), Unread>>,
  expired: impl Future<Output = ()>,
  feeding: impl Future<Output = ()>,
  stopped: oneshot::Receiver<()>,
  group: &mut Group,
) -> Result<Exited, Cut> {
  let (mut shell, mut streams) = (pin!(shell), pin!(streams.fuse()));
  let (mut expired, mut feeding) = (pin!(expired.fuse()), pin!(feeding.fuse()));
  let mut stopped = pin!(stopped);
  let mut phase = Phase::Running;
  loop {
    let until = if let Phase::Draining { until, .. } = phase { Some(until) } else { None };
    tokio::select! {
      biased;
      _ = &mut stopped => return Err(Cut::Stopped),
      got = &mut shell, if matches!(phase, Phase::Running) => {
        let status = got.map_err(std::io::Error::other).and_then(|waited| waited).map_err(Cut::Unwaited)?;
        phase = Phase::Draining { status, until: Instant::now() + DRAIN };
      }
      read = &mut streams => read.map_err(Cut::Unread)?,
      () = &mut expired => group.slay().map_err(Cut::Unslain)?,
      () = &mut feeding => {}
      () = sleep_until(until.unwrap_or_else(Instant::now)), if until.is_some() => {
        if let Phase::Draining { status, .. } = phase {
          phase = Phase::Done(status);
        }
      }
    }
    if let Phase::Draining { status, .. } = phase
      && streams.is_terminated()
    {
      phase = Phase::Done(status);
    }
    if let Phase::Done(status) = phase {
      return Ok(Exited { status, late: expired.is_terminated() });
    }
  }
}

async fn ran(
  begun: Begun,
  fed: spsc::Receiver<Option<String>>,
  stopped: oneshot::Receiver<()>,
  says: UnboundedSender<Said>,
) -> Option<Said> {
  let Begun { about, command, shown, dir, timeout, .. } = begun;
  let (mut child, mut group) = match spawned(command, &shown, &dir).await {
    Ok(one) => one,
    Err(fault) => return failed(&says, &about, fault),
  };
  let (stdout, stderr, stdin) = (child.stdout.take(), child.stderr.take(), child.stdin.take());
  let (mut out, mut err) = (Kept::new(HEAD, TAIL), Kept::new(HEAD, TAIL));
  let streams = async {
    let (one, two) = tokio::join!(
      read(stdout, Stream::Out, &about, &says, &mut out),
      read(stderr, Stream::Err, &about, &says, &mut err)
    );
    one.and(two)
  };
  let expired = async {
    match timeout {
      Some(left) => sleep(left).await,
      None => future::pending().await,
    }
  };
  let shell = reaped(child);
  let watching = Box::pin(watch(shell, streams, expired, feed(stdin, fed), stopped, &mut group));
  let exited = match watching.await {
    Ok(exited) => exited,
    Err(Cut::Stopped) => return None,
    Err(Cut::Unread(no)) => {
      return failed(&says, &about, Fault::refused(format!("{shown} failed: {no}")));
    }
    Err(Cut::Unwaited(no)) => {
      let why = format!("{shown} failed: the wait on its shell failed: {no}");
      return failed(&says, &about, Fault::refused(why));
    }
    Err(Cut::Unslain(no)) => {
      let why = format!("{shown} outlived its timeout and could not be ended: {no}");
      return failed(&says, &about, Fault::refused(why));
    }
  };
  group.free();
  let exit = Exit {
    code: if exited.late { None } else { exited.status.map(i64::from) },
    stdout: Text::new(format!("{about}/stdout"), out.finish()),
    stderr: Text::new(format!("{about}/stderr"), err.finish()),
  };
  says.unbounded_send(Said::Exit { about: about.clone(), exit }).err().map(TrySendError::into_inner)
}

/// A command that did not run to its end: the fault that ends it, said once, or what the channel
/// cannot take.
fn failed(says: &UnboundedSender<Said>, about: &str, fault: Fault) -> Option<Said> {
  says
    .unbounded_send(Said::Failed { about: about.to_owned(), fault })
    .err()
    .map(TrySendError::into_inner)
}

/// What is fed to a command, written into its stdin, until nothing closes it or the command reads
/// no more.
async fn feed(stdin: Option<ChildStdin>, mut fed: spsc::Receiver<Option<String>>) {
  let Some(mut stdin) = stdin else { return };
  while let Some(Some(text)) = fed.recv().await {
    if stdin.write_all(text.as_bytes()).await.is_err() || stdin.flush().await.is_err() {
      return;
    }
  }
}

/// How long the streams of a command have to give what they hold once its shell exited, before the
/// command ends with a process of it still holding one.
const DRAIN: Duration = Duration::from_millis(250);

#[cfg(test)]
#[path = "bash.test.rs"]
mod test;
