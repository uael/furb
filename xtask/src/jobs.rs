//! The budget of time of each job of the gates, in `.github/budgets.json`: one and a half times its
//! median time in the last green runs of `gates.yml` on main, rounded up to a whole minute.

use std::{
  collections::BTreeMap,
  fs,
  path::Path,
  process::{Command as Process, ExitCode},
};

use fancy_regex::{Regex, escape};
use serde_json::Value;

use crate::Fault;

pub(crate) fn set(root: &Path, runs: u32) -> Result<ExitCode, Fault> {
  let kept = named(&root.join(".github/workflows/gates.yml"))?;
  let path = "repos/{owner}/{repo}/actions/workflows/gates.yml/runs";
  let green = gh(
    root,
    &[
      &format!("{path}?branch=main&status=success&per_page={runs}"),
      "--jq",
      "[.workflow_runs[].id]",
    ],
  )?;
  let mut took: BTreeMap<String, Vec<i64>> = BTreeMap::new();
  for run in green.as_array().into_iter().flatten() {
    let jobs = gh(
      root,
      &[&format!("repos/{{owner}}/{{repo}}/actions/runs/{run}/jobs?per_page=100"), "--jq", ".jobs"],
    )?;
    for job in jobs.as_array().into_iter().flatten() {
      let name = job["name"].as_str().unwrap_or_default();
      let mut matched = false;
      for one in &kept {
        matched |= one.is_match(name)?;
      }
      if job["conclusion"] == "success" && matched {
        let at = |key: &str| seconds(job[key].as_str().unwrap_or_default());
        took.entry(name.to_owned()).or_default().push(at("completed_at") - at("started_at"));
      }
    }
  }
  let mut budgets = BTreeMap::new();
  for (name, times) in &mut took {
    // Twice the median, so a median between two seconds stays whole; the budget is 1.5 times the
    // median, rounded up to a minute.
    let twice = twice_median(times);
    let budget = 60 * ((3 * twice + 239) / 240);
    budgets.insert(name.clone(), Value::from(budget));
    println!("{name}: median {} s of {} runs, budget {budget} s", twice / 2, times.len());
  }
  let path = root.join(".github/budgets.json");
  let text = serde_json::to_string_pretty(&budgets).unwrap_or_default() + "\n";
  fs::write(&path, text).map_err(|source| Fault::Read { path, source })?;
  Ok(ExitCode::SUCCESS)
}

/// The name of each job of the gates that has a budget, as a pattern: a matrix in a name matches
/// any value. A job of a workflow that a job of the gates calls is named under the name of the job
/// that calls it, as GitHub names it, and the job that calls has no budget of its own. The job that
/// only holds the check of the others has no budget.
fn named(workflow: &Path) -> Result<Vec<Regex>, Fault> {
  let matrix = Regex::new(r"\$\{\{.*?\}\}")?;
  let mut made = Vec::new();
  for name in names(workflow, "")? {
    let parts: Vec<String> = matrix
      .split(&name)
      .map(|part| part.map(|one| escape(one).into_owned()))
      .collect::<Result<_, _>>()?;
    made.push(Regex::new(&format!("^{}$", parts.join(".+")))?);
  }
  Ok(made)
}

fn names(workflow: &Path, under: &str) -> Result<Vec<String>, Fault> {
  let text = fs::read_to_string(workflow)
    .map_err(|source| Fault::Read { path: workflow.to_owned(), source })?;
  let (job, name, called) = (
    Regex::new(r"^  [\w-]+:$")?,
    Regex::new(r"^    name: (.+)$")?,
    Regex::new(r"^    uses: \./(\S+)$")?,
  );
  let mut chunks: Vec<Vec<&str>> = vec![Vec::new()];
  for line in text.lines() {
    if job.is_match(line)? {
      chunks.push(Vec::new());
    }
    if let Some(chunk) = chunks.last_mut() {
      chunk.push(line);
    }
  }
  let mut found = Vec::new();
  for chunk in chunks {
    let first = |pattern: &Regex| -> Result<Option<String>, Fault> {
      for line in &chunk {
        if let Some(hit) = pattern.captures(*line)? {
          return Ok(Some(hit[1].to_owned()));
        }
      }
      Ok(None)
    };
    let Some(title) = first(&name)?.filter(|title| title != "the gates") else { continue };
    match first(&called)? {
      Some(called) => {
        let file = Path::new(&called)
          .file_name()
          .map(|one| workflow.with_file_name(one))
          .unwrap_or_default();
        found.extend(names(&file, &format!("{under}{title} / "))?);
      }
      None => found.push(format!("{under}{title}")),
    }
  }
  Ok(found)
}

fn gh(root: &Path, args: &[&str]) -> Result<Value, Fault> {
  let out = Process::new("gh")
    .arg("api")
    .args(args)
    .current_dir(root)
    .output()
    .map_err(|source| Fault::Spawn { program: "gh".to_owned(), source })?;
  if !out.status.success() {
    return Err(Fault::Git {
      args: format!("gh api {}", args.join(" ")),
      stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    });
  }
  Ok(serde_json::from_slice(&out.stdout).unwrap_or_default())
}

fn twice_median(times: &mut [i64]) -> i64 {
  times.sort_unstable();
  let half = times.len() / 2;
  match times.len() {
    0 => 0,
    len if len % 2 == 1 => 2 * times[half],
    _ => times[half - 1] + times[half],
  }
}

/// The seconds of the epoch of a time of GitHub, as `2026-11-15T12:00:00Z`.
fn seconds(stamp: &str) -> i64 {
  let numbers: Vec<i64> =
    stamp.split(|one: char| !one.is_ascii_digit()).filter_map(|one| one.parse().ok()).collect();
  let [year, month, day, hour, minute, second, ..] = numbers[..] else { return 0 };
  // The days from the epoch of a date of the calendar, as Howard Hinnant gives them.
  let year = if month <= 2 { year - 1 } else { year };
  let era = year.div_euclid(400);
  let of_era = year - era * 400;
  let of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
  let days = era * 146_097 + of_era * 365 + of_era / 4 - of_era / 100 + of_year - 719_468;
  days * 86_400 + hour * 3600 + minute * 60 + second
}
