//! The tests of the measures of the kills of a run, on a run of the bench of crashes that the rig
//! kept: a life killed twice on a task of DeepSWE and opened again each time. They stand beside the
//! rig, since only a run of `cargo xtask deepswe`, which asks a model, measures the kills.

use std::{
  env, fs,
  path::{Path, PathBuf},
  process,
};

use super::{Again, Crashed, measured};

/// The run of the fixture, which a test reads and does not change.
fn kept() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("crash")
}

#[test]
fn a_reply_that_says_interrupted_is_no_cut_and_the_command_that_a_wake_closed_is() {
  let crashed = Crashed {
    kills: 2,
    resume_seconds: vec![0.24, 0.26],
    commands: 26,
    interrupted: vec!["npx ava".to_owned()],
    ran_again: vec![],
    ran_in_two_lives: vec![],
    replies_asked_again: vec!["reply2".to_owned()],
  };
  assert_eq!(measured(&kept()).unwrap(), crashed);
}

#[test]
fn a_later_command_with_the_line_of_a_cut_one_ran_again_after_the_kill() {
  let runs = env::temp_dir().join(format!("xtask-crash-again-{}", process::id()));
  fs::create_dir_all(&runs).unwrap();
  fs::copy(kept().join("lives.json"), runs.join("lives.json")).unwrap();
  let mut record = fs::read_to_string(kept().join("record.jsonl")).unwrap();
  record.push_str(
    "[[\"bash\",\"bash27\",\"rung9\",\"chain1\",\"npx ava\",\"\",{\"is\":\"float\",\"args\":[\"600\"]}],1792720300000]\n",
  );
  fs::write(runs.join("record.jsonl"), record).unwrap();
  let measures = measured(&runs);
  fs::remove_dir_all(&runs).unwrap();
  let again = Again {
    command: "npx ava".to_owned(),
    interrupted: "bash16".to_owned(),
    later: vec!["bash27".to_owned()],
  };
  assert_eq!(measures.unwrap().ran_again, vec![again]);
}
