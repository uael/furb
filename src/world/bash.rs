//! Commands: each runs in a shell of the machine, in the working directory of its chain, says what it writes as it
//! writes it, and ends at its exit, at its timeout, or at the done that a control said of it first.

use std::{
  collections::{HashMap, HashSet},
  env, future,
  io::ErrorKind,
  path::{Path, PathBuf},
  pin::pin,
  process::{Command, Stdio},
  sync::OnceLock,
  time::Duration,
};

use futures::{
  Stream, StreamExt,
  channel::mpsc::{UnboundedSender, unbounded},
  stream,
};
use tokio::{
  io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
  process::{Child, ChildStdin},
};
use unsync::{oneshot, spsc};

use super::{Scratch, here};
use crate::{
  ear::{Co, Ear, Next, hearing, say},
  engine::left,
  fact::{Fact, named},
  value::{Exit, Fault, Object, Text},
};

/// The POSIX sh that runs a command: `/bin/sh`, and on Windows, which has none of its own, the sh of Git for Windows,
/// which [`git_sh`] finds, or `sh` when it finds none, which a command then names in the refusal of its start.
pub fn shell() -> &'static Path {
  static FOUND: OnceLock<PathBuf> = OnceLock::new();
  FOUND.get_or_init(|| {
    if !cfg!(windows) {
      return PathBuf::from("/bin/sh");
    }
    let folders: Vec<PathBuf> =
      env::split_paths(&env::var_os("PATH").unwrap_or_default()).collect();
    git_sh(&folders, env::var_os("ProgramFiles").map(PathBuf::from))
      .unwrap_or_else(|| PathBuf::from("sh"))
  })
}

/// The sh of Git for Windows: the `sh` in these folders, else the one of the Git whose `git` stands in them, in a
/// `bin` or a `usr/bin` of a folder above that program, as a Git that puts only its `cmd` on PATH holds it, else the one
/// of the Git in the folder of the programs of the machine.
pub(crate) fn git_sh(folders: &[PathBuf], programs: Option<PathBuf>) -> Option<PathBuf> {
  program("sh", folders.iter().cloned())
    .or_else(|| {
      let git = program("git", folders.iter().cloned())?;
      let roots = git.ancestors().skip(2).take(3).map(Path::to_path_buf).collect::<Vec<_>>();
      program("sh", roots.iter().flat_map(|one| [one.join("bin"), one.join("usr").join("bin")]))
    })
    .or_else(|| program("sh", [programs?.join("Git").join("bin")]))
}

/// The first program of a name in these folders, with the endings of a program on Windows.
pub fn program(name: &str, folders: impl IntoIterator<Item = PathBuf>) -> Option<PathBuf> {
  let endings: &[&str] = if cfg!(windows) { &[".exe", ".cmd", ""] } else { &[""] };
  let folders = folders.into_iter().filter(|one| !one.as_os_str().is_empty());
  folders
    .flat_map(|one| endings.iter().map(move |end| one.join(format!("{name}{end}"))))
    .find(|one| runs(one))
}

/// Whether a path is a program that this machine runs.
fn runs(path: &Path) -> bool {
  let Ok(info) = path.metadata() else { return false };
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    info.is_file() && info.permissions().mode() & 0o111 != 0
  }
  #[cfg(not(unix))]
  {
    info.is_file()
  }
}

/// The ear of commands: it takes a command, says what it writes, answers a write of a door of it, feeds it what a
/// write of its stdin gives, and says it done with its exit.
///
/// A command that the record shows started and not done was cut by the end of the life that ran it: the ear runs it
/// no more, and when a wake puts it to the ear, the ear takes it and closes it with a refusal that says it was
/// interrupted, so the model sees that outcome and runs it again if it wants, and its doors keep what it wrote before.
/// It takes no write.
///
/// A command runs with `TMPDIR` set to the scratch directory of the life, and `TMP` and `TEMP` too on Windows, which
/// the ear makes first, so a command finds there the files of `scratch://`. A command ends with every process it started, which its group holds. A
/// command that a control ended first, which the engine says done, is ended so and says nothing more.
pub fn bash(scratch: Scratch, record: &[Object]) -> Box<dyn Ear> {
  let left: HashSet<String> = left(record).into_iter().collect();
  // Each command that a crash cut, with its chain and its line, so a later act that only shares its name, as after a
  // drift, is no command of the record.
  let mut cut: HashMap<String, (String, String)> = record
    .iter()
    .filter_map(|one| Fact::of(*one.as_ref().items()?.first()?))
    .filter(|a| a.kind() == "bash" && a.question() && left.contains(a.about()))
    .map(|a| (a.about().to_owned(), held(&a)))
    .collect();
  hearing(&["bash", "write"], move |mut co: Co<Said>| async move {
    let mut running: HashMap<String, Running> = HashMap::new();
    // The commands whose stdin a write closed.
    let mut closed: HashSet<String> = HashSet::new();
    loop {
      let a = match co.next().await {
        Next::Heard(a) => a,
        // What a command says while it runs, which a control that ended it first silences.
        Next::Worked(one) => {
          let saying = match one {
            Said::Out { about, text, stream } if running.contains_key(&about) => {
              Fact::says("out", &about, [Object::string(text), Object::string(stream)])
            }
            Said::Exit { about, exit } if running.remove(&about).is_some() => {
              Fact::says("done", &about, [exit.object()])
            }
            _ => continue,
          };
          say(&mut co, saying);
          continue;
        }
      };
      let about = a.about().to_owned();
      match a.kind() {
        "bash" if a.question() && cut.get(&about).is_some_and(|it| *it == held(&a)) => {
          cut.remove(&about);
          say(&mut co, Fact::says("started", &about, []));
          let line = a.word(1).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default();
          let why = Fault::refused(format!(
            "{line:?} was interrupted: the life that ran it ended before it did"
          ));
          co.call("close", vec![why.object()], vec![("id", Object::string(&about))]).await?;
        }
        "bash" if a.question() => {
          say(&mut co, Fact::says("started", &about, []));
          match begun(&mut co, &a, &scratch).await {
            Ok(mut begun) => {
              let (stdin, fed_in) = spsc::unbounded();
              let (stop, stopped) = oneshot::channel();
              let mut one = Running { stdin, _stop: stop };
              // A stdin of text goes in whole at the start, and then it closes.
              if let Some(text) = begun.given.take().filter(|text| !text.is_empty()) {
                one.feed(Some(text));
                one.feed(None);
              }
              co.work(command(begun, fed_in, stopped));
              running.insert(about, one);
            }
            // The machine would not start it, so the ear closes it with why, as a prompt that the operator cannot
            // answer is closed, and the chain is told why.
            Err(fault) => {
              co.call("close", vec![fault.object()], vec![("id", Object::string(&about))]).await?;
            }
          }
        }
        // A write of a door of a command: the stdin of a command whose stdin is None takes it while the command
        // runs and until a write of nothing closes it, and every other door takes no write.
        "write" if a.question() => {
          let Some((text, command, stream)) = door(&a) else { continue };
          let act = co.call("get", vec![Object::string(&command)], vec![]).await?;
          let Some(act) = Fact::of(act.as_ref()) else { continue };
          let over = co.call("over", vec![Object::string(&command)], vec![]).await?;
          let open = stream == "stdin"
            && over.as_ref().as_bool() != Some(true)
            && act.word(2).is_some_and(|one| one.type_name() == "NoneType")
            && !closed.contains(&command)
            && !cut.contains_key(&command);
          let answer = if open {
            written(&mut running, &mut closed, &command, &text.content);
            text.object()
          } else {
            Fault::refused(format!("{} takes no write", text.path)).object()
          };
          say(&mut co, Fact::says("done", &about, [answer]));
        }
        "done" => {
          running.remove(&about);
          closed.remove(&about);
          cut.remove(&about);
        }
        _ => {}
      }
    }
  })
}

/// The chain and the line of a command, which say whether an act is the command of the record under its name.
fn held(a: &Fact) -> (String, String) {
  let line = a.word(1).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default();
  (a.on().to_owned(), line)
}

/// The text of a write of a door of a command, the command and the stream of the door, when the write is of one.
pub(super) fn door(a: &Fact) -> Option<(Text, String, String)> {
  let text = a.word(1).and_then(Text::of)?;
  let (command, stream) = text.path.split_once('/')?;
  let (command, stream) = (command.to_owned(), stream.to_owned());
  (named(&command, "bash") && matches!(stream.as_str(), "stdin" | "stdout" | "stderr"))
    .then_some((text, command, stream))
}

/// What a write of the stdin of a command gives it: the text, or nothing for an empty text, which closes the stdin.
fn written(
  running: &mut HashMap<String, Running>,
  closed: &mut HashSet<String>,
  command: &str,
  content: &str,
) {
  let text = (!content.is_empty()).then(|| content.to_owned());
  if text.is_none() {
    closed.insert(command.to_owned());
  }
  if let Some(one) = running.get_mut(command) {
    one.feed(text);
  }
}

/// One command that runs, as the ear holds it: the door to its stdin, and its stop, which ends it when it drops.
struct Running {
  stdin: spsc::Sender<Option<String>>,
  _stop: oneshot::Sender<()>,
}

impl Running {
  /// One text into its stdin, or nothing to close it.
  fn feed(&mut self, text: Option<String>) {
    let _ = self.stdin.try_send(text);
  }
}

/// What a command says: a text that one of its streams wrote, or its exit.
enum Said {
  Out { about: String, text: String, stream: &'static str },
  Exit { about: String, exit: Exit },
}

/// A command that just started: its act, its process, the group of its processes, its timeout, and the text that its
/// stdin is given, or nothing when its stdin stays open for what is fed to it.
struct Begun {
  about: String,
  child: Child,
  group: Group,
  timeout: Option<Duration>,
  given: Option<String>,
}

/// A command begun: its shell spawned in the directory of its chain, with the scratch directory as its `TMPDIR`.
async fn begun(co: &mut Co<Said>, a: &Fact, scratch: &Scratch) -> Result<Begun, Fault> {
  let about = a.about().to_owned();
  let on = a.on().to_owned();
  let line = a.word(1).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default();
  let shown = format!("{line:?}");
  // A stdin of text goes in whole and closes, None stays open for what is fed, and any other word, as the flag of a
  // record made before the stdin was a text, gives a stdin that is empty and closed.
  let given = match a.word(2) {
    Some(one) if one.type_name() == "NoneType" => None,
    one => Some(one.and_then(|it| it.as_str()).unwrap_or_default().to_owned()),
  };
  let timeout = a.word(3).and_then(|one| one.as_float().or_else(|| one.as_int().map(|n| n as f64)));
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
  grouped(&mut command);
  let mut child = tokio::process::Command::from(command).spawn().map_err(|no| {
    let why = if cfg!(windows) && no.kind() == ErrorKind::NotFound {
      format!("no POSIX sh runs it: install Git for Windows, whose sh furb runs: {no}")
    } else {
      no.to_string()
    };
    Fault::refused(format!("{shown} did not start: {why}: {}", dir.display()))
  })?;
  match Group::of(&child) {
    Ok(group) => Ok(Begun { about, child, group, timeout, given }),
    Err(no) => {
      let _ = child.start_kill();
      Err(Fault::refused(format!("{shown} could not be held: {no}")))
    }
  }
}

/// What a command says as it runs: each text its streams write, then its exit; or nothing more once it stops.
fn command(
  begun: Begun,
  fed: spsc::Receiver<Option<String>>,
  stopped: oneshot::Receiver<()>,
) -> impl Stream<Item = Said> {
  let (says, said) = unbounded();
  let ran = ran(begun, fed, stopped, says);
  // The command says through the channel alone, so its exit comes after all it wrote.
  stream::select(said, stream::once(ran).filter_map(|()| future::ready(None)))
}

/// A command run to its end: what its streams write is said as it comes, what is fed goes into its stdin, and its
/// exit is said at its end; or the command is ended at its stop, with every process of it.
///
/// A command that outlives its timeout is ended with every process of it, and exits with no code. Its exit comes once
/// its shell exited and its streams closed, or once its shell exited and no process of its group lives, as when a
/// process that left the group holds a stream: then the exit gives what its streams wrote until then. The shell is
/// reaped as it exits, and its group keeps its number for as long as a process of it lives, so an end signals no
/// group that another took, but in the moment between the last probe of an empty group and that end.
async fn ran(
  begun: Begun,
  fed: spsc::Receiver<Option<String>>,
  stopped: oneshot::Receiver<()>,
  says: UnboundedSender<Said>,
) {
  let Begun { about, mut child, mut group, timeout, .. } = begun;
  let (stdout, stderr, stdin) = (child.stdout.take(), child.stderr.take(), child.stdin.take());
  let (mut out, mut err) = (String::new(), String::new());
  let streams = async {
    tokio::join!(
      read(stdout, "stdout", &about, &says, &mut out),
      read(stderr, "stderr", &about, &says, &mut err)
    );
  };
  let shell = child.wait();
  let expired = async {
    match timeout {
      Some(left) => tokio::time::sleep(left).await,
      None => future::pending().await,
    }
  };
  let (late, status) = {
    let (mut streams, mut shell) = (pin!(streams), pin!(shell));
    let (mut expired, mut feeding) = (pin!(expired), pin!(feed(stdin, fed)));
    let (mut late, mut closed, mut drained, mut status) = (false, false, false, None);
    let (mut gone, mut ended) = (false, false);
    let mut stopped = pin!(stopped);
    loop {
      tokio::select! {
        biased;
        _ = &mut stopped => return,
        got = &mut shell, if status.is_none() => status = Some(got),
        () = &mut streams, if !drained => drained = true,
        () = &mut expired, if !late => {
          late = true;
          group.slay();
        }
        () = &mut feeding, if !closed => closed = true,
        // A process that left the group can hold a stream once no process of the group lives, so the group is
        // probed while the shell is gone and a stream is open, and a probe that finds it empty gives the streams
        // one more probe to give what they hold.
        () = tokio::time::sleep(PROBE), if status.is_some() && !drained => {
          ended = gone;
          gone = !group.lives();
        }
      }
      if let Some(got) = status.take_if(|_| drained || ended) {
        break (late, got);
      }
    }
  };
  group.free();
  let code = status.ok().and_then(coded).map(i64::from);
  let exit = Exit {
    code: if late { None } else { code },
    stdout: Text::new(format!("{about}/stdout"), out),
    stderr: Text::new(format!("{about}/stderr"), err),
  };
  let _ = says.unbounded_send(Said::Exit { about: about.clone(), exit });
}

/// The code of an exit, and for a command that a signal ended, 128 and the number of the signal, as a shell gives, so
/// no code says only that the timeout ended the command.
fn coded(status: std::process::ExitStatus) -> Option<i32> {
  #[cfg(unix)]
  {
    use std::os::unix::process::ExitStatusExt;
    status.code().or_else(|| status.signal().map(|signal| 128 + signal))
  }
  #[cfg(not(unix))]
  status.code().map(signalled)
}

/// The code of an exit of the sh of Git for Windows, which a signal ends with the number of the signal times 256, as
/// 128 and that number, as a shell gives; and every other code as it is.
#[cfg_attr(unix, allow(dead_code))]
pub(super) fn signalled(code: i32) -> i32 {
  if code % 256 == 0 && (1..=64).contains(&(code / 256)) { 128 + code / 256 } else { code }
}

/// What is fed to a command, written into its stdin, until nothing closes it or the command reads no more.
async fn feed(stdin: Option<ChildStdin>, mut fed: spsc::Receiver<Option<String>>) {
  let Some(mut stdin) = stdin else { return };
  while let Some(Some(text)) = fed.recv().await {
    if stdin.write_all(text.as_bytes()).await.is_err() || stdin.flush().await.is_err() {
      return;
    }
  }
}

/// How long the World waits between two probes of the group of a command whose shell exited while a stream is open.
const PROBE: Duration = Duration::from_millis(100);

/// One stream of a command: each text said as it comes, whole in utf-8, and kept in all until the stream ends.
async fn read(
  stream: Option<impl AsyncRead + Unpin>,
  name: &'static str,
  about: &str,
  says: &UnboundedSender<Said>,
  all: &mut String,
) {
  let Some(mut stream) = stream else { return };
  let (mut held, mut chunk) = (Vec::new(), [0u8; 8192]);
  loop {
    let n = stream.read(&mut chunk).await.unwrap_or(0);
    held.extend_from_slice(&chunk[..n]);
    let text = whole(&mut held, n == 0);
    if !text.is_empty() {
      let _ = says.unbounded_send(Said::Out {
        about: about.to_owned(),
        text: text.clone(),
        stream: name,
      });
      all.push_str(&text);
    }
    if n == 0 {
      return;
    }
  }
}

/// The text of the bytes held that is whole in utf-8, which leaves them: a character cut at the end of a read waits
/// for the next, unless the stream is over, and a byte that no utf-8 holds reads as the character that replaces it.
pub(super) fn whole(held: &mut Vec<u8>, over: bool) -> String {
  let mut text = String::new();
  loop {
    match std::str::from_utf8(held) {
      Ok(all) => {
        text.push_str(all);
        held.clear();
        return text;
      }
      Err(no) => {
        let good = no.valid_up_to();
        text.push_str(std::str::from_utf8(&held[..good]).unwrap_or_default());
        match no.error_len() {
          Some(bad) => {
            text.push(char::REPLACEMENT_CHARACTER);
            held.drain(..good + bad);
          }
          None if over => {
            text.push(char::REPLACEMENT_CHARACTER);
            held.clear();
            return text;
          }
          None => {
            held.drain(..good);
            return text;
          }
        }
      }
    }
  }
}

/// The command leads a process group of its own, or on Windows opens no window.
fn grouped(command: &mut Command) {
  #[cfg(unix)]
  {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
  }
  #[cfg(windows)]
  {
    use std::os::windows::process::CommandExt;
    const NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(NO_WINDOW);
  }
}

/// Every process a command started: on Unix the process group it leads, and on Windows, where a shell of Git does
/// not keep the tree of its processes, a job that holds the command and every process it starts.
///
/// Every process of it ends when the group goes before the command is over: at a stop, or at the end of the ear.
struct Group {
  #[cfg(unix)]
  leader: Option<rustix::process::Pid>,
  #[cfg(windows)]
  job: Option<win32job::Job>,
}

impl Group {
  /// The group of a command that just started. On Windows the job takes the command before its shell has read a
  /// word, since the shell starts slower than the job takes it, and it ends every process it holds when it closes.
  fn of(child: &Child) -> std::io::Result<Self> {
    let gone = || std::io::Error::other("the command is over already");
    #[cfg(unix)]
    {
      let pid = child.id().and_then(|pid| i32::try_from(pid).ok());
      let leader = pid.and_then(rustix::process::Pid::from_raw).ok_or_else(gone)?;
      Ok(Group { leader: Some(leader) })
    }
    #[cfg(windows)]
    {
      let mut ends = win32job::ExtendedLimitInfo::new();
      ends.limit_kill_on_job_close();
      let job = win32job::Job::create_with_limit_info(&ends).map_err(std::io::Error::other)?;
      let process = child.raw_handle().ok_or_else(gone)?;
      job.assign_process(process as isize).map_err(std::io::Error::other)?;
      Ok(Group { job: Some(job) })
    }
  }

  /// Every process of the group, ended.
  fn slay(&mut self) {
    #[cfg(unix)]
    if let Some(leader) = self.leader {
      let _ = rustix::process::kill_process_group(leader, rustix::process::Signal::KILL);
    }
    #[cfg(windows)]
    self.job.take();
  }

  /// Whether a process of the group lives. On Windows no job here says it, so the group lives until the command is
  /// over, and the command waits for its streams.
  fn lives(&self) -> bool {
    #[cfg(unix)]
    let lives =
      self.leader.is_some_and(|leader| rustix::process::test_kill_process_group(leader).is_ok());
    #[cfg(windows)]
    let lives = self.job.is_some();
    lives
  }

  /// The group let go once the command is over, and every process of it that outlives the command left to run.
  fn free(&mut self) {
    #[cfg(unix)]
    self.leader.take();
    #[cfg(windows)]
    if let Some(job) = self.job.take() {
      let _ = job.set_extended_limit_info(&win32job::ExtendedLimitInfo::new());
    }
  }
}

impl Drop for Group {
  fn drop(&mut self) {
    self.slay();
  }
}

#[cfg(test)]
#[path = "bash.test.rs"]
mod test;
