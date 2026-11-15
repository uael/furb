//! The tasks of a developer of furb, which `cargo xtask` runs from any folder of the tree.
//!
//! Each script of `script/` that runs on its own is a task too: its name is the name of its file,
//! and its help is the first sentence of its docstring, so a script says what it does in one place.
//! The tasks route to the tools that do the work (cargo, pre-commit, uv, bun), and hold no copy of
//! their commands.

use std::{
  fs, io,
  path::{Path, PathBuf},
  process::{Command as Process, ExitCode},
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
  /// Run one suite, or each suite in turn when none is named.
  Test {
    /// The suite to run.
    suite: Option<Suite>,
    /// The words that the runner of the suite takes, after `--`: for rust the arguments of cargo
    /// test (each crate when none is given), for python those of pytest, for typescript a filter of
    /// the files.
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
}

/// A suite of the repository, by the language that its tests are written in.
#[derive(Clone, Copy, ValueEnum)]
enum Suite {
  /// The tests of each crate of the workspace, with cargo test.
  Rust,
  /// The suite of the contract on both engines and the suites of the extensions, with pytest.
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
  let command = scripts.iter().fold(Xtask::command(), |command, script| {
    command.subcommand(
      Command::new(script.name.clone())
        .about(script.about.clone())
        .disable_help_flag(true)
        .arg(Arg::new("words").num_args(0..).trailing_var_arg(true).allow_hyphen_values(true)),
    )
  });
  let matches = command.get_matches();
  if let Some((name, words)) = matches.subcommand()
    && let Some(script) = scripts.iter().find(|script| script.name == name)
  {
    return script.run(&root, &words_of(words));
  }
  let task = Xtask::from_arg_matches(&matches).unwrap_or_else(|error| error.exit()).task;
  match task {
    Task::Check { base, all } => check(&root, (!all).then_some(base.as_str())),
    Task::Test { suite: Some(suite), words } => suite.run(&root, &words),
    Task::Test { suite: None, .. } => {
      for suite in Suite::value_variants() {
        let code = suite.run(&root, &[])?;
        if code != ExitCode::SUCCESS {
          return Ok(code);
        }
      }
      Ok(ExitCode::SUCCESS)
    }
    Task::Frames { words } => spawn(
      &root,
      "cargo",
      &[&["run", "--quiet", "-p", "furb-frames", "--"], &strs(&words)[..]].concat(),
    ),
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
  fn run(self, root: &Path, words: &[String]) -> Result<ExitCode, Fault> {
    let words = strs(words);
    match self {
      Self::Rust if words.is_empty() => spawn(root, "cargo", &["test", "--workspace"]),
      Self::Rust => spawn(root, "cargo", &[&["test"], &words[..]].concat()),
      Self::Python => spawn(root, "uv", &[&["run", "pytest", "-q"], &words[..]].concat()),
      Self::Typescript => spawn(root, "bun", &[&["run", "test"], &words[..]].concat()),
    }
  }
}

impl Script {
  fn run(&self, root: &Path, words: &[String]) -> Result<ExitCode, Fault> {
    let path = self.path.to_string_lossy();
    let words = strs(words);
    if self.path.extension().is_some_and(|extension| extension == "sh") {
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
      let (Some(stem), Some(extension)) = (path.file_stem(), path.extension()) else { continue };
      let python = extension == "py";
      if !python && extension != "sh" {
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
fn spawn(root: &Path, program: &str, args: &[&str]) -> Result<ExitCode, Fault> {
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
