//! The tasks of a developer of furb, which `cargo xtask` runs from any folder of the tree.
//!
//! Each script of `xtask/python/` and `script/` that runs on its own is a task too: its name is the
//! name of its file,
//! and its help is the first sentence of its docstring, so a script says what it does in one place.
//! The tasks route to the tools that do the work (cargo, pre-commit, uv, bun), and hold no copy of
//! their commands; a task that no tool does, as `clean-caches`, does its work here.

mod budget;
mod caches;
mod catalog;
mod deepswe;
mod jobs;
mod links;
mod lives;
mod loops;
mod memwatch;
mod opens;
mod paths;
mod promise;
mod prove;
mod targets;
mod tree;
mod unused;
mod watch;
mod width;
#[cfg(test)]
#[path = "../../tests/common/yard.rs"]
mod yard;

use std::{
  ffi::OsStr,
  fs, io,
  path::{Path, PathBuf},
  process::{Command as Process, ExitCode, Stdio},
  thread,
  time::{Duration, Instant},
};

#[cfg(unix)]
use rustix::process::setpriority_process;

use clap::{
  Arg, ArgMatches, Command, CommandFactory, FromArgMatches, Parser, Subcommand, ValueEnum,
};

#[derive(Parser)]
#[command(
  name = "cargo xtask",
  bin_name = "cargo xtask",
  about = "The tasks of a developer of furb. A script of xtask/python/ or script/ is a task, whose --help is its own.",
  arg_required_else_help = true
)]
struct Xtask {
  #[command(subcommand)]
  task: Task,
}

#[derive(Subcommand)]
enum Task {
  /// Run the checks that the changed files reach.
  ///
  /// The hooks of the stage manual run on each file that changed since the base, committed or not,
  /// and on each new file: the formats, the lints, and the tests of each crate and suite that a
  /// changed file reaches.
  Check {
    /// The ref that the change starts from: the files differ from the commit where HEAD left it.
    #[arg(long, default_value = "origin/main")]
    base: String,
    /// Check each file of the tree, not only the changed ones.
    #[arg(long, conflicts_with = "base")]
    all: bool,
  },
  /// Run one suite, or each suite at once when none is named, each into a log of its own.
  ///
  /// The suites share no file, so they run together, and the slowest alone sets the time of the
  /// run. Each log stands in target/xtask/<suite>.log, and a suite that fails prints its tail.
  Test {
    /// The suite to run.
    suite: Option<Suite>,
    /// The words that the runner of the suite takes, after `--`: for rust the arguments of cargo
    /// nextest run (each crate when none is given), for python those of pytest, for typescript a
    /// filter of the files.
    #[arg(last = true, requires = "suite")]
    words: Vec<String>,
  },
  /// Capture, replay, compare and probe the frames of the window, with the commands of furb-frames.
  #[command(disable_help_flag = true)]
  Frames {
    /// The command of furb-frames and its options; `cargo xtask frames --help` lists them.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    words: Vec<String>,
  },
  /// Remove what earlier runs left in the caches of the machine, and nothing else.
  ///
  /// A scratch directory of a record that holds nothing but its mark, and that no life holds by a
  /// lock on its mark, goes: a life makes it again from its record. The cache of uv is pruned,
  /// which removes each unpacked wheel and each editable build that no entry points to. The prune
  /// reads no config, since the project sets no-cache for uv, and a prune with it prunes an empty
  /// cache.
  CleanCaches {
    /// The folder of the scratch directories of the records, by default the one of this machine.
    #[arg(long, value_name = "DIR")]
    scratch: Option<PathBuf>,
    /// The cache of uv to prune, by default the one of this machine.
    #[arg(long, value_name = "DIR")]
    uv: Option<PathBuf>,
  },
  /// Check the head of a branch whole when it moved: the hooks of a commit on the files that
  /// changed, then each suite, then clippy and biome.
  ///
  /// The check runs in a tree of its own, detached at the head, at a low priority, and blocks
  /// nothing. A green writes one line in the table; a red writes one entry in the inbox, with the
  /// tests that failed, the owner of each, and the commits since the last green, or since the
  /// head checked before.
  Watch {
    /// The branch whose head is checked.
    #[arg(long, default_value = "w/integrate")]
    branch: String,
    /// The tree of the check, a worktree of this clone with no change of its own.
    #[arg(long, value_name = "DIR")]
    tree: PathBuf,
    /// The fewest minutes between the starts of two checks; the watch wakes when the branch
    /// moves.
    #[arg(long, default_value_t = 30)]
    every: u64,
    /// The table of the checks, one line each: epoch, head, green or red, seconds, failed tests.
    #[arg(long, value_name = "FILE")]
    table: PathBuf,
    /// The file where a red goes, as one entry.
    #[arg(long, value_name = "FILE")]
    inbox: PathBuf,
    /// A file of lines `<prefix of a test> <owner>`, which names the owner of each red test.
    #[arg(long, value_name = "FILE")]
    owners: Option<PathBuf>,
    /// The file that holds the pid of the watch, which a second watch on it refuses to replace.
    #[arg(long, value_name = "FILE")]
    pid: Option<PathBuf>,
    /// Check the head once when it moved, then end.
    #[arg(long)]
    once: bool,
  },
  /// Put one life of furb against one task of the DeepSWE collection, and grade what it left with
  /// the verifier of the task.
  ///
  /// The one law of the rig is that no rule of it reaches the model. The system prompt is the
  /// engine, and the life runs the extensions that the configs turn on, with compaction on in the
  /// checkout, so the chain compacts before it fills its window. The message is the instruction of
  /// the task as the task wrote it, and one line that asks the model to close with a float from 0
  /// to 1 for how sure it is that the work is done. Under 0.9, the operator says that the level is
  /// too low and asks the model to go on, on the same chain. When the model closes at 0.9 or more
  /// and its checkout holds no change against the base, the operator says once that nothing is
  /// submitted and asks it to go on. The model reads no other word.
  ///
  /// Before the first run, `tools` installs the reporters, and `validate <task>` grades the
  /// reference solution of a task once, before a run spends money on it. `turns <task>` shows what
  /// the model read, turn by turn, which is where a friction of the engine shows, and `traces/`
  /// keeps the record and the result of each graded run.
  Deepswe {
    #[command(flatten)]
    places: deepswe::Places,
    #[command(subcommand)]
    verb: deepswe::Verb,
  },
  /// Run one real life on a model, then a second life on its record, which the record answers.
  ///
  /// It runs `furb prompt` twice on one record in a folder of its own, under a grant of a dollar,
  /// so it spends one answer of one model: the model reads a file to answer, and the second life
  /// takes the answer from the record and asks no model.
  Smoke {
    /// The actor, as provider:model/effort; the default actor of furb when unsaid.
    #[arg(long)]
    to: Option<String>,
  },
  /// Run one real life that uses each part of its runtime, hold it to what it did, and take one
  /// prompt more on its record.
  ///
  /// The model reads the engine, writes and runs a module, opens a chain on its own and prompts
  /// it, asks the operator, who answers from a line, debugs, peeks and moves its directory. A word
  /// on the record then checks each of those, in a life that asks no model, and a last prompt asks
  /// the life how many items it gave back. It spends a few dollars at most.
  Play {
    /// The actor, as provider:model/effort; the default actor of furb when unsaid.
    #[arg(long)]
    to: Option<String>,
  },
  /// Compact one chain of a copy of a record with the skill compaction, and print the share of the
  /// window that the chain read before the cut and after it.
  ///
  /// The task copies the record into a folder of its own and never changes the record. A life on
  /// the copy plays the skill and condenses the chain, as `compacted()` does once: the model of the
  /// chain chooses what it keeps, and a prompt on the chain after the cut reads what stays.
  Compaction {
    /// The record, which stays as it is.
    record: PathBuf,
    /// The chain to compact, as chain3.
    chain: String,
    /// Stop at the cut, and ask no model on the chain after it.
    #[arg(long)]
    dry: bool,
  },
  /// Prove that a record replays inert: before its tip, a life that only inspects it puts no act
  /// on to the World, and no ear of the World says that it started one.
  ///
  /// The test of the corpus proves it on each record of tests/corpus. This task runs that proof on
  /// the record given, as an ignored test of the suite of the crate.
  Inert {
    /// The record, which stays as it is.
    record: PathBuf,
  },
  /// Fail on each path of the repository that a tool of a developer or a workflow names and that
  /// git does not hold, with its file and its line.
  ///
  /// It reads each file of script/, xtask/src/, xtask/python/ and .github/workflows/. A path whose
  /// first part is a folder of the repository stands when git holds it as a file or a folder, or
  /// ignores it as what a build makes. A path with a variable or a pattern in it is not checked,
  /// and a path after `$root/` is a path of the repository.
  Paths,
  /// Lint the workspace on Linux and Windows, which this machine does not build, so code under a
  /// cfg of those systems shows its warnings before the gates of CI fail on it.
  ///
  /// zig compiles the C of the dependencies for each system. It needs zig on PATH and the targets
  /// of rust: `rustup target add x86_64-unknown-linux-gnu x86_64-pc-windows-gnu`. A first run in a
  /// tree checks every crate for both systems, in minutes; a run after a change takes seconds.
  Targets,
  /// Fail on each comment line of Rust that a change adds past 100 characters, which rustfmt does
  /// not wrap.
  ///
  /// It reads the lines that the working tree adds to the base, and given files, as a hook gives
  /// the staged ones, those files alone. A line inside a string is no comment, a line of code is
  /// rustfmt's, and a line that the base holds is no debt of the change.
  CommentWidth {
    /// The ref that the change starts from; the merge base of the head with origin/main when
    /// unsaid.
    #[arg(long)]
    base: Option<String>,
    /// The files to read; each Rust file of the tree when none is given.
    files: Vec<PathBuf>,
  },
  /// Fail on each link of a README that a package carries that breaks on the page of its registry.
  ///
  /// The registry of a package shows its README and reads no relative link, so each link is
  /// absolute. It lists each package from its manifest and the README that it carries, and
  /// fails on a relative link and on an absolute link to a path of this repository that git does
  /// not hold. A link in code is no link.
  Links {
    /// Write each relative link that points at a file of the repository as its absolute link.
    #[arg(long)]
    fix: bool,
  },
  /// Run each proof that the page of the promise names, and fail when one is gone or does not pass.
  ///
  /// The page lists under its heading of the proofs each test that proves a part of the promise,
  /// as `file::name`, and a release publishes nothing while one fails. The proofs of python run in
  /// one pytest, and those of rust in one cargo test for each target that holds them.
  Promise {
    /// The page of the promise.
    #[arg(default_value = "docs/promise.md")]
    page: PathBuf,
  },
  /// List each exported name of the TypeScript side and each public item of the crates that no
  /// file but a test names, and fail when the list holds one.
  ///
  /// A name of TypeScript is used when a file that is no test, and is not its own, imports it
  /// through a path that leads to its file, through a barrel or not, or when the README of the
  /// package names it. An item of Rust is used when a file that is no test, and is not its own,
  /// names it, an associated item through its type or its method call. An item of the door is used
  /// when a source of the language of the host names it. A name that its own file names, and a
  /// name that the declaration of a used name of its file holds, are used too.
  Unused {
    /// List each helper that its own file names and a test names too, with those tests: the
    /// work of the law that a test reaches no file past its public door.
    #[arg(long)]
    tests: bool,
  },
  /// Run the command of a part of the suite, and fail when the run passes the limit of that part.
  ///
  /// The limits stand in xtask/budgets.toml: the table local for this machine and ci for a runner
  /// of the gates. A part has a limit of wall time, <part>, in seconds, and may have one of CPU
  /// time, <part>_cpu, that the command and its children use. A run past its wall limit while the
  /// load at its start was over the cores is judged by its CPU time, when the part has a limit of
  /// it. A run that fails names its slowest tests, and a command that fails gives its own code. A
  /// kill that the log of the guard of memory holds for this run is named as one of the guard.
  Budget {
    /// The part, as pytest or cargo.
    part: String,
    /// The command of the part, after `--`.
    #[arg(last = true, required = true)]
    command: Vec<String>,
  },
  /// Set the budget of time of each job of the gates from its last green runs on main, in
  /// .github/budgets.json, through gh.
  JobBudgets {
    /// How many green runs of main to measure.
    #[arg(long, default_value_t = 60)]
    runs: u32,
  },
  /// Make the snapshot of the catalog of models that the crate carries again, from models.dev and
  /// from the latest release of pi-ai.
  ///
  /// It writes src/world/provider/catalog.json, one model on each line, and the file of its
  /// sources beside it only when the snapshot or the version of pi-ai changed, so a run with no
  /// change in the data makes no change in the tree.
  Catalog,
  /// Open a copy of each record on a build of furb, with no model and no network, and say whether
  /// each opens.
  ///
  /// It opens each copy with `furb turns`, which makes the life again from the record and only
  /// inspects it, one record at a time: the records that --list names, one path on each line, or
  /// else those of tests/corpus and tests/replay. A line for each goes to the table: its path, its
  /// verdict (opens, refused:<code>, bound past --bound seconds, memory past 8 GB), its seconds,
  /// its most MB, and the first line of furb that it wrote. Given the table of a run before, it
  /// fails when a record that opened then does not open now.
  Opens(opens::Opens),
  /// Prove the crates of furb as a user will take them from crates.io, from the packed crates
  /// alone.
  ///
  /// It packs the crates of the fork of monty at the revision that Cargo.toml names, and furb-wire,
  /// furb, furb-host and furb-cli on them by their version, as patches of crates.io, then adds furb
  /// to a new crate that takes every other crate from crates.io with no lock of furb, runs the
  /// example chains there on a claude command line that answers from a script, and installs
  /// furb-cli. It takes about 5 minutes.
  ProveCrate {
    /// The folder of the proof, which keeps the packed crates in crates/ and its build in
    /// target/; by default one for each tree in the temporary folder, which stays, so only a
    /// first build is cold.
    folder: Option<PathBuf>,
  },
  /// Watch the memory beside the work, and end the largest test process while the memory is tight
  /// and the work can be the cause.
  ///
  /// The memory is tight on macOS at the warn level of the kernel or above, and on Linux when less
  /// than `FURB_WATCH_FREE` percent of it is available, 20. The work can be the cause when its
  /// largest test process holds `FURB_WATCH_MIN` KB, 4 GB, or the work holds `FURB_WATCH_LANE` KB,
  /// 8 GB; the work is each test process and each process that names a tree of the clone of this
  /// folder, less a daemon of furb. `FURB_WATCH_POLL` gives the seconds between two looks, 2, and
  /// `FURB_WATCH_AFTER` the seconds after an end, 30. Each end is a line of `FURB_WATCH_LOG`.
  Memwatch {
    /// Look one time, then end.
    #[arg(long)]
    once: bool,
  },
  /// The bench of a loop: furb that triages support tickets against the claude command line, on
  /// the same local server, scored on the totals of the server and the share of right labels.
  ///
  /// `make` writes the tickets of a seed with a known label each. The arms run alone on the server,
  /// which counts every client: `furb` asks one prompt per ticket, `furb-own` gives the life the
  /// task as one prompt so its model writes its own words, `furb-batch` asks one prompt per batch,
  /// and `claude` runs the claude command line through a wrapper that points it at the server.
  /// The arms read `FURB_LOCAL_URL` and `FURB_LOCAL_KEY`. `score` compares each label with the
  /// known one. It asks a model and spends what the server serves.
  BenchLoop(loops::Loop),
  /// Make, land and close a worktree of this clone.
  Tree {
    #[command(subcommand)]
    verb: tree::Verb,
  },
  /// Print each task and each option whose name or help holds each of the words, in any case.
  Search {
    #[arg(required = true)]
    words: Vec<String>,
  },
}

/// A suite of the repository, by the language that its tests are written in.
#[derive(Clone, Copy, ValueEnum)]
enum Suite {
  /// The tests of each crate of the workspace, with cargo-nextest.
  Rust,
  /// The suite of the contract on both engines and the suites of the skills, with pytest.
  Python,
  /// The tests of the TypeScript side, with the runner of `bun run test`.
  Typescript,
}

#[derive(Debug, thiserror::Error)]
enum Fault {
  #[error("{program} did not start: {source}")]
  Spawn { program: String, source: io::Error },
  #[error("{} cannot be read: {source}", path.display())]
  Read { path: PathBuf, source: io::Error },
  #[error("git {args} failed: {stderr}")]
  Git { args: String, stderr: String },
  #[error(
    "this machine has no folder of caches: name the folder of the scratch directories with --scratch"
  )]
  NoCache,
  #[error("{} has the name of the task {name}: move it into the task, or rename it", path.display())]
  Clash { name: String, path: PathBuf },
  #[error("the run of the suite {0} panicked")]
  Panicked(String),
  #[error("the watch {pid} runs already, as {} says", file.display())]
  Running { pid: u32, file: PathBuf },
  #[error("the watch of the refs failed: {0}")]
  Watch(#[from] notify::Error),
  #[error("{0}")]
  Unproven(String),
  #[error("{url} cannot be fetched: {stderr}")]
  Fetch { url: String, stderr: String },
  #[error("the watch of the refs ended")]
  Unwatched,
  #[error("the pattern of the paths failed: {0}")]
  Pattern(#[from] fancy_regex::Error),
  #[error(transparent)]
  Deepswe(#[from] deepswe::Fault),
  #[error("{} stays: {why}", path.display())]
  Kept { path: PathBuf, why: tree::Kept },
  #[error(
    "the head conflicts with {0}, so the rebase is aborted: rebase the tree there by hand, then land again"
  )]
  Conflict(String),
  #[error("{0} moved 5 times while the head landed: land again")]
  Moved(String),
}

/// A script of `xtask/python/` or `script/` that runs on its own, as a task.
struct Script {
  name: String,
  path: PathBuf,
  about: String,
  /// Whether it imports furb, whose build `uv run` makes again before it runs.
  furb: bool,
}

fn main() -> ExitCode {
  match run() {
    Ok(code) => code,
    Err(fault) => {
      eprintln!("cargo xtask: {fault}");
      ExitCode::FAILURE
    }
  }
}

/// Run the task at the lowest priority, which each program that it starts inherits: three agents
/// share one machine with its owner, whose window stays responsive while each build and each
/// runner of tests takes the width of the machine.
fn lowered() {
  #[cfg(unix)]
  if let Err(error) = setpriority_process(None, 19) {
    eprintln!("cargo xtask: the task keeps its priority: {error}");
  }
}

fn run() -> Result<ExitCode, Fault> {
  lowered();
  let root = root();
  let scripts = scripts(&root)?;
  let tasks = Xtask::command();
  if let Some(script) = scripts.iter().find(|script| tasks.find_subcommand(&script.name).is_some())
  {
    return Err(Fault::Clash { name: script.name.clone(), path: script.path.clone() });
  }
  let command = scripts.iter().fold(tasks, |command, script| {
    command.subcommand(
      Command::new(script.name.clone())
        .about(script.about.clone())
        .disable_help_flag(true)
        .arg(Arg::new("words").num_args(0..).trailing_var_arg(true).allow_hyphen_values(true)),
    )
  });
  let matches = command.clone().get_matches();
  if let Some((name, words)) = matches.subcommand()
    && let Some(script) = scripts.iter().find(|script| script.name == name)
  {
    return script.run(&root, &words_of(words));
  }
  Xtask::from_arg_matches(&matches).unwrap_or_else(|error| error.exit()).task.run(&root, &command)
}

impl Task {
  fn run(self, root: &Path, command: &Command) -> Result<ExitCode, Fault> {
    match self {
      Self::Check { base, all } => check(root, (!all).then_some(base.as_str())),
      Self::Test { suite: Some(suite), words } => suite.run(root, &words),
      Self::Test { suite: None, .. } => every(root),
      Self::Watch { branch, tree, every, table, inbox, owners, pid, once } => {
        let every = Duration::from_secs(every * 60);
        watch::Watch { branch, tree, every, table, inbox, owners, pid, once }.run(root)
      }
      Self::CleanCaches { scratch, uv } => caches::clean(root, scratch, uv.as_deref()),
      Self::Paths => paths::check(),
      Self::Tree { verb } => tree::run(root, verb),
      Self::Links { fix } => links::check(root, fix),
      Self::BenchLoop(bench) => bench.run(root),
      Self::Memwatch { once } => memwatch::watch(once),
      Self::ProveCrate { folder } => prove::crates(root, folder),
      Self::Opens(opens) => opens.run(root),
      Self::Catalog => catalog::refresh(root),
      Self::Budget { part, command } => budget::check(root, &part, &command),
      Self::JobBudgets { runs } => jobs::set(root, runs),
      Self::Unused { tests } => unused::check(root, tests),
      Self::Promise { page } => promise::check(root, &root.join(page)),
      Self::Targets => targets::lint(root),
      Self::CommentWidth { base, files } => width::check(root, base.as_deref(), &files),
      Self::Deepswe { places, verb } => Ok(deepswe::run(root, places, verb)?),
      Self::Smoke { to } => Ok(lives::smoke(root, to)?),
      Self::Play { to } => Ok(lives::play(root, to)?),
      Self::Compaction { record, chain, dry } => Ok(lives::compaction(root, &record, &chain, dry)?),
      Self::Inert { record } => inert(root, &record),
      Self::Search { words } => {
        search(command, &words);
        Ok(ExitCode::SUCCESS)
      }
      Self::Frames { words } => spawn(
        root,
        "cargo",
        &[&["run", "--quiet", "-p", "furb-frames", "--"], &strs(&words)[..]].concat(),
      ),
    }
  }
}

/// The proof that a record replays inert, the ignored test of the suite of the crate that reads the
/// record that `FURB_INERT` names.
fn inert(root: &Path, record: &Path) -> Result<ExitCode, Fault> {
  let record =
    fs::canonicalize(record).map_err(|source| Fault::Read { path: record.to_owned(), source })?;
  let test = "promise::the_record_that_furb_inert_names_replays_inert";
  let status = Process::new("cargo")
    .args(["nextest", "run", "--package", "furb", "--test", "suite", "--run-ignored", "only", test])
    .env("FURB_INERT", record)
    .current_dir(root)
    .status()
    .map_err(|source| Fault::Spawn { program: "cargo".to_owned(), source })?;
  Ok(if status.success() { ExitCode::SUCCESS } else { ExitCode::FAILURE })
}

/// Print each task and each option of a task whose name or help holds each word, as a line of its
/// usage and its help.
fn search(command: &Command, words: &[String]) {
  let words: Vec<String> = words.iter().map(|word| word.to_lowercase()).collect();
  let holds = |text: &str| {
    let text = text.to_lowercase();
    words.iter().all(|word| text.contains(word.as_str()))
  };
  for task in command.get_subcommands() {
    let about = task.get_about().map(ToString::to_string).unwrap_or_default();
    if holds(&format!("{} {about}", task.get_name())) {
      println!("cargo xtask {}: {about}", task.get_name());
    }
    for arg in task.get_arguments().filter(|arg| arg.get_id() != "help") {
      let help = arg.get_help().map(ToString::to_string).unwrap_or_default();
      let name =
        arg.get_long().map_or_else(|| arg.get_id().to_string(), |long| format!("--{long}"));
      if holds(&format!("{name} {help}")) {
        println!("cargo xtask {} {name}: {help}", task.get_name());
      }
    }
  }
}

/// Whether a file is of the language that its extension names.
fn written(file: &str, language: &str) -> bool {
  Path::new(file).extension().is_some_and(|one| one == language)
}

/// The root of the tree, which holds the workspace of this crate.
fn root() -> PathBuf {
  PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
}

fn check(root: &Path, base: Option<&str>) -> Result<ExitCode, Fault> {
  let hook = ["script/hooks/pre-commit", "run", "--hook-stage", "manual"];
  let Some(base) = base else {
    return spawn(root, "sh", &[&hook[..], &["--all-files"]].concat());
  };
  let start = git(root, &["merge-base", "HEAD", base])?;
  let mut files = git(root, &["diff", "--name-only", "--diff-filter=d", start.trim()])?;
  files.push_str(&git(root, &["ls-files", "--others", "--exclude-standard"])?);
  let files: Vec<&str> = files.lines().filter(|line| !line.is_empty()).collect();
  if files.is_empty() {
    println!("No file changed since {base}.");
    return Ok(ExitCode::SUCCESS);
  }
  spawn(root, "sh", &[&hook[..], &["--files"], &files[..]].concat())
}

impl Suite {
  /// The commands of the suite in order. Each runner builds what its tests read, but the tests of
  /// TypeScript read the package that `bun run build` makes, and fail on a build older than its
  /// sources.
  fn steps(self, words: &[String]) -> Vec<(&'static str, Vec<String>)> {
    let with = |head: &[&str], tail: &[String]| {
      head.iter().map(|one| (*one).to_owned()).chain(tail.iter().cloned()).collect()
    };
    let crates = if words.is_empty() { &["--workspace".to_owned()][..] } else { words };
    match self {
      Self::Rust => vec![("cargo", with(&["nextest", "run"], crates))],
      Self::Python => vec![("uv", with(&["run", "pytest", "-q"], words))],
      Self::Typescript => {
        vec![("bun", with(&["run", "build"], &[])), ("bun", with(&["run", "test"], words))]
      }
    }
  }

  fn run(self, root: &Path, words: &[String]) -> Result<ExitCode, Fault> {
    for (program, args) in self.steps(words) {
      let code = spawn(root, program, &args)?;
      if code != ExitCode::SUCCESS {
        return Ok(code);
      }
    }
    Ok(ExitCode::SUCCESS)
  }

  /// The run of the suite into a log, and whether it passed.
  fn logged(self, root: &Path, log: &Path) -> Result<bool, Fault> {
    let file =
      fs::File::create(log).map_err(|source| Fault::Read { path: log.to_owned(), source })?;
    for (program, args) in self.steps(&[]) {
      let out = |file: &fs::File| {
        file
          .try_clone()
          .map(Stdio::from)
          .map_err(|source| Fault::Read { path: log.to_owned(), source })
      };
      let status = Process::new(program)
        .args(&args)
        .current_dir(root)
        .stdout(out(&file)?)
        .stderr(out(&file)?)
        .status()
        .map_err(|source| Fault::Spawn { program: program.to_owned(), source })?;
      if !status.success() {
        return Ok(false);
      }
    }
    Ok(true)
  }

  fn name(self) -> String {
    self.to_possible_value().map(|one| one.get_name().to_owned()).unwrap_or_default()
  }
}

/// Each suite at once, each into its log, then one line for each, and the tail of the log of each
/// that failed.
fn every(root: &Path) -> Result<ExitCode, Fault> {
  let logs = root.join("target").join("xtask");
  fs::create_dir_all(&logs).map_err(|source| Fault::Read { path: logs.clone(), source })?;
  let began = Instant::now();
  let ran: Vec<(Suite, Result<bool, Fault>, u64)> = thread::scope(|scope| {
    let runs: Vec<_> = Suite::value_variants()
      .iter()
      .map(|&suite| {
        let log = logs.join(format!("{}.log", suite.name()));
        (suite, scope.spawn(move || (suite.logged(root, &log), began.elapsed().as_secs())))
      })
      .collect();
    runs
      .into_iter()
      .map(|(suite, run)| match run.join() {
        Ok((passed, seconds)) => (suite, passed, seconds),
        Err(_) => (suite, Err(Fault::Panicked(suite.name())), began.elapsed().as_secs()),
      })
      .collect()
  });
  let mut failed = false;
  for (suite, passed, seconds) in ran {
    let log = logs.join(format!("{}.log", suite.name()));
    if passed? {
      println!("{}: passed in {seconds} s", suite.name());
    } else {
      failed = true;
      let text = fs::read_to_string(&log).unwrap_or_default();
      let tail: Vec<&str> = text.lines().rev().take(40).collect();
      println!("{}: FAILED in {seconds} s, its log is {}:", suite.name(), log.display());
      tail.iter().rev().for_each(|line| println!("  {line}"));
    }
  }
  Ok(if failed { ExitCode::FAILURE } else { ExitCode::SUCCESS })
}

impl Script {
  fn run(&self, root: &Path, words: &[String]) -> Result<ExitCode, Fault> {
    let path = self.path.to_string_lossy();
    let words = strs(words);
    if self.path.extension().is_some_and(|suffix| suffix == "sh") {
      return spawn(root, "sh", &[&[path.as_ref()], &words[..]].concat());
    }
    // A script that imports furb runs on the build of this tree, which a plain `uv run` makes again
    // when a source changed; the others need only the packages of the environment, with no build.
    let runner: &[&str] =
      if self.furb { &["run", "python"] } else { &["run", "--no-sync", "python"] };
    spawn(root, "uv", &[runner, &[path.as_ref()], &words[..]].concat())
  }
}

/// Each script of `xtask/python/` and `script/` that runs on its own: each shell script, and each
/// python script that calls its main, since the others are libraries that the scripts import. The
/// python of a task whose truth only python reads, as an AST of python, stands in `xtask/python/`.
fn scripts(root: &Path) -> Result<Vec<Script>, Fault> {
  let mut scripts = Vec::new();
  let mut entries = Vec::new();
  for folder in [root.join("xtask/python"), root.join("script")] {
    let read =
      fs::read_dir(&folder).map_err(|source| Fault::Read { path: folder.clone(), source })?;
    for entry in read {
      entries.push(entry.map_err(|source| Fault::Read { path: folder.clone(), source })?.path());
    }
  }
  for path in entries {
    let (Some(stem), Some(suffix)) = (path.file_stem(), path.extension()) else { continue };
    let python = suffix == "py";
    if !python && suffix != "sh" {
      continue;
    }
    let source =
      fs::read_to_string(&path).map_err(|source| Fault::Read { path: path.clone(), source })?;
    if python && !source.contains("__main__") && !source.contains("sys.exit(main())") {
      continue;
    }
    scripts.push(Script {
      name: stem.to_string_lossy().replace('_', "-"),
      about: about(&source),
      furb: python && source.lines().any(|line| imports_furb(line.trim_start())),
      path: path.strip_prefix(root).unwrap_or(&path).to_owned(),
    });
  }
  scripts.sort_by(|one, other| one.name.cmp(&other.name));
  Ok(scripts)
}

/// The first sentence of the docstring of a python script, or of the first comment of a shell
/// script.
fn about(source: &str) -> String {
  let mut lines = source.lines().map(str::trim).skip_while(|line| !is_doc(line)).peekable();
  let shell = lines.peek().is_some_and(|line| line.starts_with('#'));
  let text = lines
    .take_while(|line| if shell { line.starts_with('#') } else { !line.is_empty() })
    .map(|line| line.trim_start_matches('#').trim_matches('"').trim())
    .collect::<Vec<_>>()
    .join(" ");
  let end =
    [". ", "? "].iter().filter_map(|stop| text.find(stop)).min().map_or(text.len(), |at| at + 1);
  text[..end].to_owned()
}

fn is_doc(line: &str) -> bool {
  line.starts_with("\"\"\"") || (line.starts_with("# ") && !line.starts_with("#!"))
}

fn imports_furb(line: &str) -> bool {
  ["import furb", "from furb"].iter().any(|start| line.starts_with(start))
}

fn words_of(matches: &ArgMatches) -> Vec<String> {
  matches.get_many::<String>("words").map(|words| words.cloned().collect()).unwrap_or_default()
}

fn strs(words: &[String]) -> Vec<&str> {
  words.iter().map(String::as_str).collect()
}

fn git(root: &Path, args: &[&str]) -> Result<String, Fault> {
  let output = Process::new("git")
    .args(args)
    .current_dir(root)
    .output()
    .map_err(|source| Fault::Spawn { program: "git".to_owned(), source })?;
  if !output.status.success() {
    return Err(Fault::Git {
      args: args.join(" "),
      stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
    });
  }
  Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Run a program in the root of the tree and give its exit code, so the task fails as the program
/// fails.
fn spawn(root: &Path, program: &str, args: &[impl AsRef<OsStr>]) -> Result<ExitCode, Fault> {
  let status = Process::new(program)
    .args(args)
    .current_dir(root)
    .status()
    .map_err(|source| Fault::Spawn { program: program.to_owned(), source })?;
  Ok(
    status
      .code()
      .and_then(|code| u8::try_from(code).ok())
      .map_or(ExitCode::FAILURE, ExitCode::from),
  )
}
