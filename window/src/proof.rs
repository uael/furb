//! The screen that proves the base of the window. It is no screen of the design: it says that the
//! window reached the daemon of a machine, and what that daemon told. The first screen of the
//! design takes its place.

use opentui::{
  App,
  prelude::{Container, Span, Style, Text, View, Widget},
};

use crate::{
  screen::{Screen, Told},
  spinner::Spinner,
  tokens::{HEAD, Loader, Role, TEXT, Tokens},
};

/// The screen on an app: a head, then the machine. While the window dials, a loader moves before
/// one line that says so. Then the name of the machine stands there, over the protocol of its
/// daemon and the count of its records. A connection that failed says why in the same place.
pub(crate) fn drawn(app: &App, tokens: &Tokens, machine: &str) -> Screen {
  let head = Text::new(app, Span::styled("furb", tokens.style(Role::Title)));
  let loader = Spinner::new(app, Loader::Model, tokens.accent);
  let (line, under) = (Text::new(app, ""), Text::new(app, ""));
  let named = View::new(app).row().gap(1).child(&loader).child(&line);
  let body = View::new(app).padding_x(TEXT).child(&named).child(&under);
  app.root().gap(1).child(&View::new(app).padding_x(HEAD).child(&head)).child(&body);

  let (tokens, machine) = (*tokens, machine.to_owned());
  let mut screen = Screen::default();
  screen.part(
    |told| *told == Told::Dialing,
    move |dialing| {
      loader.visible(*dialing);
    },
  );
  let named = move |told: &Told| match told {
    Told::Dialing => (tokens.style(Role::Muted), format!("Dials the daemon of {machine}")),
    Told::Welcomed { .. } => (tokens.style(Role::Prose), machine.clone()),
    Told::Lost(why) => (Style::new().fg(tokens.danger), format!("✗ {why}")),
  };
  screen.part(named, move |(style, text)| {
    line.set_text(Span::styled(text.as_str(), *style));
  });
  screen.part(told, move |text| {
    under.set_text(Span::styled(text.as_str(), tokens.style(Role::Muted)));
  });
  screen
}

/// What the daemon told of itself, on one line.
fn told(told: &Told) -> String {
  match told {
    Told::Welcomed { protocol, records: None } => format!("protocol {protocol}"),
    Told::Welcomed { protocol, records: Some(1) } => format!("protocol {protocol} · 1 record"),
    Told::Welcomed { protocol, records: Some(count) } => {
      format!("protocol {protocol} · {count} records")
    }
    Told::Dialing | Told::Lost(_) => String::new(),
  }
}
