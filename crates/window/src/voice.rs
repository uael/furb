//! The voice of the operator in the input. While he holds the space bar, the program of the speech
//! of the system writes what he says into the input as he says it, and the record ends when he lets
//! the bar go. A terminal tells a held key only by its repeats, so a space that comes as soon after
//! the one before as a repeat comes starts the record, and the spaces of the hold leave the input:
//! one tap of the bar stays a space.

use std::{
  fs, io,
  path::{Path, PathBuf},
  process::{self, Stdio},
  time::{Duration, Instant},
};

use serde::Deserialize;
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

/// A space this soon after the space before it is a repeat of the bar that he holds, which a
/// terminal sends far sooner than a hand types two spaces.
const REPEATS: Duration = Duration::from_millis(100);

/// How long the repeats of the bar stop before the record ends: he let the bar go.
pub(crate) const RELEASED: Duration = Duration::from_millis(200);

/// The program of the speech of macOS that the build carries, which is empty where it has none.
const CARRIED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/listen"));

/// The spaces that he typed one after the other, and the record while one runs.
#[derive(Default)]
pub(crate) struct Voice {
  run: Option<(Instant, usize)>,
  pub(crate) record: Option<Record>,
}

/// A record of his voice: the program that listens, its input, which the window closes to end the
/// record, the text of the input around the caret where the voice writes, the last repeat of the
/// bar, and the last level of the microphone, from 0 to 100.
pub(crate) struct Record {
  pub(crate) child: Child,
  pub(crate) input: Option<ChildStdin>,
  pub(crate) before: String,
  pub(crate) after: String,
  pub(crate) last: Instant,
  pub(crate) level: u8,
}

/// What a space does.
pub(crate) enum Space {
  /// It types a space, as a key of his typing.
  Types,
  /// It starts a record, once the spaces of the hold, which it counts, leave the input.
  Starts(usize),
  /// It is a repeat of the bar during the record.
  Holds,
}

/// A line of the program of the speech.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum Said {
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

impl Voice {
  /// A space that he pressed at a moment.
  pub(crate) fn space(&mut self, now: Instant) -> Space {
    if let Some(record) = &mut self.record {
      record.last = now;
      return Space::Holds;
    }
    let (count, soon) = match self.run {
      Some((last, count)) => (count + 1, now.saturating_duration_since(last) <= REPEATS),
      None => (1, false),
    };
    self.run = Some((now, count));
    if soon {
      self.run = None;
      return Space::Starts(count - 1);
    }
    Space::Types
  }

  /// A key that is no space, which ends a run of spaces.
  pub(crate) fn other(&mut self) {
    self.run = None;
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

/// The program started on the runtime of the app, with its input and its output.
pub(crate) fn started(program: &Path) -> io::Result<(Child, ChildStdin, ChildStdout)> {
  let mut child = Command::new(program)
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::null())
    .kill_on_drop(true)
    .spawn()?;
  let gone = || io::Error::other("the program of the speech gave no pipe");
  let input = child.stdin.take().ok_or_else(gone)?;
  let output = child.stdout.take().ok_or_else(gone)?;
  Ok((child, input, output))
}
