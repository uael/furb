//! Commands: each runs in a shell of the machine, in the working directory of its chain, says what it writes as it
//! writes it, and ends at its exit, at its timeout, or at the done that a control said of it first.

use std::{
  collections::HashMap,
  io::{Read, Write},
  process::{Child, ChildStdin, Command, Stdio},
  sync::{Arc, Mutex, mpsc},
  thread,
  time::Duration,
};

use super::here;
use crate::{
  ear::{Co, Ear, Voice, call, ear, hear, say},
  fact::Fact,
  value::{Exit, Fault, Object, Text},
};

/// The POSIX shell that runs a command: `/bin/sh`, and on Windows, which has none of its own, the `sh` on PATH, such
/// as the one of Git for Windows.
pub const SHELL: &str = if cfg!(windows) { "sh" } else { "/bin/sh" };

/// The ear of commands: it takes a command, says what it writes, feeds it, and says it done with its exit.
///
/// A command ends with every process it started: on Unix its process group, which it leads, and on Windows, which
/// has no process group that a program can signal, its tree of processes. A command that a control ended first,
/// which the engine says done, is ended so and says nothing more.
pub fn bash() -> Box<dyn Ear> {
  ear(|co, voice| async move {
    let mut running: HashMap<String, Running> = HashMap::new();
    // What the operator fed a command that runs nowhere yet, which a later life holds until a wake starts it.
    let mut fed: HashMap<String, Vec<Option<String>>> = HashMap::new();
    loop {
      let a = hear(&co).await;
      let about = a.about().to_owned();
      match a.kind() {
        "bash" if a.question() => {
          say(&co, Fact::says("started", &about, [])).await;
          match begun(&co, &a, voice.clone()).await {
            Ok(one) => {
              for text in fed.remove(&about).unwrap_or_default() {
                one.feed(text);
              }
              running.insert(about, one);
            }
            Err(fault) => {
              say(&co, Fact::says("done", &about, [fault.object()])).await;
            }
          }
        }
        "feed" => {
          let text = a.word(0).and_then(|one| one.as_str().map(str::to_owned));
          match running.get(&about) {
            Some(one) => one.feed(text),
            None => fed.entry(about).or_default().push(text),
          }
        }
        "done" => {
          fed.remove(&about);
          if running.remove(&about).is_some() {
            voice.hush(&about);
          }
        }
        _ => {}
      }
    }
  })
}

/// Whether the command is over, and whether it was ended at its timeout.
#[derive(Default)]
struct State {
  over: bool,
  late: bool,
}

/// One command that runs: its state, which its threads share, its process, and the door to its stdin.
struct Running {
  state: Arc<Mutex<State>>,
  pid: u32,
  stop: mpsc::Sender<()>,
  stdin: Option<mpsc::Sender<Option<String>>>,
}

impl Drop for Running {
  /// A command whose ear lets it go, at a done or at the end of the ear, ends.
  fn drop(&mut self) {
    self.end();
  }
}

impl Running {
  /// One text into its stdin, or nothing to close it.
  fn feed(&self, text: Option<String>) {
    if let Some(stdin) = &self.stdin {
      let _ = stdin.send(text);
    }
  }

  /// The command ended, with every process it started, unless it is over already.
  fn end(&self) {
    let _ = self.stop.send(());
    if let Ok(state) = self.state.lock()
      && !state.over
    {
      slay(self.pid);
    }
  }
}

/// A command begun: its shell spawned in the directory of its chain, its streams read, its stdin fed, its timeout
/// kept, and its end said as its exit.
async fn begun(co: &Co, a: &Fact, voice: Voice) -> Result<Running, Fault> {
  let about = a.about().to_owned();
  let on = a.on().to_owned();
  let line = a.word(1).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default();
  let fed = a.word(2).and_then(|one| one.as_bool()).unwrap_or_default();
  let timeout = a.word(3).and_then(|one| one.as_float().or_else(|| one.as_int().map(|n| n as f64)));
  let dir = here(co, &on).await?;
  let asked = vec![Object::string("merged"), Object::string(on), Object::string(about.clone())];
  let merged = call(co, "ask", asked, vec![]).await?.as_ref().as_bool().unwrap_or_default();
  let mut command = Command::new(SHELL);
  command
    .arg("-c")
    .arg(if merged { format!("exec 2>&1\n{line}") } else { line })
    .current_dir(&dir)
    .stdin(if fed { Stdio::piped() } else { Stdio::null() })
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
  grouped(&mut command);
  let mut child =
    command.spawn().map_err(|no| Fault::refused(format!("{no}: {}", dir.display())))?;
  let state = Arc::new(Mutex::new(State::default()));
  let pid = child.id();
  let stdin = child.stdin.take().map(fed_by);
  let readers = [
    child.stdout.take().map(|out| read(out, "stdout", &about, &voice)),
    child.stderr.take().map(|out| read(out, "stderr", &about, &voice)),
  ];
  let (stop, stopped) = mpsc::channel::<()>();
  // A timeout past what the machine counts runs to the end of the command, as no timeout does.
  if let Some(left) = timeout.and_then(|seconds| Duration::try_from_secs_f64(seconds).ok()) {
    let state = Arc::clone(&state);
    thread::spawn(move || {
      if let Err(mpsc::RecvTimeoutError::Timeout) = stopped.recv_timeout(left) {
        let Ok(mut state) = state.lock() else { return };
        if !state.over {
          state.late = true;
          slay(pid);
        }
      }
    });
  }
  let shared = Arc::clone(&state);
  thread::spawn(move || {
    exited(&mut child);
    let [out, err] = readers.map(|one| one.and_then(|held| held.join().ok()).unwrap_or_default());
    let late = shared.lock().map(|mut state| {
      state.over = true;
      state.late
    });
    let code = child.wait().ok().and_then(|status| status.code()).map(i64::from);
    let exit = Exit {
      code: if late.unwrap_or_default() { None } else { code },
      stdout: Text::new(format!("{about}/stdout"), out),
      stderr: Text::new(format!("{about}/stderr"), err),
    };
    voice.say("done", &about, [exit.object()]);
  });
  Ok(Running { state, pid, stop, stdin })
}

/// The door to the stdin of a command: a thread that writes each text it is given, and closes the stdin at nothing,
/// so a command that reads nothing never holds the life.
fn fed_by(mut stdin: ChildStdin) -> mpsc::Sender<Option<String>> {
  let (feed, fed) = mpsc::channel::<Option<String>>();
  thread::spawn(move || {
    while let Ok(Some(text)) = fed.recv() {
      if stdin.write_all(text.as_bytes()).and_then(|()| stdin.flush()).is_err() {
        return;
      }
    }
  });
  feed
}

/// One stream of a command, read on a thread of its own: each text said as it comes, whole in utf-8, and the whole
/// stream given at its end.
fn read(
  mut stream: impl Read + Send + 'static,
  name: &'static str,
  about: &str,
  voice: &Voice,
) -> thread::JoinHandle<String> {
  let (about, voice) = (about.to_owned(), voice.clone());
  thread::spawn(move || {
    let (mut all, mut held, mut chunk) = (String::new(), Vec::new(), [0u8; 8192]);
    loop {
      let n = match stream.read(&mut chunk) {
        Ok(0) | Err(_) => 0,
        Ok(n) => n,
      };
      held.extend_from_slice(&chunk[..n]);
      let text = whole(&mut held, n == 0);
      if !text.is_empty() {
        voice.say("out", &about, [Object::string(text.clone()), Object::string(name)]);
        all.push_str(&text);
      }
      if n == 0 {
        return all;
      }
    }
  })
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

/// The command exited. On Unix it stays unreaped, which keeps its process group its own for as long as a process
/// of it holds a stream open, so a control that ends the command never signals a group that another took.
fn exited(child: &mut Child) {
  #[cfg(unix)]
  {
    let pid = child.id();
    // SAFETY: waitid is given the pid of a child of this process and a siginfo it may fill; WNOWAIT leaves the child
    // to the wait that reads its code.
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    unsafe {
      libc::waitid(libc::P_PID, pid, &raw mut info, libc::WEXITED | libc::WNOWAIT);
    }
  }
  #[cfg(windows)]
  let _ = child.wait();
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

/// Every process a command started, ended: on Unix its process group, and on Windows its tree.
fn slay(pid: u32) {
  #[cfg(unix)]
  {
    let Ok(group) = libc::pid_t::try_from(pid) else { return };
    // SAFETY: kill is given the group the command leads, whose leader stays unreaped while the command is not over.
    unsafe {
      libc::kill(-group, libc::SIGKILL);
    }
  }
  #[cfg(windows)]
  {
    let _ = Command::new("taskkill")
      .args(["/pid", &pid.to_string(), "/t", "/f"])
      .stdin(Stdio::null())
      .stdout(Stdio::null())
      .stderr(Stdio::null())
      .status();
  }
}
