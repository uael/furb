//! Commands: each runs in a shell of the machine, in the working directory of its chain, says what it writes as it
//! writes it, and ends at its exit, at its timeout, or at the done that a control said of it first.

use std::{
  collections::HashMap,
  io::{Read, Write},
  path::{Path, PathBuf},
  process::{Child, ChildStdin, Command, Stdio},
  sync::{Arc, Mutex, mpsc},
  thread,
  time::Duration,
};

use super::here;
use crate::{
  ear::{Ear, Voice, call, ear, hear, say},
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
          match begun(&a, voice.clone()) {
            Ok(one) => {
              for text in fed.remove(&about).unwrap_or_default() {
                one.feed(text);
              }
              running.insert(about, one);
            }
            // The machine would not start it, so the ear closes it with why, as a thread that the operator cannot
            // answer is closed, and the chain is told why.
            Err(fault) => {
              call("close", vec![fault.object()], vec![("id", Object::string(&about))])?;
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

/// One command that runs: its state, which its threads share, its group, and the door to its stdin.
struct Running {
  state: Arc<Mutex<State>>,
  group: Arc<Group>,
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
      self.group.slay();
    }
  }
}

/// A command begun: its shell spawned in the directory of its chain, its streams read, its stdin fed, its timeout
/// kept, and its end said as its exit.
fn begun(a: &Fact, voice: Voice) -> Result<Running, Fault> {
  let about = a.about().to_owned();
  let on = a.on().to_owned();
  let line = a.word(1).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default();
  let shown = format!("{line:?}");
  let fed = a.word(2).and_then(|one| one.as_bool()).unwrap_or_default();
  let timeout = a.word(3).and_then(|one| one.as_float().or_else(|| one.as_int().map(|n| n as f64)));
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
  let mut child = command
    .spawn()
    .map_err(|no| Fault::refused(format!("{shown} did not start: {no}: {}", dir.display())))?;
  let group = match Group::of(&child) {
    Ok(group) => Arc::new(group),
    Err(no) => {
      let _ = child.kill();
      return Err(Fault::refused(format!("{shown} could not be held: {no}")));
    }
  };
  let state = Arc::new(Mutex::new(State::default()));
  let stdin = child.stdin.take().map(fed_by);
  let readers = [
    child.stdout.take().map(|out| read(out, "stdout", &about, &voice)),
    child.stderr.take().map(|out| read(out, "stderr", &about, &voice)),
  ];
  let (stop, stopped) = mpsc::channel::<()>();
  // A timeout past what the machine counts runs to the end of the command, as no timeout does.
  if let Some(left) = timeout.and_then(|seconds| Duration::try_from_secs_f64(seconds).ok()) {
    let state = Arc::clone(&state);
    let group = Arc::clone(&group);
    thread::spawn(move || {
      if let Err(mpsc::RecvTimeoutError::Timeout) = stopped.recv_timeout(left) {
        let Ok(mut state) = state.lock() else { return };
        if !state.over {
          state.late = true;
          group.slay();
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
  Ok(Running { state, group, stop, stdin })
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

/// Every process a command started: on Unix the process group it leads, and on Windows, where a shell of Git does
/// not keep the tree of its processes, a job that holds the command and every process it starts.
struct Group(#[cfg(unix)] libc::pid_t, #[cfg(windows)] windows_sys::Win32::Foundation::HANDLE);

// SAFETY: the handle of a job is a kernel object that any thread may terminate and close.
#[cfg(windows)]
unsafe impl Send for Group {}
#[cfg(windows)]
unsafe impl Sync for Group {}

impl Group {
  /// The group of a command that just started. On Windows the job takes the command before its shell has read a
  /// word, since the shell starts slower than the job takes it.
  fn of(child: &Child) -> std::io::Result<Self> {
    #[cfg(unix)]
    {
      libc::pid_t::try_from(child.id()).map(Self).map_err(std::io::Error::other)
    }
    #[cfg(windows)]
    {
      use std::os::windows::io::AsRawHandle;

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
        if AssignProcessToJobObject(job, child.as_raw_handle()) == 0 {
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
    // SAFETY: on Unix kill is given the group the command leads, whose leader stays unreaped while the command is not
    // over; on Windows the job is open until the group drops.
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
