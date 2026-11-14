use opentui::{prelude::*, testing::TestApp};

use super::Spinner;
use crate::tokens::{Loader, Tokens};

#[test]
fn a_loader_shows_each_frame_of_its_mark_for_one_step_in_one_cell() {
  for loader in [Loader::Model, Loader::Command] {
    let app = TestApp::new(4, 1);
    let spinner = Spinner::new(&app, loader, Tokens::on(None).accent);
    app.root().add(&spinner);
    let frames = loader.frames();
    for step in 0..=frames.len() {
      assert_eq!(app.text(), frames[step % frames.len()]);
      app.advance(loader.step());
    }
  }
}

#[test]
fn a_hidden_loader_asks_no_frame() {
  let app = TestApp::new(4, 1);
  let spinner = Spinner::new(&app, Loader::Model, Tokens::on(None).accent);
  app.root().add(&spinner);
  app.render();
  assert_eq!(app.idle(Loader::Model.step() * 10).frames, 10);
  spinner.visible(false);
  // The clock of the animation wakes the loop twice more at most, then the app sleeps.
  app.idle(Loader::Model.step() * 3);
  assert_eq!(app.idle(Loader::Model.step() * 10).frames, 0);
}
