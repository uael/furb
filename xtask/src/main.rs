//! The tasks of a developer of furb, which `cargo xtask` runs from any folder of the tree.
//!
//! Each script of `script/` that runs on its own is a task too: its name is the name of its file,
//! and its help is the first sentence of its docstring, so a script says what it does in one place.
//! The tasks route to the tools that do the work (cargo, pre-commit, uv, bun), and hold no copy of
//! their commands; a task that no tool does, as `clean-caches`, does its work here.

mod caches;
mod watch;

use std::{
  ffi::OsStr,
  fs, io,
  path::{Path, PathBuf},
  process::{Command as Process, ExitCode, Stdio},
  thread,
  time::{Duration, Instant},
};

use clap::{
  Arg, ArgMatches, Command, CommandFactory, FromArgMatches, Parser, Subcommand, ValueEnum,
};

#[derive(Parser)]
#[command(
  name = "cargo xtask",
  bin_name = "cargo xtask",
  about = "The tasks of a developer of furb. A script of script/ is a task, whose --help is its own.",
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
  /// tests that failed, the owner of each, and the commits since the last green.
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
  #[error("the watch of the refs ended")]
  Unwatched,
}

/// A script of `script/` that runs on its own, as a task.
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

fn run() -> Result<ExitCode, Fault> {
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
  let task = Xtask::from_arg_matches(&matches).unwrap_or_else(|error| error.exit()).task;
  match task {
    Task::Check { base, all } => check(&root, (!all).then_some(base.as_str())),
    Task::Test { suite: Some(suite), words } => suite.run(&root, &words),
    Task::Test { suite: None, .. } => every(&root),
    Task::Watch { branch, tree, every, table, inbox, owners, pid, once } => {
      let every = Duration::from_secs(every * 60);
      watch::Watch { branch, tree, every, table, inbox, owners, pid, once }.run(&root)
    }
    Task::CleanCaches { scratch, uv } => caches::clean(&root, scratch, uv.as_deref()),
    Task::Search { words } => {
      search(&command, &words);
      Ok(ExitCode::SUCCESS)
    }
    Task::Frames { words } => spawn(
      &root,
      "cargo",
      &[&["run", "--quiet", "-p", "furb-frames", "--"], &strs(&words)[..]].concat(),
    ),
  }
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

/// Each script of `script/` and `script/bench/` that runs on its own: each shell script, and each
/// python script that calls its main, since the others are libraries that the scripts import.
fn scripts(root: &Path) -> Result<Vec<Script>, Fault> {
  let mut scripts = Vec::new();
  for (folder, prefix) in [("script", ""), ("script/bench", "bench-")] {
    let read = |path: &Path| {
      fs::read_to_string(path).map_err(|source| Fault::Read { path: path.to_owned(), source })
    };
    let entries = fs::read_dir(root.join(folder))
      .map_err(|source| Fault::Read { path: root.join(folder), source })?;
    for entry in entries {
      let path = entry.map_err(|source| Fault::Read { path: root.join(folder), source })?.path();
      let (Some(stem), Some(suffix)) = (path.file_stem(), path.extension()) else { continue };
      let python = suffix == "py";
      if !python && suffix != "sh" {
        continue;
      }
      let source = read(&path)?;
      if python && !source.contains("__main__") && !source.contains("sys.exit(main())") {
        continue;
      }
      scripts.push(Script {
        name: format!("{prefix}{}", stem.to_string_lossy().replace('_', "-")),
        about: about(&source),
        furb: python && source.lines().any(|line| imports_furb(line.trim_start())),
        path: path.strip_prefix(root).unwrap_or(&path).to_owned(),
      });
    }
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
  ["import furb", "from furb", "import real", "from real"]
    .iter()
    .any(|start| line.starts_with(start))
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
