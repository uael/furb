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
  sync::OnceLock,
  time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use furb_host::{
  protocol::{Answer, Frame, Hello, Kind, PROTOCOL, Request, Verb},
  transport::{Dialer, Local, LocalApi},
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
  openai,
  replay::put,
  scenario::{Act, Size, Step, Walk},
  tailscale::tailnet,
};

/// The login that the stand-in of Tailscale gives the window on the far machine of a walk.
const LOGIN: &str = "demo@github";

/// The claude command line of the tests, which answers the Nth turn with the file `word.N`
/// beside it.
const CLAUDE: &str = include_str!("../../../src/world/provider/claude/fake-claude.sh");

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

/// The thought of a model that a walk asks to think, as a real model thinks before it answers: long
/// enough that its line shows its first sentence and a click opens it on more than one row.
const THOUGHT: &str = "The open reads the record, then boots the engine and replays each word since the last checkpoint. A sleep of two seconds stands in for the slow part, so I time it with a command first, then say where the time goes: the seed, which each open reads again.";

/// The browser of a capture, which opens a page of the stand-in of `OpenAI` as a person who signs
/// in there, and no other page.
const BROWSER: &str = r#"#!/bin/sh
case $1 in "$FURB_OPENAI_URL"/*) exec curl -s -o /dev/null "$1" ;; esac
"#;

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

/// The walk of a scenario captured at a size, in a folder of its own that it removes at its end. A
/// walk that fails leaves its folder, with the record and the log of its daemon, to trace the
/// failure, until the next capture of the walk at that size.
pub(crate) async fn capture(
  walk: &Walk,
  size: Size,
  programs: &Programs,
  root: &Path,
) -> Result<Captured, Error> {
  let _turn = turn(root).await?;
  let openai = openai::served().await?;
  let rig = Rig::made(walk, programs, root, &openai.url)?;
  let captured = rig.captured(walk, size).await?;
  fs::remove_dir_all(&rig.root)
    .map_err(|source| Error::Write { path: rig.root.clone(), source })?;
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

/// An image of the project, a warm sun over grey hills of 192 by 108 pixels, which the input
/// takes when he pastes its path, and which the feed then draws in a box of 6 rows.
const SHOT: (&str, &[u8]) = ("shot.png", include_bytes!("shot.png"));

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
  openai: String,
  /// The tailnet of the far machine of a walk, once it listens there: the variables by which a
  /// window finds it through the stand-in of Tailscale.
  tailnet: OnceLock<[(&'static str, String); 2]>,
}

impl Rig {
  /// The rig in a new folder. Its path is the real one, as the window reads its own folder, so the
  /// window finds its project under the home and writes it `~/demo`.
  fn made(walk: &Walk, programs: &Programs, root: &Path, openai: &str) -> Result<Self, Error> {
    let made = |source| Error::Write { path: root.to_owned(), source };
    // A capture that was killed leaves its root.
    if root.exists() {
      fs::remove_dir_all(root).map_err(made)?;
    }
    fs::create_dir_all(root).map_err(made)?;
    let root = &fs::canonicalize(root).map_err(made)?;
    let rig = Self {
      root: root.clone(),
      bin: programs.bin.clone(),
      moving: programs.moving,
      openai: openai.to_owned(),
      tailnet: OnceLock::new(),
    };
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
    let mut skills = serde_json::json!({ "naming": false });
    for kit in walk.kits {
      skills[*kit] = true.into();
    }
    write(root.join("config/config.json"), &serde_json::json!({ "skills": skills }).to_string())?;
    let listed = serde_json::json!({ "workspaces": [{ "directory": rig.project() }] });
    write(root.join("config/workspaces.json"), &listed.to_string())?;
    if walk.far.is_some() {
      rig.far_made()?;
    }
    write(root.join("fake/browser"), BROWSER)?;
    executable(&root.join("fake/browser"))?;
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
    write(root.join("fake/thought"), THOUGHT)?;
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
    let mut far = match walk.far {
      Some(name) => Some(self.far_daemon(name).await?),
      None => None,
    };
    let mut daemon = self.daemon().await?;
    // The server listens in the folder of the capture. It can outlive the turn of the capture
    // before it for a moment, so each run speaks to a server of its own.
    let tmux = Tmux { socket: self.root.join(format!("tmux-{}", process::id())) };
    let walked = self.walked(&tmux, walk, size).await;
    let ended = tmux.run(&["kill-server"]).await;
    // The daemon ends by its grace once the window is gone. A kill before that lets a window that
    // still dies dial again, find no daemon and start one of its own, which no peer reaches and
    // which outlives the capture: so the kill comes only when the daemon outlived its patience.
    if timeout(PATIENCE, daemon.wait()).await.is_err() {
      daemon.kill().await.map_err(|source| Error::Process { what: "the daemon", source })?;
    }
    // A daemon on the tailnet ends by no grace, and the window never starts one there.
    if let Some(far) = far.as_mut() {
      far.kill().await.map_err(|source| Error::Process { what: "the far daemon", source })?;
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
    self.prompted(&self.environment(), &record, message).await
  }

  /// A message said to a new session of a record, with no window, in an environment, in the
  /// project whose sessions hold the record.
  async fn prompted(
    &self,
    environment: &[(&'static str, String)],
    record: &Path,
    message: &str,
  ) -> Result<(), Error> {
    let project = record.ancestors().nth(3).unwrap_or(&self.root);
    let said = Command::new(self.bin.join("furb"))
      .arg("prompt")
      .arg(message)
      .arg("--record")
      .arg(record)
      .arg("--cwd")
      .arg(project)
      .env_clear()
      .envs(environment.iter().cloned())
      .current_dir(project)
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

  /// The folders of the far machine of a walk: its home, its cache, its data and its config.
  fn far(&self) -> PathBuf {
    self.root.join("far")
  }

  /// The folders of the far machine of a walk: its project `lab`, and its config, which lists that
  /// project and turns the tailnet on, by TCP on a port that the system gives.
  fn far_made(&self) -> Result<(), Error> {
    let lab = self.far().join("home/lab");
    let write = |path: PathBuf, text: String| -> Result<(), Error> {
      fs::create_dir_all(path.parent().unwrap_or(&self.root))
        .map_err(|source| Error::Write { path: path.clone(), source })?;
      put(&path, text.as_bytes())
    };
    write(lab.join("README.md"), "# lab\n".to_owned())?;
    committed(&lab)?;
    let config = self.far().join("config");
    write(config.join("config.json"), r#"{"skills": {"naming": false}}"#.to_owned())?;
    let listed = serde_json::json!({ "workspaces": [{ "directory": lab }] });
    write(config.join("workspaces.json"), listed.to_string())?;
    let host = serde_json::json!({ "tailnet": { "tcp": 0 }, "roles": { LOGIN: "write" } });
    write(config.join("host.json"), host.to_string())
  }

  /// The far machine of a walk on the tailnet, which a stand-in of the local API of Tailscale names
  /// online under a name, and whose identity it gives: one session of its project said first, then
  /// its daemon, which this process ends, and the variables by which the window finds it at the
  /// port where it listens. A release build takes no stand-in of Tailscale, so its far daemon does
  /// not start.
  async fn far_daemon(&self, name: &str) -> Result<Child, Error> {
    let LocalApi::Port { port, token } = tailnet(LOGIN, &[name]).await.0 else {
      return Err(Error::Listen);
    };
    let mut environment = self.environment_of(&self.far());
    environment.push(("FURB_TAILSCALE_API", format!("{port}:{token}")));
    let lab = self.far().join("home/lab");
    let record = lab.join(".furb/sessions/2026-11-01T09-00-00-0000.jsonl");
    self.prompted(&environment, &record, "Set up the lab.").await?;
    // A daemon on the tailnet ends by no bound, so it ends with this process.
    let mut daemon = Command::new("sh")
      .arg("-c")
      .arg(format!("{} exec \"$0\" host --grace 1", tied()))
      .arg(self.bin.join("furb"))
      .env_clear()
      .envs(environment)
      .stdout(Stdio::null())
      .stderr(Stdio::piped())
      .kill_on_drop(true)
      .spawn()
      .map_err(|source| Error::Process { what: "the far daemon", source })?;
    let mut said = BufReader::new(daemon.stderr.take().ok_or(Error::Listen)?).lines();
    let listens = async {
      while let Ok(Some(line)) = said.next_line().await {
        if let Some(at) = line.strip_prefix("furb host: listens on the tailnet at ") {
          return at.split_once(" by ").map(|one| one.0.to_owned());
        }
      }
      None
    };
    let address = timeout(PATIENCE, listens).await.ok().flatten().ok_or(Error::Listen)?;
    let at = address.rsplit_once(':').map(|one| one.1.to_owned()).ok_or(Error::Listen)?;
    // What the daemon says later goes nowhere, and never fills the pipe that it writes to.
    drop(task::spawn(async move { while let Ok(Some(_)) = said.next_line().await {} }));
    let api = ("FURB_TAILSCALE_API", format!("{port}:{token}"));
    self.tailnet.set([api, ("FURB_TAILNET_PORT", at)]).ok();
    Ok(daemon)
  }

  fn claude(&self) -> PathBuf {
    self.root.join("fake/claude")
  }

  /// The environment of the window and of its daemon, and nothing of the environment of this
  /// process, so a capture is the same on each machine.
  fn environment(&self) -> Vec<(&'static str, String)> {
    let mut environment = self.environment_of(&self.root);
    environment.extend(self.tailnet.get().into_iter().flatten().cloned());
    environment
  }

  /// The environment of a machine of the capture whose folders stand in a place, with the
  /// programs and the stand-ins of the capture.
  fn environment_of(&self, place: &Path) -> Vec<(&'static str, String)> {
    let path = format!("{}:/usr/bin:/bin", self.bin.display());
    let at = |folder: &str| place.join(folder).display().to_string();
    let still = (!self.moving).then(|| ("FURB_STILL_CLOCK", STILL.to_owned()));
    let mut environment = vec![
      ("HOME", at("home")),
      ("PATH", path),
      ("TERM", "xterm-256color".to_owned()),
      ("COLORTERM", "truecolor".to_owned()),
      ("LANG", "en_US.UTF-8".to_owned()),
      ("TZ", "UTC".to_owned()),
      ("XDG_CACHE_HOME", at("cache")),
      ("XDG_DATA_HOME", at("data")),
      ("FURB_CONFIG_DIR", at("config")),
      ("FURB_CLAUDE_BIN", self.claude().display().to_string()),
      // A click on a link opens no page in front of the person at this machine.
      ("BROWSER", self.root.join("fake/browser").display().to_string()),
      ("FURB_OPENAI_URL", self.openai.clone()),
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
    // The server of tmux, left with no pane once the window ends with this process, ends too.
    let started = format!(
      "env -i {} sh -c '{} until [ -e {} ]; do sleep 0.01; done; exec furb-window{read}'",
      env.join(" "),
      tied(),
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
    // A wait passes on the screen that the input before it made, so the screen just before that
    // input must not hold its text: else the wait passes before the window drew what the input
    // asked, as on a window that a load slows.
    let mut before = None;
    for act in step.acts {
      if let Some(read) = act.read() {
        if before.is_none() {
          before = Some(tmux.screen().await?);
        }
        marks.push((SystemTime::now(), What::Input(read)));
      }
      match act {
        Act::Until(text) => tmux.waits(step.name, text, before.take()).await?,
        _ => tmux.act(act).await?,
      }
    }
    tmux.waits(step.name, step.until, before).await?;
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

/// The words of a shell that tie the program that it then runs with `exec` to this process: a watch
/// beside it ends it once this process is gone, as when the bound of a test kills it before its
/// end, so no program of a capture outlives its run. The watch ends with the program.
fn tied() -> String {
  let watch = "while kill -0 {} 2>/dev/null && kill -0 $w 2>/dev/null; do sleep 0.2; done";
  let watch = watch.replace("{}", &process::id().to_string());
  format!("w=$$; ({watch}; kill -9 $w 2>/dev/null) </dev/null >/dev/null 2>&1 &")
}

/// A server of tmux of one capture.
struct Tmux {
  socket: PathBuf,
}

impl Tmux {
  async fn run(&self, args: &[&str]) -> Result<String, Error> {
    let done = Command::new("tmux")
      .arg("-S")
      .arg(&self.socket)
      .args(["-f", "/dev/null", "-u"])
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

  /// The screen once it shows a text that the screen before the input of the wait did not hold;
  /// no text is no wait.
  async fn waits(&self, step: &str, text: &str, before: Option<String>) -> Result<(), Error> {
    if text.is_empty() {
      return Ok(());
    }
    if before.is_some_and(|screen| screen.contains(text)) {
      return Err(Error::Early { step: step.to_owned(), until: text.to_owned() });
    }
    self.shows(step, text).await
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
      // A wait runs in its step, which knows the screen before the input of the wait; another
      // window says its message before the step, and no key of this window.
      Act::Until(_) | Act::Said(_) => Ok(()),
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
