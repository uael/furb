//! The commands of a life, through the verb bash of the engine.

use std::{fs, os::unix::fs::PermissionsExt, path::Path, process, time::Duration};

use furb::{life::Opening, verbs};

use crate::{
  child::{child, here, ran},
  drive::{block_on, park},
  opened,
  yard::Yard,
};

#[test]
fn the_first_command_of_a_life_waits_for_no_read_of_the_path_of_the_login_shell() {
  if let Some(at) = child() {
    let opening = Opening { skills: Some(false), ..opened::opening(&at, false) };
    let mut engine = opening.boot::<String>([]).expect("the life opens").0;
    let root = engine.root().to_owned();
    let with = verbs::Bash { on: Some(root), ..Default::default() };
    let command = engine.bash("true", with).expect("a command is made");
    block_on(command).expect("the command ends");
    // The login shell holds its profile until the test lets it go, and its read ends only then.
    assert!(!at.join("ended").exists(), "the command ran before the read of the login shell");
    fs::write(at.join("go"), "").expect("the login shell is let go");
    let ended = (0..200).any(|_| {
      park(Duration::from_millis(50));
      at.join("ended").exists()
    });
    assert!(ended, "the read of the login shell went on past the command");
    return;
  }
  let at = Yard::new(format!("furb-bash-login-{}", process::id()));
  let shell = at.join("sh");
  let marks = |name: &str| at.join(name).display().to_string();
  let script = format!(
    "#!/bin/sh\nwhile [ ! -e '{}' ]; do sleep 0.05; done\ntouch '{}'\n\
     printf '<furb-path>%s</furb-path>' \"$PATH\"\n",
    marks("go"),
    marks("ended")
  );
  fs::write(&shell, script).expect("the login shell is written");
  fs::set_permissions(&shell, fs::Permissions::from_mode(0o755)).expect("the login shell runs");
  let shell = shell.display().to_string();
  ran(here!(), &at, &[("SHELL", shell.as_str())]);
  assert!(Path::new(&marks("ended")).exists(), "the login shell ended its profile");
}
