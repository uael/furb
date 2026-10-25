//! The examples of Rust in docs/examples/rust, each run as a user runs it, on a claude command line that answers from
//! a script, so no test asks a model. `cargo test` builds the examples before it runs this test.
#![cfg(unix)]

use std::{
  fs,
  io::Write,
  os::unix::fs::PermissionsExt,
  path::PathBuf,
  process::{Command, Stdio},
  thread,
  time::{Duration, Instant},
};

/// The time an example may take, since a scripted life that waits for ever would hold the run.
const BOUND: Duration = Duration::from_secs(60);

/// The fake claude of the tests of the crate.
const FAKE: &str = include_str!("../src/world/provider/claude/fake-claude.sh");

/// The directory an example runs in, and the fake claude beside it.
struct Yard {
  at: PathBuf,
  fake: PathBuf,
}

impl Yard {
  fn new(name: &str, words: &[&str]) -> Yard {
    let base = std::env::temp_dir().join(format!("furb-example-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).expect("the root of the yard");
    let root = fs::canonicalize(&base).expect("the root of the yard, with no link");
    let (at, fake) = (root.join("work"), root.join("fake"));
    fs::create_dir_all(&at).expect("the yard");
    fs::create_dir_all(&fake).expect("the home of the fake");
    fs::write(fake.join("claude"), FAKE).expect("the fake");
    fs::set_permissions(fake.join("claude"), fs::Permissions::from_mode(0o755))
      .expect("the fake runs");
    let yard = Yard { at, fake };
    yard.words(words);
    yard
  }

  fn words(&self, words: &[&str]) {
    for (at, word) in words.iter().enumerate() {
      let said = serde_json::to_string(word).expect("a word as JSON");
      fs::write(self.fake.join(format!("word.{}", at + 1)), said).expect("the word");
    }
  }

  /// The example, as a process in the yard, on the fake claude and on the config and the cache of the yard.
  fn command(&self, name: &str) -> Command {
    let here = std::env::current_exe().expect("this test");
    let example =
      here.parent().and_then(|deps| deps.parent()).expect("the target").join("examples").join(name);
    let mut command = Command::new(example);
    command.current_dir(&self.at).env("FURB_CLAUDE_BIN", self.fake.join("claude"));
    for name in ["FURB_LOCAL_URL", "FURB_LOCAL_KEY", "FURB_TUI"] {
      command.env_remove(name);
    }
    let root = self.fake.parent().expect("the root of the yard");
    command.env("FURB_CONFIG_DIR", root.join("config")).env("XDG_CACHE_HOME", root.join("cache"));
    command
  }

  /// What the example printed, once it ended well.
  fn run(&self, name: &str, stdin: &str) -> String {
    let mut command = self.command(name);
    command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().expect("the example starts");
    child
      .stdin
      .take()
      .expect("its stdin")
      .write_all(stdin.as_bytes())
      .expect("its stdin is written");
    let started = Instant::now();
    while child.try_wait().expect("the example").is_none() {
      if started.elapsed() > BOUND {
        child.kill().expect("the example ends");
        panic!("{name} ran for more than {BOUND:?}");
      }
      thread::sleep(Duration::from_millis(20));
    }
    let out = child.wait_with_output().expect("the example ends");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout).expect("its stdout")
  }
}

#[test]
fn a_life_goes_on_in_a_second_process_after_its_first_process_ends() {
  let yard = Yard::new("durable", &[]);
  fs::write(yard.fake.join("deaf"), "").expect("the fake is deaf");
  let mut first = yard.command("durable").stdout(Stdio::null()).spawn().expect("the first process");
  let started = Instant::now();
  while !yard.fake.join("pids").exists() {
    assert!(first.try_wait().expect("the first process").is_none(), "the first process ended");
    assert!(started.elapsed() < BOUND, "the first process asked no model");
    thread::sleep(Duration::from_millis(50));
  }
  first.kill().expect("the first process ends");
  first.wait().expect("the first process is gone");
  fs::remove_file(yard.fake.join("deaf")).expect("the fake hears");
  yard.words(&["close(\"Ship version 2 on Friday.\")"]);
  assert_eq!(yard.run("durable", ""), "Ship version 2 on Friday.\n");
  let record = fs::read_to_string(yard.at.join("release.jsonl")).expect("the record");
  assert_eq!(record.lines().filter(|one| one.starts_with("[[\"prompt\"")).count(), 1);
}

#[test]
fn a_reviewer_on_a_chain_of_its_own_reviews_the_draft_of_the_root() {
  let yard =
    Yard::new("chains", &["close(\"Login is twice as fast.\")", "close(\"The notes read well.\")"]);
  assert_eq!(yard.run("chains", ""), "The notes read well.\nreviewer under root\n");
}

#[test]
fn the_model_asks_the_operator_before_it_deploys() {
  let word = "go = await prompt(bool, \"Deploy version 2 now?\", to=OPERATOR)\nclose(\"deployed\" if go else \"held\")";
  let yard = Yard::new("questions", &[word]);
  assert_eq!(yard.run("questions", "yes\n"), "Deploy version 2 now? deployed\n");
}

#[test]
fn a_grant_pauses_the_chain_at_its_ceiling_and_a_larger_grant_lets_it_go_on() {
  let yard = Yard::new("grant", &["files = 3", "files = 3", "close(1200)"]);
  fs::write(yard.fake.join("cost"), "0.3").expect("the cost of a turn");
  assert_eq!(yard.run("grant", ""), "paused at 0.60 dollars\n1200\n");
}

#[test]
fn an_edit_of_the_door_of_a_thread_changes_the_plan_of_the_model() {
  let yard = Yard::new("door", &["await wait(3600)\nclose(\"released\")", "await wait(3600)"]);
  assert_eq!(yard.run("door", ""), "await wait(3600)\nclose(\"released\")\nreleased\n");
}
