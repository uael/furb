//! The loss of a machine while the window holds work on its way there: a message that it did not
//! take, and a word of python whose end the window awaits. The end of the runtime that serves a
//! daemon closes its peers on unix.

#![cfg(all(test, unix))]

use std::fs::File;

use furb_window::open;

use crate::{
  cards::{app, shows},
  daemon::{Daemon, PATIENCE},
  input::sent,
  open::{card, homed, message, record},
  thread::clicked,
};

#[test]
fn a_loss_gives_each_message_on_its_way_back_to_the_input_before_his_draft_and_a_new_hello_hears_the_machine_again()
 {
  let title = "remove the gate from the engine";
  let (app, host, again) = (app(), app(), app());
  let mut daemon = Daemon::bound(&host);
  daemon.project("furb");
  let go = daemon.holding();
  daemon.served();
  let mut peer = daemon.peer();
  peer.rested(&record(&daemon, 0), &message(title), None);
  open(&app, vec![daemon.machine("mac")], homed(&daemon));
  card(&app, title);
  // The life of the thread opens on a claude command line that waits, so his message waits too,
  // and he writes more.
  sent(&app, "and the seed?");
  app.paste("and the boot");
  shows(&app, "┃ and the boot");
  drop((peer, host));
  File::create(go).unwrap();
  let frame = shows(&app, "mac went away before it took your message.");
  assert!(frame.contains("┃ and the seed?") && frame.contains("┃ and the boot"), "{frame}");
  let (seed, boot) = (frame.find("┃ and the seed?"), frame.find("┃ and the boot"));
  assert!(seed < boot, "{frame}");
  daemon.back(&again);
  daemon.served();
  shows(&app, "This computer");
}

#[test]
fn the_end_of_a_word_is_asked_again_once_its_life_stands_after_a_loss() {
  let title = "remove the gate from the engine";
  let (app, host, again) = (app(), app(), app());
  let mut daemon = Daemon::bound(&host);
  daemon.project("furb");
  daemon.served();
  let mut peer = daemon.peer();
  peer.rested(&record(&daemon, 0), &message(title), None);
  open(&app, vec![daemon.machine("mac")], homed(&daemon));
  card(&app, title);
  shows(&app, "Two stages hold most");
  clicked(&app, " Python ", 0, 1);
  shows(&app, "┃ >>> A word of Python to run");
  // A command that runs until its life ends: the next life of the record closes it as cut.
  sent(&app, "await bash('sleep 600')");
  shows(&app, "  await bash('sleep 600')");
  drop((peer, host));
  // The word runs as long as the window has not heard its end.
  let away = shows(&app, "mac offline");
  assert!(away.contains("◐  await bash('sleep 600')"), "{away}");
  daemon.back(&again);
  daemon.served();
  shows(&app, "This computer");
  let ended = app.wait_until(PATIENCE, || !app.text().contains("◐  await bash('sleep 600')"));
  assert!(ended, "{}", app.text());
}
