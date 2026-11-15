//! The task `deepswe`, which puts one life of furb against one task of the DeepSWE collection,
//! at <https://github.com/datacurve-ai/deep-swe>, and grades what the life left with the verifier
//! of the task.
//!
//! Where everything stands:
//!
//! - Each task works under `<work>/<task>`: `base` is the pristine checkout with the dependencies
//!   of the task, `app` the checkout that the life works in, and `.run` holds the record, the
//!   numbers, the frozen submission and the reward. `.validate` holds the reference grade.
//! - The interpreter of a python task is `.venv` in the checkout, at the version of python that the
//!   image of the task runs, which is the python of its verifier: the rig reads `PYTHON_VERSION`
//!   from the config of the image in its registry, and `uv` makes the interpreter, which starts
//!   with each distribution that the image took from an index, at its version, pip and poetry
//!   among them. The steps of the Dockerfile then run on it. `constraints.txt` in the workroot
//!   pins each of them, and each `pip install` of a seed takes it on its command line, not in
//!   `PIP_CONSTRAINT`, since pip gives its environment to the builds that it isolates.
//! - The rig reads, from the layers of the image, each distribution that its python holds, and
//!   keeps what a layer holds under `layers` of the homes, so each layer is read once. After the
//!   steps, a seed installs again from the tree each distribution that the image built from it, at
//!   the version that the image holds, and each `requirements*.txt` at the root whose name holds
//!   `dev` or `test`, since a Dockerfile can leave out the packages that the suite needs.
//! - The environment of the image holds for the install, the commands of the model and a local
//!   grade, with its paths moved to the checkout that runs, and the python of the checkout first
//!   on PATH, then the tools of the host. A step of poetry reads and writes its config under
//!   `poetry` in the workroot.
//! - `app` and the tree of a grade outside docker are copies of the base, and the venv of each
//!   copy is its own, as [`Rig::pristine`] says.
//! - The reaper of a host takes the files of an old workroot and leaves its folders, so a seed
//!   writes `.seeded` last, with the commit of the base, the digest of the config of the image and
//!   the format of the environment, and a later seed keeps the base only when that seal agrees, no
//!   tracked file is gone and its python stands.
//! - The reporters stand under the homes, `~/.cache/furb-deepswe`, when the path that a task names
//!   is not this host's to write. Each graded run is archived in `traces/` of the tree, which git
//!   ignores, as the record that it kept and a result beside it.
//!
//! The bounds: a grant bounds the money, and two more bounds end a run that goes nowhere. The rig
//! keeps a mark at the last thing that the life did, and ends a run that bought `STUCK` answers
//! since that mark. Each step of the rig ends after `STEP`, so an install that waits for input
//! that it never gets does not hold the run.

mod crash;
mod grade;
mod image;
mod seed;

use std::{
  collections::BTreeMap,
  env,
  ffi::{OsStr, OsString},
  fs, io,
  path::{Path, PathBuf},
  process::{Child, Command, ExitCode, ExitStatus, Output, Stdio},
  time::{Duration, Instant},
};

use clap::{Args, Subcommand};
use serde::Serialize;
use serde_json::{Value, json};
use toml::de;
use wait_timeout::ChildExt;

use crash::Crash;
use grade::Mode;
use seed::{Seeded, short};

/// The seconds that one step of the rig may take, since a step that waits for ever holds the whole
/// of it.
const STEP: Duration = Duration::from_mins(30);
/// The time between two looks at a life that works.
const TICK: Duration = Duration::from_secs(2);
/// How many looks stand between two reads of the record, since each read reads the whole of it.
const LOOK: u32 = 10;
/// How many answers a life may buy and do nothing before the rig reads it as wedged. A model that
/// answers with a value and no call leaves its prompt open, so the chain asks again, and one run
/// bought 412 such answers before the ceiling of its grant ended it.
const STUCK: u64 = 25;
/// The confidence that the rig takes for done, under which it asks the model to go on.
const CONFIDENT: f64 = 0.9;
/// What the rig adds to the instruction of the task, which says the shape of the answer and no
/// more.
const ASKED: &str = "\n\nWhen the work is done, close with a float from 0 to 1: how sure you are that this task is complete.";
/// What the operator says to a model that closed under the confidence that the rig wants.
const AGAIN: &str = "confidence level {} is too low, continue";
/// What the operator says, once, to a model that closed sure of itself while its checkout held no
/// change.
const NOTHING: &str = "your tree has no diff against the base, so nothing is submitted. Change the files that the task asks for, then close again.";

/// One line of the log of the rig, on stderr, since stdout holds what a verb gives a program.
macro_rules! say {
  ($($told:tt)*) => {
    eprintln!("[deepswe] {}", format_args!($($told)*))
  };
}
use say;

/// The verbs of the rig.
#[derive(Subcommand)]
pub(crate) enum Verb {
  /// Install the reporters that the verifiers of the tasks open by absolute path, which their
  /// images hold and this host does not. It needs npm, and says when go-ctrf-json-reporter or
  /// cargo-nextest is missing, without which a go or a rust task cannot be graded.
  Tools,
  /// Stand the workroot of a task up, and stop before any model is asked, so a run that costs
  /// money starts warm. It prints one object of JSON, which a bench of DeepSWE reads: the
  /// workroot, the checkout, the commit of the base, the instruction of the task, and the words of
  /// the rig, `asked`, `again` and `confident`.
  Seed {
    /// The task, as the collection names it.
    task: String,
  },
  /// Grade the reference solution of a task, the control: a rig that scores it under 1 measures
  /// itself and not the life. For a task of pytest, the reference grade keeps the names of the
  /// cases, and a later grade gives a case the reference name of the same test definition and
  /// parameter values when one case matches on each side.
  Validate {
    /// The task, as the collection names it.
    task: String,
  },
  /// Seed, let one life work the task, freeze what it left, grade it, and archive the whole of it.
  Run(Run),
  /// Take the submission from the checkout as it stands, as one diff against the base.
  Freeze {
    /// The task, as the collection names it.
    task: String,
  },
  /// Grade what the checkout holds now, or the submission that a run froze.
  Grade {
    /// The task, as the collection names it.
    task: String,
  },
  /// Put a task to the claude command line in place of a life of furb, the baseline of a bench:
  /// seed it fresh, let claude work it once, freeze what it left, grade it, and archive it with
  /// what claude said, its cost among it.
  Baseline {
    /// The task, as the collection names it.
    task: String,
    /// The claude command line, as the one that asks the local server.
    #[arg(long, default_value = "claude")]
    claude: PathBuf,
    /// The seconds that claude may take.
    #[arg(long, default_value_t = 1800.0)]
    timeout: f64,
  },
  /// Print each turn of the root of a run, as the model read it, with `furb turns`.
  Turns {
    /// The task, as the collection names it.
    task: String,
    /// The actor that the life of the record stands on; the default actor when unsaid.
    #[arg(long)]
    to: Option<String>,
  },
}

/// What a run of a task takes.
#[derive(Args)]
pub(crate) struct Run {
  /// The task, as the collection names it.
  task: String,
  /// The actor that the task is put to, as provider:model/effort; the default actor of furb when
  /// unsaid.
  #[arg(long)]
  to: Option<String>,
  /// The dollars that the life may spend, a grant on the root chain, which pauses the chain at it;
  /// 0 is no ceiling.
  #[arg(long, default_value_t = 8.0)]
  ceiling: f64,
  /// The seconds that the run may take; 0 is no cap.
  #[arg(long, default_value_t = 1800.0)]
  timeout: f64,
  /// Go on with the checkout and the record that stand, in place of a fresh seed.
  #[arg(long)]
  resume: bool,
  /// How many times the rig kills the life, as a crash would, at a random point after the first
  /// answer of a life, and opens it again on its record, which takes the work up. What the kills
  /// cost goes into numbers.json under crash.
  #[arg(long, default_value_t = 0)]
  kills: u32,
  /// The seed of the points of the kills; a random seed when unsaid.
  #[arg(long)]
  seed: Option<u64>,
}

/// Where the rig keeps its work, and how it grades.
#[derive(Args, Default)]
pub(crate) struct Places {
  /// The folder of the collection, which the rig fetches once; ~/.cache/furb/deep-swe when unsaid.
  #[arg(long, env = "DEEPSWE_DIR", value_name = "DIR", global = true)]
  collection: Option<PathBuf>,
  /// The folder of the workroot of each task; ~/.cache/furb/deepswe-work when unsaid.
  #[arg(long, env = "DEEPSWE_WORK", value_name = "DIR", global = true)]
  work: Option<PathBuf>,
  /// How the verifier runs; when unsaid, a mount namespace if this host gives one, then docker if
  /// docker answers, and plain paths otherwise.
  #[arg(long, env = "DEEPSWE_GRADE", value_enum, global = true)]
  grade: Option<Mode>,
  /// A folder that a grade puts first on its PATH, as the bin of a node of your choosing.
  #[arg(long, env = "DEEPSWE_NODE_HOME", value_name = "DIR", global = true)]
  node: Option<PathBuf>,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum Fault {
  #[error("{program} did not start: {source}")]
  Spawn { program: String, source: io::Error },
  #[error("{} cannot be read or written: {source}", path.display())]
  File { path: PathBuf, source: io::Error },
  #[error("this machine has no home folder, where the rig keeps its work")]
  Homeless,
  #[error("no {0} on PATH, and the rig cannot run without it")]
  Missing(String),
  #[error("no task {task}: the tasks are the folders of {}", tasks.display())]
  NoTask { task: String, tasks: PathBuf },
  #[error("the task {task} names no {key} in its task.toml")]
  Unnamed { task: String, key: &'static str },
  #[error("the task.toml of {task} is not TOML: {source}")]
  Toml { task: String, source: de::Error },
  #[error("{what} is not the JSON that the rig reads: {source}")]
  Json { what: String, source: serde_json::Error },
  #[error("{0} could not be fetched")]
  Fetch(String),
  #[error("{} could not be unpacked", .0.display())]
  Unpacked(PathBuf),
  #[error("{0} is an index of images, and the rig reads one image only")]
  Index(String),
  #[error("{0} names no PYTHON_VERSION, so the rig cannot run the python that its verifier runs")]
  Unversioned(String),
  #[error("{0} could not be installed")]
  Install(String),
  #[error("furb did not run the word {word:?}: {why}")]
  Word { word: String, why: String },
  #[error("{0}")]
  Unproven(String),
  #[error("{} is a binary that names the base, and the rig cannot move it to {}", path.display(), into.display())]
  Binary { path: PathBuf, into: PathBuf },
  #[error("no record of a run at {}", .0.display())]
  NoRecord(PathBuf),
  #[error("furb could not be built")]
  Unbuilt,
  #[error("{} holds no object of JSON", .0.display())]
  Config(PathBuf),
  #[error("the pattern of the rig failed: {0}")]
  Pattern(#[from] fancy_regex::Error),
}

/// The rig on this machine: the tree that it runs from, where it keeps its work, how it grades,
/// and the PATH that each of its commands takes.
pub(crate) struct Rig {
  root: PathBuf,
  collection: PathBuf,
  work: PathBuf,
  /// What outlives a workroot: the reporters whose path this host does not let the rig write,
  /// and the layers that the rig read.
  homes: PathBuf,
  home: PathBuf,
  grade: Option<Mode>,
  node: Option<PathBuf>,
  /// The PATH of this process less the bin of the venv that it runs in, if any: a command of a
  /// model, a step of a seed and a grade would find the python and the pytest of that venv there.
  path: OsString,
}

/// Run a verb of the rig, and fail as the verb fails or as the grade is under 1.
pub(crate) fn run(root: &Path, places: Places, verb: Verb) -> Result<ExitCode, Fault> {
  let rig = Rig::of(root, places, &env::home_dir().ok_or(Fault::Homeless)?);
  let graded =
    |reward: &Value| if reward["reward"] == 1 { ExitCode::SUCCESS } else { ExitCode::FAILURE };
  match verb {
    Verb::Tools => rig.tools().map(|()| ExitCode::SUCCESS),
    Verb::Seed { task } => {
      let seeded = rig.seeded(&task, false)?;
      let words = json!({
        "work": seeded.work, "app": seeded.app(), "base": seeded.syn,
        "instruction": instruction(&seeded)?, "asked": ASKED, "again": AGAIN,
        "confident": CONFIDENT,
      });
      println!("{words}");
      Ok(ExitCode::SUCCESS)
    }
    Verb::Validate { task } => rig.validate(&task),
    Verb::Run(run) => rig.run(&run).map(|reward| graded(&reward)),
    Verb::Baseline { task, claude, timeout } => {
      rig.baseline(&task, &claude, Duration::from_secs_f64(timeout)).map(|reward| graded(&reward))
    }
    Verb::Freeze { task } => {
      let work = rig.work.join(&task);
      let syn = spoke(rig.git(&work.join("base")).args(["rev-parse", "HEAD"]))?;
      rig.frozen(&work.join("app"), &syn, &work.join(".run").join("model.patch"))?;
      Ok(ExitCode::SUCCESS)
    }
    Verb::Grade { task } => {
      let seeded = rig.seeded(&task, true)?;
      let runs = seeded.work.join(".run");
      let patch = runs.join("model.patch");
      if !fs::metadata(&patch).is_ok_and(|held| held.len() > 0) {
        rig.frozen(&seeded.app(), &seeded.syn, &patch)?;
      }
      rig.graded(&seeded, &patch, &runs.join("reward.json")).map(|reward| graded(&reward))
    }
    Verb::Turns { task, to } => {
      let work = rig.work.join(&task);
      let record = work.join(".run").join("record.jsonl");
      if !record.is_file() {
        return Err(Fault::NoRecord(record));
      }
      let mut turns = rig.command(furb(&rig.root)?);
      turns.args(["turns", "--record"]).arg(&record).arg("--cwd").arg(work.join("app"));
      turns.args(to.iter().flat_map(|to| ["--model", to.as_str()]));
      Ok(if ran(&mut turns)? { ExitCode::SUCCESS } else { ExitCode::FAILURE })
    }
  }
}

impl Rig {
  fn of(root: &Path, places: Places, home: &Path) -> Self {
    let cache = home.join(".cache");
    let own = env::var_os("VIRTUAL_ENV").map(|venv| PathBuf::from(venv).join("bin"));
    let path = env::var_os("PATH").unwrap_or_default();
    let kept = env::split_paths(&path).filter(|one| Some(one) != own.as_ref());
    Self {
      root: root.to_owned(),
      collection: places.collection.unwrap_or_else(|| cache.join("furb").join("deep-swe")),
      work: places.work.unwrap_or_else(|| cache.join("furb").join("deepswe-work")),
      homes: cache.join("furb-deepswe"),
      home: home.to_owned(),
      grade: places.grade,
      node: places.node,
      path: env::join_paths(kept).unwrap_or(path),
    }
  }

  /// A command of the rig, which takes the PATH of the rig and no venv of its own.
  fn command(&self, program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    command.env("PATH", &self.path).env_remove("VIRTUAL_ENV");
    command
  }

  /// Git, told the tree that it speaks of and that the repository of that tree is the one it
  /// holds: git told a folder alone climbs to the repository above it when the `.git` there is
  /// empty, so a reset or a commit would land in a tree that is not the rig's.
  fn git(&self, tree: &Path) -> Command {
    let mut git = self.command("git");
    git.arg("-C").arg(tree).args(["--git-dir", ".git"]);
    git
  }

  /// Whether a program stands on the PATH of the rig.
  fn found(&self, name: &str) -> bool {
    env::split_paths(&self.path).any(|folder| folder.join(name).is_file())
  }

  /// The control of the rig: the grade of the reference solution, which must score 1 before a run
  /// means anything.
  fn validate(&self, task: &str) -> Result<ExitCode, Fault> {
    let seeded = self.seeded(task, false)?;
    let runs = seeded.work.join(".validate");
    fs::create_dir_all(&runs).map_err(|source| Fault::File { path: runs.clone(), source })?;
    let patch = runs.join("model.patch");
    let solution = read(&seeded.task.dir.join("solution").join("solution.patch"))?;
    written(&patch, &solution)?;
    say!("grading the reference solution ({} bytes)", solution.len());
    if self.graded(&seeded, &patch, &runs.join("reward.json"))?["reward"] == 1 {
      say!("the rig grades this task faithfully");
      Ok(ExitCode::SUCCESS)
    } else {
      say!("the reference solution did not score 1: this host cannot grade this task faithfully");
      Ok(ExitCode::FAILURE)
    }
  }

  /// Seed, let one life work the task, freeze what it left, grade it, and keep the whole of it.
  /// The message of the life is the instruction of the task as the task wrote it, and `ASKED`.
  fn run(&self, run: &Run) -> Result<Value, Fault> {
    let seeded = self.seeded(&run.task, run.resume)?;
    let runs = seeded.work.join(".run");
    // A fresh checkout takes a fresh record: a record beside a tree that it never wrote is no run.
    if !run.resume {
      removed(&runs)?;
    }
    fs::create_dir_all(&runs).map_err(|source| Fault::File { path: runs.clone(), source })?;
    let app = seeded.app();
    compacted(&app)?;
    let life = Life {
      furb: furb(&self.root)?,
      env: self.task_env(&seeded.image.env, &app)?,
      record: runs.join("record.jsonl"),
      runs,
      app,
      to: run.to.clone(),
      limit: (run.timeout > 0.0).then(|| Duration::from_secs_f64(run.timeout)),
      began: Instant::now(),
    };
    let mut mark =
      Mark { crash: (run.kills > 0).then(|| Crash::new(run.kills, run.seed)), ..Mark::default() };
    say!(
      "run {}: to={} ceiling={} timeout={}s",
      run.task,
      run.to.as_deref().unwrap_or("the default actor"),
      run.ceiling,
      run.timeout
    );
    if run.ceiling > 0.0 {
      life
        .word(&format!("await skill('grant')\nrung('grant(usd={:?})')", run.ceiling), &mut mark)?;
    }
    let (value, stopped) = self.worked(&life, &seeded, &mut mark)?;
    if !stopped.is_empty() {
      say!("{stopped}");
    }
    numbered(&life, &mark, value, &run.task)?;
    let patch = life.runs.join("model.patch");
    self.frozen(&life.app, &seeded.syn, &patch)?;
    let reward = self.graded(&seeded, &patch, &life.runs.join("reward.json"))?;
    let to = run.to.clone().or_else(|| stood(&life.record)).unwrap_or_default();
    self.archived(
      &run.task,
      &life.runs,
      &json!({"to": to, "base": seeded.syn, "stopped": stopped}),
    )?;
    Ok(reward)
  }

  /// The life asked again on its chain while it closes under `CONFIDENT`, and once more when it
  /// closes sure of itself with no change: the value that it closed with last, and why the rig
  /// stopped it, if it did. Each round is a prompt under the key of its number, so a model that
  /// closes twice at one level is asked again, and a resume takes up the rounds of its record.
  fn worked(
    &self,
    life: &Life,
    seeded: &Seeded,
    mark: &mut Mark,
  ) -> Result<(Value, String), Fault> {
    let first = format!("{}{ASKED}", instruction(seeded)?);
    let (mut message, mut empty, mut value) = (first, false, Value::Null);
    let mut round = 0_u32;
    loop {
      round += 1;
      let key = round.to_string();
      let ask = Ask { message: &message, shape: "float", key: &key, operator: "" };
      let got = match life.prompt(&ask, mark)? {
        Asked::Answered(said) => match said.parse::<f64>() {
          Ok(got) => got,
          Err(_) => return Ok((Value::from(said), String::new())),
        },
        Asked::Stopped(why) => return Ok((value, why)),
      };
      value = Value::from(got);
      if got < CONFIDENT {
        say!(
          "the model is {got} sure, under the {CONFIDENT} that the rig wants; it is asked to go on"
        );
        message = AGAIN.replace("{}", &got.to_string());
      } else if empty || self.changed(&life.app, &seeded.syn)? {
        return Ok((value, String::new()));
      } else {
        say!("the model closed sure of itself with no change in its checkout; it is told so, once");
        (empty, message) = (true, NOTHING.to_owned());
      }
    }
  }

  /// The baseline: claude on a fresh checkout of a task, frozen, graded and archived as a run is.
  fn baseline(&self, task: &str, claude: &Path, limit: Duration) -> Result<Value, Fault> {
    let seeded = self.seeded(task, false)?;
    let (runs, app) = (seeded.work.join(".run"), seeded.app());
    removed(&runs)?;
    fs::create_dir_all(&runs).map_err(|source| Fault::File { path: runs.clone(), source })?;
    let said = runs.join("claude.json");
    let file = |path: &Path| {
      fs::File::create(path).map_err(|source| Fault::File { path: path.to_owned(), source })
    };
    let mut asked = self.command(claude);
    asked.args(["-p", &instruction(&seeded)?, "--dangerously-skip-permissions"]);
    asked
      .args(["--output-format", "json"])
      .current_dir(&app)
      .envs(self.task_env(&seeded.image.env, &app)?);
    asked.stdin(Stdio::null()).stdout(file(&said)?).stderr(file(&runs.join("life.log"))?);
    grouped(&mut asked);
    let began = Instant::now();
    let mut child = spawned(&mut asked)?;
    let stopped = if child.wait_timeout(limit).map_err(|source| fault(&asked, source))?.is_some() {
      String::new()
    } else {
      ended(&mut child).map_err(|source| fault(&asked, source))?;
      format!("the cap of {} seconds ran out", limit.as_secs())
    };
    let told: Value = serde_json::from_slice(&read(&said)?).unwrap_or(Value::Null);
    let seconds = (began.elapsed().as_secs_f64() * 10.0).round() / 10.0;
    let numbers =
      json!({"usd": told["total_cost_usd"], "wall_seconds": seconds, "value": told["result"]});
    written(&runs.join("numbers.json"), pretty(&numbers)?.as_bytes())?;
    let patch = runs.join("model.patch");
    self.frozen(&app, &seeded.syn, &patch)?;
    let reward = self.graded(&seeded, &patch, &runs.join("reward.json"))?;
    self.archived(task, &runs, &json!({"to": claude, "base": seeded.syn, "stopped": stopped}))?;
    Ok(reward)
  }

  /// One graded run, kept where it stands on its own: its record, and its result beside it.
  fn archived(&self, task: &str, runs: &Path, meta: &Value) -> Result<(), Fault> {
    let traces = self.root.join("traces");
    fs::create_dir_all(&traces).map_err(|source| Fault::File { path: traces.clone(), source })?;
    let stamp = spoke(self.command("date").args(["-u", "+%Y%m%d-%H%M%S"]))?;
    let name = format!("{task}-{stamp}");
    let held = |file: &str| {
      read(&runs.join(file))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .unwrap_or(Value::Null)
    };
    let (numbers, reward) = (held("numbers.json"), held("reward.json"));
    if runs.join("record.jsonl").is_file() {
      written(&traces.join(format!("{name}.jsonl")), &read(&runs.join("record.jsonl"))?)?;
    }
    let out_of = |passed: &str, total: &str| {
      (!reward[total].is_null()).then(|| format!("{}/{}", reward[passed], reward[total]))
    };
    let result = json!({
      "task": task, "to": meta["to"], "reward": reward["reward"], "partial": reward["partial"],
      "f2p": out_of("f2p_passed", "f2p_total"), "p2p": out_of("p2p_passed", "p2p_total"),
      "usd": numbers["usd"], "wall_seconds": numbers["wall_seconds"], "answers": numbers["answers"],
      "commands": numbers["commands"], "value": numbers["value"], "stopped": meta["stopped"],
      "crash": numbers["crash"],
      "engine": spoke(self.command("git").arg("-C").arg(&self.root).args(["rev-parse", "HEAD"]))?,
      "base": meta["base"], "at": stamp,
    });
    written(&traces.join(format!("{name}.result.json")), pretty(&result)?.as_bytes())?;
    say!("archived into traces/{name}.result.json");
    Ok(())
  }
}

/// One life of furb that a task drives on a record, through the program furb on monty, the engine
/// that a user runs: the program, the folder that its chains start in, the folder of its log, its
/// record, the environment of each of its commands, its actor, and the cap of its time. Each
/// command of furb on it is a process of its own, in the folder of its chains.
pub(crate) struct Life {
  pub(crate) furb: PathBuf,
  pub(crate) app: PathBuf,
  pub(crate) runs: PathBuf,
  pub(crate) record: PathBuf,
  pub(crate) env: BTreeMap<String, String>,
  pub(crate) to: Option<String>,
  pub(crate) limit: Option<Duration>,
  pub(crate) began: Instant,
}

/// One prompt of the operator on the root of a life.
pub(crate) struct Ask<'a> {
  /// The message, which the actor reads.
  pub(crate) message: &'a str,
  /// The shape of the answer, as python names it.
  pub(crate) shape: &'a str,
  /// The key of the prompt on the record, so the same command after a kill takes it up.
  pub(crate) key: &'a str,
  /// What the operator answers, a line for each prompt that the life puts to him.
  pub(crate) operator: &'a str,
}

/// What one command of furb on a life came to.
pub(crate) enum Asked {
  /// What furb printed, the value of the act.
  Answered(String),
  /// The task or furb stopped the life, with why.
  Stopped(String),
}

impl Life {
  /// furb on the record of the life, in its folder, with the environment of its commands, writing
  /// to the log of the life.
  fn furb(&self, verb: &str) -> Result<Command, Fault> {
    let mut furb = Command::new(&self.furb);
    furb.arg(verb).arg("--record").arg(&self.record).arg("--cwd").arg(&self.app);
    furb.args(self.to.iter().flat_map(|to| ["--model", to.as_str()]));
    furb.current_dir(&self.app).envs(&self.env).env_remove("VIRTUAL_ENV");
    let path = self.runs.join("life.log");
    let failed = |source| Fault::File { path: path.clone(), source };
    let log = fs::OpenOptions::new().create(true).append(true).open(&path).map_err(failed)?;
    furb.stderr(log.try_clone().map_err(failed)?).stdout(log);
    Ok(furb)
  }

  /// A word of the operator on the root, as the grant, and what it gave.
  pub(crate) fn word(&self, word: &str, mark: &mut Mark) -> Result<String, Fault> {
    let mut run = self.furb("run")?;
    run.arg(word);
    match self.watched(run, "", mark)? {
      Asked::Answered(said) => Ok(said),
      Asked::Stopped(why) => Err(Fault::Word { word: short(word, 80).to_owned(), why }),
    }
  }

  /// One prompt on the root under its key, watched until it closes, the cap of the life ends it,
  /// or the watch reads the life as going nowhere.
  pub(crate) fn prompt(&self, ask: &Ask<'_>, mark: &mut Mark) -> Result<Asked, Fault> {
    let mut prompt = self.furb("prompt")?;
    prompt.args(["--shape", ask.shape, "--key", ask.key, "--", ask.message]);
    self.watched(prompt, ask.operator, mark)
  }

  /// One command of furb on the life, in a process group of its own, which the operator answers
  /// from his lines. A kill of the watch, as a crash, ends the group and runs the command again,
  /// which takes up the work that the record holds.
  fn watched(&self, mut command: Command, operator: &str, mark: &mut Mark) -> Result<Asked, Fault> {
    let (answer, lines) = (self.runs.join("answer.txt"), self.runs.join("operator.txt"));
    written(&lines, operator.as_bytes())?;
    grouped(&mut command);
    loop {
      let opened = |path: &Path, file: io::Result<fs::File>| {
        file.map_err(|source| Fault::File { path: path.to_owned(), source })
      };
      command.stdout(opened(&answer, fs::File::create(&answer))?);
      command.stdin(opened(&lines, fs::File::open(&lines))?);
      mark.spawned(&self.record)?;
      let mut child = spawned(&mut command)?;
      let mut looks = 0;
      let stopped = loop {
        if let Some(status) = child.wait_timeout(TICK).map_err(|source| fault(&command, source))? {
          return self.closed(status, &answer);
        }
        if let Some(limit) = self.limit.filter(|limit| self.began.elapsed() > *limit) {
          break Some(format!("the cap of {} seconds ran out", limit.as_secs()));
        }
        looks += 1;
        if looks % LOOK == 0 {
          match mark.watched(&self.record, &self.app, self.began)? {
            Watched::Goes => {}
            Watched::Kill => break None,
            Watched::Stops(why) => break Some(why),
          }
        }
      };
      ended(&mut child).map_err(|source| fault(&command, source))?;
      if let Some(why) = stopped {
        return Ok(Asked::Stopped(why));
      }
      say!("the rig kills the life, as a crash would, and opens it again on its record");
    }
  }

  /// What a command that furb ended came to: what it printed, or why furb ended it, as a pause
  /// that the grant says at its ceiling, from the last line of the log of the life.
  fn closed(&self, status: ExitStatus, answer: &Path) -> Result<Asked, Fault> {
    if !status.success() {
      let log = String::from_utf8_lossy(&read(&self.runs.join("life.log"))?).into_owned();
      let last =
        log.lines().rev().find(|line| !line.trim().is_empty()).unwrap_or_default().to_owned();
      return Ok(Asked::Stopped(format!("furb ended the command, {status}: {last}")));
    }
    Ok(Asked::Answered(String::from_utf8_lossy(&read(answer)?).trim().to_owned()))
  }
}

/// What the watch makes of a life at one look.
#[derive(Debug, PartialEq)]
pub(crate) enum Watched {
  /// The life goes on.
  Goes,
  /// The point of a kill came, as a crash.
  Kill,
  /// The life goes nowhere, for this reason.
  Stops(String),
}

/// The counts of the life at the last look, the mark of the last thing that it did, the dollars
/// that the whole record may cost, and the kills of a run.
#[derive(Default)]
pub(crate) struct Mark {
  seen: Option<[u64; 4]>,
  did: u64,
  answers: u64,
  /// The ceiling of the spend of each chain of the record, since a grant bounds one chain alone.
  ceiling: Option<f64>,
  crash: Option<Crash>,
}

impl Mark {
  /// A watch that also ends the life when its record costs more than the ceiling, in dollars.
  pub(crate) fn spending(ceiling: f64) -> Self {
    Self { ceiling: Some(ceiling), ..Self::default() }
  }

  /// A command of furb that starts on the record, which the kills count as a life.
  fn spawned(&mut self, record: &Path) -> Result<(), Fault> {
    self.crash.as_mut().map_or(Ok(()), |crash| crash.spawned(record))
  }

  /// What the life comes to at this look: it goes nowhere when its checkout or its record
  /// vanished, its record went backwards, it spent past the ceiling, or it bought `STUCK` answers
  /// since the last thing that it did; and the kills say when one is due.
  fn watched(&mut self, record: &Path, app: &Path, began: Instant) -> Result<Watched, Fault> {
    if !app.is_dir() {
      return Ok(Watched::Stops(format!("the app checkout vanished: {}", app.display())));
    }
    if !record.is_file() {
      return Ok(Watched::Stops(format!("the life record vanished: {}", record.display())));
    }
    let numbers = Numbers::of(record)?;
    let counts = [numbers.answers, numbers.commands, numbers.reads, numbers.writes];
    say!(
      "{:.1}s: answers={} commands={} reads={} writes={} ${:.4}",
      began.elapsed().as_secs_f64(),
      counts[0],
      counts[1],
      counts[2],
      counts[3],
      numbers.usd
    );
    let names = ["answers", "commands", "reads", "writes"];
    let fell: Vec<&str> = self.seen.map_or_else(Vec::new, |seen| {
      names
        .iter()
        .zip(counts.iter().zip(seen))
        .filter(|(_, (now, was))| *now < was)
        .map(|(name, _)| *name)
        .collect()
    });
    if !fell.is_empty() {
      let fell = fell.join(", ");
      return Ok(Watched::Stops(format!(
        "the life record went backwards: {fell} fell below an earlier mark"
      )));
    }
    self.seen = Some(counts);
    if let Some(ceiling) = self.ceiling.filter(|ceiling| numbers.usd > *ceiling) {
      return Ok(Watched::Stops(format!("the life spent ${:.4}, over ${ceiling}", numbers.usd)));
    }
    let did = counts[1] + counts[2] + counts[3];
    if did > self.did {
      (self.did, self.answers) = (did, counts[0]);
    } else if counts[0].saturating_sub(self.answers) >= STUCK {
      let bought = counts[0] - self.answers;
      return Ok(Watched::Stops(format!(
        "{bought} answers bought and nothing done with them: the life is wedged"
      )));
    }
    let due = self.crash.as_mut().is_some_and(|crash| crash.due(counts[0]));
    Ok(if due { Watched::Kill } else { Watched::Goes })
  }
}

/// What a life did and what it cost, read off the record that it kept: a file of lines, each a
/// fact of the record and its time, as the store of furb writes it. Its names are the keys of the
/// file of the numbers of a run.
#[derive(Default, Serialize)]
pub(crate) struct Numbers {
  facts: u64,
  pub(crate) answers: u64,
  rungs: u64,
  prompts: u64,
  commands: u64,
  reads: u64,
  writes: u64,
  input: u64,
  output: u64,
  cache_read: u64,
  pub(crate) usd: f64,
}

impl Numbers {
  pub(crate) fn of(record: &Path) -> Result<Self, Fault> {
    Ok(Self::of_entries(&entries(record)?))
  }

  pub(crate) fn of_entries(entries: &[Value]) -> Self {
    let mut numbers = Self::default();
    for entry in entries {
      numbers.facts += 1;
      let fact = &entry[0];
      match fact[0].as_str().unwrap_or_default() {
        // The record keeps a rung that the operator wrote, and of a rung that a prompt made, its
        // reply.
        "rung" | "reply" => numbers.rungs += 1,
        "prompt" => numbers.prompts += 1,
        "bash" => numbers.commands += 1,
        "read" => numbers.reads += 1,
        "write" => numbers.writes += 1,
        "done" if replied(fact) => numbers.answered(&fact[3]),
        _ => {}
      }
    }
    numbers
  }

  /// One answer of a model: the done of a reply that came to a turn, with what it cost.
  fn answered(&mut self, turn: &Value) {
    let Some(turn) = items(turn) else { return };
    self.answers += 1;
    let usage = turn.get(2).and_then(items).unwrap_or_default();
    let count = |at: usize| usage.get(at).map_or(0.0, number);
    #[expect(
      clippy::cast_possible_truncation,
      clippy::cast_sign_loss,
      reason = "a count of tokens is a whole number"
    )]
    let tokens = |at: usize| count(at) as u64;
    (self.input, self.output, self.cache_read) =
      (self.input + tokens(0), self.output + tokens(1), self.cache_read + tokens(2));
    self.usd += count(4);
  }
}

/// Each entry of a record, a fact and its time, as the store of furb writes it, and none when the
/// record is not there yet.
pub(crate) fn entries(record: &Path) -> Result<Vec<Value>, Fault> {
  if !record.is_file() {
    return Ok(Vec::new());
  }
  let text = String::from_utf8_lossy(&read(record)?).into_owned();
  let parsed = |line: &str| {
    serde_json::from_str(line)
      .map_err(|source| Fault::Json { what: record.display().to_string(), source })
  };
  text.lines().filter(|line| !line.trim().is_empty()).map(parsed).collect()
}

/// Whether a fact is the done of a reply, by the name of the act that it is about.
fn replied(fact: &Value) -> bool {
  fact[1]
    .as_str()
    .and_then(|about| about.strip_prefix("reply"))
    .is_some_and(|n| !n.is_empty() && n.bytes().all(|one| one.is_ascii_digit()))
}

/// The items of a tuple or of a list, as the store writes them: a tuple is a mark `is` with its
/// args.
fn items(value: &Value) -> Option<Vec<Value>> {
  match value {
    Value::Array(items) => Some(items.clone()),
    Value::Object(_) if value["is"] == "tuple" => value["args"][0].as_array().cloned(),
    _ => None,
  }
}

/// A number as the store writes it: a float is a mark `is` with its text.
fn number(value: &Value) -> f64 {
  value
    .as_f64()
    .or_else(|| value["args"][0].as_str().and_then(|one| one.parse().ok()))
    .unwrap_or_default()
}

/// The actor that the life stood on last, from the answer of its last stand.
fn stood(record: &Path) -> Option<String> {
  let text = String::from_utf8_lossy(&read(record).ok()?).into_owned();
  text.lines().rev().filter_map(|line| serde_json::from_str::<Value>(line).ok()).find_map(|entry| {
    let fact = &entry[0];
    let stand =
      fact[0] == "done" && fact[1].as_str().is_some_and(|about| about.starts_with("stand"));
    stand.then(|| items(&fact[3])?.get(2)?.as_str().map(str::to_owned)).flatten()
  })
}

/// The numbers of a run in numbers.json: what its life did and cost, the value that it closed
/// with last, its seconds, and what its kills came to.
fn numbered(life: &Life, mark: &Mark, value: Value, task: &str) -> Result<(), Fault> {
  let numbers = Numbers::of(&life.record)?;
  let seconds = (life.began.elapsed().as_secs_f64() * 10.0).round() / 10.0;
  say!(
    "answers={} commands={} reads={} writes={} | ${:.4} | {seconds}s",
    numbers.answers,
    numbers.commands,
    numbers.reads,
    numbers.writes,
    numbers.usd
  );
  let json = |what: &'static str| move |source| Fault::Json { what: what.to_owned(), source };
  let mut kept = serde_json::to_value(&numbers).map_err(json("the numbers of a run"))?;
  (kept["value"], kept["wall_seconds"], kept["root"]) = (value, json!(seconds), json!(task));
  if let Some(crash) = &mark.crash {
    kept["crash"] = serde_json::to_value(crash.kept(&life.runs)?).map_err(json("the kills"))?;
  }
  written(&life.runs.join("numbers.json"), pretty(&kept)?.as_bytes())
}

/// The instruction of a task as the task wrote it, which the rig asks with `ASKED`.
fn instruction(seeded: &Seeded) -> Result<String, Fault> {
  let told = read(&seeded.task.dir.join("instruction.md"))?;
  Ok(String::from_utf8_lossy(&told).trim_end().to_owned())
}

/// The skill compaction turned on in the local config of a checkout, with the other settings kept:
/// a task can fill the window of a model, and the chain of the life compacts before it does.
fn compacted(app: &Path) -> Result<(), Fault> {
  let config = app.join(".furb").join("config.local.json");
  let mut held = if config.is_file() {
    serde_json::from_slice(&read(&config)?)
      .map_err(|source| Fault::Json { what: config.display().to_string(), source })?
  } else {
    json!({})
  };
  let skills = held
    .as_object_mut()
    .ok_or_else(|| Fault::Config(config.clone()))?
    .entry("skills")
    .or_insert_with(|| json!({}));
  skills
    .as_object_mut()
    .ok_or_else(|| Fault::Config(config.clone()))?
    .insert("compaction".to_owned(), Value::Bool(true));
  written(&config, format!("{}\n", pretty(&held)?).as_bytes())
}

fn pretty(value: &Value) -> Result<String, Fault> {
  serde_json::to_string_pretty(value)
    .map_err(|source| Fault::Json { what: "a result of the rig".to_owned(), source })
}

/// A command started, with its stdin closed, since no step of the rig is answered.
fn spawned(command: &mut Command) -> Result<Child, Fault> {
  command.spawn().map_err(|source| fault(command, source))
}

fn fault(command: &Command, source: io::Error) -> Fault {
  Fault::Spawn { program: command.get_program().to_string_lossy().into_owned(), source }
}

fn shown(command: &Command) -> String {
  let words = command.get_args().map(OsStr::to_string_lossy);
  [command.get_program().to_string_lossy()].into_iter().chain(words).collect::<Vec<_>>().join(" ")
}

/// One step of the rig, bounded by `STEP`, and whether it ended with 0.
fn ran(command: &mut Command) -> Result<bool, Fault> {
  let mut child = spawned(command)?;
  if let Some(status) = child.wait_timeout(STEP).map_err(|source| fault(command, source))? {
    return Ok(status.success());
  }
  ended(&mut child).map_err(|source| fault(command, source))?;
  let seconds = STEP.as_secs();
  say!("the step gave nothing in {seconds} seconds and was ended: {}", short(&shown(command), 80));
  Ok(false)
}

/// One short step whose answer is the point, out of the log of the operator.
fn captured(command: &mut Command) -> Result<Output, Fault> {
  command.stdin(Stdio::null()).output().map_err(|source| fault(command, source))
}

/// What one short step said, as text, and nothing when it failed.
fn spoke(command: &mut Command) -> Result<String, Fault> {
  let done = captured(command)?;
  let said = String::from_utf8_lossy(&done.stdout).trim().to_owned();
  Ok(if done.status.success() { said } else { String::new() })
}

/// A command in a process group of its own, so its end ends each process that it started.
fn grouped(command: &mut Command) {
  #[cfg(unix)]
  {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
  }
}

/// The end of a child and of its group, when it has one.
fn ended(child: &mut Child) -> io::Result<()> {
  if cfg!(unix) {
    Command::new("kill").args(["-KILL", &format!("-{}", child.id())]).status()?;
  }
  match child.kill() {
    Ok(()) => {}
    Err(no) if no.kind() == io::ErrorKind::InvalidInput => {}
    Err(no) => return Err(no),
  }
  child.wait().map(drop)
}

fn read(path: &Path) -> Result<Vec<u8>, Fault> {
  fs::read(path).map_err(|source| Fault::File { path: path.to_owned(), source })
}

fn written(path: &Path, bytes: &[u8]) -> Result<(), Fault> {
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).map_err(|source| Fault::File { path: parent.to_owned(), source })?;
  }
  fs::write(path, bytes).map_err(|source| Fault::File { path: path.to_owned(), source })
}

/// A file or a folder removed, and nothing when it is not there.
pub(crate) fn removed(path: &Path) -> Result<(), Fault> {
  let gone = match fs::symlink_metadata(path) {
    Err(no) if no.kind() == io::ErrorKind::NotFound => return Ok(()),
    Ok(held) if held.is_dir() => fs::remove_dir_all(path),
    _ => fs::remove_file(path),
  };
  gone.map_err(|source| Fault::File { path: path.to_owned(), source })
}

/// The program furb of this tree, built: each life that a task drives is one, on the engine that a
/// user runs.
pub(crate) fn furb(root: &Path) -> Result<PathBuf, Fault> {
  let mut built = Command::new("cargo");
  built.args(["build", "--quiet", "--package", "furb-cli"]).current_dir(root);
  if !ran(&mut built)? {
    return Err(Fault::Unbuilt);
  }
  let told = captured(
    Command::new("cargo")
      .args(["metadata", "--format-version", "1", "--no-deps"])
      .current_dir(root),
  )?;
  let metadata: Value = serde_json::from_slice(&told.stdout)
    .map_err(|source| Fault::Json { what: "cargo metadata".to_owned(), source })?;
  let target =
    metadata["target_directory"].as_str().map_or_else(|| root.join("target"), PathBuf::from);
  Ok(target.join("debug").join(format!("furb{}", env::consts::EXE_SUFFIX)))
}

#[cfg(test)]
#[path = "deepswe.test.rs"]
mod test;
