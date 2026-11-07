//! The ear of the commands.

use super::Running;
#[cfg(unix)]
use std::io;
#[cfg(unix)]
use tokio::process;
use unsync::spsc;

#[test]
fn a_text_fed_to_a_command_that_reads_no_more_is_refused_and_its_close_is_not() {
  let (stdin, fed) = spsc::unbounded();
  let mut one = Running { stdin };
  assert!(one.feed(Some("one\n".to_owned())).is_ok());
  drop(fed);
  let refused = one.feed(Some("two\n".to_owned())).unwrap_err();
  assert_eq!(refused.message(), "the command reads no more");
  assert!(one.feed(None).is_ok());
}

#[test]
#[cfg(unix)]
#[expect(clippy::disallowed_methods, reason = "a test blocks on its runtime and fails by a panic")]
fn an_error_of_the_wait_on_the_shell_ends_the_watch_with_that_error_not_an_exit_with_no_code() {
  use super::{Cut, watch};
  use crate::ear::runtime;
  use crate::world::child::spawn;
  use std::future;

  let outcome = runtime().block_on(async {
    let mut command = process::Command::new("sh");
    command.arg("-c").arg("sleep 5");
    let (_child, mut group) = spawn(&mut command).unwrap();
    let shell = runtime().spawn(async { Err(io::Error::other("wait broke")) });
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
