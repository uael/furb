//! `furb prompt` on the record of a life that the daemon of the machine holds: the command line
//! says the message through the daemon, as a window says it, so the life hears it at once, and
//! prints what it came to.

#![cfg(test)]
#![cfg(unix)]
#![expect(
  clippy::disallowed_methods,
  reason = "the tests block on the runtime that tokio starts for them"
)]

use std::process::Output;

use furb_host::Event;
use tokio::time::timeout;

use crate::kills::{Daemon, PATIENCE, Window, Yard};

#[tokio::test]
async fn furb_says_a_message_to_a_life_that_the_daemon_holds_and_prints_what_it_came_to() {
  let yard = Yard::new();
  let _daemon = Daemon::started(&yard).await;
  let mut window = Window::attached(&yard).await;
  yard.words(&["close('heard')"]);
  let life = yard.life();
  let said = yard.furb(&["prompt", "--record", &life, "Are you there?"]).output();
  let Output { status, stdout, .. } = timeout(PATIENCE, said).await.expect("furb ends").unwrap();
  assert!(status.success(), "furb prompt ends well");
  assert_eq!(String::from_utf8_lossy(&stdout), "'heard'\n");
  let heard = |event: &Event| matches!(event, Event::Fact(fact) if fact[0] == "prompt" && fact[5] == "Are you there?");
  window.until(heard).await;
}

#[tokio::test]
async fn a_process_that_opens_the_record_of_a_held_life_is_refused_and_the_life_goes_on() {
  let yard = Yard::new();
  let _daemon = Daemon::started(&yard).await;
  let mut window = Window::attached(&yard).await;
  let life = yard.life();
  for words in [&["run", "--record", &life, "x = 1"][..], &["checkpoint", "--record", &life]] {
    let opened = yard.furb(words).output();
    let Output { status, .. } = timeout(PATIENCE, opened).await.expect("furb ends").unwrap();
    assert!(!status.success(), "furb {} takes no record that the daemon holds", words[0]);
  }
  window.ran("y = 2").await;
}
