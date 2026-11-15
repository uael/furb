//! The tests of the rig of DeepSWE: the environment that it gives a checkout, and how it watches a
//! life. They stand beside the rig, since its one door, `cargo xtask deepswe`, reads a task and its
//! image from the network, asks a model, and grades in docker.

#[cfg(unix)]
use std::{collections::BTreeMap, env, os::unix::fs::symlink, path::PathBuf};
use std::{
  fs,
  process::{self, Command},
  time::Instant,
};

use crate::yard::Yard;
use serde_json::json;

use super::{Mark, Numbers, Places, Rig, Watched, compacted, seed::requirements};

fn rig(yard: &Yard) -> Rig {
  Rig::of(yard, Places::default(), &yard.join("home"))
}

#[cfg(unix)]
#[test]
fn a_copy_uses_the_image_environment_and_its_own_python_and_source() {
  let yard = Yard::new(format!("xtask-deepswe-copy-{}", process::id()));
  let rig = rig(&yard);
  let python = env::split_paths(&rig.path).map(|one| one.join("python3")).find(|one| one.is_file());
  let python = python.expect("a python3 stands on PATH");
  // The python3 of macOS is a shim that finds its interpreter by the name it runs as, which a link
  // named python does not give, so the link goes to the interpreter itself.
  let python =
    Command::new(python).args(["-c", "import sys; print(sys.executable)"]).output().unwrap();
  let python = PathBuf::from(String::from_utf8_lossy(&python.stdout).trim());
  let (base, copy) = (yard.join("base"), yard.join("copy"));
  for (repo, value) in [(&base, "base"), (&copy, "copy")] {
    fs::create_dir_all(repo.join("pkg")).unwrap();
    fs::write(repo.join("pkg/seed_payload.py"), format!("VALUE = \"{value}\"\n")).unwrap();
    fs::create_dir_all(repo.join(".venv/bin")).unwrap();
    symlink(&python, repo.join(".venv/bin/python")).unwrap();
  }
  let held =
    [("PYTHONPATH", "/app/pkg"), ("PATH", "/opt/venv/bin:/usr/bin"), ("PROBE_VALUE", "image")];
  let held: BTreeMap<String, String> =
    held.iter().map(|(key, value)| ((*key).to_owned(), (*value).to_owned())).collect();
  let env = rig.task_env(&held, &copy).unwrap();
  let found =
    env::split_paths(&env["PATH"]).map(|one| one.join("python")).find(|one| one.is_file());
  assert_eq!(found, Some(copy.join(".venv/bin/python")));
  let probe = "import os, seed_payload; print(seed_payload.VALUE, os.environ['PROBE_VALUE'])";
  let got = Command::new(copy.join(".venv/bin/python"))
    .args(["-c", probe])
    .current_dir(&base)
    .envs(&env)
    .output()
    .unwrap();
  assert!(got.status.success(), "{}", String::from_utf8_lossy(&got.stderr));
  assert_eq!(String::from_utf8_lossy(&got.stdout), "copy image\n");
}

#[test]
fn a_checkout_is_changed_once_a_file_differs_from_the_base_edited_added_or_committed() {
  let yard = Yard::new(format!("xtask-deepswe-changed-{}", process::id()));
  let (rig, repo) = (rig(&yard), yard.join("repo"));
  fs::create_dir_all(&repo).unwrap();
  let git = |args: &[&str]| {
    let done = Command::new("git")
      .args(["-c", "user.email=t@t", "-c", "user.name=t"])
      .args(args)
      .current_dir(&repo)
      .output()
      .unwrap();
    String::from_utf8_lossy(&done.stdout).trim().to_owned()
  };
  git(&["init", "-q"]);
  fs::write(repo.join("a.txt"), "a\n").unwrap();
  git(&["add", "-A"]);
  git(&["commit", "-q", "-m", "base"]);
  let base = git(&["rev-parse", "HEAD"]);
  assert!(!rig.changed(&repo, &base).unwrap());
  fs::write(repo.join("a.txt"), "b\n").unwrap();
  assert!(rig.changed(&repo, &base).unwrap());
  git(&["checkout", "--", "a.txt"]);
  fs::write(repo.join("new.txt"), "n\n").unwrap();
  assert!(rig.changed(&repo, &base).unwrap());
  git(&["add", "-A"]);
  git(&["commit", "-q", "-m", "work"]);
  assert!(rig.changed(&repo, &base).unwrap());
}

#[test]
fn a_python_checkout_gets_each_requirements_file_that_names_dev_or_test_and_no_other() {
  let yard = Yard::new(format!("xtask-deepswe-requirements-{}", process::id()));
  for name in ["requirements.txt", "requirements-dev.txt", "requirements_test.txt", "setup.py"] {
    fs::write(yard.join(name), "").unwrap();
  }
  assert_eq!(requirements(&yard).unwrap(), ["requirements-dev.txt", "requirements_test.txt"]);
}

#[test]
fn the_watch_ends_a_run_when_its_checkout_or_its_record_vanishes() {
  let yard = Yard::new(format!("xtask-deepswe-vanished-{}", process::id()));
  let (app, record) = (yard.join("app"), yard.join(".run/record.jsonl"));
  fs::create_dir_all(&app).unwrap();
  let why = Mark::default().watched(&record, &app, Instant::now()).unwrap();
  assert_eq!(why, Watched::Stops(format!("the life record vanished: {}", record.display())));
  fs::create_dir_all(yard.join(".run")).unwrap();
  fs::write(&record, "").unwrap();
  fs::remove_dir(&app).unwrap();
  let why = Mark::default().watched(&record, &app, Instant::now()).unwrap();
  assert_eq!(why, Watched::Stops(format!("the app checkout vanished: {}", app.display())));
}

/// A record of one answer that cost half a dollar and of one refused reply, as the store writes it.
const RECORD: &str = r#"[["prompt","prompt1","operator","chain1","float","task",""],1]
[["reply","reply1","prompt1","chain1"],2]
[["done","reply1","provider",{"is":"tuple","args":[["assistant","x",{"is":"tuple","args":[[100,20,300,0,{"is":"float","args":["0.5"]}]]},null]]}],3]
[["reply","reply2","prompt1","chain1"],4]
[["done","reply2","provider",{"is":"Refused","args":["no"]}],5]
"#;

#[test]
fn the_watch_ends_a_run_when_its_record_numbers_fall_below_an_earlier_mark() {
  let yard = Yard::new(format!("xtask-deepswe-fell-{}", process::id()));
  let (app, record) = (yard.join("app"), yard.join("record.jsonl"));
  fs::create_dir_all(&app).unwrap();
  fs::write(&record, RECORD).unwrap();
  let mut mark = Mark { seen: Some([2, 1, 0, 0]), ..Mark::default() };
  let why = mark.watched(&record, &app, Instant::now()).unwrap();
  let fell = "the life record went backwards: answers, commands fell below an earlier mark";
  assert_eq!(why, Watched::Stops(fell.to_owned()));
}

#[test]
fn the_numbers_of_a_run_count_each_answer_of_a_model_and_its_cost_off_the_record() {
  let yard = Yard::new(format!("xtask-deepswe-numbers-{}", process::id()));
  fs::write(yard.join("record.jsonl"), RECORD).unwrap();
  let kept = serde_json::to_value(Numbers::of(&yard.join("record.jsonl")).unwrap()).unwrap();
  let counted = json!({
    "facts": 5, "answers": 1, "rungs": 2, "prompts": 1, "commands": 0, "reads": 0, "writes": 0,
    "input": 100, "output": 20, "cache_read": 300, "usd": 0.5,
  });
  assert_eq!(kept, counted);
}

#[test]
fn a_checkout_turns_compaction_on_and_keeps_its_other_local_settings() {
  let yard = Yard::new(format!("xtask-deepswe-compaction-{}", process::id()));
  let config = yard.join(".furb/config.local.json");
  fs::create_dir_all(yard.join(".furb")).unwrap();
  fs::write(&config, r#"{"actor": "fixture:model", "skills": {"skills": false}}"#).unwrap();
  compacted(&yard).unwrap();
  let got: serde_json::Value = serde_json::from_slice(&fs::read(&config).unwrap()).unwrap();
  assert_eq!(
    got,
    json!({"actor": "fixture:model", "skills": {"skills": false, "compaction": true}})
  );
}
