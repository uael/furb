//! The audit of a record: a replay with no model and no network that checks each word and each
//! answer of the record, as `docs/audit.md` says.

use std::{env, fs, path::Path, process};

use furb::{Engine, Object, ObjectRef, engine::OPERATOR, life::Opening, value::entry, wire, world};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::error::CliError;

/// The audit of a record that its maker says hashes as `sha256`: its report, printed and kept
/// where `report` names, and a refusal when one thing of it does not agree.
pub(crate) fn audited(record: &Path, sha256: &str, report: Option<&Path>) -> Result<(), CliError> {
  let held =
    fs::read(record).map_err(|source| CliError::Kept { path: record.to_owned(), source })?;
  let hashed = hex(&held);
  let mut wrong = Vec::new();
  let mut told = json!({"record": record.display().to_string(), "sha256": hashed});
  if hashed == sha256.trim().to_lowercase() {
    // The life reads a copy, so the audit holds no lease of the record and leaves nothing beside
    // it.
    let folder = env::temp_dir().join(format!("furb-audit-{}", process::id()));
    fs::create_dir_all(&folder)
      .map_err(|source| CliError::Kept { path: folder.clone(), source })?;
    let copy = folder.join("record.jsonl");
    fs::write(&copy, &held).map_err(|source| CliError::Kept { path: copy.clone(), source })?;
    let checked = checked(&folder, &copy, &mut told, &mut wrong);
    if let Err(no) = fs::remove_dir_all(&folder) {
      eprintln!("furb: {} stays: {no}", folder.display());
    }
    checked?;
  } else {
    wrong.push(format!("the record hashes as {hashed}, not as {sha256}"));
  }
  let count = wrong.len();
  if let Some(fields) = told.as_object_mut() {
    fields.insert("agrees".to_owned(), json!(wrong.is_empty()));
    fields.insert("wrong".to_owned(), json!(wrong));
  }
  let text = serde_json::to_string_pretty(&told).unwrap_or_default() + "\n";
  if let Some(report) = report {
    fs::write(report, &text)
      .map_err(|source| CliError::Kept { path: report.to_owned(), source })?;
  }
  print!("{text}");
  if count == 0 { Ok(()) } else { Err(CliError::Disagrees(count)) }
}

/// The place of each fact made whose engine differs from the one of the fact made before it,
/// from the place and the engine of each fact made of a record, in order.
fn crossings(made: impl Iterator<Item = (usize, String)>) -> Vec<usize> {
  let mut was = None;
  made
    .filter_map(|(at, engine)| {
      let crosses = was.as_ref().is_some_and(|was| *was != engine);
      was = Some(engine);
      crosses.then_some(at)
    })
    .collect()
}

/// What the record says and what a life that only inspects its copy makes again, told in the
/// report, with each thing that does not agree.
fn checked(
  folder: &Path,
  copy: &Path,
  told: &mut Value,
  wrong: &mut Vec<String>,
) -> Result<(), CliError> {
  let kept = world::kept(copy).map_err(CliError::Unread)?;
  let facts: Vec<ObjectRef<'_>> = kept.iter().filter_map(|one| entry(&one.as_ref(), 0)).collect();
  let word = |fact: &ObjectRef<'_>, at: usize| {
    entry(fact, at).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default()
  };
  told["engine"] = facts
    .iter()
    .find(|fact| word(fact, 0) == "made")
    .map_or(Value::Null, |fact| json!(word(fact, 3)));
  told["entries"] = json!(facts.len());
  told["acts"] =
    json!(facts.iter().filter(|fact| question(&word(fact, 0), &word(fact, 1))).count());
  // The life keeps nothing, enables no new extension and offers no model, and the World is silent
  // in it, so it runs no command, touches no file and reaches no network. It opens by the record,
  // so each engine of the record replays the segment that it made, and makes each act of it again.
  let opening = Opening {
    directory: folder.display().to_string(),
    record: Some(copy.display().to_string()),
    keeps: Some(false),
    inspecting: Some(true),
    skills: Some(false),
    roster: Some(Vec::<String>::new().into()),
    ..Opening::default()
  };
  let mut engine = match opening.boot::<String>([]) {
    Ok((engine, _)) => engine,
    Err(no) => {
      wrong.push(format!("the replay stops: {no}"));
      return Ok(());
    }
  };
  // Each segment of the record before the last one was proven as the open replayed it on its own
  // engine and carried it across, and the life at the tip holds the acts of the last segment.
  let made = facts.iter().enumerate().filter(|(_, fact)| word(fact, 0) == "made");
  let crossings = crossings(made.map(|(at, fact)| (at, word(fact, 3))));
  told["crossings"] = json!(crossings.len());
  let tip = crossings.last().copied().unwrap_or(0);
  let (mut answers, mut words) = (Vec::new(), Vec::new());
  for fact in facts.iter().skip(tip) {
    let (kind, about, maker) = (word(fact, 0), word(fact, 1), word(fact, 2));
    if kind == "done" && question("reply", &about) {
      let said = entry(fact, 3).map_or_else(String::new, |one| hashed(&wire::outward(one)));
      let again = hashed_of(&mut engine, "peek", &about, 2)?;
      if said != again {
        wrong.push(format!("the answer of {about} does not hash as the record says"));
      }
      answers.push(json!({"reply": about, "record": said, "replay": again}));
    } else if question(&kind, &about) && (maker == OPERATOR || made_by_act(&maker)) {
      let said = hashed(&wire::outward(*fact));
      let again = hashed_of(&mut engine, "get", &about, 1)?;
      if said != again {
        wrong.push(format!("the life did not make {about} again as the record holds it"));
      }
      if kind == "run" {
        let rung = word(fact, 4);
        let ran = engine.verb("get", vec![Object::string(&about)], vec![])?;
        let ran =
          entry(&ran.as_ref(), 5).and_then(|one| one.as_str().map(|one| hex(one.as_bytes())));
        words.push(json!({"rung": rung, "record": hex(word(fact, 5).as_bytes()), "run": ran}));
      }
    }
  }
  told["answers"] = json!(answers);
  told["words"] = json!(words);
  Ok(())
}

/// The hash of what a view of the engine gives for an act, its fact or its value, in the form
/// that the record keeps it in at its place: a fact in its entry, at depth 1, and a value in its
/// fact, at depth 2. The record marks a tuple and a whole float only below the entry and its fact,
/// so both sides hash one form, and a tuple that the life makes again as a list does not agree.
fn hashed_of(
  engine: &mut Engine,
  view: &str,
  about: &str,
  depth: usize,
) -> Result<String, CliError> {
  let made = engine.verb(view, vec![Object::string(about)], vec![])?;
  let line = (0..depth).fold(made, |held, _| Object::list(vec![held]));
  let kept = (0..depth).fold(wire::record(line.as_ref()), |mut held, _| {
    held.get_mut(0).map(Value::take).unwrap_or_default()
  });
  Ok(hashed(&kept))
}

/// The hash of a value as JSON, which a line of the record holds.
fn hashed(value: &Value) -> String {
  hex(value.to_string().as_bytes())
}

fn hex(bytes: &[u8]) -> String {
  let digits: Vec<String> =
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect();
  digits.concat()
}

/// Whether a name is the name of an act of its kind: the kind, then its number.
fn question(kind: &str, about: &str) -> bool {
  about
    .strip_prefix(kind)
    .is_some_and(|number| !number.is_empty() && number.bytes().all(|one| one.is_ascii_digit()))
}

/// Whether a maker is an act of the life, as a rung or a chain: a word that ends in its number. An
/// act that an ear of the outside made is of the World, which a life that inspects holds silent.
fn made_by_act(maker: &str) -> bool {
  let word = maker.trim_end_matches(|one: char| one.is_ascii_digit());
  word.len() < maker.len()
    && !word.is_empty()
    && word.bytes().all(|one| one.is_ascii_alphanumeric() || one == b'_')
}
