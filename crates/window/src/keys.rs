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
  Resolve,
  Revert,
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
  Keys,
  Up,
  Down,
  Lean,
  Tick,
  NextQuestion,
  Agent,
  Forward,
  Take,
  TakeWord,
  Machines,
  Projects,
  History,
  Model,
  Effort,
  Changes,
  Context,
  Running,
  Rewind,
  Chains,
  Aside,
  Stash,
  Picture,
  Settings,
  Skills,
  Queue,
  Pause,
  Rename,
  Copy,
  Cd,
  Views,
}

/// One stroke of a key: whether it holds control, alt and command, and its key. Command is the
/// key of macOS, super elsewhere, which a terminal tells when it gives the keys of Kitty.
#[derive(Debug, Clone, Copy)]
struct Stroke {
  ctrl: bool,
  alt: bool,
  cmd: bool,
  code: KeyCode,
}

const fn plain(code: KeyCode) -> Stroke {
  Stroke { ctrl: false, alt: false, cmd: false, code }
}

const fn ctrl(letter: char) -> Stroke {
  Stroke { ctrl: true, ..plain(KeyCode::Char(letter)) }
}

const fn alt(code: KeyCode) -> Stroke {
  Stroke { alt: true, ..plain(code) }
}

const fn cmd(letter: char) -> Stroke {
  Stroke { cmd: true, ..plain(KeyCode::Char(letter)) }
}

/// The key that the keys of the window that need no chord follow, for two seconds.
const LEADER: Stroke = ctrl('x');

/// After the leader.
const fn led(letter: char) -> [Stroke; 2] {
  [LEADER, plain(KeyCode::Char(letter))]
}

/// A key of the table: what it does, as a text of the window says it, the name of its command
/// when it is one, which a slash at the start of a message and the list of the commands offer, and
/// its forms, the form that a text names first: one stroke, the leader then a stroke, or a stroke
/// twice.
pub(crate) struct Bound {
  pub(crate) does: Does,
  pub(crate) says: &'static str,
  pub(crate) command: Option<&'static str>,
  forms: &'static [&'static [Stroke]],
}

const fn bound(
  does: Does,
  says: &'static str,
  command: Option<&'static str>,
  forms: &'static [&'static [Stroke]],
) -> Bound {
  Bound { does, says, command, forms }
}

/// The keys that go from one place of the window to another. A key with `alt` has a twin after the
/// leader, since `alt` needs a setting of the terminal on macOS.
const GO: &[Bound] = &[
  bound(
    Does::GoTo,
    "go to anything, or run a command",
    Some("go"),
    &[&[ctrl('p')], &[cmd('p')], &[ctrl('o')]],
  ),
  bound(Does::Keys, "each key", Some("keys"), &[&led('?')]),
  bound(
    Does::Needed,
    "the next thread that needs you",
    Some("needed"),
    &[&[alt(KeyCode::Char('a'))], &led('a')],
  ),
  bound(Does::Next, "the next card", Some("next"), &[&[alt(KeyCode::Down)], &led('j')]),
  bound(Does::Previous, "the card before", Some("previous"), &[&[alt(KeyCode::Up)], &led('k')]),
  bound(Does::List, "show or hide the list", Some("list"), &[&led('l')]),
];

/// The keys of the open thread.
const THREAD: &[Bound] = &[
  bound(Does::NewThread, "a new thread, and back to the thread", Some("new"), &[&[ctrl('n')]]),
  bound(Does::Jump, "jump to the latest", Some("latest"), &[&[plain(KeyCode::End)]]),
  bound(Does::PageUp, "a page up", None, &[&[plain(KeyCode::PageUp)]]),
  bound(Does::PageDown, "a page down", None, &[&[plain(KeyCode::PageDown)]]),
  bound(
    Does::Stop,
    "stop the work of the thread",
    Some("stop"),
    &[&[plain(KeyCode::Escape), plain(KeyCode::Escape)]],
  ),
  bound(Does::Resolve, "resolve the thread", Some("resolve"), &[&led('r')]),
  bound(Does::Revert, "revert the files that the last answer wrote", Some("revert"), &[&led('d')]),
  bound(Does::Pause, "pause the thread, or resume it", Some("pause"), &[&led('z')]),
  bound(Does::Running, "pause, kill or keep the session", Some("running"), &[&led('s')]),
  bound(Does::Context, "the context: kinds, cache, folded", Some("context"), &[&led('c')]),
  bound(Does::Rewind, "back in the time of the thread, and to now", Some("rewind"), &[&led('b')]),
  bound(Does::Chains, "the chains of the life on the time", Some("chains"), &[&led('t')]),
  bound(
    Does::Aside,
    "ask aside, in a fork that the thread does not keep",
    Some("btw"),
    &[&led('q')],
  ),
  bound(
    Does::Agent,
    "the next agent of the thread",
    Some("agent"),
    &[&[LEADER, plain(KeyCode::Down)]],
  ),
  bound(
    Does::Forward,
    "forward to the agent that you went back from",
    Some("forward"),
    &[&[LEADER, plain(KeyCode::Right)]],
  ),
];

/// The keys of the input.
const INPUT: &[Bound] = &[
  bound(Does::Send, "send", None, &[&[plain(KeyCode::Enter)]]),
  bound(Does::Queue, "queue, sent once the thread rests", None, &[&[alt(KeyCode::Enter)]]),
  bound(Does::Empty, "empty the input, then take back the last", None, &[&[ctrl('c')]]),
  bound(
    Does::Stash,
    "put the input aside, or bring it back",
    Some("stash"),
    &[&[ctrl('s')], &led('u')],
  ),
  bound(Does::Picture, "paste the image of the clipboard", Some("paste"), &[&[ctrl('v')]]),
  bound(Does::Leave, "leave, twice on an empty input", Some("quit"), &[&[ctrl('d')]]),
  bound(Does::Sessions, "the session that the input sends to", Some("session"), &[&led('o')]),
  bound(Does::Rename, "name the session", Some("rename"), &[&led('n')]),
  bound(Does::Cd, "move the chain of the thread to a folder", Some("cd"), &[&led('f')]),
  bound(Does::Copy, "copy what the thread said last", Some("copy"), &[&led('i')]),
  bound(Does::Views, "reach the views of the session with the keys", Some("view"), &[&led('v')]),
  bound(Does::Python, "a word of Python, and back to a message", Some("python"), &[&led('y')]),
  bound(Does::Machines, "the machine of a new session", Some("machine"), &[&led('h')]),
  bound(Does::Projects, "the project of a new session", Some("project"), &[&led('p')]),
  bound(Does::History, "search what you sent here", Some("history"), &[&[ctrl('r')]]),
  bound(Does::Model, "the model of the input", Some("model"), &[&led('m')]),
  bound(Does::Effort, "the next effort of the model", Some("effort"), &[&led('e')]),
  bound(Does::Changes, "the changes of the project and their diff", Some("changes"), &[&led('g')]),
  bound(Does::Settings, "the settings of the session", Some("config"), &[&led(',')]),
  bound(Does::Skills, "the extensions, on or off", Some("extensions"), &[&led('x')]),
  bound(
    Does::Take,
    "take the grey rest of what you sent",
    None,
    &[&[plain(KeyCode::Right)], &[ctrl('f')]],
  ),
  bound(
    Does::TakeWord,
    "take one word of the grey rest",
    None,
    &[&[alt(KeyCode::Right)], &led('w')],
  ),
];

/// The keys of a list and of the form of a question, which the editor of the input hears when
/// neither shows.
const FORM: &[Bound] = &[
  bound(Does::Close, "close a list, or go back", None, &[&[plain(KeyCode::Escape)]]),
  bound(Does::Up, "the row above", None, &[&[plain(KeyCode::Up)]]),
  bound(Does::Down, "the row below", None, &[&[plain(KeyCode::Down)]]),
  bound(
    Does::Lean,
    "a lower or a higher effort of the chosen model",
    None,
    &[&[plain(KeyCode::Left)], &[plain(KeyCode::Right)]],
  ),
  bound(Does::Tick, "tick a choice of a question", None, &[&[plain(KeyCode::Char(' '))]]),
  bound(Does::NextQuestion, "the next question of the thread", None, &[&[plain(KeyCode::Tab)]]),
];

/// Each group of keys, with its name, in the order of the table.
const TABLE: [(&str, &[Bound]); 4] =
  [("Go", GO), ("The thread", THREAD), ("The input", INPUT), ("A list or a question", FORM)];

/// Each command of the table, in its order.
pub(crate) fn commands() -> impl Iterator<Item = &'static Bound> {
  keys().filter(|one| one.command.is_some())
}

/// Each key of the table.
fn keys() -> impl Iterator<Item = &'static Bound> {
  TABLE.iter().flat_map(|one| one.1)
}

impl Stroke {
  fn matches(self, key: &KeyEvent) -> bool {
    let held = |one| key.modifiers.contains(one);
    key.code == self.code
      && held(Modifiers::CTRL) == self.ctrl
      && held(Modifiers::ALT) == self.alt
      && held(Modifiers::SUPER) == self.cmd
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
      (KeyCode::Left, _) => "←".to_owned(),
      (KeyCode::Right, _) => "→".to_owned(),
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
    let held = [(self.ctrl, "⌃", "Ctrl+"), (self.alt, "⌥", "Alt+"), (self.cmd, "⌘", "Super+")];
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

/// Each form of a key, one after the other, as `←→`.
pub(crate) fn drawn_each(does: Does, system: System) -> String {
  forms(does).iter().map(|one| form(one, system)).collect()
}

/// What a key does, as a text of the window says it.
pub(crate) fn says(does: Does) -> &'static str {
  keys().find(|one| one.does == does).map_or("", |one| one.says)
}

/// Each key of the table, in its order, with the name of its group and each of its forms as a
/// system writes them. A form with command shows only on macOS, and only while the terminal tells
/// the keys that hold super: elsewhere the desktop most often takes super for itself, and a
/// terminal that does not tell it never gives the key, so the window still hears it but does not
/// offer it.
pub(crate) fn each(
  system: System,
  supers: bool,
) -> impl Iterator<Item = (&'static str, &'static Bound, Vec<String>)> {
  let reaches = system == System::MacOs && supers;
  let offered = move |strokes: &&&[Stroke]| reaches || !strokes.iter().any(|one| one.cmd);
  let forms = move |one: &'static Bound| {
    one.forms.iter().filter(offered).map(|strokes| form(strokes, system)).collect()
  };
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
