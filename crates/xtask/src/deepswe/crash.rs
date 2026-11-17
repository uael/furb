//! The kills of a run of the rig, as crashes: the rig kills the life at a random point after the
//! first answer of each life, and opens it again on its record, which takes the work up. The
//! measures read the record and the lives that the rig started, which `.run/lives.json` keeps.

use std::{
  collections::{BTreeMap, BTreeSet},
  path::Path,
  time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Fault, Numbers, entries, read, written};

/// The latest point of a kill after the first answer of a life, which the rig draws from zero.
const LATE: Duration = Duration::from_mins(5);

/// The kills that are left, the draw of each point, and each life that the rig started.
pub(crate) struct Crash {
  left: u32,
  draw: fastrand::Rng,
  lives: Vec<Lived>,
  /// The answers that the record held when the life that runs started.
  held: u64,
  /// The point of the next kill, which the first answer of the life that runs sets.
  due: Option<Instant>,
}

/// One life that the rig started: when, in seconds since the epoch, how many entries its record
/// held then, and whether the rig killed it.
#[derive(Serialize, Deserialize)]
struct Lived {
  spawned: f64,
  entries: usize,
  killed: bool,
}

impl Crash {
  pub(crate) fn new(kills: u32, seed: Option<u64>) -> Self {
    let draw = seed.map_or_else(fastrand::Rng::new, fastrand::Rng::with_seed);
    Self { left: kills, draw, lives: Vec::new(), held: 0, due: None }
  }

  /// A life that starts on the record.
  pub(crate) fn spawned(&mut self, record: &Path) -> Result<(), Fault> {
    let held = entries(record)?;
    // A clock before the epoch is no clock of this machine, and its life reads as spawned at 0.
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    self.held = Numbers::of_entries(&held).answers;
    self.lives.push(Lived { spawned: now.as_secs_f64(), entries: held.len(), killed: false });
    self.due = None;
    Ok(())
  }

  /// Whether the point of a kill came, from the answers that the record holds now.
  pub(crate) fn due(&mut self, answers: u64) -> bool {
    match self.due {
      _ if self.left == 0 => false,
      None if answers > self.held => {
        self.due = Some(Instant::now() + LATE.mul_f64(self.draw.f64()));
        false
      }
      Some(at) if Instant::now() >= at => {
        self.left -= 1;
        if let Some(life) = self.lives.last_mut() {
          life.killed = true;
        }
        true
      }
      _ => false,
    }
  }

  /// The lives, kept in `.run/lives.json`, and what the kills came to.
  pub(crate) fn kept(&self, runs: &Path) -> Result<Crashed, Fault> {
    let lives = serde_json::to_vec_pretty(&self.lives)
      .map_err(|source| Fault::Json { what: "the lives of a run".to_owned(), source })?;
    written(&runs.join("lives.json"), &lives)?;
    measured(runs)
  }
}

/// What the kills of a run came to.
#[derive(Debug, PartialEq, Serialize)]
pub(crate) struct Crashed {
  kills: usize,
  /// The seconds from the start of each life after a kill to the first entry that it kept.
  resume_seconds: Vec<f64>,
  commands: usize,
  /// The line of each command that a kill cut, as its end says.
  interrupted: Vec<String>,
  /// Each later command with the line of one that a kill cut.
  ran_again: Vec<Again>,
  /// Each command that two lives started, less one that a kill cut, which the later life starts
  /// only to close it.
  ran_in_two_lives: Vec<String>,
  /// Each reply that a life after a kill found open, and so asked again.
  replies_asked_again: Vec<String>,
}

/// The commands with the line of one that a kill cut.
#[derive(Debug, PartialEq, Serialize)]
pub(crate) struct Again {
  command: String,
  interrupted: String,
  later: Vec<String>,
}

/// What the kills of a run came to, from its record and its lives.
pub(crate) fn measured(runs: &Path) -> Result<Crashed, Fault> {
  let path = runs.join("lives.json");
  let lives: Vec<Lived> = serde_json::from_slice(&read(&path)?)
    .map_err(|source| Fault::Json { what: path.display().to_string(), source })?;
  let held = entries(&runs.join("record.jsonl"))?;
  let facts: Vec<&Value> = held.iter().map(|entry| &entry[0]).collect();
  let commands: Vec<(&str, &str)> = facts
    .iter()
    .filter(|fact| fact[0] == "bash")
    .filter_map(|fact| Some((fact[1].as_str()?, fact[4].as_str()?)))
    .collect();
  let line = |id: &str| commands.iter().find(|one| one.0 == id).map(|one| one.1);
  let mut cut: Vec<&str> = Vec::new();
  let mut starts: BTreeMap<&str, usize> = BTreeMap::new();
  for fact in &facts {
    let Some(id) = fact[1].as_str().filter(|id| line(id).is_some()) else { continue };
    match fact[0].as_str() {
      Some("done" | "close") if interrupted(&fact[3]) && !cut.contains(&id) => cut.push(id),
      Some("started") => *starts.entry(id).or_default() += 1,
      _ => {}
    }
  }
  let ran_again = cut
    .iter()
    .filter_map(|id| {
      let (at, command) = (commands.iter().position(|one| one.0 == *id)?, line(id)?);
      let later = commands.iter().skip(at + 1).filter(|one| one.1 == command);
      let later: Vec<String> = later.map(|one| one.0.to_owned()).collect();
      (!later.is_empty()).then(|| Again {
        command: command.to_owned(),
        interrupted: (*id).to_owned(),
        later,
      })
    })
    .collect();
  let after =
    || lives.iter().zip(lives.iter().skip(1)).filter(|(was, _)| was.killed).map(|(_, now)| now);
  Ok(Crashed {
    kills: lives.iter().filter(|life| life.killed).count(),
    resume_seconds: after()
      .filter_map(|life| {
        let first = held.get(life.entries)?.get(1)?.as_f64()? / 1000.0;
        Some(((first - life.spawned) * 100.0).round() / 100.0)
      })
      .collect(),
    commands: commands.len(),
    interrupted: cut.iter().filter_map(|id| line(id)).map(str::to_owned).collect(),
    ran_again,
    ran_in_two_lives: starts
      .into_iter()
      .filter(|one| one.1 > 1 && !cut.contains(&one.0))
      .map(|one| one.0.to_owned())
      .collect(),
    replies_asked_again: after()
      .flat_map(|life| open(facts.get(..life.entries).unwrap_or(&facts)))
      .filter(|id| id.starts_with("reply"))
      .collect(),
  })
}

/// Whether the end of a command says that a kill cut it: a fault, as no exit of the command is.
fn interrupted(value: &Value) -> bool {
  value["is"].as_str().is_some_and(|is| is != "Exit")
    && value.to_string().contains("was interrupted")
}

/// Each act that the facts say started and do not say done.
fn open(facts: &[&Value]) -> BTreeSet<String> {
  let said = |kind: &str| -> BTreeSet<String> {
    facts
      .iter()
      .filter(|fact| fact[0] == kind)
      .filter_map(|fact| fact[1].as_str())
      .map(str::to_owned)
      .collect()
  };
  &said("started") - &said("done")
}

#[cfg(test)]
#[path = "crash.test.rs"]
mod test;
