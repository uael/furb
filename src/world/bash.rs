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
  FutureExt, StreamExt,
  channel::mpsc::{UnboundedSender, unbounded},
  future::LocalBoxFuture,
  stream::FuturesUnordered,
};
use tokio::{
  io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
  process::{Child, ChildStdin},
};
use unsync::{oneshot, spsc};

use super::here;
use crate::{
  ear::{Ear, call, ear, hear, say},
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
  ear(|co| async move {
    let mut running: HashMap<String, Running> = HashMap::new();
    // What the operator fed a command that runs nowhere yet, which a later life holds until a wake starts it.
    let mut fed: HashMap<String, Vec<Option<String>>> = HashMap::new();
    let mut commands: FuturesUnordered<LocalBoxFuture<'static, ()>> = FuturesUnordered::new();
    // What every command says, in the order it says it.
    let (says, mut said) = unbounded::<Said>();
    loop {
      let a = tokio::select! {
        biased;
        a = hear(&co) => a,
        Some(one) = co.working(said.next()), if !running.is_empty() => {
          let saying = match one {
            Said::Out { about, text, stream } if running.contains_key(&about) => {
              Fact::says("out", &about, [Object::string(text), Object::string(stream)])
            }
            Said::Exit { about, exit } if running.remove(&about).is_some() => {
              Fact::says("done", &about, [exit.object()])
            }
            _ => continue,
          };
          say(&co, saying).await;
          continue;
        }
        Some(()) = co.working(commands.next()), if !commands.is_empty() => continue,
      };
      let about = a.about().to_owned();
      match a.kind() {
        "bash" if a.question() => {
          say(&co, Fact::says("started", &about, [])).await;
          match begun(&a) {
            Ok(begun) => {
              let (stdin, fed_in) = spsc::unbounded();
              let (stop, stopped) = oneshot::channel();
              let mut one = Running { stdin, _stop: stop };
              for text in fed.remove(&about).unwrap_or_default() {
                one.feed(text);
              }
              commands.push(ran(begun, fed_in, stopped, says.clone()).boxed_local());
              running.insert(about, one);
            }
            // The machine would not start it, so the ear closes it with why, as a prompt that the operator cannot
            // answer is closed, and the chain is told why.
            Err(fault) => {
              call("close", vec![fault.object()], vec![("id", Object::string(&about))])?;
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
fn begun(a: &Fact) -> Result<Begun, Fault> {
  let about = a.about().to_owned();
  let on = a.on().to_owned();
  let line = a.word(1).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default();
  let shown = format!("{line:?}");
  let fed = a.word(2).and_then(|one| one.as_bool()).unwrap_or_default();
  let timeout = a.word(3).and_then(|one| one.as_float().or_else(|| one.as_int().map(|n| n as f64)));
  // A timeout past what the machine counts runs to the end of the command, as no timeout does.
  let timeout = timeout.and_then(|seconds| Duration::try_from_secs_f64(seconds).ok());
  let dir = here(&on)?;
  let asked = vec![Object::string("merged"), Object::string(on), Object::string(about.clone())];
  let merged = call("ask", asked, vec![])?.as_ref().as_bool().unwrap_or_default();
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
  let Begun { about, mut child, group, timeout } = begun;
  let mut leash = Leash(Some(group));
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
        leash.slay();
      }
      () = &mut feeding, if !closed => closed = true,
    }
  };
  leash.0 = None;
  let code = status.ok().and_then(|status| status.code()).map(i64::from);
  let exit = Exit {
    code: if late { None } else { code },
    stdout: Text::new(format!("{about}/stdout"), out),
    stderr: Text::new(format!("{about}/stderr"), err),
  };
  let _ = says.unbounded_send(Said::Exit { about: about.clone(), exit });
}

/// The group of a command that runs, which ends with every process of it when it is let go before the command is
/// over: at a stop, or at the end of the ear.
struct Leash(Option<Group>);

impl Leash {
  fn slay(&self) {
    if let Some(group) = &self.0 {
      group.slay();
    }
  }
}

impl Drop for Leash {
  fn drop(&mut self) {
    self.slay();
  }
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
struct Group(#[cfg(unix)] libc::pid_t, #[cfg(windows)] windows_sys::Win32::Foundation::HANDLE);

impl Group {
  /// The group of a command that just started. On Windows the job takes the command before its shell has read a
  /// word, since the shell starts slower than the job takes it.
  fn of(child: &Child) -> std::io::Result<Self> {
    let gone = || std::io::Error::other("the command is over already");
    #[cfg(unix)]
    {
      let pid = child.id().ok_or_else(gone)?;
      libc::pid_t::try_from(pid).map(Self).map_err(std::io::Error::other)
    }
    #[cfg(windows)]
    {
      use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::JobObjects::{AssignProcessToJobObject, CreateJobObjectW},
      };
      // SAFETY: the job is made with no name and no attributes, and it takes the process that the child holds open.
      unsafe {
        let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if job.is_null() {
          return Err(std::io::Error::last_os_error());
        }
        let Some(process) = child.raw_handle() else {
          CloseHandle(job);
          return Err(gone());
        };
        if AssignProcessToJobObject(job, process) == 0 {
          let no = std::io::Error::last_os_error();
          CloseHandle(job);
          return Err(no);
        }
        Ok(Self(job))
      }
    }
  }

  /// Every process of the group, ended.
  fn slay(&self) {
    // SAFETY: on Unix kill is given the group the command leads, whose leader stays unreaped while its leash holds
    // it; on Windows the job is open until the group drops.
    unsafe {
      #[cfg(unix)]
      libc::kill(-self.0, libc::SIGKILL);
      #[cfg(windows)]
      windows_sys::Win32::System::JobObjects::TerminateJobObject(self.0, 1);
    }
  }
}

#[cfg(windows)]
impl Drop for Group {
  fn drop(&mut self) {
    // SAFETY: the group alone holds the handle of its job.
    unsafe {
      windows_sys::Win32::Foundation::CloseHandle(self.0);
    }
  }
}
