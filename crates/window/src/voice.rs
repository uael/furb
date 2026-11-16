//! The voice of the operator in the input. While he holds the space bar, the program of the
//! speech, Whisper on this Mac, writes what he says into the input as he says it, in the language
//! that he speaks, and the record ends when he lets the bar go. A terminal tells a held key only
//! by its repeats, so a space that comes as soon after the one before as a repeat comes starts the
//! record, and the spaces of the hold leave the input: one tap of the bar stays a space. The
//! program starts with the window and readies the speech with no microphone, so a record only
//! opens the microphone, and a tap never opens it.

use std::{
  fs, io,
  path::{Path, PathBuf},
  process::{self, Stdio},
  time::{Duration, Instant},
};

use furb_client::{Form, Time};
use futures::SinkExt;
use opentui::input::KeyEvent;
use serde::Deserialize;
use tokio::{
  process::{Child, ChildStdout, Command},
  runtime::Handle,
  sync::mpsc::{self, UnboundedSender},
};
use tokio_util::codec::{FramedWrite, LinesCodec};

/// A space this soon after the space before it is a repeat of the bar that he holds, which a
/// terminal sends far sooner than a hand types two spaces.
const REPEATS: Duration = Duration::from_millis(100);

/// The longest pause of a terminal between the press of a key and its first repeat.
const FIRST: Duration = Duration::from_secs(2);

/// How long the repeats of the bar stop before the record ends: he let the bar go.
pub(crate) const RELEASED: Duration = Duration::from_millis(200);

/// The program of the speech that the build carries, `listen`, which is empty where it has none.
const CARRIED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/listen"));

/// The spaces that he typed one after the other, the program of the speech while it runs, and the
/// record while one runs.
#[derive(Default)]
pub(crate) struct Voice {
  run: Option<Run>,
  /// Whether the terminal told the release of a key, which a terminal tells only with the keys of
  /// Kitty that tell each kind of event.
  pub(crate) releases: bool,
  /// The press of a space that waits for its release to play or stop the bar of the time.
  pub(crate) waits: Option<KeyEvent>,
  pub(crate) listener: Option<Listener>,
  pub(crate) record: Option<Record>,
}

/// The program of the speech, and what tells it to start and to stop a record. It ends with the
/// window, which drops it.
pub(crate) struct Listener {
  _child: Child,
  says: UnboundedSender<&'static str>,
}

/// A record of his voice: the text of the input around the caret where the voice writes, the last
/// repeat of the bar, and whether the window told the program to stop.
pub(crate) struct Record {
  pub(crate) before: String,
  pub(crate) after: String,
  pub(crate) last: Instant,
  pub(crate) stops: bool,
}

/// The spaces of a run, which come one after the other: when the last came, how many the input
/// typed, and the parts that a tap changes as they stood before the run: the bar of the time, which
/// a tap plays, and the form of a question, which a tap ticks.
#[derive(Clone)]
pub(crate) struct Run {
  last: Instant,
  pub(crate) typed: usize,
  pub(crate) parts: (Option<Time>, Option<Form>),
}

/// What a space does.
pub(crate) enum Space {
  /// It is a tap: the input or a part takes it, as a key of his typing. The first tap of a run
  /// keeps the parts as they stood.
  Taps(bool),
  /// It starts a record, once the taps of the run are undone: the spaces that the input typed
  /// leave it, and the parts stand again as before the run.
  Starts(Box<Run>),
  /// It is a repeat of the bar during the record.
  Holds,
}

/// A line of the program of the speech.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum Said {
  Ready {
    #[serde(rename = "ready")]
    _ready: bool,
  },
  Loading {
    loading: Stage,
    #[serde(default)]
    done: Option<u8>,
  },
  Level {
    level: u8,
  },
  Text {
    text: String,
    #[serde(default, rename = "final")]
    ends: bool,
  },
  Fault {
    error: String,
  },
}

/// What the program of the speech does before it is ready: it downloads its model, with the share
/// done, or prepares it for this Mac.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Stage {
  Download,
  Prepare,
}

impl Voice {
  /// A space that he pressed at a moment.
  pub(crate) fn space(&mut self, now: Instant) -> Space {
    if let Some(record) = &mut self.record {
      record.last = now;
      return Space::Holds;
    }
    match self.run.take() {
      Some(run) if now.saturating_duration_since(run.last) <= REPEATS => {
        Space::Starts(Box::new(run))
      }
      // A run holds the taps that the first repeat of a hold can follow, which a terminal sends at
      // most two seconds after the press.
      Some(run) if now.saturating_duration_since(run.last) <= FIRST => {
        self.run = Some(Run { last: now, ..run });
        Space::Taps(false)
      }
      _ => {
        self.run = Some(Run { last: now, typed: 0, parts: (None, None) });
        Space::Taps(true)
      }
    }
  }

  /// Where the last tap went: to a part, or to the input, which typed it.
  pub(crate) fn tapped(&mut self, parted: bool) {
    if let Some(run) = self.run.as_mut().filter(|_| !parted) {
      run.typed += 1;
    }
  }

  /// The parts that a tap changes, as they stand before the first tap of a run.
  pub(crate) fn keep(&mut self, parts: (Option<Time>, Option<Form>)) {
    if let Some(run) = &mut self.run {
      run.parts = parts;
    }
  }

  /// A key that is no space, which ends a run of spaces.
  pub(crate) fn other(&mut self) {
    self.run = None;
  }
}

impl Listener {
  /// A line told to the program: `start` or `stop`. Whether the program still hears.
  pub(crate) fn say(&self, line: &'static str) -> bool {
    self.says.send(line).is_ok()
  }
}

/// The program of the speech: the one that the window names, else the one that the build carries,
/// written once in a folder. None where the build carries none.
pub(crate) fn program(named: Option<&Path>, folder: &Path) -> io::Result<Option<PathBuf>> {
  if let Some(named) = named {
    return Ok(Some(named.to_owned()));
  }
  if CARRIED.is_empty() {
    return Ok(None);
  }
  // The size names the build of the program, so a new build writes a new file.
  let path = folder.join(format!("listen-{}", CARRIED.len()));
  if !path.is_file() {
    fs::create_dir_all(folder)?;
    let written = folder.join(format!("listen-{}.part", process::id()));
    fs::write(&written, CARRIED)?;
    #[cfg(unix)]
    {
      use std::os::unix::fs::PermissionsExt;
      fs::set_permissions(&written, fs::Permissions::from_mode(0o755))?;
    }
    fs::rename(&written, &path)?;
  }
  Ok(Some(path))
}

/// The program started on a runtime, on the folder of its models, with what tells it each line, and
/// its output.
pub(crate) fn started(
  program: &Path,
  models: &Path,
  runtime: &Handle,
) -> io::Result<(Listener, ChildStdout)> {
  let _runtime = runtime.enter();
  let mut child = Command::new(program)
    .arg(models)
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::null())
    .kill_on_drop(true)
    .spawn()?;
  let gone = || io::Error::other("the program of the speech gave no pipe");
  let input = child.stdin.take().ok_or_else(gone)?;
  let output = child.stdout.take().ok_or_else(gone)?;
  let (says, mut heard) = mpsc::unbounded_channel();
  // The task ends once the window drops the listener, and the input of the program closes with it,
  // which ends the program.
  drop(runtime.spawn(async move {
    let mut lines = FramedWrite::new(input, LinesCodec::new());
    while let Some(line) = heard.recv().await {
      if lines.send(line).await.is_err() {
        break;
      }
    }
  }));
  Ok((Listener { _child: child, says }, output))
}
