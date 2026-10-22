//! The places that the app reads records in: the projects that the TUI knows, and the records of each, which the TUI
//! keeps in the folder `.furb/sessions` of the project.

use std::{
  fs,
  io::{BufRead, BufReader},
  path::{Path, PathBuf},
  time::UNIX_EPOCH,
};

use serde_json::{Value, json};

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

/// The projects that the TUI lists in its `workspaces.json`, each once.
pub fn projects(config: &Path) -> Vec<PathBuf> {
  let read = |name: &str| {
    fs::read_to_string(config.join(name))
      .ok()
      .and_then(|text| serde_json::from_str::<Value>(&text).ok())
      .unwrap_or(Value::Null)
  };
  let mut found: Vec<PathBuf> = Vec::new();
  let workspaces = read("workspaces.json");
  let listed = workspaces["workspaces"]
    .as_array()
    .into_iter()
    .flatten()
    .filter_map(|one| one["directory"].as_str());
  for directory in listed.map(PathBuf::from) {
    if !found.contains(&directory) && directory.is_dir() {
      found.push(directory);
    }
  }
  found
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
