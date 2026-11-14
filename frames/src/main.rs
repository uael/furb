//! `furb-frames`: the frames of the window of furb, in the one format that a model reads for the
//! window and for any other program in a terminal. `capture` walks the real window through each
//! scenario at each size and writes its recording and its key frames; `replay` turns any
//! recording into its key frames.

mod capture;
mod cast;
mod compare;
mod error;
mod frame;
mod probe;
mod raster;
mod replay;
mod scenario;

use std::{
  env, fs,
  path::{Path, PathBuf},
  process::ExitCode,
  sync::Arc,
  time::{Duration, Instant},
};

use clap::{Parser, Subcommand};
use tokio::task::JoinSet;

use crate::{
  cast::Cast,
  error::Error,
  replay::put,
  scenario::{SCENARIOS, Scenario, Size, Walk},
};

/// The step of time of the frames of motion of a capture: the step of the captures of the other
/// programs.
const MOTION: Duration = Duration::from_millis(200);

#[derive(Parser)]
#[command(about = "The frames of the window of furb, and of any program in a terminal")]
struct Command {
  #[command(subcommand)]
  does: Does,
}

#[derive(Subcommand)]
enum Does {
  /// Walk the real window through scenarios at each size, with no model and no network, and write
  /// in `<out>/<cols>x<rows>/` the recording of each and its key frames.
  Capture {
    /// The scenarios, by number or name: by default each that the window has.
    scenarios: Vec<String>,
    #[arg(long, default_value = "target/frames")]
    out: PathBuf,
    /// The folder of the programs `furb-window` and `furb`: by default the folder of this one.
    #[arg(long)]
    bin: Option<PathBuf>,
    /// How many captures run at once.
    #[arg(long, default_value_t = 4)]
    jobs: usize,
  },
  /// Write the key frames of recordings of asciinema (format 3): one at each marker, as text,
  /// cells and image, and one at each step of time where the screen changed, as text and cells.
  Replay {
    casts: Vec<PathBuf>,
    /// The folder of the frames: by default `replay` beside each recording.
    #[arg(long)]
    out: Option<PathBuf>,
    /// The step of time in milliseconds; none writes the markers alone.
    #[arg(long)]
    every: Option<u64>,
  },
  /// The frame of the window in a scenario beside the two winner frames of the other programs, the
  /// scores of each on each aspect and the findings of the probes. The other programs stand in the
  /// folder that `FURB_RIVALS` names.
  Compare {
    scenario: String,
    #[arg(long, default_value = "target/frames")]
    out: PathBuf,
  },
  /// The findings of the probes on frames of the window, from their cells as JSON.
  Probe { frames: Vec<PathBuf> },
  /// What tmux pipes from the pane of a capture, stamped with its time, into a file.
  #[command(hide = true)]
  Stamp { file: PathBuf },
}

#[tokio::main]
#[expect(
  clippy::disallowed_methods,
  reason = "the program blocks on the runtime of its command, which is all that it runs"
)]
async fn main() -> ExitCode {
  let done = match Command::parse().does {
    Does::Capture { scenarios, out, bin, jobs } => captured(&scenarios, out, bin, jobs).await,
    Does::Replay { casts, out, every } => casts
      .iter()
      .try_for_each(|cast| replayed(cast, out.as_deref(), every.map(Duration::from_millis))),
    Does::Compare { scenario, out } => match scenario::named(&scenario) {
      Some(scenario) => {
        compare::compare(scenario, &out, rivals().as_deref()).map(|text| print!("{text}"))
      }
      None => Err(Error::Scenario(scenario)),
    },
    Does::Probe { frames } => frames.iter().try_for_each(|path| {
      for found in probe::probe(&frame::Frame::read(path)?) {
        println!("{} row {} col {}: {}", path.display(), found.row, found.col, found.says);
      }
      Ok(())
    }),
    Does::Stamp { file } => capture::stamp(&file),
  };
  match done {
    Ok(()) => ExitCode::SUCCESS,
    Err(fault) => {
      eprintln!("furb-frames: {fault}");
      ExitCode::FAILURE
    }
  }
}

/// Each walk of the scenarios that the words name at each size, a few at once. Each says a line
/// when it ends.
async fn captured(
  words: &[String],
  out: PathBuf,
  bin: Option<PathBuf>,
  jobs: usize,
) -> Result<(), Error> {
  let bin = bin
    .map_or_else(|| env::current_exe().map(|me| me.with_file_name("")), Ok)
    .map_err(|source| Error::Process { what: "this program", source })?;
  for program in ["furb-window", "furb"] {
    if !bin.join(program).is_file() {
      return Err(Error::Missing { folder: bin, program });
    }
  }
  let scenarios: Vec<&'static Scenario> = if words.is_empty() {
    SCENARIOS.iter().filter(|scenario| scenario.walk().is_some()).collect()
  } else {
    words
      .iter()
      .map(|word| scenario::named(word).ok_or_else(|| Error::Scenario(word.clone())))
      .collect::<Result<_, _>>()?
  };
  let setup = Arc::new(Setup { bin, out });
  let walks = scenarios.iter().filter_map(|scenario| Some((scenario.name, scenario.walk()?)));
  let mut jobs_left = walks.flat_map(|(name, walk)| scenario::SIZES.map(|size| (name, walk, size)));
  let mut runs = JoinSet::new();
  let started = Instant::now();
  let mut failed = 0;
  loop {
    while runs.len() < jobs.max(1) {
      let Some((name, walk, size)) = jobs_left.next() else { break };
      let setup = Arc::clone(&setup);
      // The path of a session hues its thread, so a job takes the same root at each run, and two
      // captures of one scenario at one size do not run at once.
      let number = name.split('-').next().unwrap_or(name);
      let root = PathBuf::from(format!("/tmp/ff{number}-{}", size.name()));
      runs.spawn(async move {
        let done = walked(&Job { name, walk, size, root }, &setup).await;
        match &done {
          Ok(line) => println!("{line}"),
          Err(fault) => println!("{name} {}: {fault}", size.name()),
        }
        done.is_ok()
      });
    }
    match runs.join_next().await {
      Some(Ok(true)) => {}
      Some(Ok(false) | Err(_)) => failed += 1,
      None => break,
    }
  }
  let out = &setup.out;
  for scenario in &scenarios {
    if let Some(line) = probed(scenario, out)? {
      println!("{line}");
    }
    let compared = compare::compare(scenario, out, rivals().as_deref())?;
    put(&out.join(format!("{}.compare.txt", scenario.name)), compared.as_bytes())?;
    if scenarios.len() == 1 {
      print!("{compared}");
    }
  }
  println!("all in {:.1} s", started.elapsed().as_secs_f64());
  if failed == 0 { Ok(()) } else { Err(Error::Failed(failed)) }
}

/// What each walk stands on: the folder of the programs of the build and the folder of the frames.
struct Setup {
  bin: PathBuf,
  out: PathBuf,
}

/// One walk of a scenario at one size, in a folder of its own.
struct Job {
  name: &'static str,
  walk: &'static Walk,
  size: Size,
  root: PathBuf,
}

/// One walk at one size, written in `<out>/<size>/`: its recording, its key frames, and the text
/// that tmux showed at a marker where the replay shows another one, as `<step>.tmux.txt`.
async fn walked(job: &Job, setup: &Setup) -> Result<String, Error> {
  let Job { name, walk, size, root } = job;
  let started = Instant::now();
  let captured = capture::capture(walk, *size, &setup.bin, root).await?;
  let took = started.elapsed();
  let folder = setup.out.join(size.name());
  fs::create_dir_all(&folder).map_err(|source| Error::Write { path: folder.clone(), source })?;
  put(&folder.join(format!("{name}.cast")), captured.cast.write().as_bytes())?;
  let keys = replay::keys(&captured.cast, Some(MOTION))?;
  let missing = replay::write(&keys, &folder, name)?;
  let markers = keys.iter().filter(|key| key.marker);
  let mut differ = Vec::new();
  for (key, (step, shown)) in markers.zip(&captured.shown) {
    let soon = key.at + Duration::from_millis(500);
    if key.frame.text() != *shown && !replay::shows(&captured.cast, key.at, soon, shown)? {
      put(&folder.join(format!("{name}.{step}.tmux.txt")), shown.as_bytes())?;
      differ.push(step.as_str());
    }
  }
  let lacks = if missing.is_empty() {
    String::new()
  } else {
    format!(", no glyph for {}", String::from_iter(missing))
  };
  let proof = if differ.is_empty() {
    "each marker as tmux shows it, or a frame of it within 0.5 s".to_owned()
  } else {
    format!("the replay differs from tmux at {}", differ.join(" "))
  };
  let steps = captured.shown.len();
  Ok(format!(
    "{name} {}: {steps} steps in {:.1} s, {proof}{lacks}",
    size.name(),
    took.as_secs_f64()
  ))
}

/// The folder of the frames and of the judgment of the other programs, which stays out of the
/// repository.
fn rivals() -> Option<PathBuf> {
  env::var_os("FURB_RIVALS").map(PathBuf::from)
}

/// The findings of the probes on each marker frame of a walk at each size, and on each step across
/// the sizes, written in `<out>/<name>.probes.txt`. None when no size of the walk was captured.
fn probed(scenario: &Scenario, out: &Path) -> Result<Option<String>, Error> {
  let Some(walk) = scenario.walk() else { return Ok(None) };
  let mut lines = Vec::new();
  let mut count = 0;
  for step in walk.steps {
    let mut frames = Vec::new();
    for size in scenario::SIZES {
      let path = out.join(size.name()).join(format!("{}.{}.json", scenario.name, step.name));
      if !path.is_file() {
        continue;
      }
      let frame = frame::Frame::read(&path)?;
      for found in probe::probe(&frame) {
        lines.push(format!(
          "{} {} row {} col {}: {}",
          size.name(),
          step.name,
          found.row,
          found.col,
          found.says
        ));
        count += 1;
      }
      frames.push((size.name(), frame));
    }
    if frames.is_empty() {
      return Ok(None);
    }
    for found in probe::sizes(&frames) {
      lines.push(format!("sizes {}: {found}", step.name));
      count += 1;
    }
  }
  let path = out.join(format!("{}.probes.txt", scenario.name));
  let text = if lines.is_empty() { String::new() } else { lines.join("\n") + "\n" };
  put(&path, text.as_bytes())?;
  Ok(Some(format!("{}: {count} findings in {}", scenario.name, path.display())))
}

/// The key frames of one recording, written in a folder, with a line that says what it wrote.
fn replayed(path: &Path, out: Option<&Path>, every: Option<Duration>) -> Result<(), Error> {
  let text =
    fs::read_to_string(path).map_err(|source| Error::Read { path: path.to_owned(), source })?;
  let cast = Cast::read(&text)?;
  let keys = replay::keys(&cast, every)?;
  let folder = out.map_or_else(|| path.with_file_name("replay"), Path::to_owned);
  let stem = path.file_stem().map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
  let missing = replay::write(&keys, &folder, &stem)?;
  let markers = keys.iter().filter(|key| key.marker).count();
  let lacks = if missing.is_empty() {
    String::new()
  } else {
    format!(", no glyph for {}", String::from_iter(missing))
  };
  println!(
    "{}: {markers} markers, {} steps of time in {}{lacks}",
    path.display(),
    keys.len() - markers,
    folder.display()
  );
  Ok(())
}
