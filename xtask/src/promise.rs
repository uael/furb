//! The proofs of the promise: each test that the page of the promise names, which a release waits
//! for.

use std::{
  fs,
  path::Path,
  process::{Command as Process, ExitCode},
};

use fancy_regex::{Regex, escape};

use crate::{Fault, written};

/// The file and the name of a test that proves a part of the promise.
type Proof = (String, String);

/// One command that runs proofs, and the line by which its output says that each one passed.
struct Run {
  program: &'static str,
  args: Vec<String>,
  passed: Vec<String>,
  held: Vec<Proof>,
}

pub(crate) fn check(root: &Path, page: &Path) -> Result<ExitCode, Fault> {
  let text =
    fs::read_to_string(page).map_err(|source| Fault::Read { path: page.to_owned(), source })?;
  let found = proofs(&text)?;
  if found.is_empty() {
    println!("promise: {} names no proof", page.display());
    return Ok(ExitCode::FAILURE);
  }
  let failed = proven(root, &found)?;
  for (file, name) in &failed {
    println!("promise: {file}::{name} did not pass");
  }
  println!("promise: {} of {} proofs passed", found.len() - failed.len(), found.len());
  Ok(if failed.is_empty() { ExitCode::SUCCESS } else { ExitCode::FAILURE })
}

/// Each proof that its run does not report as passed, after the tail of the output of that run.
fn proven(root: &Path, found: &[Proof]) -> Result<Vec<Proof>, Fault> {
  let mut failed = Vec::new();
  for run in runs(found) {
    println!("promise: {} {}", run.program, run.args.join(" "));
    let done = Process::new(run.program)
      .args(&run.args)
      .current_dir(root)
      .output()
      .map_err(|source| Fault::Spawn { program: run.program.to_owned(), source })?;
    let out =
      format!("{}{}", String::from_utf8_lossy(&done.stdout), String::from_utf8_lossy(&done.stderr));
    let mut missed = Vec::new();
    for (pattern, proof) in run.passed.iter().zip(&run.held) {
      if !Regex::new(pattern)?.is_match(&out)? {
        missed.push(proof.clone());
      }
    }
    if !missed.is_empty() {
      println!("{}", &out[out.floor_char_boundary(out.len().saturating_sub(4000))..]);
    }
    failed.extend(missed);
  }
  Ok(failed)
}

/// The file and the name of each proof that the page names under its heading of the proofs and
/// before what no machine proves yet, once each.
fn proofs(text: &str) -> Result<Vec<Proof>, Fault> {
  let Some(start) = text.find("## The proofs that a machine runs") else { return Ok(Vec::new()) };
  let end = text[start..].find("What no machine proves yet").map_or(text.len(), |at| start + at);
  let mut found: Vec<Proof> = Vec::new();
  for one in Regex::new(r"`([\w./\\:-]+\.(?:py|rs))::(\w+)`")?.captures_iter(&text[start..end]) {
    let one = one?;
    let proof = (one[1].to_owned(), one[2].to_owned());
    if !found.contains(&proof) {
      found.push(proof);
    }
  }
  Ok(found)
}

/// One pytest for the proofs of python, and one cargo test for each target that holds proofs of
/// rust.
fn runs(found: &[Proof]) -> Vec<Run> {
  let mut made = Vec::new();
  let python: Vec<Proof> = found.iter().filter(|(file, _)| written(file, "py")).cloned().collect();
  if !python.is_empty() {
    let mut args: Vec<String> = [
      "run",
      "--no-sync",
      "python",
      "-m",
      "pytest",
      "-q",
      "--no-cov",
      "-p",
      "no:cacheprovider",
      "-n",
      "0",
      "-rA",
    ]
    .map(str::to_owned)
    .into();
    args.extend(python.iter().map(|(file, name)| format!("{file}::{name}")));
    // pytest names a file by its path from its root, so the line of a proof matches the name of its
    // file; on Windows, a file on another drive than the root has no path from it, and pytest names
    // it by the test alone.
    let passed = python
      .iter()
      .map(|(file, name)| {
        let base = file.rsplit(['/', '\\']).next().unwrap_or(file);
        format!(r"(?m)^PASSED (\S*[/\\])?(?:{})?::{name}\b", escape(base))
      })
      .collect();
    made.push(Run { program: "uv", args, passed, held: python });
  }
  let mut rust: Vec<(Vec<String>, Vec<Proof>)> = Vec::new();
  for proof in found.iter().filter(|(file, _)| written(file, "rs")) {
    let args = target(&proof.0);
    match rust.iter_mut().find(|(held, _)| *held == args) {
      Some((_, proofs)) => proofs.push(proof.clone()),
      None => rust.push((args, vec![proof.clone()])),
    }
  }
  for (target, held) in rust {
    let mut args = vec!["test".to_owned()];
    args.extend(target);
    args.push("--".to_owned());
    args.extend(held.iter().map(|(_, name)| name.clone()));
    let passed =
      held.iter().map(|(_, name)| format!(r"(?m)^test (\S+::)?{name} \.\.\. ok$")).collect();
    made.push(Run { program: "cargo", args, passed, held });
  }
  made
}

/// The arguments of cargo test that build the target which holds a test of rust in this file.
fn target(file: &str) -> Vec<String> {
  let (package, rest) = match file.split_once('/') {
    Some(("host", rest)) => ("furb-host", rest),
    Some(("cli", rest)) => ("furb-cli", rest),
    _ => ("furb", file),
  };
  let test = rest.strip_prefix("tests/").and_then(|test| test.strip_suffix(".rs"));
  let test = test.map(|test| if test.starts_with("suite/") { "suite" } else { test });
  match test.filter(|test| !test.contains('/')) {
    Some(test) => ["-p", package, "--test", test].map(str::to_owned).into(),
    None => ["-p", package, "--lib"].map(str::to_owned).into(),
  }
}
