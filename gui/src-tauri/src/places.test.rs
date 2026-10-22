use std::{fs, path::PathBuf};

use super::{project_of, projects, records};

/// A new empty directory for one test.
fn folder(name: &str) -> PathBuf {
  let path = std::env::temp_dir().join(format!("furb-gui-places-{name}-{}", std::process::id()));
  let _ = fs::remove_dir_all(&path);
  fs::create_dir_all(&path).expect("the folder of the test is made");
  path
}

#[test]
fn a_record_in_the_sessions_of_a_project_stands_in_that_project() {
  let project = folder("project");
  let record = project.join(".furb/sessions/a.jsonl");
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
fn the_projects_are_those_that_the_tui_lists_and_that_stand() {
  let config = folder("config");
  let one = folder("one");
  let gone = std::env::temp_dir().join("furb-gui-places-gone");
  let listed = format!(
    r#"{{"workspaces":[{{"directory":{:?}}},{{"directory":{:?}}},{{"directory":{:?}}}]}}"#,
    one.display().to_string(),
    gone.display().to_string(),
    one.display().to_string()
  );
  fs::write(config.join("workspaces.json"), listed).expect("the list of the TUI is written");
  assert_eq!(projects(&config), vec![one]);
}
