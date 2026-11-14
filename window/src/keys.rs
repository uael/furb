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

/// A key of the table: what it does, as a text of the window says it, whether it is a command,
/// which the list of the commands offers, and its forms, the form that a text names first: one
/// stroke, the leader then a stroke, or a stroke twice.
pub(crate) struct Bound {
  pub(crate) does: Does,
  pub(crate) says: &'static str,
  pub(crate) command: bool,
  forms: &'static [&'static [Stroke]],
}

const fn bound(
  does: Does,
  says: &'static str,
  command: bool,
  forms: &'static [&'static [Stroke]],
) -> Bound {
  Bound { does, says, command, forms }
}

/// The keys that go from one place of the window to another. A key with `alt` has a twin after the
/// leader, since `alt` needs a setting of the terminal on macOS.
const GO: &[Bound] = &[
  bound(Does::GoTo, "go to a project or a session", true, &[&[ctrl('o')]]),
  bound(Does::Commands, "commands", false, &[&[ctrl('p')]]),
  bound(Does::Keys, "each key", true, &[&led('?')]),
  bound(
    Does::Needed,
    "the next thread that needs you",
    true,
    &[&[alt(KeyCode::Char('a'))], &led('a')],
  ),
  bound(Does::Next, "the next card", true, &[&[alt(KeyCode::Down)], &led('j')]),
  bound(Does::Previous, "the card before", true, &[&[alt(KeyCode::Up)], &led('k')]),
  bound(Does::List, "show or hide the list", true, &[&led('l')]),
];

/// The keys of the open thread.
const THREAD: &[Bound] = &[
  bound(Does::NewThread, "a new thread, and back to the thread", true, &[&[ctrl('n')]]),
  bound(Does::Jump, "jump to the latest", true, &[&[plain(KeyCode::End)]]),
  bound(Does::PageUp, "a page up", false, &[&[plain(KeyCode::PageUp)]]),
  bound(Does::PageDown, "a page down", false, &[&[plain(KeyCode::PageDown)]]),
  bound(
    Does::Stop,
    "stop the work of the thread",
    true,
    &[&[plain(KeyCode::Escape), plain(KeyCode::Escape)]],
  ),
];

/// The keys of the input.
const INPUT: &[Bound] = &[
  bound(Does::Send, "send", false, &[&[plain(KeyCode::Enter)]]),
  bound(Does::Empty, "empty the input", false, &[&[ctrl('c')]]),
  bound(Does::Leave, "leave, on an empty input", true, &[&[ctrl('d')]]),
  bound(Does::Sessions, "the session that the input sends to", true, &[&led('o')]),
  bound(Does::Python, "a word of Python, and back to a message", true, &[&led('y')]),
];

/// The keys of a list and of the form of a question, which the editor of the input hears when
/// neither shows.
const FORM: &[Bound] = &[
  bound(Does::Close, "close a list, or go back", false, &[&[plain(KeyCode::Escape)]]),
  bound(Does::Up, "the row above", false, &[&[plain(KeyCode::Up)]]),
  bound(Does::Down, "the row below", false, &[&[plain(KeyCode::Down)]]),
  bound(Does::Tick, "tick a choice of a question", false, &[&[plain(KeyCode::Char(' '))]]),
  bound(Does::NextQuestion, "the next question of the thread", false, &[&[plain(KeyCode::Tab)]]),
];

/// Each group of keys, with its name, in the order of the table.
const TABLE: [(&str, &[Bound]); 4] =
  [("Go", GO), ("The thread", THREAD), ("The input", INPUT), ("A list or a question", FORM)];

/// Each key of the table.
fn keys() -> impl Iterator<Item = &'static Bound> {
  TABLE.iter().flat_map(|one| one.1)
}

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
  keys().find(|one| one.does == does).map_or(&[], |one| one.forms)
}

/// A form of a key, as a system writes it.
fn form(strokes: &[Stroke], system: System) -> String {
  strokes.iter().map(|one| one.drawn(system)).collect::<Vec<_>>().join(" ")
}

/// The key that does a thing, as a system writes it.
pub(crate) fn drawn(does: Does, system: System) -> String {
  form(forms(does).first().copied().unwrap_or_default(), system)
}

/// What a key does, as a text of the window says it.
pub(crate) fn says(does: Does) -> &'static str {
  keys().find(|one| one.does == does).map_or("", |one| one.says)
}

/// Each key of the table, in its order, with the name of its group and each of its forms as a
/// system writes them.
pub(crate) fn each(
  system: System,
) -> impl Iterator<Item = (&'static str, &'static Bound, Vec<String>)> {
  let forms =
    move |one: &'static Bound| one.forms.iter().map(|strokes| form(strokes, system)).collect();
  TABLE.iter().flat_map(move |(group, keys)| keys.iter().map(move |one| (*group, one, forms(one))))
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
  keys().find(|one| one.forms.iter().any(form)).map(|one| one.does)
}

#[cfg(test)]
#[path = "keys.test.rs"]
mod test;
