//! The task `budget`, each run on limits, a load and a log of the guard of memory that the
//! environment gives it, as the task reads them.

use std::{fs, path::Path};
#[cfg(unix)]
use std::{
  process,
  time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use crate::{ended, xtask, yard::Yard};

/// The code of the task and what it printed, for a part and its command, in a yard whose table of
/// limits holds these lines, on a machine of this load.
#[cfg(unix)]
fn ran(
  yard: &Path,
  (limits, load): (&str, &str),
  part: &str,
  command: &[&str],
) -> (Option<i32>, String) {
  fs::write(yard.join("budgets.toml"), format!("[local]\n{limits}"))
    .unwrap_or_else(|no| panic!("the limits were not written: {no}"));
  let mut run = xtask(yard, [&["budget", part, "--"][..], command].concat());
  run
    .env("FURB_BUDGETS", yard.join("budgets.toml"))
    .env("FURB_BUDGET_LOAD", load)
    .env("FURB_WATCH_LOG", yard.join("watch.log"))
    .env_remove("CI");
  let (code, out, _) = ended(&mut run);
  (code, out)
}

/// A command that uses 1.2 s of CPU, just over the limit of 1 s of the tests.
#[cfg(unix)]
const BUSY: [&str; 3] = [
  "python3",
  "-c",
  "import time\nt = time.process_time()\nwhile time.process_time() - t < 1.2:\n  pass",
];

#[cfg(unix)]
#[test]
fn a_run_past_its_wall_limit_fails_and_names_its_slowest_tests_and_one_under_it_passes() {
  let yard = Yard::new(format!("xtask-budget-wall-{}", process::id()));
  let slow = "sleep 1.2; echo '        PASS [   1.200s] (1/2) furb a::slow'; \
              echo '        PASS [   0.100s] (2/2) furb a::quick'";
  let (code, said) = ran(&yard, ("cargo = 1\n", "0"), "cargo", &["sh", "-c", slow]);
  assert_eq!(code, Some(1), "{said}");
  let tail = said.split("Its slowest tests:").nth(1).unwrap_or_default();
  assert_eq!(tail, "\n     1.20 s  furb a::slow\n     0.10 s  furb a::quick\n");
  let (code, said) = ran(&yard, ("cargo = 30\n", "0"), "cargo", &["true"]);
  assert!(code == Some(0) && said.contains("of 30 s") && !said.contains("passed its"), "{said}");
  let (code, _) = ran(&yard, ("cargo = 30\n", "0"), "cargo", &["sh", "-c", "exit 3"]);
  assert_eq!(code, Some(3), "a command that fails gives its own code");
  let (code, said) = ran(&yard, ("", "0"), "cargo", &["sleep", "1.2"]);
  assert!(
    code == Some(0) && said.ends_with("with no limit yet\n"),
    "a part with no limit reports and never fails: {said}"
  );
}

#[cfg(unix)]
#[test]
fn a_loaded_machine_judges_a_run_past_its_wall_limit_by_its_cpu_time_and_a_quiet_one_by_its_wall() {
  let yard = Yard::new(format!("xtask-budget-load-{}", process::id()));
  let (wait, limits) = (["sleep", "1.2"], "pytest = 1\npytest_cpu = 30\n");
  let (code, said) = ran(&yard, (limits, "999"), "pytest", &wait);
  assert!(code == Some(0) && said.contains("its CPU time fits"), "{said}");
  let (code, said) = ran(&yard, (limits, "0"), "pytest", &wait);
  assert!(code == Some(1) && said.contains("passed its wall limit"), "{said}");
  for load in ["0", "999"] {
    let (code, said) = ran(&yard, ("pytest = 100\npytest_cpu = 1\n", load), "pytest", &BUSY);
    assert!(
      code == Some(1) && said.contains("of CPU, ") && said.contains("over its limit"),
      "{load}: {said}"
    );
  }
}

#[cfg(unix)]
#[test]
fn a_kill_is_named_as_one_of_the_guard_of_memory_only_from_a_line_of_its_log_of_this_run() {
  let yard = Yard::new(format!("xtask-budget-guard-{}", process::id()));
  let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs());
  let ours = yard.join("target/debug/deps/furb-3bfcea354560c7bc").to_string_lossy().into_owned();
  let killed = |line: String, said: &str| {
    fs::write(yard.join("watch.log"), line).unwrap_or_else(|no| panic!("the log: {no}"));
    ran(&yard, ("cargo = 100\n", "0"), "cargo", &["sh", "-c", &format!("echo '{said}'; exit 137")])
  };
  let (code, said) = killed(format!("{now} 4242 {ours} --test-threads 4\n"), "");
  assert_eq!(code, Some(137));
  let named = format!(
    "the guard of memory ended a test process of cargo, by its log {}:\n  \
     pid 4242: {ours} --test-threads 4",
    yard.join("watch.log").display()
  );
  assert!(said.contains(&named), "{said}");
  for (line, printed) in [
    (format!("{} 4242 {ours}\n", now - 3600), ""),
    (format!("{now} 4242 /elsewhere/target/debug/deps/x-1a\n"), ""),
    (String::new(), "(signal: 9, SIGKILL: kill)"),
    (format!("{now} 4242 bun test ./a.test.ts\n"), "bun test v1.3.14"),
  ] {
    let (code, said) = killed(line.clone(), printed);
    assert!(code != Some(0) && !said.contains("guard of memory"), "{line}: {said}");
  }
}

#[test]
fn the_hooks_and_the_gates_run_each_part_through_the_one_home_of_its_limits() {
  let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
  let read = |path: &str| {
    fs::read_to_string(root.join(path)).unwrap_or_else(|no| panic!("{path} cannot be read: {no}"))
  };
  let limits: toml::Table = toml::from_str(&read("crates/xtask/budgets.toml"))
    .unwrap_or_else(|no| panic!("the limits: {no}"));
  let pytest = |table: &str| limits[table]["pytest"].as_integer().unwrap_or_default();
  assert!(0 < pytest("local") && pytest("local") < pytest("ci"));
  assert!(limits["local"]["pytest_cpu"].as_integer().unwrap_or_default() > 0);
  let hooks = read(".pre-commit-config.yaml");
  for part in ["pytest", "cargo"] {
    assert!(hooks.contains(&format!("entry: python3 script/hooks/reach.py {part}")), "{part}");
  }
  assert!(
    read("script/hooks/reach.py").contains(r#"["cargo", "xtask", "budget", args[0], "--", *run]"#)
  );
  assert!(
    read(".github/workflows/system.yml").contains("cargo xtask budget pytest -- uv run pytest")
  );
}
