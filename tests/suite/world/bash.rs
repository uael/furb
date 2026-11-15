//! The commands of a life, through the verb bash of the engine.

#![expect(clippy::expect_used, reason = "a helper of a test fails the test where it fails")]

use std::{fs, os::unix::fs::PermissionsExt, path::Path, process, thread, time::Duration};

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
