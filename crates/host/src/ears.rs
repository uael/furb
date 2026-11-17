//! The ears that the host gives each life, before the ears of the crate: one list, so each life of
//! a machine opens the same way, and the record of one host opens in another.

use std::{
  collections::HashSet,
  env, io,
  process::{Command, Stdio},
  sync::{Arc, Mutex, PoisonError},
};

use furb::{
  Ear, Fact, Fault, Object,
  ear::{Co, ear, hear, hearing, say},
  view::{Fold, Name, Pause, followed},
  wire::outward,
  world::{Asked, CONSOLE},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
  changes::Journal,
  event::Event,
  serve::{PROGRAM, built},
};

/// The ears of the host, each under its name, in the order of boot: the console of the operator,
/// the ear that tells each fact as events, the ear of the upgrade, and the ear of the journal of
/// the changes of files of a life that keeps a record.
pub(crate) fn ears(
  told: UnboundedSender<Event>,
  journal: Option<&Journal>,
  record: Option<&str>,
) -> Vec<(&'static str, Box<dyn Ear>)> {
  let upgrade = upgrade(record.map(str::to_owned));
  let mut ears = vec![(CONSOLE, console()), ("events", events(told)), (UPGRADE, upgrade)];
  if let Some(journal) = journal {
    ears.push(("changes", journal.ear()));
  }
  ears
}

/// The console of the operator, which takes each prompt to the operator as every console of furb
/// takes it. A client sees the prompt in the events of its act, and answers it with a command.
fn console() -> Box<dyn Ear> {
  hearing(&["prompt"], |mut co| async move {
    loop {
      let a = hear(&mut co).await;
      Asked::taken(&mut co, &a).await?;
    }
  })
}

/// The act by which a word moves furb to its newest build.
const UPGRADE: &str = "upgrade";

/// The build of the program of this process that the ear of the upgrade started last.
static STARTED: Mutex<Option<String>> = Mutex::new(None);

/// The ear of the upgrade, which answers the act `upgrade` with what it did. When a build replaced
/// the program of this process since it started, it starts that program as `furb host upgrade`,
/// once for each build, which takes the place of the daemon of this machine, and each life opens
/// again on it. The new program names the record of this life, whose commands its handover does
/// not wait for. A program that a build writes now cannot start, and the act is refused.
fn upgrade(record: Option<String>) -> Box<dyn Ear> {
  hearing(&[UPGRADE], move |mut co| async move {
    loop {
      let a = hear(&mut co).await;
      if !a.question() || a.kind() != UPGRADE {
        continue;
      }
      let answer = match newest(record.as_deref()) {
        Ok(said) => Object::string(said),
        Err(no) => Fault::refused(format!("furb cannot start its newest build: {no}")).object(),
      };
      say(&mut co, Fact::says("done", a.about(), [answer]));
    }
  })
}

/// What the upgrade did: the start of the newest build, in a group of its own, so the end of the
/// daemon that it takes the place of does not end it, for the life of a record when one asked it.
/// The daemon and each act upgrade start the newest build here alone.
pub(crate) fn newest(from: Option<&str>) -> io::Result<&'static str> {
  let program = env::current_exe()?;
  let unread = || io::Error::other("its program cannot be read, as while a build writes it");
  let now = built(&program).ok_or_else(unread)?;
  if PROGRAM.as_ref() == Some(&now) {
    return Ok("furb runs its newest build.");
  }
  let mut started = STARTED.lock().unwrap_or_else(PoisonError::into_inner);
  if started.as_ref() == Some(&now) {
    return Ok("furb moves to its newest build already.");
  }
  let mut command = Command::new(&program);
  command.args(["host", "upgrade"]).stdin(Stdio::null());
  if let Some(record) = from {
    command.args(["--from", record]);
  }
  #[cfg(unix)]
  {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
  }
  #[cfg(windows)]
  {
    use std::os::windows::process::CommandExt;
    use windows_sys::Win32::System::Threading::{CREATE_NEW_PROCESS_GROUP, DETACHED_PROCESS};
    command.creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS);
  }
  // The new build outlives this process, which ends at the handover, and nothing waits for it.
  drop(command.spawn()?);
  *started = Some(now);
  Ok("The newest build of furb takes the place of the daemon, and each life opens again on it.")
}

/// What the ear of the events keeps of the facts that it heard, which the image of a life holds:
/// the fold of the acts, the last pause or wake of each act or chain, in order, and each name that
/// a fact told.
#[derive(Default, Serialize, Deserialize)]
struct Heard {
  fold: Fold,
  controls: Vec<Fact>,
  seen: HashSet<String>,
}

/// The ear that hears each fact, and tells it, each act that it changes with its meaning, and the
/// chain that it changes. An act that an ear of the engine took is said to no other ear, so its
/// first fact tells it first, as the log of a life shows it.
fn events(told: UnboundedSender<Event>) -> Box<dyn Ear> {
  ear(move |mut co| async move {
    co.keeps(Heard::default())?;
    loop {
      let a = hear(&mut co).await;
      // The ear holds what it keeps while it steps, and gives it back before it waits again,
      // where the image of the life reads it, between two runs.
      let mut heard: Heard = co.held()?;
      let goes = stepped(&mut co, &mut heard, a, &told).await;
      co.keep(heard);
      if !goes? {
        return Ok(());
      }
    }
  })
}

/// One fact told, after the act that it is about when no fact told that act yet, and whether the
/// life still hears the ear.
async fn stepped<W: 'static>(
  co: &mut Co<W>,
  heard: &mut Heard,
  a: Fact,
  told: &UnboundedSender<Event>,
) -> Result<bool, Fault> {
  let Heard { fold, controls, seen } = heard;
  let fresh = seen.insert(a.about().to_owned());
  // A wake puts an act that an earlier life took to the outside again: its question is no new fact.
  if a.question() && !fresh {
    return Ok(true);
  }
  let mut facts = Vec::new();
  if fresh && !a.question() && !a.about().is_empty() {
    let made = co.call("get", vec![Object::string(a.about())], vec![]).await?;
    facts.extend(Fact::owned(made));
  }
  facts.push(a);
  for one in &facts {
    let (covered, born) = covered(co, fold, controls, one).await?;
    let fact = json_of(one);
    let acts = fold.hear(&fact, &covered, born);
    let chain = fold.chain_of(one.about()).map(|chain| Event::Changed(chain.to_owned()));
    let events =
      [Event::Fact(fact)].into_iter().chain(acts.into_iter().map(|act| Event::Act(Arc::new(act))));
    for event in events.chain(chain) {
      if told.send(event).is_err() {
        return Ok(false);
      }
    }
  }
  Ok(true)
}

/// A fact as JSON reads it. The fact of a module tells no words, since a module is no plain value.
pub(crate) fn json_of(a: &Fact) -> Value {
  match a.kind() {
    "module" => json!(["module", a.about(), a.by()]),
    _ => outward(a.object().as_ref()),
  }
}

/// A fact as [`json_of`] gives it, read from a line of JSON of what a record tells, as the log of
/// a life writes it: one parse, and no value of the sandbox made. Nothing for a line that holds no
/// fact.
pub(crate) fn json_of_line(line: &str) -> Option<Value> {
  let mut fact: Value = serde_json::from_str(line).ok()?;
  let held = fact.as_array_mut()?;
  if held.len() < 2 || !held.iter().take(2).all(Value::is_string) {
    return None;
  }
  if held.first().and_then(Value::as_str) == Some("module") {
    held.resize(3, json!(""));
  }
  Some(fact)
}

/// What the engine alone says of a fact: each live act that a pause or a wake covers, and the pause
/// that holds a new act, which is the last control that covers it when that control is a pause.
async fn covered<W: 'static>(
  co: &mut Co<W>,
  fold: &Fold,
  controls: &mut Vec<Fact>,
  a: &Fact,
) -> Result<(Vec<Name>, Option<Pause>), Fault> {
  let mut found = Vec::new();
  if !a.question() && matches!(a.kind(), "pause" | "wake") {
    controls.retain(|one| one.about() != a.about());
    controls.push(a.clone());
    for id in fold.open(a.kind() == "pause") {
      if covers(co, a, &id).await? {
        found.push(id);
      }
    }
  } else if a.question() && followed(a.kind()) {
    let oldest = controls.iter().position(|one| one.kind() == "pause").unwrap_or(controls.len());
    for control in controls.iter().skip(oldest).rev() {
      if covers(co, control, a.about()).await? {
        return Ok((found, Pause::of(&json_of(control))));
      }
    }
  }
  Ok((found, None))
}

/// Whether a pause or a wake covers an act, as the engine reads it.
async fn covers<W: 'static>(co: &mut Co<W>, control: &Fact, id: &str) -> Result<bool, Fault> {
  let got = co.call("covers", vec![control.object().clone(), Object::string(id)], vec![]).await?;
  Ok(got.as_ref().as_bool().unwrap_or_default())
}
