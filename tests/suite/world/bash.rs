//! The commands of a life, through the verb bash of the engine.

#![expect(clippy::expect_used, reason = "a helper of a test fails the test where it fails")]

use std::{env, fs, os::unix::fs::PermissionsExt, path::Path, process, thread, time::Duration};

use furb::{Exit, life::Opening, verbs};

use crate::{
  child::{child, here, ran},
  drive::{block_on, park},
  opened,
  yard::Yard,
};

#[test]
fn the_first_command_runs_on_the_path_of_the_login_shell_and_a_later_life_reads_it_no_more() {
  if let Some(at) = child() {
    // The login shell holds its profile until its read runs and a moment more, so a command that
    // did not wait for the read would run first, on the PATH of the process.
    let reads = at.join("reads");
    let go = at.join("go");
    let holds = thread::spawn(move || {
      while !reads.exists() {
        park(Duration::from_millis(10));
      }
      park(Duration::from_millis(300));
      fs::write(go, "").expect("the login shell is let go");
    });
    assert_eq!(probed(&at, "first"), "login\n", "the first command waits for the login PATH");
    holds.join().expect("the login shell is let go");
    assert_eq!(probed(&at, "second"), "login\n", "a later life runs on the login PATH");
    let reads = fs::read_to_string(at.join("reads")).expect("the login shell ran");
    assert_eq!(reads, "read\n", "the process read the login shell once");
    return;
  }
  let at = Yard::new(format!("furb-bash-login-{}", process::id()));
  let marks = |name: &str| at.join(name).display().to_string();
  let program = |path: &Path, text: String| {
    fs::write(path, text).expect("a program is written");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("a program runs");
  };
  fs::create_dir_all(at.join("bin")).expect("the folder of the profile is made");
  program(&at.join("bin").join("furb-login-probe"), "#!/bin/sh\necho login\n".to_owned());
  let shell = at.join("sh");
  let script = format!(
    "#!/bin/sh\necho read >> '{}'\nwhile [ ! -e '{}' ]; do sleep 0.05; done\n\
     printf '<furb-path>%s:%s</furb-path>' '{}' \"$PATH\"\n",
    marks("reads"),
    marks("go"),
    marks("bin")
  );
  program(&shell, script);
  let shell = shell.display().to_string();
  ran(here!(), &at, &[("SHELL", shell.as_str())]);
}

/// What the program that only the login PATH holds says, from the first command of a new life in
/// the folder `name` of the yard.
fn probed(at: &Path, name: &str) -> String {
  let at = at.join(name);
  fs::create_dir_all(&at).expect("the folder of the life is made");
  let opening = Opening { skills: Some(false), ..opened::opening(&at, false) };
  let mut engine = opening.boot::<String>([]).expect("the life opens").0;
  let with = verbs::Bash { on: Some(engine.root().to_owned()), ..Default::default() };
  let exit: Exit = block_on(engine.bash("furb-login-probe", with).expect("a command is made"))
    .expect("the command ends");
  exit.stdout.content
}

/// The PATH of the process of each child of a test of the login shell, which no folder of it
/// repeats in order with the folders that the login shell says.
const CURRENT: &str = "/usr/bin:/bin:/opt/login";

/// The variable that names to the child the PATH that its first command must run on.
const WANT: &str = "FURB_TEST_WANT";

/// The test at this path, run again as a child whose login shell runs `body`, which leaves the
/// `left` files of an earlier process of its id, and whose first command must run on `want`.
fn on_login(test: &str, name: &str, body: &str, want: &str) {
  if let Some(at) = child() {
    let left: Vec<_> = (0..2)
      .map(|at| env::temp_dir().join(format!("furb-login-path-{}-{at}", process::id())))
      .filter(|_| name == "left")
      .collect();
    for one in &left {
      fs::write(one, "").expect("a file of an earlier process is left");
    }
    let said = path_of(&at);
    for one in &left {
      fs::remove_file(one).expect("the file of an earlier process goes");
    }
    // A process that the profile left in the background waits for this file.
    fs::write(at.join("release"), "").expect("the process of the profile is let go");
    assert_eq!(said, env::var(WANT).expect("the PATH to run on"));
    return;
  }
  let at = Yard::new(format!("furb-bash-{name}-{}", process::id()));
  let shell = at.join("sh");
  let body = body.replace("{release}", &at.join("release").display().to_string());
  fs::write(&shell, format!("#!/bin/sh\n{body}\n")).expect("the login shell is written");
  fs::set_permissions(&shell, fs::Permissions::from_mode(0o755)).expect("the login shell runs");
  let shell = shell.display().to_string();
  let shell = if name == "missing" { "/no/such/shell" } else { shell.as_str() };
  ran(test, &at, &[("SHELL", shell), ("PATH", CURRENT), (WANT, want)]);
}

/// The PATH that the first command of a new life in the yard runs on.
fn path_of(at: &Path) -> String {
  let opening = Opening { skills: Some(false), ..opened::opening(at, false) };
  let mut engine = opening.boot::<String>([]).expect("the life opens").0;
  let with = verbs::Bash { on: Some(engine.root().to_owned()), ..Default::default() };
  let exit: Exit = block_on(engine.bash("printf %s \"$PATH\"", with).expect("a command is made"))
    .expect("the command ends");
  exit.stdout.content
}

#[test]
fn the_path_of_the_login_shell_is_read_between_its_marks_and_its_folders_come_first_with_no_repeat()
{
  let body = "echo noise of a profile; printf '<furb-path>/opt/homebrew/bin:/usr/bin</furb-path>'";
  on_login(here!(), "marks", body, "/opt/homebrew/bin:/usr/bin:/bin:/opt/login");
}

#[test]
fn the_path_of_a_login_shell_whose_profile_leaves_a_process_on_its_output_is_read_when_the_shell_ends()
 {
  let body = "(n=0; while [ ! -e '{release}' ] && [ $n -lt 300 ]; do sleep 0.1; n=$((n+1)); done) & \
              printf '<furb-path>/a:/b</furb-path>'";
  on_login(here!(), "held", body, "/a:/b:/usr/bin:/bin:/opt/login");
}

#[test]
fn the_path_of_the_login_shell_is_read_past_a_file_that_an_earlier_process_of_the_same_id_left() {
  on_login(
    here!(),
    "left",
    "printf '<furb-path>/c:/d</furb-path>'",
    "/c:/d:/usr/bin:/bin:/opt/login",
  );
}

#[test]
fn a_login_shell_that_fails_or_gives_no_path_leaves_the_path_of_the_process() {
  let test = here!();
  for (name, body) in [
    ("fails", "printf '<furb-path>/a</furb-path>'; exit 1"),
    ("empty", "printf '<furb-path></furb-path>'"),
    ("silent", "true"),
    ("missing", "true"),
  ] {
    on_login(test, name, body, CURRENT);
  }
}
