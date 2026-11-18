//! A session as it stood at a moment, drawn by the window of this build: a copy of its record in a
//! folder of its own, a daemon of this process that opens it in a life that only inspects it, and
//! the window, headless at a size, on the thread and at the moment of the time of that thread, as
//! the operator holds the bar of the time. Its frame is written as text, cells and an image, as
//! each key frame of a capture.

use std::{
  env, fs,
  path::{Path, PathBuf},
  process,
  sync::OnceLock,
  time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use furb::life::Opening;
use furb_host::{
  STALL, STARTING, Settings,
  protocol::{Frame as Said, Hello, Kind, PROTOCOL, Request, Verb},
  run,
  transport::{Dialer, Local},
};
use furb_window::{Machine, Options, Reach, Seen, Stood, open};
use opentui::testing::TestApp;
use serde_json::Value;
use tokio::{
  io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
  process::Command,
  time::timeout,
};

use crate::{error::Error, frame::Frame, raster, replay::put, scenario::Size};

/// How long the frame of the window holds still before it is the frame of the moment.
const STILL: Duration = Duration::from_secs(5);

/// The millisecond that the clocks of the window read: the moment of the render.
static CLOCK: OnceLock<u64> = OnceLock::new();

/// What a render draws: a record at a moment, on a thread, at a size, or what the operator saw.
pub(crate) struct Asked {
  pub(crate) record: Option<PathBuf>,
  pub(crate) at: Option<String>,
  pub(crate) thread: Option<String>,
  pub(crate) size: Size,
  pub(crate) seen: Option<PathBuf>,
  pub(crate) out: PathBuf,
  pub(crate) wait: Duration,
}

/// The frame of the window at the moment, written in `out` as `frame.txt`, `frame.json` and
/// `frame.png`, and the path of its text.
pub(crate) fn render(asked: &Asked) -> Result<PathBuf, Error> {
  let (record, stood, size) = placed(asked)?;
  let yard = env::temp_dir().join(format!("furb-render-{}", process::id()));
  let copy = copied(&record, &yard.join("project/.furb/sessions"), stood.moment)?;
  let moment = stood.moment.unwrap_or(u64::MAX);
  CLOCK.get_or_init(|| millis_at(&record, moment).unwrap_or_else(now));
  // The copy ends at the moment, so the thread shows its tip, which is the moment.
  let thread = stood.thread.map(|key| (0, copy.display().to_string(), key.2));
  let stood = Stood { thread, moment: None, ..stood };
  let kept = yard.join("window/cards.json");
  let window = yard.join("window");
  fs::create_dir_all(&window).map_err(|source| Error::Write { path: window, source })?;
  put(&kept.with_file_name("reload.json"), &serde_json::to_vec(&stood)?)?;
  let app = TestApp::new(to_u16(size.cols), to_u16(size.rows));
  let machine = served(&app, &yard, &copy, asked.wait)?;
  let options = Options::here().map_err(|no| Error::Render(no.to_string()))?;
  let options = Options {
    folder: yard.join("project"),
    kept,
    clock: || CLOCK.get().copied().unwrap_or_default(),
    still: true,
    program: None,
    reloaded: true,
    tailnet: None,
    listen: Some(yard.join("no-speech")),
    ..options
  };
  open(&app, vec![machine], options);
  let frame = settled(&app, asked.wait);
  fs::remove_dir_all(&yard).ok();
  written(&frame, &asked.out)
}

/// A copy of a record in a folder of sessions, the path of the copy: up to the fact of a moment,
/// as a life that ended there left it, so the life opens at that moment and says nothing of what
/// came after; else whole with its image, which lets the life open at its tip with no replay
/// when its key agrees.
fn copied(record: &Path, sessions: &Path, moment: Option<u64>) -> Result<PathBuf, Error> {
  let failed = |path: &Path| {
    let path = path.to_path_buf();
    move |source| Error::Write { path, source }
  };
  fs::create_dir_all(sessions).map_err(failed(sessions))?;
  let name = record.file_name().map_or_else(|| "record.jsonl".into(), PathBuf::from);
  let copy = sessions.join(&name);
  if let Some(moment) = moment {
    let text = fs::read_to_string(record)
      .map_err(|source| Error::Read { path: record.to_path_buf(), source })?;
    let place = |line: &str| match serde_json::from_str::<Value>(line).ok()? {
      Value::Array(parts) => parts.get(1)?.as_u64(),
      _ => None,
    };
    let kept: String = text
      .split_inclusive('\n')
      .take_while(|line| place(line).is_none_or(|at| at <= moment))
      .collect();
    fs::write(&copy, kept).map_err(failed(&copy))?;
    return Ok(copy);
  }
  fs::copy(record, &copy).map_err(failed(&copy))?;
  let image = PathBuf::from(format!("{}.image", record.display()));
  if image.is_file() {
    let to = PathBuf::from(format!("{}.image", copy.display()));
    fs::copy(&image, &to).map_err(failed(&to))?;
  }
  Ok(copy)
}

/// The machine of the window: a daemon of this process on the runtime of the app, in the yard,
/// whose lives only inspect their records, keep nothing and ask no model.
#[expect(clippy::disallowed_methods, reason = "a render blocks on the runtime of its window")]
fn served(app: &TestApp, yard: &Path, record: &Path, wait: Duration) -> Result<Machine, Error> {
  let host = yard.join("host");
  let local = Local::new(&host);
  let runtime = app.app().runtime();
  let listener = runtime.block_on(local.listen()).map_err(|no| Error::Render(no.to_string()))?;
  let opening = Opening {
    directory: yard.join("project").display().to_string(),
    inspecting: Some(true),
    keeps: Some(false),
    skills: Some(false),
    config: Some(yard.join("config").display().to_string()),
    claude: Some(yard.join("no-claude").display().to_string()),
    ..Opening::default()
  };
  let settings = Settings {
    opening,
    grace: Duration::from_secs(3600),
    first: STARTING,
    stall: STALL,
    tailnet: None,
    door: None,
    data: Some(yard.join("data")),
  };
  let daemon = local.clone();
  drop(runtime.spawn(async move { run(&daemon, listener, settings).await }));
  let cwd = yard.join("project").display().to_string();
  runtime.block_on(timeout(wait, opened(&local, record.display().to_string(), cwd))).map_err(
    |_elapsed| {
      Error::Render(format!("the life of the record did not open within {} s", wait.as_secs()))
    },
  )??;
  let served = Box::new(Command::new(yard.join("no-furb")));
  Ok(Machine {
    name: furb_host::machine(),
    reach: Reach::Here { folder: host, served },
    reads: false,
  })
}

/// The life of a record opened on the daemon, as a window opens it, once its open ended, so the
/// window that attaches to it draws its pages at once. A record of a long life replays for a
/// while, and the daemon keeps the life for its grace.
async fn opened(local: &Local, life: String, cwd: String) -> Result<(), Error> {
  let failed = |why: String| Error::Render(format!("the daemon did not open the life: {why}"));
  let stream = local.dial().await.map_err(|why| failed(why.to_string()))?;
  let mut lines = BufReader::new(stream);
  let hello = Hello { protocol: PROTOCOL, kind: Kind::Window, name: None, role: None };
  let (actor, roster, skills, config, claude) = (None, None, None, None, None);
  let cwd = Some(cwd);
  let open = Verb::Open {
    life: life.clone(),
    cwd,
    actor,
    roster,
    skills,
    config,
    claude,
    answers: false,
    attach: None,
  };
  // The open answers at once, and a verb of the life waits until the life opened.
  let standing = Verb::Standing { life };
  for (id, verb) in [(0, Verb::Hello(hello)), (1, open), (2, standing)] {
    let request = serde_json::to_string(&Request { id, verb })? + "\n";
    lines.get_mut().write_all(request.as_bytes()).await.map_err(|why| failed(why.to_string()))?;
    loop {
      let mut heard = String::new();
      if lines.read_line(&mut heard).await.map_err(|why| failed(why.to_string()))? == 0 {
        return Err(failed("it closed the connection".to_owned()));
      }
      match serde_json::from_str(&heard)? {
        Said::Reply { id: got, .. } if got == id => break,
        Said::Refused { id: Some(got), why, .. } if got == id => return Err(failed(why)),
        _ => {}
      }
    }
  }
  Ok(())
}

/// The record, where the window stood, and its size: those that a file of what the operator saw
/// names, else the record, the moment and the thread that the words name.
fn placed(asked: &Asked) -> Result<(PathBuf, Stood, Size), Error> {
  if let Some(file) = &asked.seen {
    let text =
      fs::read_to_string(file).map_err(|source| Error::Read { path: file.clone(), source })?;
    let seen: Seen = serde_json::from_str(&text)?;
    let record = seen.stood.thread.as_ref().map(|key| PathBuf::from(&key.1));
    let record = record.ok_or_else(|| Error::Render("the operator saw no thread".to_owned()))?;
    let size = Size { cols: usize::from(seen.cols), rows: usize::from(seen.rows) };
    return Ok((record, seen.stood, size));
  }
  let record =
    asked.record.clone().ok_or_else(|| Error::Render("name a record or --seen".to_owned()))?;
  let (moment, thread) = match &asked.at {
    Some(at) => moment(&record, at)?,
    None => (None, None),
  };
  let thread = asked.thread.clone().or(thread);
  let thread = thread.map(|one| (0, record.display().to_string(), one));
  Ok((record, Stood { thread, moment, ..Stood::default() }, asked.size))
}

/// Each fact of a record with its place among the facts of the life and its millisecond, as each
/// line of a record holds them.
fn facts(record: &Path) -> Result<Vec<(Value, u64, u64)>, Error> {
  let text = fs::read_to_string(record)
    .map_err(|source| Error::Read { path: record.to_path_buf(), source })?;
  let line = |one: &str| {
    let Value::Array(parts) = serde_json::from_str::<Value>(one).ok()? else { return None };
    let fact = parts.first()?.clone();
    Some((fact, parts.get(1)?.as_u64()?, parts.last()?.as_u64()?))
  };
  Ok(text.lines().filter_map(line).collect())
}

/// The moment that the words name, as the count of the facts that the life said: an act of the
/// record, as `prompt660`, at the fact that made it, a millisecond of the epoch, at the last fact
/// before it, or the count itself. An act that is a prompt on another prompt names that thread,
/// and a prompt of its own names itself.
fn moment(record: &Path, at: &str) -> Result<(Option<u64>, Option<String>), Error> {
  let no =
    || Error::Render(format!("{at} is no act of the record, no millisecond and no count of facts"));
  if let Ok(number) = at.parse::<u64>() {
    if number < 100_000_000_000 {
      return Ok((Some(number), None));
    }
    let before = facts(record)?.into_iter().take_while(|one| one.2 <= number).last();
    return before.map(|one| (Some(one.1), None)).ok_or_else(no);
  }
  let made =
    facts(record)?.into_iter().find(|one| one.0.get(1).and_then(Value::as_str) == Some(at));
  let (fact, place, _) = made.ok_or_else(no)?;
  let word = |at: usize| fact.get(at).and_then(Value::as_str).map(str::to_owned);
  let thread = match (word(0).as_deref(), word(3)) {
    (Some("prompt"), Some(on)) if on.starts_with("prompt") => Some(on),
    (Some("prompt"), _) => Some(at.to_owned()),
    _ => None,
  };
  Ok((Some(place), thread))
}

/// The millisecond at which the life said the fact of a place, or its last fact before it.
fn millis_at(record: &Path, place: u64) -> Option<u64> {
  facts(record).ok()?.into_iter().take_while(|one| one.1 <= place).last().map(|one| one.2)
}

fn now() -> u64 {
  let since = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
  u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
}

fn to_u16(count: usize) -> u16 {
  u16::try_from(count).unwrap_or(u16::MAX)
}

/// The frame of the window once it held still for [`STILL`], or the last one at the bound. It says
/// on stderr each ten seconds that it waits, so a slow open shows.
fn settled(app: &TestApp, bound: Duration) -> Frame {
  let start = Instant::now();
  let (mut last, mut since, mut told) = (String::new(), Instant::now(), Instant::now());
  loop {
    app.wait_until(Duration::from_millis(250), || false);
    let text = app.text();
    if text != last {
      (last, since) = (text, Instant::now());
    }
    if since.elapsed() >= STILL || start.elapsed() >= bound {
      return Frame::of(&app.render());
    }
    if told.elapsed() >= Duration::from_secs(10) {
      eprintln!("furb-frames: the window draws the session, {} s", start.elapsed().as_secs());
      told = Instant::now();
    }
  }
}

/// A frame written in a folder as its text, its cells and its image, and the path of its text.
fn written(frame: &Frame, out: &Path) -> Result<PathBuf, Error> {
  fs::create_dir_all(out).map_err(|source| Error::Write { path: out.to_path_buf(), source })?;
  let text = out.join("frame.txt");
  put(&text, frame.text().as_bytes())?;
  put(&out.join("frame.json"), &serde_json::to_vec(frame)?)?;
  put(&out.join("frame.png"), &raster::png(frame)?.0)?;
  Ok(text)
}
