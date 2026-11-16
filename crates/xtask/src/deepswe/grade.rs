//! The grade of a submission: the verifier of the task, held out of the life, over the diff that
//! the life left, in the image of the task, in a mount namespace of its own, or on plain paths.
#![expect(clippy::indexing_slicing, reason = "debt: an index or a slice that can panic")]

use std::{
  collections::BTreeMap,
  env, fs,
  path::{Path, PathBuf},
  process,
  time::{SystemTime, UNIX_EPOCH},
};

use clap::ValueEnum;
use fancy_regex::{Captures, Regex};
use serde_json::{Value, json};

use super::{Fault, Rig, captured, ran, read, removed, say, seed::Seeded, spoke, written};

/// The plugin of pytest that keeps the names of the cases of the reference grade.
const PYTEST_IDS: &str = include_str!("pytest_ids.py");
/// The profile that the verifier of a rust task reads its results by.
const NEXTEST: &str =
  "[profile.junit]\nfail-fast = false\n\n[profile.junit.junit]\npath = \"junit.xml\"\n";
/// The verifier of a python grade, which runs the frame of the task with the plugin of the names of
/// the cases: a project can replace PYTHONPATH, so the plugin also stands in the site of the python
/// of the grade.
const VERIFY: &str = r#"python3 -c 'import shutil, sysconfig; shutil.copy("/tests/_furb_pytest_ids.py", sysconfig.get_path("purelib"))'
export PYTHONPATH="/tests${PYTHONPATH:+:$PYTHONPATH}"
export PYTEST_PLUGINS="_furb_pytest_ids${PYTEST_PLUGINS:+,$PYTEST_PLUGINS}"
export FURB_PYTEST_CASE_MODE=@MODE@
export FURB_PYTEST_CASES=@CASES@
exec bash /tests/test.sh
"#;

/// Each reporter that the verifier of a task opens by an absolute path, with what it installs.
const REPORTERS: [(&str, &[&str]); 3] = [
  ("/opt/ctrf", &["mocha-ctrf-json-reporter@0.0.11"]),
  ("/opt/jest-ctrf", &["jest-ctrf-json-reporter@0.0.11", "jest-environment-node@29.7.0"]),
  ("/opt/nextest", &[]),
];

/// How this host runs the verifier of a task.
#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum Mode {
  /// The image of the task, with the tests and the logs alone mounted into it.
  Docker,
  /// A mount namespace of its own, where the checkout, the tests and the logs stand at their paths.
  Unshare,
  /// Plain paths, with each path of the frame rewritten to this host.
  Local,
}

impl Rig {
  /// How the verifier runs: as named, or a mount namespace when this host gives one, then the
  /// image when docker answers, and plain paths otherwise.
  fn mode(&self) -> Mode {
    let answers =
      |command: &mut process::Command| captured(command).is_ok_and(|done| done.status.success());
    match self.grade {
      Some(mode) => mode,
      None if answers(self.command("unshare").args(["-m", "true"])) => Mode::Unshare,
      None if answers(self.command("docker").arg("info")) => Mode::Docker,
      None => Mode::Local,
    }
  }

  /// Where a reporter that a frame opens by an absolute path stands on this host: there when its
  /// folder takes a file, and else in the homes of the rig.
  fn reporter(&self, at: &str) -> PathBuf {
    let path = Path::new(at);
    let probe = path.parent().map(|one| one.join(format!(".furb-deepswe-{}", process::id())));
    let writable =
      probe.is_some_and(|probe| fs::write(&probe, "").is_ok() && fs::remove_file(&probe).is_ok());
    if writable { path.to_owned() } else { self.homes.join(path.file_name().unwrap_or_default()) }
  }

  /// Install the reporters that the verifiers of the tasks open by absolute path, which their
  /// images hold and this host does not.
  pub(super) fn tools(&self) -> Result<(), Fault> {
    if !self.found("npm") {
      return Err(Fault::Missing("npm".to_owned()));
    }
    let npm = ["--no-audit", "--no-fund"];
    if !self.found("junit-to-ctrf") {
      say!("npm -g junit-to-ctrf@0.0.14");
      ran(self.command("npm").args(["install", "-g", "junit-to-ctrf@0.0.14"]).args(npm))?;
    }
    for (at, packages) in REPORTERS {
      let into = self.reporter(at);
      fs::create_dir_all(&into).map_err(|source| Fault::File { path: into.clone(), source })?;
      if !packages.is_empty() && !into.join("node_modules").is_dir() {
        say!("npm --prefix {} {}", into.display(), packages.join(" "));
        ran(
          self.command("npm").arg("install").arg("--prefix").arg(&into).args(packages).args(npm),
        )?;
      }
    }
    written(&self.reporter("/opt/nextest").join("nextest.toml"), NEXTEST.as_bytes())?;
    for name in ["go-ctrf-json-reporter", "cargo-nextest"] {
      if !self.found(name) {
        say!("no {name} on PATH: a go or rust task cannot be graded until it is installed");
      }
    }
    say!("reporters under {}", self.homes.display());
    Ok(())
  }

  /// The submission, taken the moment the tree is final: all that the life left, as one diff
  /// against the base.
  pub(super) fn frozen(&self, app: &Path, syn: &str, into: &Path) -> Result<(), Fault> {
    if !spoke(self.git(app).args(["status", "--porcelain"]))?.is_empty() {
      ran(self.git(app).args(["add", "-A"]))?;
      let named = ["-c", "user.email=deepswe@local", "-c", "user.name=deepswe"];
      ran(self.git(app).args(named).args(["commit", "-q", "-m", "submission", "--no-verify"]))?;
    }
    let said = captured(self.git(app).args(["diff", "--binary", syn, "HEAD"]))?.stdout;
    written(into, &said)?;
    say!("submission frozen: {} bytes into {}", said.len(), into.display());
    Ok(())
  }

  /// Whether the checkout differs from the base: a file edited or new, or a commit on top of it.
  pub(super) fn changed(&self, app: &Path, syn: &str) -> Result<bool, Fault> {
    Ok(
      !spoke(self.git(app).args(["status", "--porcelain"]))?.is_empty()
        || !captured(self.git(app).args(["diff", "--quiet", syn]))?.status.success(),
    )
  }

  /// The verifier of the task over a submission, held out of the life, and the reward that it
  /// writes, which `out` keeps, with the log and the reports of that grade beside it.
  pub(super) fn graded(&self, seeded: &Seeded, patch: &Path, out: &Path) -> Result<Value, Fault> {
    let mode = self.mode();
    say!(
      "grading by {}",
      mode.to_possible_value().map(|one| one.get_name().to_owned()).unwrap_or_default()
    );
    let root = yard()?;
    let cases = seeded.work.join(".validate").join("cases.json");
    let reference = out.parent() == cases.parent();
    let (tests, logs) = (root.join("tests"), root.join("logs"));
    for folder in [logs.join("verifier"), logs.join("artifacts")] {
      fs::create_dir_all(&folder).map_err(|source| Fault::File { path: folder.clone(), source })?;
    }
    written(&logs.join("artifacts/model.patch"), &read(patch)?)?;
    ran(self.command("cp").arg("-R").arg(seeded.task.dir.join("tests")).arg(&tests))?;
    let frame = framed(&seeded.task.language, &tests, &cases, reference)?;
    let grading = Grading { app: root.join("app"), tests, logs, frame };
    if let Mode::Docker = mode {
      let mounts = [
        format!("{}:/tests:ro", grading.tests.display()),
        format!("{}:/logs", grading.logs.display()),
      ];
      let mut run = self.command("docker");
      run.args(["run", "--rm", "-v", &mounts[0], "-v", &mounts[1], &seeded.image.named]);
      ran(run.args(["bash", &format!("/tests/{}", grading.frame)]))?;
    } else {
      self.pristine(&seeded.base(), &seeded.syn, &grading.app)?;
      self.rewritten(&grading, &seeded.syn)?;
      self.verified(mode, &grading, &seeded.image.env)?;
    }
    let reward = kept(&grading.logs, out)?;
    let collected = grading.logs.join("artifacts/cases.json");
    if reference && reward["reward"] == 1 && collected.is_file() {
      written(&cases, &read(&collected)?)?;
    }
    // The log and the reports beside a reward are those of that grade, so those of an earlier
    // grade go first.
    for held in ["run.log", "reports"] {
      let beside = out.with_file_name(held);
      removed(&beside)?;
      if grading.logs.join("verifier").join(held).exists() {
        ran(self.command("mv").arg(grading.logs.join("verifier").join(held)).arg(&beside))?;
      }
    }
    removed(&root)?;
    say!("reward {}: {}", reward["reward"], reward);
    Ok(reward)
  }

  /// The verifier on the checkout of the grade, in a mount namespace or on plain paths, with the
  /// environment of the image and a node of the operator's choosing first.
  fn verified(
    &self,
    mode: Mode,
    grading: &Grading,
    held: &BTreeMap<String, String>,
  ) -> Result<(), Fault> {
    let mut env = self.task_env(held, &grading.app)?;
    env.insert("npm_config_confirm_modules_purge".to_owned(), "false".to_owned());
    if let (Some(node), Some(path)) = (&self.node, env.get_mut("PATH")) {
      *path = format!("{}:{path}", node.display());
    }
    let frame = &grading.frame;
    let mut verifier = if let Mode::Unshare = mode {
      let bind = format!(
        "mount --bind $1 /app && mount --bind $2 /tests && mount --bind $3 /logs && cd /app && exec bash /tests/{frame}"
      );
      let mut unshared = self.command("unshare");
      unshared
        .args(["-m", "bash", "-c", &bind, "_"])
        .arg(&grading.app)
        .arg(&grading.tests)
        .arg(&grading.logs);
      unshared
    } else {
      let mut bash = self.command("bash");
      bash.arg(grading.tests.join(frame)).current_dir(&grading.app);
      bash
    };
    ran(verifier.envs(&env)).map(drop)
  }

  /// The frame of the task, aimed at the roots of this grade. The scripts open absolute paths, so
  /// those are rewritten in this copy of them. `config.json` is data: the names of its tests may
  /// hold "/app" as plain text, so only the paths of its reports and its base commit move.
  fn rewritten(&self, grading: &Grading, syn: &str) -> Result<(), Fault> {
    let roots = [("/app", &grading.app), ("/tests", &grading.tests), ("/logs", &grading.logs)];
    let mut swaps: BTreeMap<String, String> =
      roots.into_iter().map(|(at, to)| (at.to_owned(), to.display().to_string())).collect();
    swaps.extend(
      REPORTERS.iter().map(|(at, _)| ((*at).to_owned(), self.reporter(at).display().to_string())),
    );
    // One pass, so no swap lands inside another, and a relative `./tests/x.js` stays as it is.
    let pattern =
      Regex::new(r"(?<![\w./}$])(/opt/jest-ctrf|/opt/ctrf|/opt/nextest|/app|/tests|/logs)\b")?;
    for name in ["test.sh", "grader.py", "_furb_verify.sh"] {
      let path = grading.tests.join(name);
      if path.is_file() {
        let text = String::from_utf8_lossy(&read(&path)?).into_owned();
        let swap = |found: &Captures<'_, str>| swaps.get(&found[1]).cloned().unwrap_or_default();
        written(&path, pattern.try_replacen(&text, 0, swap)?.as_bytes())?;
      }
    }
    let config = grading.tests.join("config.json");
    let what = || config.display().to_string();
    let mut told: Value = serde_json::from_slice(&read(&config)?)
      .map_err(|source| Fault::Json { what: what(), source })?;
    told["base_commit"] = Value::from(syn);
    if let Some(reports) = told.pointer_mut("/grade/reports").and_then(Value::as_array_mut) {
      for report in reports {
        if let Some(rest) = report.as_str().and_then(|one| one.strip_prefix("/logs")) {
          *report = Value::from(format!("{}{rest}", grading.logs.display()));
        }
      }
    }
    let text =
      serde_json::to_string_pretty(&told).map_err(|source| Fault::Json { what: what(), source })?;
    written(&config, text.as_bytes())
  }
}

/// The folders of one grade: the checkout, the tests and the logs, and the frame that it runs.
struct Grading {
  app: PathBuf,
  tests: PathBuf,
  logs: PathBuf,
  frame: String,
}

/// The reward that a grade wrote, kept at `out`, or none when it wrote none.
fn kept(logs: &Path, out: &Path) -> Result<Value, Fault> {
  let said = logs.join("verifier/reward.json");
  let reward = if said.is_file() {
    serde_json::from_slice(&read(&said)?).unwrap_or(Value::Null)
  } else {
    json!({"reward": null})
  };
  let text = serde_json::to_string_pretty(&reward)
    .map_err(|source| Fault::Json { what: "the reward".to_owned(), source })?;
  written(out, format!("{text}\n").as_bytes())?;
  Ok(reward)
}

/// The frame of a grade: the verifier of the task, or, for a task of pytest, the verifier with the
/// names of the cases of the reference grade, which the reference grade itself captures.
fn framed(language: &str, tests: &Path, cases: &Path, reference: bool) -> Result<String, Fault> {
  if language != "python" || !(reference || cases.is_file()) {
    return Ok("test.sh".to_owned());
  }
  written(&tests.join("_furb_pytest_ids.py"), PYTEST_IDS.as_bytes())?;
  if !reference {
    written(&tests.join("_furb_reference_cases.json"), &read(cases)?)?;
  }
  let (mode, held) = if reference {
    ("capture", "/logs/artifacts/cases.json")
  } else {
    ("apply", "/tests/_furb_reference_cases.json")
  };
  written(
    &tests.join("_furb_verify.sh"),
    VERIFY.replace("@MODE@", mode).replace("@CASES@", held).as_bytes(),
  )?;
  Ok("_furb_verify.sh".to_owned())
}

/// A new folder of one grade in the temporary directory.
fn yard() -> Result<PathBuf, Fault> {
  let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_nanos());
  let path = env::temp_dir().join(format!("furb-grade-{}-{now}", process::id()));
  fs::create_dir_all(&path).map_err(|source| Fault::File { path: path.clone(), source })?;
  fs::canonicalize(&path).map_err(|source| Fault::File { path, source })
}
