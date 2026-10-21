//! The places that the app opens a life on: the projects that the TUI and the app know, and the records of each, which
//! the TUI keeps in the folder `.furb/sessions` of the project.

use std::{
  fs,
  io::{BufRead, BufReader},
  path::{Path, PathBuf},
  time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{Value, json};

/// The file of the config directory where the app keeps the projects that it opened, the newest first.
const RECENT: &str = "gui.json";

/// How many lines of a record the app reads to find the first prompt of the operator.
const FIRST: usize = 400;

/// One record of a project: its path, the name that the TUI gave its session, the first prompt of the operator, when
/// it changed last, in milliseconds since the epoch, and its size in bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct Record {
  pub path: PathBuf,
  pub name: String,
  pub title: String,
  pub changed: u64,
  pub size: u64,
}

/// The records of a project, the newest first: each `.jsonl` of its `.furb/sessions`, less the changes that the TUI
/// keeps beside a record.
pub fn records(project: &Path) -> Vec<Record> {
  let folder = project.join(".furb").join("sessions");
  let Ok(entries) = fs::read_dir(&folder) else { return Vec::new() };
  let mut found: Vec<Record> = entries
    .filter_map(Result::ok)
    .map(|entry| entry.path())
    .filter(|path| {
      let name = path.file_name().and_then(|one| one.to_str()).unwrap_or_default();
      name.ends_with(".jsonl") && !name.ends_with(".changes.jsonl")
    })
    .filter_map(|path| {
      let meta = fs::metadata(&path).ok()?;
      let changed = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_millis() as u64;
      let stem = path.file_stem().and_then(|one| one.to_str()).unwrap_or_default().to_owned();
      let name = session_name(&path).unwrap_or(stem);
      let title = first_prompt(&path).unwrap_or_default();
      Some(Record { path, name, title, changed, size: meta.len() })
    })
    .collect();
  found.sort_by_key(|one| std::cmp::Reverse(one.changed));
  found
}

/// The name that the TUI gave the session of a record, which `<record>.ui.json` keeps.
fn session_name(record: &Path) -> Option<String> {
  let text = fs::read_to_string(format!("{}.ui.json", record.display())).ok()?;
  let saved: Value = serde_json::from_str(&text).ok()?;
  saved.get("sessionName")?.as_str().map(str::to_owned)
}

/// The message of the first prompt that the operator put in a record, read from its first lines.
pub fn first_prompt(record: &Path) -> Option<String> {
  let file = fs::File::open(record).ok()?;
  for line in BufReader::new(file).lines().take(FIRST) {
    let Ok(entry) = serde_json::from_str::<Value>(&line.ok()?) else { continue };
    let fact = &entry[0];
    let word = |at: usize| fact.get(at).and_then(Value::as_str).unwrap_or_default();
    if word(0) == "prompt" && word(2) == "operator" && !word(5).is_empty() {
      return Some(word(5).to_owned());
    }
  }
  None
}

/// The project of a record: the directory that its session names in `<record>.session.json`, else the project whose
/// `.furb/sessions` holds it, else the directory of the record.
pub fn project_of(record: &Path) -> PathBuf {
  let saved = fs::read_to_string(format!("{}.session.json", record.display()))
    .ok()
    .and_then(|text| serde_json::from_str::<Value>(&text).ok())
    .and_then(|saved| saved["options"]["cwd"].as_str().map(PathBuf::from));
  if let Some(directory) = saved {
    return directory;
  }
  let folder = record.parent().unwrap_or(Path::new("."));
  let sessions = folder.file_name().is_some_and(|one| one == "sessions")
    && folder.parent().and_then(Path::file_name).is_some_and(|one| one == ".furb");
  match folder.parent().and_then(Path::parent) {
    Some(project) if sessions => project.to_path_buf(),
    _ => folder.to_path_buf(),
  }
}

/// The path of a new record in a project, as the TUI names one: the time in ISO 8601 with no colon, and eight
/// characters of chance. It makes `.furb` with its rule for git, as the TUI does, and `.furb/sessions`, and writes the
/// directory of the session beside the record, so the TUI opens it on the same project.
pub fn fresh(project: &Path) -> Result<PathBuf, String> {
  let furb = project.join(".furb");
  if !furb.exists() {
    fs::create_dir_all(&furb).map_err(|no| no.to_string())?;
    fs::write(furb.join(".gitignore"), "*\n!config.json\n!extensions/\n!extensions/**\n")
      .map_err(|no| no.to_string())?;
  }
  let folder = furb.join("sessions");
  fs::create_dir_all(&folder).map_err(|no| no.to_string())?;
  let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
  let chance = (now.as_nanos() as u64 ^ (u64::from(std::process::id()) << 32))
    .wrapping_mul(0x9e37_79b9_7f4a_7c15);
  let record = folder.join(format!("{}-{:08x}.jsonl", stamp(now.as_millis() as u64), chance >> 32));
  let saved = json!({"options": {"cwd": project.display().to_string()}});
  fs::write(format!("{}.session.json", record.display()), saved.to_string())
    .map_err(|no| no.to_string())?;
  Ok(record)
}

/// A time in milliseconds since the epoch, in ISO 8601 with a dash in place of each colon, as 2026-10-21T09-34-26.954Z.
pub fn stamp(millis: u64) -> String {
  let (days, rest) = ((millis / 86_400_000) as i64, millis % 86_400_000);
  let shifted = days + 719_468;
  let era = shifted.div_euclid(146_097);
  let day = shifted.rem_euclid(146_097);
  let year = (day - day / 1460 + day / 36_524 - day / 146_096) / 365;
  let of_year = day - (365 * year + year / 4 - year / 100);
  let month = (5 * of_year + 2) / 153;
  let date = of_year - (153 * month + 2) / 5 + 1;
  let month = if month < 10 { month + 3 } else { month - 9 };
  let year = year + era * 400 + i64::from(month <= 2);
  let (hours, minutes, seconds) = (rest / 3_600_000, rest / 60_000 % 60, rest / 1000 % 60);
  format!("{year:04}-{month:02}-{date:02}T{hours:02}-{minutes:02}-{seconds:02}.{:03}Z", rest % 1000)
}

/// The projects that the TUI lists in its `workspaces.json`, then those that the app opened, each once.
pub fn projects(config: &Path) -> Vec<PathBuf> {
  let read = |name: &str| {
    fs::read_to_string(config.join(name))
      .ok()
      .and_then(|text| serde_json::from_str::<Value>(&text).ok())
      .unwrap_or(Value::Null)
  };
  let mut found: Vec<PathBuf> = Vec::new();
  let recent = read(RECENT);
  let opened = recent["projects"].as_array().into_iter().flatten().filter_map(Value::as_str);
  let workspaces = read("workspaces.json");
  let listed = workspaces["workspaces"]
    .as_array()
    .into_iter()
    .flatten()
    .filter_map(|one| one["directory"].as_str());
  for directory in opened.chain(listed).map(PathBuf::from) {
    if !found.contains(&directory) && directory.is_dir() {
      found.push(directory);
    }
  }
  found
}

/// A project that the app opened, kept first in the list of the app.
pub fn remember(config: &Path, project: &Path) -> Result<(), String> {
  let path = config.join(RECENT);
  let held =
    fs::read_to_string(&path).ok().and_then(|text| serde_json::from_str::<Value>(&text).ok());
  let mut list: Vec<String> = held
    .as_ref()
    .and_then(|one| one["projects"].as_array())
    .into_iter()
    .flatten()
    .filter_map(Value::as_str)
    .map(str::to_owned)
    .collect();
  let named = project.display().to_string();
  list.retain(|one| *one != named);
  list.insert(0, named);
  list.truncate(24);
  fs::create_dir_all(config).map_err(|no| no.to_string())?;
  fs::write(&path, json!({"projects": list}).to_string() + "\n").map_err(|no| no.to_string())
}

/// The places as the picker of the app shows them: each project with its name and its records.
pub fn listed(config: &Path) -> Value {
  let projects: Vec<Value> = projects(config)
    .into_iter()
    .map(|directory| {
      let records: Vec<Value> = records(&directory)
        .into_iter()
        .map(|one| {
          json!({"path": one.path, "name": one.name, "title": one.title, "changed": one.changed, "size": one.size})
        })
        .collect();
      let name = directory.file_name().and_then(|one| one.to_str()).unwrap_or_default().to_owned();
      json!({"directory": directory, "name": name, "records": records})
    })
    .collect();
  json!({"projects": projects})
}

#[cfg(test)]
#[path = "places.test.rs"]
mod test;
