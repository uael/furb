//! The window and the program furb: a window whose machine has no daemon starts `furb host`.

#![cfg(test)]
#![expect(
  clippy::disallowed_methods,
  reason = "the test blocks on the runtime that tokio starts for it"
)]

use crate::yard::Yard;

use std::{process, time::Duration};

use furb_host::{
  protocol::{Answer, Frame, Hello, Kind, PROTOCOL, Request, Verb},
  transport::{Dialer, Local},
};
use furb_window::{Error, Machine};
use futures::{SinkExt, StreamExt};
use tokio::{
  process::Command,
  time::{sleep, timeout},
};
use tokio_util::codec::{Framed, LinesCodec};

/// How long the test waits for the daemon before it fails.
const PATIENCE: Duration = Duration::from_secs(60);

#[tokio::test]
async fn a_window_starts_the_daemon_of_its_machine_when_none_listens_and_no_daemon_outlives_it() {
  let yard = Yard::new(format!("fcw{}", process::id()));
  let folder = yard.join("furb").join("host");
  // The daemon reads no file of this machine, and ends with its last window.
  let mut served = Command::new(env!("CARGO_BIN_EXE_furb"));
  served.args(["host", "--grace", "0"]).env_remove("FURB_WINDOW");
  served.env("XDG_CACHE_HOME", &*yard).env("FURB_CONFIG_DIR", yard.join("config"));
  let machine = Machine { name: "test-machine".to_owned(), folder: folder.clone(), served };

  let stream = timeout(PATIENCE, machine.reach()).await.expect("the daemon listens in time");
  let mut lines = Framed::new(stream.expect("the window reaches its daemon"), LinesCodec::new());
  let hello = Hello { protocol: PROTOCOL, kind: Kind::Window, name: None, role: None };
  let request = Request { id: 0, verb: Verb::Hello(hello) };
  lines.send(serde_json::to_string(&request).unwrap()).await.unwrap();
  let welcome = timeout(PATIENCE, lines.next()).await.expect("the daemon answers in time");
  let welcome: Frame = serde_json::from_str(&welcome.unwrap().unwrap()).unwrap();
  assert!(matches!(welcome, Frame::Reply { id: 0, answer: Answer::Welcome { .. } }), "{welcome:?}");

  drop(lines);
  let local = Local::new(&folder);
  let gone = async {
    while local.dial().await.is_ok() {
      sleep(Duration::from_millis(25)).await;
    }
  };
  timeout(PATIENCE, gone).await.expect("the daemon ends once its last window left");
}

#[tokio::test]
async fn a_daemon_that_ends_before_it_listens_tells_the_window_the_last_line_that_it_said() {
  let yard = Yard::new(format!("fce{}", process::id()));
  let folder = yard.join("furb").join("host");
  // The command line refuses a grace that is no number, says why, and ends.
  let mut served = Command::new(env!("CARGO_BIN_EXE_furb"));
  served.args(["host", "--grace", "never"]).env_remove("FURB_WINDOW");
  let machine = Machine { name: "test-machine".to_owned(), folder, served };
  let ended = timeout(PATIENCE, machine.reach()).await.expect("the daemon ends in time");
  let Err(Error::Ended { status, said }) = ended else { panic!("the daemon listened") };
  assert!(!status.success() && !said.is_empty(), "{status}: {said}");
}
