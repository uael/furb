//! The window given the extremes that a terminal may tell: a size of no row or no column, a size
//! far past the usual one, a point of the mouse past the edges, bytes that no key makes, a cut
//! sequence and a large paste. It draws each with no panic.

#![cfg(test)]

use std::panic::{AssertUnwindSafe, catch_unwind};

use opentui::{
  input::{Event, MouseButton, MouseEventKind},
  testing::TestApp,
};

use crate::keys::threaded;

/// What the window did with one extreme, named so that a failure says which.
fn takes(what: &str, app: &TestApp, change: impl FnOnce(&TestApp)) {
  let drawn = catch_unwind(AssertUnwindSafe(|| {
    change(app);
    app.render();
  }));
  assert!(drawn.is_ok(), "the window panicked on {what}");
}

#[test]
fn a_thread_draws_each_size_and_each_event_at_the_edges_with_no_panic() {
  let (app, _daemon) = threaded();
  for (columns, rows) in [(0, 0), (1, 1), (0, 30), (100, 0), (2, 1), (1, 200), (600, 3), (300, 200)]
  {
    // The terminal tells each size as it stands, a size of no row as well, while the screen that
    // OpenTUI draws counts no less than one.
    let size = |app: &TestApp| {
      drop(app.app().resize(columns, rows));
      app.event(Event::Resize { width: columns, height: rows });
    };
    takes(&format!("a size of {columns} by {rows}"), &app, size);
    for (x, y) in [(0, 0), (u16::MAX, u16::MAX), (columns, rows), (columns / 2, rows / 2)] {
      takes(&format!("a click at {x}, {y} in {columns} by {rows}"), &app, |app| app.click(x, y));
      takes(&format!("a hover at {x}, {y}"), &app, |app| app.hover(x, y));
      takes(&format!("a scroll at {x}, {y}"), &app, |app| app.scroll(x, y, true));
      // A terminal tells a drag as the cells that the pointer reaches, which may jump far.
      takes(&format!("a drag to {x}, {y}"), &app, |app| {
        app.mouse(MouseEventKind::Down(MouseButton::Left), 0, 0);
        app.mouse(MouseEventKind::Drag(MouseButton::Left), x, y);
        app.mouse(MouseEventKind::Up(MouseButton::Left), x, y);
      });
    }
  }
  takes("the usual size again", &app, |app| app.resize(100, 30));
  let bytes: [&[u8]; 8] = [
    b"\x1b",
    b"\x1b[",
    b"\x1b[999999999;999999999R",
    b"\x1b[<0;65535;65535M",
    b"\x1b[<64;0;0M",
    b"\x1b[200~",
    &[0xff, 0xfe, 0xc3],
    b"\x1b[15;99999~",
  ];
  for one in bytes {
    takes(&format!("the bytes {one:?}"), &app, |app| app.feed(one));
  }
  takes("a paste of 16,000 lines", &app, |app| app.paste(&"é\n".repeat(1 << 14)));
  takes("an empty paste", &app, |app| app.paste(""));
}
