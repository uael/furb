//! The capture of a walk of the window: the real program `furb-window` in tmux, at one size, on a
//! daemon of its own whose model is the claude command line of the tests, which answers each turn
//! from the words of the walk. What the window writes is recorded through `tmux pipe-pane` with
//! the time of each part, and each step of the walk puts a marker in the recording once its screen
//! shows what the step waits for and the window stopped writing. No model and no network.

use std::{
  env,
  fmt::Write as _,
  fs::{self, File},
  io::{self, Read, Write},
  path::{Path, PathBuf},
  process::{self, Stdio},
  str,
  time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use furb_host::{
  protocol::{Answer, Frame, Hello, Kind, PROTOCOL, Request, Verb},
  transport::{Dialer, Local},
};
use tokio::{
  io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
  process::{Child, Command},
  task,
  time::{sleep, timeout},
};
use unicode_width::UnicodeWidthStr;

use crate::{
  cast::{Cast, Event, What},
  error::Error,
  replay::put,
  scenario::{Act, Size, Step, Walk},
};

/// The claude command line of the tests, which answers the Nth turn with the file `word.N`
/// beside it.
const CLAUDE: &str = include_str!("../../src/world/provider/claude/fake-claude.sh");

/// The claude command line of a walk with a pace, which stands before the one of the tests, named
/// `answers` beside it: a turn whose line holds a text of the file `pace` waits the seconds beside
/// that text, then reaches `answers`, so the model of the walk takes the time of a real one.
const PACED: &str = r#"#!/bin/sh
here=$(dirname "$0")
for arg in "$@"; do
  if [ "$arg" = --help ]; then exec "$here/answers" "$@"; fi
done
if [ -n "${FURB_CLAUDE_DISCOVER-}" ]; then exec "$here/answers" "$@"; fi
while IFS= read -r line; do
  while IFS='	' read -r seconds text; do
    case $line in *"$text"*) sleep "$seconds"; break ;; esac
  done < "$here/pace"
  printf '%s\n' "$line"
done | "$here/answers" "$@"
"#;

/// The usage of each request of a capture: 31,840 tokens read, 24,600 of them from the cache.
const USAGE: &str = r#"{"input_tokens":1340,"output_tokens":420,"cache_read_input_tokens":24600,"cache_creation_input_tokens":5900}"#;

/// The credential of Codex of the user of a capture, whose account is a claim of its access token,
/// which is no signed token here, and the models of its account.
const CODEX: &str = r#"{"access":"e30.eyJodHRwczovL2FwaS5vcGVuYWkuY29tL2F1dGgiOnsiY2hhdGdwdF9hY2NvdW50X2lkIjoiYWNjdCJ9fQ.s","refresh":"r","expires":4000000000000,"account":"acct"}"#;
const CODEX_MODELS: &str = r#"[{"slug":"gpt-5.5","context_window":400000,"visibility":"list","supported_reasoning_levels":[{"effort":"medium"},{"effort":"high"}],"input_modalities":["text","image"]}]"#;

/// The text of the empty input, which a window shows once it is ready for a walk.
pub(crate) const READY: &str = "Ask for a change";

/// What the window shows once it is ready, when it only reads.
const READS: &str = "Read only · you watch";

/// The second at which the clocks of the window stand still, 09:12 of the first day of 2026, so two
/// captures of one walk are equal: no mark of the window moves, and no time passes in it.
const STILL: &str = "1767258720";

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

/// What a capture runs: the folder of the programs `furb-window` and `furb` of the build, and
/// whether the clocks of the window move as a person sees them, or stand still, so each run of a
/// walk gives the same frames.
pub(crate) struct Programs {
  pub(crate) bin: PathBuf,
  pub(crate) moving: bool,
}

/// The walk of a scenario captured at a size, in a folder of its own that it removes at its end.
pub(crate) async fn capture(
  walk: &Walk,
  size: Size,
  programs: &Programs,
  root: &Path,
) -> Result<Captured, Error> {
  let _turn = turn(root).await?;
  let rig = Rig::made(walk, programs, root)?;
  let captured = rig.captured(walk, size).await;
  let removed = fs::remove_dir_all(&rig.root);
  let captured = captured?;
  removed.map_err(|source| Error::Write { path: rig.root.clone(), source })?;
  Ok(captured)
}

/// The turn of a capture at its root, which it holds until it ends. Two captures of one walk at
/// one size, from two trees or from a gate, take turns, since the root stays the same at each run:
/// the path of a session hues its thread. The system frees the turn of a capture that dies.
async fn turn(root: &Path) -> Result<File, Error> {
  let path = root.with_extension("lock");
  let locked = path.clone();
  let taken = task::spawn_blocking(move || {
    let file = File::create(&locked)?;
    file.lock()?;
    Ok(file)
  });
  taken.await.map_err(io::Error::other).flatten().map_err(|source| Error::Lock { path, source })
}

/// The files of the project of the window: a small package of python, so a mention finds a file
/// deep in folders, a dot file, and a folder that git leaves out.
const PROJECT: [(&str, &str); 8] = [
  ("README.md", "# demo\n\nThe project of the captures of the window.\n"),
  (".gitignore", "build/\n"),
  ("pyproject.toml", "[project]\nname = \"inventory\"\nversion = \"0.1.0\"\n"),
  ("src/inventory/__init__.py", ""),
  ("src/inventory/models.py", "class Item:\n  def __init__(self, name):\n    self.name = name\n"),
  (
    "src/inventory/storage/backends/json_backend.py",
    "import json\n\n\ndef save(items, path):\n  json.dump([one.__dict__ for one in items], open(path, 'w'))\n",
  ),
  ("src/inventory/storage/backends/sqlite_backend.py", "import sqlite3\n"),
  ("tests/test_storage.py", "def test_save():\n  pass\n"),
];

/// An image of the project, of one pixel, which the input takes when he pastes its path.
const SHOT: (&str, &[u8]) = ("shot.png", b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x02\x00\x00\x00\x90wS\xde\x00\x00\x00\x0cIDATx\x9cc\xf8\xb5\xa9\x19\x00\x04\xd9\x020\xdd z\x8c\x00\x00\x00\x00IEND\xaeB`\x82");

/// The project made a checkout of git on the branch `main`, whose files stand in its first commit,
/// so the window names its branch and a change of the model shows as a change of the checkout.
fn committed(project: &Path) -> Result<(), Error> {
  let user = ["-c", "user.name=furb", "-c", "user.email=frames@furb.local"];
  let steps: [&[&str]; 3] = [
    &["init", "-q", "-b", "main"],
    &["add", "-A"],
    &[&user[..], &["commit", "-q", "-m", "demo"]].concat(),
  ];
  for args in steps {
    let done = process::Command::new("git").arg("-C").arg(project).args(args).status();
    let fine = done.map_err(|source| Error::Process { what: "git", source })?.success();
    if !fine {
      let source = io::Error::other(format!("git {} failed", args.join(" ")));
      return Err(Error::Process { what: "git", source });
    }
  }
  Ok(())
}

/// The folders of a capture: the home of the operator with the project of the window, the cache
/// of furb, its config, and the claude command line with its words.
struct Rig {
  root: PathBuf,
  bin: PathBuf,
  moving: bool,
}

impl Rig {
  /// The rig in a new folder. Its path is the real one, as the window reads its own folder, so the
  /// window finds its project under the home and writes it `~/demo`.
  fn made(walk: &Walk, programs: &Programs, root: &Path) -> Result<Self, Error> {
    let made = |source| Error::Write { path: root.to_owned(), source };
    // A capture that was killed leaves its root.
    if root.exists() {
      fs::remove_dir_all(root).map_err(made)?;
    }
    fs::create_dir_all(root).map_err(made)?;
    let root = &fs::canonicalize(root).map_err(made)?;
    let rig = Self { root: root.clone(), bin: programs.bin.clone(), moving: programs.moving };
    let write = |path: PathBuf, text: &str| -> Result<(), Error> {
      fs::create_dir_all(path.parent().unwrap_or(root))
        .map_err(|source| Error::Write { path: path.clone(), source })?;
      put(&path, text.as_bytes())
    };
    for (path, text) in PROJECT {
      write(rig.project().join(path), text)?;
    }
    put(&rig.project().join(SHOT.0), SHOT.1)?;
    committed(&rig.project())?;
    write(root.join("config/config.json"), r#"{"skills": {"naming": false}}"#)?;
    let listed = serde_json::json!({ "workspaces": [{ "directory": rig.project() }] });
    write(root.join("config/workspaces.json"), &listed.to_string())?;
    // The account of Codex of the user serves, with a new list of its models, so a sign-in from
    // the models connects at once and no request goes out. Codex stays off until a sign-in.
    write(root.join("config/auth/codex.json"), CODEX)?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
    let models = format!(r#"{{"account":"acct","fetched":{now},"models":{CODEX_MODELS}}}"#);
    write(root.join("config/auth/codex.models.json"), &models)?;
    if walk.pace.is_empty() {
      write(rig.claude(), CLAUDE)?;
    } else {
      let answers = root.join("fake/answers");
      write(answers.clone(), CLAUDE)?;
      executable(&answers)?;
      write(rig.claude(), PACED)?;
      let mut pace = String::new();
      for (text, time) in walk.pace {
        writeln!(pace, "{:.3}\t{text}", time.as_secs_f64()).ok();
      }
      write(root.join("fake/pace"), &pace)?;
    }
    executable(&rig.claude())?;
    // Each request reads as much as a turn of a real life does, most of it from the cache, and
    // costs what the price of the model makes of it, so the box of the context shows real counts.
    write(root.join("fake/usage"), USAGE)?;
    write(root.join("fake/cost"), "0.11")?;
    // The sessions of the walk take the first turns, with the replies of the claude command line.
    for (n, word) in walk.words.iter().enumerate() {
      write(
        root.join(format!("fake/word.{}", walk.sessions.len() + n + 1)),
        &serde_json::to_string(word)?,
      )?;
    }
    Ok(rig)
  }

  /// The walk in tmux, on the sessions that it starts on and on a daemon of its own, which it ends
  /// with tmux, whatever the walk gave.
  async fn captured(&self, walk: &Walk, size: Size) -> Result<Captured, Error> {
    for (n, message) in walk.sessions.iter().enumerate() {
      self.seeded(n, message).await?;
      // A card stands by the second of its last fact, so each session takes a second of its own,
      // and the list orders them the same way at each capture.
      let into = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos();
      sleep(Duration::from_secs(1).saturating_sub(Duration::from_nanos(u64::from(into)))).await;
    }
    let mut daemon = self.daemon().await?;
    let name =
      self.root.file_name().map_or_else(String::new, |name| name.to_string_lossy().into_owned());
    // A tmux server can outlive the turn of the capture before it for a moment, so each run
    // speaks to a server of its own.
    let tmux = Tmux { socket: format!("ff{name}-{}", process::id()) };
    let walked = self.walked(&tmux, walk, size).await;
    let ended = tmux.run(&["kill-server"]).await;
    // The daemon ends by its grace once the window is gone. A kill before that lets a window that
    // still dies dial again, find no daemon and start one of its own, which no peer reaches and
    // which outlives the capture: so the kill comes only when the daemon outlived its patience.
    if timeout(PATIENCE, daemon.wait()).await.is_err() {
      daemon.kill().await.map_err(|source| Error::Process { what: "the daemon", source })?;
    }
    let (shown, marks, start) = walked?;
    ended?;
    Ok(Captured { cast: self.cast(size, start, marks)?, shown })
  }

  /// A session of the project before the window, which says one message and takes the answer of the
  /// model. Its record is named as the window names one, at a fixed moment, so its card is the same
  /// at each capture.
  async fn seeded(&self, n: usize, message: &str) -> Result<(), Error> {
    let record =
      self.project().join(format!(".furb/sessions/2026-11-{:02}T09-00-00-0000.jsonl", n + 1));
    let said = Command::new(self.bin.join("furb"))
      .arg("prompt")
      .arg(message)
      .arg("--record")
      .arg(&record)
      .arg("--cwd")
      .arg(self.project())
      .env_clear()
      .envs(self.environment())
      .current_dir(self.project())
      .stdout(Stdio::null())
      .stderr(Stdio::piped())
      .kill_on_drop(true)
      .output();
    let said = timeout(PATIENCE, said)
      .await
      .map_err(|_elapsed| Error::Seed(message.to_owned(), "it took too long".to_owned()))?;
    let said = said.map_err(|source| Error::Process { what: "furb prompt", source })?;
    if said.status.success() {
      Ok(())
    } else {
      Err(Error::Seed(message.to_owned(), String::from_utf8_lossy(&said.stderr).trim().to_owned()))
    }
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
    let still = (!self.moving).then(|| ("FURB_STILL_CLOCK", STILL.to_owned()));
    let mut environment = vec![
      ("HOME", at("home")),
      ("PATH", path),
      ("TERM", "xterm-256color".to_owned()),
      ("COLORTERM", "truecolor".to_owned()),
      ("LANG", "en_US.UTF-8".to_owned()),
      ("TZ", "UTC".to_owned()),
      ("XDG_CACHE_HOME", at("cache")),
      ("FURB_CONFIG_DIR", at("config")),
      ("FURB_CLAUDE_BIN", self.claude().display().to_string()),
      // A click on a link opens no page in front of the person at this machine.
      ("BROWSER", "true".to_owned()),
    ];
    environment.extend(still);
    environment
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
  /// markers and what the window read, each at its moment, and the moment the window started.
  async fn walked(
    &self,
    tmux: &Tmux,
    walk: &Walk,
    size: Size,
  ) -> Result<(Vec<(String, String)>, Vec<(SystemTime, What)>, SystemTime), Error> {
    let go = self.root.join("go");
    let env: Vec<String> =
      self.environment().into_iter().map(|(key, value)| format!("{key}={value}")).collect();
    let read = if walk.reads { " --read" } else { "" };
    let started = format!(
      "env -i {} sh -c 'until [ -e {} ]; do sleep 0.01; done; exec furb-window{read}'",
      env.join(" "),
      go.display()
    );
    let (cols, rows) = (size.cols.to_string(), size.rows.to_string());
    let project = self.project().display().to_string();
    tmux
      .run(&["new-session", "-d", "-x", &cols, "-y", &rows, "-s", "w", "-c", &project, &started])
      .await?;
    tmux.run(&["set-option", "-g", "status", "off"]).await?;
    // A key after Escape is its own key, as a person types it, and no chord of alt.
    tmux.run(&["set-option", "-g", "escape-time", "0"]).await?;
    let me =
      env::current_exe().map_err(|source| Error::Process { what: "this program", source })?;
    let stamped = format!("'{}' stamp '{}'", me.display(), self.stamps().display());
    tmux.run(&["pipe-pane", "-O", "-t", "w", &stamped]).await?;
    let start = SystemTime::now();
    put(&go, b"")?;
    tmux.shows("ready", if walk.reads { READS } else { READY }).await?;
    let mut shown = Vec::new();
    let mut marks = Vec::new();
    // Each other window holds its session until the walk ends.
    let mut others = Vec::new();
    for step in walk.steps {
      // The messages of other windows go before the acts of this one in the step.
      for act in step.acts {
        if let Act::Said(message) = act {
          others.push(self.said(message).await?);
        }
      }
      let screen = self.stepped(tmux, step, &mut marks).await?;
      shown.push((step.name.to_owned(), screen));
    }
    Ok((shown, marks, start))
  }

  /// Another window of the operator on the daemon of the walk, which opens a new session of the
  /// project and says a message in it, as a person who writes in a second window. It gives the
  /// stream of that window, whose session lives while the stream stands.
  async fn said(&self, message: &str) -> Result<BufReader<<Local as Dialer>::Stream>, Error> {
    let failed = |why: String| Error::Said(message.to_owned(), why);
    let local = Local::new(self.root.join("cache/furb/host"));
    let stream = local.dial().await.map_err(|why| failed(why.to_string()))?;
    let mut lines = BufReader::new(stream);
    let life = self.project().join(".furb/sessions/2026-11-15T09-30-00-0000.jsonl");
    let life = life.display().to_string();
    let hello = Hello { protocol: PROTOCOL, kind: Kind::Window, name: None, role: None };
    let open = Verb::Open {
      life: life.clone(),
      cwd: Some(self.project().display().to_string()),
      actor: None,
      roster: None,
      skills: None,
      config: None,
      claude: None,
      answers: false,
      attach: Some(0),
    };
    let prompt = Verb::Prompt {
      life,
      shape: "None".to_owned(),
      message: message.to_owned(),
      to: None,
      on: None,
    };
    for (id, verb) in [(0, Verb::Hello(hello)), (1, open), (2, prompt)] {
      let request = serde_json::to_string(&Request { id, verb })? + "\n";
      lines.get_mut().write_all(request.as_bytes()).await.map_err(|why| failed(why.to_string()))?;
      loop {
        let mut heard = String::new();
        let read = timeout(PATIENCE, lines.read_line(&mut heard)).await;
        let read = read.map_err(|_elapsed| failed("the daemon took too long".to_owned()))?;
        if read.map_err(|why| failed(why.to_string()))? == 0 {
          return Err(failed("the daemon closed the window".to_owned()));
        }
        match serde_json::from_str(&heard)? {
          Frame::Reply { id: got, answer } if got == id => {
            if let Answer::Welcome { .. } | Answer::Done | Answer::Act(_) = answer {
              break;
            }
            return Err(failed(format!("the daemon answered {answer:?}")));
          }
          Frame::Refused { id: Some(got), why, .. } if got == id => return Err(failed(why)),
          _ => {}
        }
      }
    }
    Ok(lines)
  }

  /// One step: its acts, each marked as what the window read at its moment, then the wait for its
  /// screen and for the silence of the window, marked by the name of the step. It gives the screen
  /// that tmux showed just after its marker.
  async fn stepped(
    &self,
    tmux: &Tmux,
    step: &Step,
    marks: &mut Vec<(SystemTime, What)>,
  ) -> Result<String, Error> {
    for act in step.acts {
      if let Some(read) = act.read() {
        marks.push((SystemTime::now(), What::Input(read)));
      }
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
    marks.push((SystemTime::now(), What::Marker(step.name.to_owned())));
    tmux.screen().await
  }

  fn stamps(&self) -> PathBuf {
    self.root.join("stamps")
  }

  fn written(&self) -> u64 {
    fs::metadata(self.stamps()).map_or(0, |file| file.len())
  }

  /// The recording: what the window wrote, each part at its time, what it read, and the markers.
  fn cast(
    &self,
    size: Size,
    start: SystemTime,
    marks: Vec<(SystemTime, What)>,
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
    events.extend(marks.into_iter().map(|(at, what)| Event { at: since(at), what }));
    events.sort_by_key(|event| event.at);
    Ok(Cast { cols: size.cols, rows: size.rows, events })
  }
}

/// What `tmux pipe-pane` gives, written as lines of the time in microseconds and the text as JSON.
/// A glyph cut between two reads waits for its end.
pub(crate) fn stamp(path: &Path) -> Result<(), Error> {
  let wrote = |source| Error::Write { path: path.to_owned(), source };
  let mut file = File::create(path).map_err(wrote)?;
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
      Act::Key(key) => {
        self.run(&["send-keys", "-t", "w", key]).await?;
        // A terminal reads Escape and the key after it in one read as a chord of alt; a person's
        // next key comes later.
        if *key == "Escape" {
          sleep(Duration::from_millis(100)).await;
        }
        Ok(())
      }
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
      Act::Until(text) => self.shows("until", text).await,
      // Another window says it before the step, and no key of this window.
      Act::Said(_) => Ok(()),
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
#[expect(
  clippy::unnecessary_wraps,
  reason = "the twin of unix, which can fail, gives the same type"
)]
fn executable(_path: &Path) -> Result<(), Error> {
  Ok(())
}
