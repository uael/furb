use std::{fs, path::PathBuf};

use super::{fresh, project_of, projects, records, remember, stamp};

/// A new empty directory for one test.
fn folder(name: &str) -> PathBuf {
  let path = std::env::temp_dir().join(format!("furb-gui-places-{name}-{}", std::process::id()));
  let _ = fs::remove_dir_all(&path);
  fs::create_dir_all(&path).expect("the folder of the test is made");
  path
}

#[test]
fn a_time_reads_as_iso_with_no_colon() {
  assert_eq!(stamp(1_792_575_266_954), "2026-10-21T09-34-26.954Z");
}

#[test]
fn a_new_record_stands_in_the_sessions_of_its_project() {
  let project = folder("fresh");
  let record = fresh(&project).expect("a record is named");
  assert_eq!(record.parent(), Some(project.join(".furb/sessions").as_path()));
  assert!(project.join(".furb/.gitignore").is_file());
  assert_eq!(project_of(&record), project);
}

#[test]
fn the_records_of_a_project_leave_out_the_changes_and_give_the_first_prompt() {
  let project = folder("records");
  let sessions = project.join(".furb/sessions");
  fs::create_dir_all(&sessions).expect("the sessions are made");
  let prompt = r#"[["prompt","prompt1","operator","chain1","str","Fix the build",""],1]"#;
  fs::write(sessions.join("a.jsonl"), format!("{prompt}\n")).expect("the record is written");
  fs::write(sessions.join("a.jsonl.changes.jsonl"), "").expect("the changes are written");
  let found = records(&project);
  assert_eq!(found.len(), 1);
  assert_eq!(found[0].title, "Fix the build");
  assert_eq!(found[0].name, "a");
}

#[test]
fn a_project_that_the_app_opened_comes_first() {
  let config = folder("config");
  let (one, other) = (folder("one"), folder("other"));
  let listed = format!(r#"{{"workspaces":[{{"directory":{:?}}}]}}"#, one.display().to_string());
  fs::write(config.join("workspaces.json"), listed).expect("the list of the TUI is written");
  remember(&config, &other).expect("the project is kept");
  assert_eq!(projects(&config), vec![other, one]);
}
