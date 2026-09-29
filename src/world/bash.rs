//! Commands: each runs in a shell of the machine, in the working directory of its chain, says what it writes as it
//! writes it, and ends at its exit, at its timeout, or at the done that a control said of it first.

use std::{
  collections::HashMap,
  future,
  path::{Path, PathBuf},
  pin::pin,
  process::{Command, Stdio},
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

use super::here;
use crate::{
  ear::{Co, Ear, Next, ear, say},
  fact::Fact,
  value::{Exit, Fault, Object, Text},
};

/// The POSIX shell that runs a command: `/bin/sh`, and on Windows, which has none of its own, the `sh` on PATH, such
/// as the one of Git for Windows.
pub const SHELL: &str = if cfg!(windows) { "sh" } else { "/bin/sh" };

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

/// The ear of commands: it takes a command, says what it writes, feeds it, and says it done with its exit.
///
/// A command ends with every process it started, which its group holds. A command that a control ended first, which
/// the engine says done, is ended so and says nothing more.
pub fn bash() -> Box<dyn Ear> {
  ear(|mut co: Co<Said>| async move {
    let mut running: HashMap<String, Running> = HashMap::new();
    // What the operator fed a command that runs nowhere yet, which a later life holds until a wake starts it.
    let mut fed: HashMap<String, Vec<Option<String>>> = HashMap::new();
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
          say(&mut co, saying).await;
          continue;
        }
      };
      let about = a.about().to_owned();
      match a.kind() {
        "bash" if a.question() => {
          say(&mut co, Fact::says("started", &about, [])).await;
          match begun(&mut co, &a).await {
            Ok(begun) => {
              let (stdin, fed_in) = spsc::unbounded();
              let (stop, stopped) = oneshot::channel();
              let mut one = Running { stdin, _stop: stop };
              for text in fed.remove(&about).unwrap_or_default() {
                one.feed(text);
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
        "feed" => {
          let text = a.word(0).and_then(|one| one.as_str().map(str::to_owned));
          match running.get_mut(&about) {
            Some(one) => one.feed(text),
            None => fed.entry(about).or_default().push(text),
          }
        }
        "done" => {
          fed.remove(&about);
          running.remove(&about);
        }
        _ => {}
      }
    }
  })
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

/// A command that just started: its act, its process, the group of its processes, and its timeout.
struct Begun {
  about: String,
  child: Child,
  group: Group,
  timeout: Option<Duration>,
}

/// A command begun: its shell spawned in the directory of its chain.
async fn begun(co: &mut Co<Said>, a: &Fact) -> Result<Begun, Fault> {
  let about = a.about().to_owned();
  let on = a.on().to_owned();
  let line = a.word(1).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default();
  let shown = format!("{line:?}");
  let fed = a.word(2).and_then(|one| one.as_bool()).unwrap_or_default();
  let timeout = a.word(3).and_then(|one| one.as_float().or_else(|| one.as_int().map(|n| n as f64)));
  // A timeout past what the machine counts runs to the end of the command, as no timeout does.
  let timeout = timeout.and_then(|seconds| Duration::try_from_secs_f64(seconds).ok());
  let dir = here(co, &on).await?;
  let asked = vec![Object::string("merged"), Object::string(on), Object::string(about.clone())];
  let merged = co.call("ask", asked, vec![]).await?.as_ref().as_bool().unwrap_or_default();
  let mut command = Command::new(SHELL);
  command
    .arg("-c")
    .arg(if merged { format!("exec 2>&1\n{line}") } else { line })
    .current_dir(&dir)
    .stdin(if fed { Stdio::piped() } else { Stdio::null() })
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
  grouped(&mut command);
  let mut child = tokio::process::Command::from(command)
    .spawn()
    .map_err(|no| Fault::refused(format!("{shown} did not start: {no}: {}", dir.display())))?;
  match Group::of(&child) {
    Ok(group) => Ok(Begun { about, child, group, timeout }),
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
/// A command that outlives its timeout is ended with every process of it, and exits with no code. Its streams are
/// read to their end before its exit is read, and only that read reaps it, so its process group stays its own for
/// as long as a process of it holds a stream open, and an end never signals a group that another took.
async fn ran(
  begun: Begun,
  fed: spsc::Receiver<Option<String>>,
  stopped: oneshot::Receiver<()>,
  says: UnboundedSender<Said>,
) {
  let Begun { about, mut child, mut group, timeout } = begun;
  let (stdout, stderr, stdin) = (child.stdout.take(), child.stderr.take(), child.stdin.take());
  let ended = async {
    let (out, err) =
      tokio::join!(read(stdout, "stdout", &about, &says), read(stderr, "stderr", &about, &says),);
    (out, err, child.wait().await)
  };
  let expired = async {
    match timeout {
      Some(left) => tokio::time::sleep(left).await,
      None => future::pending().await,
    }
  };
  let (mut ended, mut expired, mut feeding) = (pin!(ended), pin!(expired), pin!(feed(stdin, fed)));
  let (mut late, mut closed) = (false, false);
  let mut stopped = pin!(stopped);
  let (out, err, status) = loop {
    tokio::select! {
      biased;
      _ = &mut stopped => return,
      got = &mut ended => break got,
      () = &mut expired, if !late => {
        late = true;
        group.slay();
      }
      () = &mut feeding, if !closed => closed = true,
    }
  };
  group.free();
  let code = status.ok().and_then(|status| status.code()).map(i64::from);
  let exit = Exit {
    code: if late { None } else { code },
    stdout: Text::new(format!("{about}/stdout"), out),
    stderr: Text::new(format!("{about}/stderr"), err),
  };
  let _ = says.unbounded_send(Said::Exit { about: about.clone(), exit });
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

/// One stream of a command: each text said as it comes, whole in utf-8, and the whole stream given at its end.
async fn read(
  stream: Option<impl AsyncRead + Unpin>,
  name: &'static str,
  about: &str,
  says: &UnboundedSender<Said>,
) -> String {
  let Some(mut stream) = stream else { return String::new() };
  let (mut all, mut held, mut chunk) = (String::new(), Vec::new(), [0u8; 8192]);
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
      return all;
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
/// Every process of it ends when the group goes before the command is over: at a stop, or at the end of the ear. Its
/// leader stays unreaped until the command is over, since only the read of its exit reaps it, so an end never reaches
/// a group that another took.
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
