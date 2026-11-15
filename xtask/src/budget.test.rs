//! The tests of the limit of time of a part, each on a machine that the test gives.

use std::{fs, path::Path, process::id, time::SystemTime};

use super::{Machine, Verdict};
use crate::{root, yard::Yard};

/// A machine of this load whose table of limits holds these lines.
fn machine(yard: &Path, limits: &str, load: f64) -> Machine {
  fs::write(yard.join("budgets.toml"), format!("[local]\n{limits}")).unwrap();
  Machine {
    home: yard.join("budgets.toml"),
    table: "local",
    load,
    cores: 4.0,
    log: yard.join("watch.log"),
    folder: yard.to_owned(),
  }
}

fn sh(script: &str) -> Vec<String> {
  ["sh", "-c", script].map(str::to_owned).into()
}

/// A command that uses 1.2 s of CPU, just over the limit of 1 s of the tests.
fn busy() -> Vec<String> {
  [
    "python3",
    "-c",
    "import time\nt = time.process_time()\nwhile time.process_time() - t < 1.2:\n  pass",
  ]
  .map(str::to_owned)
  .into()
}

fn ran(machine: &Machine, part: &str, command: &[String]) -> (u8, String) {
  let Verdict { code, said } = machine.run(part, command).unwrap();
  (code, said.join("\n"))
}

#[cfg(unix)]
#[test]
fn a_run_past_its_wall_limit_fails_and_names_its_slowest_tests_and_one_under_it_passes() {
  let yard = Yard::new(format!("xtask-budget-wall-{}", id()));
  let slow = sh(
    "sleep 1.2; echo '        PASS [   1.200s] (1/2) furb a::slow'; echo '        PASS [   0.100s] (2/2) furb a::quick'",
  );
  let (code, said) = ran(&machine(&yard, "cargo = 1\n", 0.0), "cargo", &slow);
  assert_eq!(code, 1, "{said}");
  let tail = said.split("Its slowest tests:").nth(1).unwrap_or_default();
  assert_eq!(tail, "\n     1.20 s  furb a::slow\n     0.10 s  furb a::quick");
  let (code, said) = ran(&machine(&yard, "cargo = 30\n", 0.0), "cargo", &sh("true"));
  assert!(code == 0 && said.contains("of 30 s") && !said.contains("passed its"), "{said}");
  assert_eq!(
    ran(&machine(&yard, "cargo = 30\n", 0.0), "cargo", &sh("exit 3")).0,
    3,
    "a command that fails gives its own code"
  );
  let (code, said) = ran(&machine(&yard, "", 0.0), "cargo", &sh("sleep 1.2"));
  assert!(
    code == 0 && said.ends_with("with no limit yet"),
    "a part with no limit reports and never fails: {said}"
  );
}

#[cfg(unix)]
#[test]
fn a_loaded_machine_judges_a_run_past_its_wall_limit_by_its_cpu_time_and_a_quiet_one_by_its_wall() {
  let yard = Yard::new(format!("xtask-budget-load-{}", id()));
  let wait = sh("sleep 1.2");
  for load in [999.0, 6.0] {
    let (code, said) = ran(&machine(&yard, "pytest = 1\npytest_cpu = 30\n", load), "pytest", &wait);
    assert!(code == 0 && said.contains("its CPU time fits"), "{load}: {said}");
  }
  let (code, said) = ran(&machine(&yard, "pytest = 1\npytest_cpu = 30\n", 0.0), "pytest", &wait);
  assert!(code == 1 && said.contains("passed its wall limit"), "{said}");
  for load in [0.0, 999.0] {
    let (code, said) =
      ran(&machine(&yard, "pytest = 100\npytest_cpu = 1\n", load), "pytest", &busy());
    assert!(
      code == 1 && said.contains("of CPU, ") && said.contains("over its limit"),
      "{load}: {said}"
    );
  }
}

#[cfg(unix)]
#[test]
fn a_kill_is_named_as_one_of_the_guard_of_memory_only_from_a_line_of_its_log_of_this_run() {
  let yard = Yard::new(format!("xtask-budget-guard-{}", id()));
  let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
  let ours = yard.join("target/debug/deps/furb-3bfcea354560c7bc").to_string_lossy().into_owned();
  let machine = machine(&yard, "cargo = 100\n", 0.0);
  let killed = |line: String, said: &str| {
    fs::write(yard.join("watch.log"), line).unwrap();
    ran(&machine, "cargo", &sh(&format!("echo '{said}'; exit 137")))
  };
  let (code, said) = killed(format!("{now} 4242 {ours} --test-threads 4\n"), "");
  assert_eq!(code, 137);
  assert!(said.contains(&format!("the guard of memory ended a test process of cargo, by its log {}:\n  pid 4242: {ours} --test-threads 4", yard.join("watch.log").display())), "{said}");
  for (line, printed) in [
    (format!("{} 4242 {ours}\n", now - 3600), ""),
    (format!("{now} 4242 /elsewhere/target/debug/deps/x-1a\n"), ""),
    (String::new(), "(signal: 9, SIGKILL: kill)"),
    (format!("{now} 4242 bun test ./a.test.ts\n"), "bun test v1.3.14"),
  ] {
    let (code, said) = killed(line.clone(), printed);
    assert!(code != 0 && !said.contains("guard of memory"), "{line}: {said}");
  }
}

#[test]
fn the_hooks_and_the_gates_run_each_part_through_the_one_home_of_its_limits() {
  let read = |path: &str| fs::read_to_string(root().join(path)).unwrap();
  let limits: toml::Table = toml::from_str(&read("xtask/budgets.toml")).unwrap();
  let pytest = |table: &str| limits[table]["pytest"].as_integer().unwrap();
  assert!(0 < pytest("local") && pytest("local") < pytest("ci"));
  assert!(limits["local"]["pytest_cpu"].as_integer().unwrap() > 0);
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
