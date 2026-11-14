//! The one table of the keys of the window: what each key does, and its strokes. The hand matches
//! each key that the operator presses in it, and each text that names a key draws it from it, as
//! the system of the client writes a key.

use opentui::prelude::{KeyCode, KeyEvent, Modifiers};

/// The system of the client, which says how a text writes a key: with the signs of the keys on
/// macOS, as `⌃P` and `⌃X R`, and with words on the other systems, as `Ctrl+P` and `Ctrl+X R`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum System {
  MacOs,
  Other,
}

impl System {
  /// The system that the window runs on.
  #[must_use]
  pub const fn here() -> Self {
    if cfg!(target_os = "macos") { Self::MacOs } else { Self::Other }
  }
}

/// What a key of the window does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Does {
  Next,
  Previous,
  Needed,
  List,
  Close,
  Stop,
  Send,
  Empty,
  Leave,
  NewThread,
  Sessions,
  Python,
  PageUp,
  PageDown,
  Jump,
  GoTo,
  Commands,
  Keys,
  Up,
  Down,
  Tick,
  NextQuestion,
}

/// One stroke of a key: whether it holds control and alt, and its key.
#[derive(Debug, Clone, Copy)]
struct Stroke {
  ctrl: bool,
  alt: bool,
  code: KeyCode,
}

const fn plain(code: KeyCode) -> Stroke {
  Stroke { ctrl: false, alt: false, code }
}

const fn ctrl(letter: char) -> Stroke {
  Stroke { ctrl: true, alt: false, code: KeyCode::Char(letter) }
}

const fn alt(code: KeyCode) -> Stroke {
  Stroke { ctrl: false, alt: true, code }
}

/// The key that the keys of the window that need no chord follow, for two seconds.
const LEADER: Stroke = ctrl('x');

/// After the leader.
const fn led(letter: char) -> [Stroke; 2] {
  [LEADER, plain(KeyCode::Char(letter))]
}

/// Each key with its forms, the form that a text names first: one stroke, the leader then a
/// stroke, or a stroke twice. A key with `alt` has a twin after the leader, since `alt` needs a
/// setting of the terminal on macOS.
const TABLE: &[(Does, &[&[Stroke]])] = &[
  (Does::Next, &[&[alt(KeyCode::Down)], &led('j')]),
  (Does::Previous, &[&[alt(KeyCode::Up)], &led('k')]),
  (Does::Needed, &[&[alt(KeyCode::Char('a'))], &led('a')]),
  (Does::List, &[&led('l')]),
  (Does::Close, &[&[plain(KeyCode::Escape)]]),
  (Does::Stop, &[&[plain(KeyCode::Escape), plain(KeyCode::Escape)]]),
  (Does::Send, &[&[plain(KeyCode::Enter)]]),
  (Does::Empty, &[&[ctrl('c')]]),
  (Does::Leave, &[&[ctrl('d')]]),
  (Does::NewThread, &[&[ctrl('n')]]),
  (Does::Sessions, &[&led('o')]),
  (Does::Python, &[&led('y')]),
  (Does::PageUp, &[&[plain(KeyCode::PageUp)]]),
  (Does::PageDown, &[&[plain(KeyCode::PageDown)]]),
  (Does::Jump, &[&[plain(KeyCode::End)]]),
  (Does::GoTo, &[&[ctrl('o')]]),
  (Does::Commands, &[&[ctrl('p')]]),
  (Does::Keys, &[&led('?')]),
  // The keys of the form of a question, which the editor of the input hears when no form shows.
  (Does::Up, &[&[plain(KeyCode::Up)]]),
  (Does::Down, &[&[plain(KeyCode::Down)]]),
  (Does::Tick, &[&[plain(KeyCode::Char(' '))]]),
  (Does::NextQuestion, &[&[plain(KeyCode::Tab)]]),
];

impl Stroke {
  fn matches(self, key: &KeyEvent) -> bool {
    let held = |one| key.modifiers.contains(one);
    key.code == self.code && held(Modifiers::CTRL) == self.ctrl && held(Modifiers::ALT) == self.alt
  }

  /// The stroke as a system writes it: on macOS the signs in the order of Apple, then the key in
  /// upper case, with no `+`; elsewhere each held key as a word with a `+`.
  fn drawn(self, system: System) -> String {
    let mac = system == System::MacOs;
    let name = match (self.code, mac) {
      (KeyCode::Char(' '), _) => "Space".to_owned(),
      (KeyCode::Char(letter), _) => letter.to_uppercase().to_string(),
      (KeyCode::Up, _) => "↑".to_owned(),
      (KeyCode::Down, _) => "↓".to_owned(),
      (KeyCode::Escape, true) => "⎋".to_owned(),
      (KeyCode::Enter, true) => "⏎".to_owned(),
      (KeyCode::PageUp, true) => "⇞".to_owned(),
      (KeyCode::PageDown, true) => "⇟".to_owned(),
      (KeyCode::End, true) => "↘".to_owned(),
      (KeyCode::Tab, true) => "⇥".to_owned(),
      (KeyCode::Escape, false) => "Esc".to_owned(),
      (KeyCode::Enter, false) => "Enter".to_owned(),
      (KeyCode::PageUp, false) => "PgUp".to_owned(),
      (KeyCode::PageDown, false) => "PgDn".to_owned(),
      (KeyCode::End, false) => "End".to_owned(),
      (KeyCode::Tab, false) => "Tab".to_owned(),
      (code, _) => format!("{code:?}"),
    };
    let held = [(self.ctrl, "⌃", "Ctrl+"), (self.alt, "⌥", "Alt+")];
    let signs = held.iter().filter(|one| one.0).map(|one| if mac { one.1 } else { one.2 });
    signs.chain([name.as_str()]).collect()
  }
}

fn forms(does: Does) -> &'static [&'static [Stroke]] {
  TABLE.iter().find(|one| one.0 == does).map_or(&[], |one| one.1)
}

/// The key that does a thing, as a system writes it.
pub(crate) fn drawn(does: Does, system: System) -> String {
  let strokes = forms(does).first().copied().unwrap_or_default();
  strokes.iter().map(|one| one.drawn(system)).collect::<Vec<_>>().join(" ")
}

/// Whether a key is the leader.
pub(crate) fn leads(key: &KeyEvent) -> bool {
  LEADER.matches(key)
}

/// What a key does, after the leader or not. A key of one stroke does its thing whether the
/// leader stands or not.
pub(crate) fn pressed(key: &KeyEvent, led: bool) -> Option<Does> {
  let form = |strokes: &&[Stroke]| match strokes {
    [one] => one.matches(key),
    [first, then] => led && first.code == LEADER.code && first.ctrl && then.matches(key),
    _ => false,
  };
  TABLE.iter().find(|(_, forms)| forms.iter().any(form)).map(|one| one.0)
}

#[cfg(test)]
#[path = "keys.test.rs"]
mod test;
