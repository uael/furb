//! The capture of a walk of the window: the real program `furb-window` in tmux, at one size, on a
//! daemon of its own whose model is the claude command line of the tests, which answers each turn
//! from the words of the walk. What the window writes is recorded through `tmux pipe-pane` with
//! the time of each part, and each step of the walk puts a marker in the recording once its screen
//! shows what the step waits for and the window stopped writing. No model and no network.

use std::{
  env, fs,
  io::{self, Read, Write},
  path::{Path, PathBuf},
  process::Stdio,
  str,
  time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use tokio::{
  io::{AsyncBufReadExt, BufReader},
  process::{Child, Command},
  time::{sleep, timeout},
};
use unicode_width::UnicodeWidthStr;

use crate::{
  cast::{Cast, Event, What},
  error::Error,
  replay::put,
  scenario::{Act, Size, Step, Walk},
};

/// The claude command line of the tests, which answers the Nth turn with the file `word.N` beside it.
const CLAUDE: &str = include_str!("../../src/world/provider/claude/fake-claude.sh");

/// The text of the empty input, which a window shows once it is ready for a walk.
pub(crate) const READY: &str = "Ask for a change";

/// How long a step waits for its screen, which a loaded machine does not pass.
const PATIENCE: Duration = Duration::from_secs(30);

/// How long the window writes nothing before a marker, and the longest wait for that silence, which
/// a screen that moves, as a loader, never gives.
const QUIET: Duration = Duration::from_millis(300);
const SETTLE: Duration = Duration::from_secs(3);

/// A walk recorded: the recording, and the text that tmux showed at each marker.
pub(crate) struct Captured {
  pub(crate) cast: Cast,
  pub(crate) shown: Vec<(String, String)>,
}

/// The walk of a scenario captured at a size, in a folder of its own that it removes at its end.
/// `bin` holds the programs `furb-window` and `furb` of the build.
pub(crate) async fn capture(
  walk: &Walk,
  size: Size,
  bin: &Path,
  root: &Path,
) -> Result<Captured, Error> {
  let rig = Rig::made(walk, bin, root)?;
  let mut daemon = rig.daemon().await?;
  let tmux = Tmux {
    socket: format!(
      "ff{}",
      root.file_name().map_or_else(String::new, |name| name.to_string_lossy().into_owned())
    ),
  };
  let walked = rig.walked(&tmux, walk, size).await;
  let ended = tmux.run(&["kill-server"]).await;
  daemon.kill().await.map_err(|source| Error::Process { what: "the daemon", source })?;
  let (shown, markers, start) = walked?;
  ended?;
  let cast = rig.cast(size, start, markers)?;
  fs::remove_dir_all(&rig.root)
    .map_err(|source| Error::Write { path: rig.root.clone(), source })?;
  Ok(Captured { cast, shown })
}

/// The folders of a capture: the home of the operator with the project of the window, the cache
/// of furb, its config, and the claude command line with its words.
struct Rig {
  root: PathBuf,
  bin: PathBuf,
}

impl Rig {
  /// The rig in a new folder. Its path is the real one, as the window reads its own folder, so the
  /// window finds its project under the home and writes it `~/demo`.
  fn made(walk: &Walk, bin: &Path, root: &Path) -> Result<Self, Error> {
    let made = |source| Error::Write { path: root.to_owned(), source };
    fs::create_dir_all(root).map_err(made)?;
    let root = &fs::canonicalize(root).map_err(made)?;
    let rig = Self { root: root.clone(), bin: bin.to_owned() };
    let write = |path: PathBuf, text: &str| -> Result<(), Error> {
      fs::create_dir_all(path.parent().unwrap_or(root))
        .map_err(|source| Error::Write { path: path.clone(), source })?;
      put(&path, text.as_bytes())
    };
    write(
      rig.project().join("README.md"),
      "# demo\n\nThe project of the captures of the window.\n",
    )?;
    write(root.join("config/config.json"), r#"{"skills": {"naming": false}}"#)?;
    write(rig.claude(), CLAUDE)?;
    executable(&rig.claude())?;
    for (n, word) in walk.words.iter().enumerate() {
      write(root.join(format!("fake/word.{}", n + 1)), &serde_json::to_string(word)?)?;
    }
    Ok(rig)
  }

  fn project(&self) -> PathBuf {
    self.root.join("home/demo")
  }

  fn claude(&self) -> PathBuf {
    self.root.join("fake/claude")
  }

  /// The environment of the window and of its daemon, and nothing of the environment of this
  /// process, so a capture is the same on each machine.
  fn environment(&self) -> Vec<(&'static str, String)> {
    let path = format!("{}:/usr/bin:/bin", self.bin.display());
    let at = |folder: &str| self.root.join(folder).display().to_string();
    vec![
      ("HOME", at("home")),
      ("PATH", path),
      ("TERM", "xterm-256color".to_owned()),
      ("COLORTERM", "truecolor".to_owned()),
      ("LANG", "en_US.UTF-8".to_owned()),
      ("TZ", "UTC".to_owned()),
      ("XDG_CACHE_HOME", at("cache")),
      ("FURB_CONFIG_DIR", at("config")),
      ("FURB_CLAUDE_BIN", self.claude().display().to_string()),
    ]
  }

  /// The daemon of the window, which this process starts so it ends it: it listens once it says so.
  async fn daemon(&self) -> Result<Child, Error> {
    let mut daemon = Command::new(self.bin.join("furb"))
      .args(["host", "--grace", "1"])
      .env_clear()
      .envs(self.environment())
      .stdout(Stdio::piped())
      .stderr(Stdio::null())
      .kill_on_drop(true)
      .spawn()
      .map_err(|source| Error::Process { what: "the daemon", source })?;
    let stdout = daemon.stdout.take().ok_or(Error::Listen)?;
    let said = timeout(PATIENCE, BufReader::new(stdout).lines().next_line()).await;
    match said {
      Ok(Ok(Some(_))) => Ok(daemon),
      _ => Err(Error::Listen),
    }
  }

  /// The window started in tmux, then each step of the walk. It gives the text of each marker, the
  /// markers, and the moment the window started.
  async fn walked(
    &self,
    tmux: &Tmux,
    walk: &Walk,
    size: Size,
  ) -> Result<(Vec<(String, String)>, Vec<(SystemTime, String)>, SystemTime), Error> {
    let go = self.root.join("go");
    let env: Vec<String> =
      self.environment().into_iter().map(|(key, value)| format!("{key}={value}")).collect();
    let started = format!(
      "env -i {} sh -c 'until [ -e {} ]; do sleep 0.01; done; exec furb-window'",
      env.join(" "),
      go.display()
    );
    let (cols, rows) = (size.cols.to_string(), size.rows.to_string());
    let project = self.project().display().to_string();
    tmux
      .run(&["new-session", "-d", "-x", &cols, "-y", &rows, "-s", "w", "-c", &project, &started])
      .await?;
    tmux.run(&["set-option", "-g", "status", "off"]).await?;
    let me =
      env::current_exe().map_err(|source| Error::Process { what: "this program", source })?;
    let stamped = format!("'{}' stamp '{}'", me.display(), self.stamps().display());
    tmux.run(&["pipe-pane", "-O", "-t", "w", &stamped]).await?;
    let start = SystemTime::now();
    put(&go, b"")?;
    tmux.shows("ready", READY).await?;
    let mut shown = Vec::new();
    let mut markers = Vec::new();
    for step in walk.steps {
      let screen = self.stepped(tmux, step).await?;
      markers.push((SystemTime::now(), step.name.to_owned()));
      shown.push((step.name.to_owned(), screen));
    }
    Ok((shown, markers, start))
  }

  /// One step: its acts, then the wait for its screen and for the silence of the window.
  async fn stepped(&self, tmux: &Tmux, step: &Step) -> Result<String, Error> {
    for act in step.acts {
      tmux.act(act).await?;
    }
    tmux.shows(step.name, step.until).await?;
    let settled = Instant::now();
    let mut quiet = Instant::now();
    let mut length = self.written();
    while quiet.elapsed() < QUIET && settled.elapsed() < SETTLE {
      sleep(Duration::from_millis(50)).await;
      let now = self.written();
      if now != length {
        (length, quiet) = (now, Instant::now());
      }
    }
    tmux.screen().await
  }

  fn stamps(&self) -> PathBuf {
    self.root.join("stamps")
  }

  fn written(&self) -> u64 {
    fs::metadata(self.stamps()).map_or(0, |file| file.len())
  }

  /// The recording: what the window wrote, each part at its time, and the markers.
  fn cast(
    &self,
    size: Size,
    start: SystemTime,
    markers: Vec<(SystemTime, String)>,
  ) -> Result<Cast, Error> {
    let path = self.stamps();
    let text =
      fs::read_to_string(&path).map_err(|source| Error::Read { path: path.clone(), source })?;
    let since = |at: SystemTime| at.duration_since(start).unwrap_or_default();
    let mut events = Vec::new();
    for line in text.lines() {
      let (micros, data) = line.split_once('\t').ok_or(Error::Cast("a stamp has no tab"))?;
      let micros: u64 = micros.parse().ok().ok_or(Error::Cast("a stamp is not a number"))?;
      let at = since(UNIX_EPOCH + Duration::from_micros(micros));
      events.push(Event { at, what: What::Output(serde_json::from_str(data)?) });
    }
    events.extend(
      markers.into_iter().map(|(at, label)| Event { at: since(at), what: What::Marker(label) }),
    );
    events.sort_by_key(|event| event.at);
    Ok(Cast { cols: size.cols, rows: size.rows, events })
  }
}

/// What `tmux pipe-pane` gives, written as lines of the time in microseconds and the text as JSON.
/// A glyph cut between two reads waits for its end.
pub(crate) fn stamp(path: &Path) -> Result<(), Error> {
  let wrote = |source| Error::Write { path: path.to_owned(), source };
  let mut file = fs::File::create(path).map_err(wrote)?;
  let mut input = io::stdin().lock();
  let (mut buffer, mut held) = (vec![0; 65536], Vec::new());
  loop {
    let read =
      input.read(&mut buffer).map_err(|source| Error::Read { path: PathBuf::from("-"), source })?;
    if read == 0 {
      return Ok(());
    }
    held.extend_from_slice(&buffer[..read]);
    let whole = match str::from_utf8(&held) {
      Ok(_) => held.len(),
      Err(cut) if cut.error_len().is_none() => cut.valid_up_to(),
      Err(cut) => cut.valid_up_to() + cut.error_len().unwrap_or(1),
    };
    let text = String::from_utf8_lossy(&held[..whole]).into_owned();
    held.drain(..whole);
    let micros = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_micros();
    writeln!(file, "{micros}\t{}", serde_json::to_string(&text)?).map_err(wrote)?;
    file.flush().map_err(wrote)?;
  }
}

/// A server of tmux of one capture.
struct Tmux {
  socket: String,
}

impl Tmux {
  async fn run(&self, args: &[&str]) -> Result<String, Error> {
    let done = Command::new("tmux")
      .args(["-L", &self.socket, "-f", "/dev/null", "-u"])
      .args(args)
      .output()
      .await
      .map_err(|source| Error::Process { what: "tmux", source })?;
    if !done.status.success() {
      let said = String::from_utf8_lossy(&done.stderr).trim().to_owned();
      return Err(Error::Tmux { command: args.join(" "), said });
    }
    Ok(String::from_utf8_lossy(&done.stdout).into_owned())
  }

  /// The screen once it shows a text, which a step waits for.
  async fn shows(&self, step: &str, text: &str) -> Result<(), Error> {
    let waited = Instant::now();
    loop {
      let screen = self.screen().await?;
      if screen.contains(text) {
        return Ok(());
      }
      if waited.elapsed() > PATIENCE {
        return Err(Error::Step { step: step.to_owned(), until: text.to_owned(), screen });
      }
      sleep(Duration::from_millis(50)).await;
    }
  }

  async fn screen(&self) -> Result<String, Error> {
    self.run(&["capture-pane", "-p", "-t", "w"]).await
  }

  async fn act(&self, act: &Act) -> Result<(), Error> {
    match act {
      Act::Type(text) => self.run(&["send-keys", "-t", "w", "-l", text]).await.map(drop),
      Act::Key(key) => self.run(&["send-keys", "-t", "w", key]).await.map(drop),
      Act::Paste(text) => {
        self.run(&["set-buffer", "-b", "p", text]).await?;
        self.run(&["paste-buffer", "-p", "-d", "-b", "p", "-t", "w"]).await.map(drop)
      }
      Act::Pointer { on, button } => {
        let (col, row) = self.place(on).await?;
        let (col, row) = (col + 1, row + 1);
        let press = format!("\x1b[<{button};{col};{row}M");
        let both = if *button < 32 { format!("{press}\x1b[<{button};{col};{row}m") } else { press };
        self.run(&["send-keys", "-t", "w", "-l", &both]).await.map(drop)
      }
    }
  }

  /// The first cell of a text on the screen: its column and its row.
  async fn place(&self, text: &str) -> Result<(usize, usize), Error> {
    let screen = self.screen().await?;
    let found = screen
      .lines()
      .enumerate()
      .find_map(|(row, line)| line.find(text).map(|at| (line[..at].width(), row)));
    found.ok_or_else(|| Error::Step { step: "pointer".to_owned(), until: text.to_owned(), screen })
  }
}

#[cfg(unix)]
fn executable(path: &Path) -> Result<(), Error> {
  use std::os::unix::fs::PermissionsExt;
  let made = fs::set_permissions(path, fs::Permissions::from_mode(0o755));
  made.map_err(|source| Error::Write { path: path.to_owned(), source })
}

/// A file runs on Windows by its name alone.
#[cfg(not(unix))]
fn executable(_path: &Path) -> Result<(), Error> {
  Ok(())
}
