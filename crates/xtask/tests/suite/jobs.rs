//! The task `job-budgets`, on a yard where a stand-in of gh serves the runs of the gates from
//! files, so it asks no network.

use std::{fs, path::Path, process};

use serde_json::{Value, json};

use crate::{ended, held, shimmed, stand_in, xtask, yard::Yard};

/// The gates: a job that calls a workflow for each system, a job of its own, and the job that
/// only holds the check of the others.
const GATES: &str = "jobs:\n  system:\n    name: the jobs on ${{ matrix.os }}\n\
  \x20   uses: ./.github/workflows/system.yml\n  app:\n    name: the hooks\n\
  \x20 gates:\n    name: the gates\n";
const SYSTEM: &str = "jobs:\n  suite:\n    name: the suite on ${{ inputs.os }}\n  bindings:\n    \
  name: the bindings on ${{ inputs.os }}\n";

/// A job of a run, which took the seconds from its start to its end.
fn job(name: &str, conclusion: &str, started: &str, completed: &str) -> Value {
  json!({"name": name, "conclusion": conclusion, "started_at": started, "completed_at": completed})
}

#[test]
fn the_budgets_are_set_for_the_jobs_that_the_gates_name_and_for_no_other() {
  let yard = Yard::new(format!("xtask-jobs-{}", process::id()));
  let (suite, bindings) = (
    "the jobs on macos-latest / the suite on macos-latest",
    "the jobs on windows-latest / the bindings on windows-latest",
  );
  let at = |minute: u32| format!("2026-11-15T12:{minute:02}:00Z");
  let runs = [
    vec![
      job(suite, "success", "2026-11-14T23:59:00Z", "2026-11-15T12:00:30Z"),
      job(bindings, "success", &at(0), &at(1)),
      job("the jobs on macos-latest", "success", &at(0), &at(9)),
      job("the app on macos-latest", "success", &at(0), &at(9)),
      job("the gates", "success", &at(0), &at(9)),
    ],
    vec![job(bindings, "success", &at(0), &at(2)), job(bindings, "failure", &at(0), &at(9))],
  ];
  let listed = |run: &Vec<Value>| Value::from(run.clone()).to_string();
  let (first, second) = (listed(&runs[0]), listed(&runs[1]));
  held(
    &yard,
    &[
      (".github/workflows/gates.yml", GATES),
      (".github/workflows/system.yml", SYSTEM),
      ("jobs1.json", &first),
      ("jobs2.json", &second),
    ],
  );
  let asked = |path: &str| format!("repos/{{owner}}/{{repo}}/actions/{path}");
  let runs = (asked("workflows/gates.yml/runs?branch=main&status=success&per_page=2"), "[1, 2]");
  let made = fs::write(yard.join("runs.json"), runs.1);
  made.unwrap_or_else(|no| panic!("the runs were not written: {no}"));
  let jobs = |run: u32| {
    (asked(&format!("runs/{run}/jobs?per_page=100")), yard.join(format!("jobs{run}.json")))
  };
  let (one, two, green) = (jobs(1), jobs(2), yard.join("runs.json"));
  let served: [(&str, &Path); 3] = [(&runs.0, &green), (&one.0, &one.1), (&two.0, &two.1)];
  stand_in(&yard, "gh", &served);
  let (code, out, err) =
    ended(xtask(&yard, ["job-budgets", "--runs", "2"]).env("PATH", shimmed(&yard)));
  assert_eq!(
    (code, out),
    (
      Some(0),
      format!(
        "{suite}: median 43290 s of 1 runs, budget 64980 s\n\
         {bindings}: median 90 s of 2 runs, budget 180 s\n"
      )
    ),
    "{err}"
  );
  let budgets: Value = serde_json::from_str(
    &fs::read_to_string(yard.join(".github/budgets.json")).unwrap_or_default(),
  )
  .unwrap_or_default();
  assert_eq!(
    budgets,
    json!({bindings: 180, suite: 64980}),
    "a job that calls, a job that no workflow names, the check and a failed job have none"
  );
}
