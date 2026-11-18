//! An error of the wait on the shell of a command, where no door reaches it: the wait on a child
//! that runs fails on no machine. The suite proves the rest of the ear through the commands of a
//! life.

#[cfg(unix)]
use super::{Cut, watch};
#[cfg(unix)]
use crate::{ear::runtime, world::child::spawn};
#[cfg(unix)]
use std::{future, io};
#[cfg(unix)]
use tokio::process;

#[test]
#[cfg(unix)]
#[expect(clippy::disallowed_methods, reason = "a test blocks on its runtime and fails by a panic")]
fn an_error_of_the_wait_on_the_shell_ends_the_watch_with_that_error_not_an_exit_with_no_code() {
  let outcome = runtime().unwrap().block_on(async {
    let mut command = process::Command::new("sh");
    command.arg("-c").arg("sleep 5");
    let (_child, mut group) = spawn(&mut command).unwrap();
    let shell = async { Err(io::Error::other("wait broke")) };
    let got = watch(
      shell,
      future::pending::<Result<(), super::Unread>>(),
      future::pending(),
      future::pending(),
      &mut group,
    )
    .await;
    drop(group);
    got
  });
  match outcome {
    Err(Cut::Unwaited(no)) => assert_eq!(no.to_string(), "wait broke"),
    Err(_) | Ok(_) => panic!("an error of the wait must end the watch as Unwaited"),
  }
}
