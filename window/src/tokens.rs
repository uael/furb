//! The tokens of the design of the window: each color, each loader, each role of a text and each
//! column that a screen starts at. A screen reads them here and writes no value of its own.

use std::time::Duration;

use opentui::{
  style::{Color, Style},
  theme::{Theme, ThemeMode},
  widget::DiffColors,
};

/// The background when the terminal says none, or says a light one: the window has no light set
/// of colors yet, so it draws its own dark background there. It is the darkest grey of the scale
/// of `OpenCode` v2.
const BASE: Color = Color::hex(0x0a_0a0a);

/// The color that each surface is mixed toward, and how many parts of 100 of it each step of the
/// scale takes: a panel, a raised band, a lifted thing and a border.
const WHITE: Color = Color::hex(0xff_ffff);
const STEPS: [u16; 4] = [3, 7, 13, 26];

/// The column where the head of a screen starts.
pub const HEAD: u16 = 1;

/// The column where each text under a head starts: a bar takes column 0, and one column stays
/// blank after it.
pub const TEXT: u16 = 2;

/// The width of the list of cards, at least.
pub(crate) const LIST: u16 = 38;

/// The most columns of the list of cards, which a wide window gives it: a title of about fifty
/// letters then stands whole on its card.
pub(crate) const LISTS: u16 = 56;

/// The least width of the window that shows the list beside the thread.
pub(crate) const BESIDE: u16 = 100;

/// The most columns that the left side draws: a line of code of 100 columns beside its numbers.
/// A wider left side centers its column, so a line of prose stays short to read.
pub(crate) const MEASURE: u16 = 110;

/// The columns between two things of one line, as the parts of the launcher, and the mark and the
/// button of the head of a thread.
pub(crate) const APART: usize = 3;

/// The columns that a key takes in a table of keys, before what it does: the keys of the home and
/// of the list of each key.
pub(crate) const KEYS: usize = 12;

/// How long the hand stays away from the list before a card changes its place: a change that the
/// list hears while the hand is on it waits until then, so the list moves under no pointer.
pub const AWAY: Duration = Duration::from_millis(1500);

/// The columns of a list part over the window, as Go to, the commands and each key, at least.
pub(crate) const PART: u16 = 60;

/// The most columns of a list part, whose entries say more than its least width holds, as the
/// settings and the extensions with what each does.
pub(crate) const PARTS: u16 = 100;

/// The rows of the dimmed window that stand above a list part, and under it at least.
pub(crate) const OVER: u16 = 3;

/// The colors of the window: a scale of greys from the background of the terminal, and few
/// accents. Each has one job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tokens {
  /// The background, which is the one of the terminal when the terminal says it.
  pub base: Color,
  /// The list beside the thread.
  pub panel: Color,
  /// A message of the operator, the input, a block of code, a thing under the pointer.
  pub raised: Color,
  /// The selected card.
  pub lifted: Color,
  /// A line between two parts.
  pub border: Color,
  /// Prose and code. It is never pure white.
  pub text: Color,
  /// The text of a thing that takes a click while the pointer is on it, which changes nothing else.
  pub hover: Color,
  /// The second line of a card, a thought, a hint, a placeholder, a mark of what is done.
  pub muted: Color,
  /// Work that runs, a link, a change of a file: the warm orange that gives the window its tone.
  pub accent: Color,
  /// What is for the operator: a thread that waits for him, the form of a question, a pause that
  /// furb made, an open past its bound, and the dot of an end that he did not read. It is blue, far
  /// from the orange of work, so his turn and the work of the machine never read alike.
  pub attention: Color,
  /// A failure, the cross of a line that failed, and a line that a change removes.
  pub danger: Color,
  /// A line that a change adds: a cool cyan against the warm red of a line that it removes, as a
  /// diff reads, with no green, which the window shows nowhere.
  pub added: Color,
  /// The mark of a session whose life runs, which a person can pause or end, where a session that
  /// only rests in its record has none. It takes the bright text, not the orange of work, since a
  /// life runs while its threads rest, and a card that works shows its work beside it.
  pub live: Color,
  /// The ground of a line that a change adds, of its hue, so its code keeps the colors of its
  /// grammar.
  pub added_ground: Color,
  /// The ground of a line that a change removes.
  pub removed_ground: Color,
  /// The veil over the window while a list part stands over it, which dims each cell under it.
  pub veil: Color,
  /// The hues of the work: the bar of a thread, and under the bar of its time each chain that
  /// works, the chain of the thread in the hue of the thread. Each is far from the others and from
  /// the colors of attention, of a failure and of a line that a change adds, so two threads side by
  /// side read apart: purple, rose and gold. None is the orange of the accent, which is the bar of
  /// a new thread, so a turn of the input to a new thread always changes the color of its bar.
  pub hues: [Color; 3],
  /// The square badges of projects: ice, denim and clay. No hue of the work takes them, so a reader
  /// never takes a project for a thread or a chain, and a badge is a ground of its own where each
  /// hue of the work colors a text.
  pub projects: [Badge; 3],
}

/// The square badge of a project: its letters in its hue, on its hue mixed into the background,
/// a ground that it keeps on each surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Badge {
  pub letters: Color,
  pub ground: Color,
}

impl Tokens {
  /// The tokens on the background of a terminal. A surface is that background mixed toward white
  /// by the steps of the scale, so the window shows no band of another black. The background
  /// stays the default color of the terminal, so a terminal that draws its own background, as an
  /// image, keeps it.
  pub fn on(terminal: Option<Color>) -> Self {
    let dark = terminal.filter(|told| ThemeMode::from_background(*told) == ThemeMode::Dark);
    let base = dark.map_or(BASE, |told| Color::default_with(told.r, told.g, told.b));
    let [panel, raised, lifted, border] = STEPS.map(|share| mixed(base, WHITE, share));
    let (orange, text) = (Color::hex(0xfa_b283), Color::hex(0xe4_e4e4));
    let hues = [0xa3_83e2, 0xe8_89a8, 0xd6_b860].map(Color::hex);
    let projects = [0xa9_c4d9, 0x7f_9dd0, 0xc0_8a6e]
      .map(|hue| Badge { letters: Color::hex(hue), ground: mixed(base, Color::hex(hue), 24) });
    let (added, danger) = (Color::hex(0x56_b6c2), Color::hex(0xef_6c6c));
    Self {
      base,
      panel,
      raised,
      lifted,
      border,
      text,
      hover: WHITE,
      muted: Color::hex(0x94_9494),
      accent: orange,
      attention: Color::hex(0x76_99fe),
      danger,
      added,
      live: text,
      added_ground: mixed(base, added, 18),
      removed_ground: mixed(base, danger, 18),
      veil: Color::rgba(base.r, base.g, base.b, 160),
      hues,
      projects,
    }
  }

  /// What a text of a role looks like.
  pub fn style(&self, role: Role) -> Style {
    match role {
      Role::Title => Style::new().fg(self.text).bold(),
      Role::Prose => Style::new().fg(self.text),
      Role::Muted => Style::new().fg(self.muted),
      Role::Button => Style::new().fg(self.text).bg(self.lifted),
    }
  }

  /// The colors of a diff: a line that a change adds or removes on its ground, and the numbers
  /// muted.
  pub(crate) fn diff(&self) -> DiffColors {
    let (added, removed) = (self.added_ground, self.removed_ground);
    DiffColors {
      line_number: self.muted,
      line_number_bg: Color::TRANSPARENT,
      added_bg: added,
      removed_bg: removed,
      context_bg: Color::TRANSPARENT,
      added_content_bg: None,
      removed_content_bg: None,
      context_content_bg: None,
      added_sign: self.added,
      removed_sign: self.danger,
      added_line_number_bg: added,
      removed_line_number_bg: removed,
    }
  }

  /// The colors that a widget of OpenTUI starts with, so a widget that a screen gives no color
  /// stands on the tokens.
  pub(crate) fn theme(&self) -> Theme {
    Theme {
      background: self.base,
      surface: self.raised,
      foreground: self.text,
      muted: self.muted,
      accent: self.accent,
      border: self.border,
      focus: self.accent,
      selection_bg: self.lifted,
      selection_fg: self.text,
      highlight_bg: self.lifted,
      highlight_fg: self.text,
      input_bg: self.raised,
      input_focused_bg: self.raised,
      cursor: self.text,
      track: self.raised,
      thumb: self.border,
      success: self.muted,
      warning: self.attention,
      error: self.danger,
    }
  }
}

/// A color mixed toward another by a share of 100.
pub(crate) fn mixed(from: Color, to: Color, share: u16) -> Color {
  let channel = |from: u8, to: u8| {
    let (from, to) = (u16::from(from), u16::from(to));
    let mixed = (from * (100 - share) + to * share) / 100;
    u8::try_from(mixed).unwrap_or(u8::MAX)
  };
  Color::rgb(channel(from.r, to.r), channel(from.g, to.g), channel(from.b, to.b))
}

/// The colors of python inside a block of code, by the name of a capture of its grammar, from the
/// syntax of `OpenCode` v2, less its green: a string takes a pale cyan. A name that this list does
/// not hold takes the text.
pub const SYNTAX: [(&str, u32); 9] = [
  ("comment", 0x80_8080),
  ("keyword", 0x9d_7cd8),
  ("function", 0xfa_b283),
  ("constructor", 0xe5_c07b),
  ("type", 0xe5_c07b),
  ("string", 0x99_d5e0),
  ("number", 0xf5_a742),
  ("constant", 0xf5_a742),
  ("escape", 0x56_b6c2),
];

/// The role of a text, which gives its look.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
  /// The first line of a head or of a card.
  Title,
  /// What a person reads: prose and code.
  Prose,
  /// What stands behind: a second line, a hint.
  Muted,
  /// A thing that acts on a click, as a button of a head or the chosen part of a switch: its word
  /// on the lifted step, with one cell of it on each side.
  Button,
}

/// The word of the state of a thing that works, on a card, in the dock and in the row of hints.
pub(crate) const WORKS: &str = "Working";

/// The one verb of furb that makes code and value: its call stands bold in code, and shines while
/// it works.
pub(crate) const PROMPT: &str = "prompt";

/// A mark that moves while something works: one cell, or the word of work for the light.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Loader {
  /// A model that writes.
  Model,
  /// A command that runs.
  Command,
  /// An agent that works for a line: an arrow that pulses from muted to its color.
  Agent,
  /// A word that says what works, as `Working` of the open thread, a model that `thinks` or that
  /// `writes`: a light coasts over it from its left with a soft tail, the word goes white whole for
  /// a moment, then fades back.
  Light(&'static str),
}

/// How long the pulse of a card that begins to call its operator takes, and how far of it goes to
/// rise: it rises fast, then settles over the rest, as a tab of `OpenCode` v2 pulses.
pub(crate) const CALL: Duration = Duration::from_millis(1400);
const RISE: i64 = 150;

/// How far the pulse of a card stands `after` the card began to call, in hundredths: none once the
/// pulse is over.
pub(crate) fn called(after: Duration) -> u16 {
  let at = i64::try_from(after.as_millis() * 1000 / CALL.as_millis()).unwrap_or(1000);
  let share = match at {
    at if at < RISE => smoother(at * 1000 / RISE),
    at => 1000 - smoother((at - RISE) * 1000 / (1000 - RISE)),
  };
  u16::try_from(share / 10).unwrap_or(0)
}

/// How long one pulse of the arrow of an agent takes.
const PULSE: Duration = Duration::from_millis(1200);

/// The beat of the light, in ms: its front crosses the word in `SWEEP`, the word stays white whole
/// for `WHOLE` and fades back in `FADE`, and it rests until the next sweep, `BEAT` after the last.
const BEAT: u128 = 2400;
const SWEEP: u128 = 900;
const WHOLE: u128 = 240;
const FADE: u128 = 240;

/// How far the light reaches, in cells: ahead of its front, and behind it as its tail.
const AHEAD: i64 = 2;
const TAIL: i64 = 6;

impl Loader {
  /// The frames of a glyph, in their order: none for the light, whose one frame is its word.
  pub const fn frames(self) -> &'static [&'static str] {
    match self {
      Self::Model => &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
      Self::Command => &["◐", "◓", "◑", "◒"],
      Self::Agent => &["↳"],
      Self::Light(_) => &[],
    }
  }

  /// The frame of the mark after a time of work. The marks of the window read one time, so two
  /// marks of one kind show the same frame.
  pub fn frame(self, after: Duration) -> &'static str {
    if let Self::Light(word) = self {
      return word;
    }
    let frames = self.frames();
    let steps = after.as_millis() / self.step().as_millis();
    frames[usize::try_from(steps).unwrap_or_default() % frames.len()]
  }

  /// How long one frame stays.
  pub const fn step(self) -> Duration {
    match self {
      Self::Model | Self::Agent | Self::Light(_) => Duration::from_millis(80),
      Self::Command => Duration::from_millis(120),
    }
  }

  /// The cells of the mark after a time of work, each with its color, from the color of its place:
  /// the frame of a glyph in that color, the arrow of an agent between muted and that color, or
  /// each letter of its word between that color and white as the light passes it.
  pub fn cells(self, after: Duration, color: Color, tokens: &Tokens) -> Vec<(&'static str, Color)> {
    match self {
      Self::Model | Self::Command => vec![(self.frame(after), color)],
      Self::Agent => {
        // The pulse eases in and out: smoothstep over each half of its period.
        let half = PULSE.as_millis() / 2;
        let x = (after.as_millis() % PULSE.as_millis()).abs_diff(half).abs_diff(half);
        let share = 100 * x * x * (3 * half - 2 * x) / (half * half * half);
        vec![(self.frame(after), mixed(tokens.muted, color, u16::try_from(share).unwrap_or(100)))]
      }
      Self::Light(word) => {
        let letters = word.char_indices().map(|(at, one)| &word[at..at + one.len_utf8()]);
        let lit = light(after.as_millis() % BEAT, i64::try_from(word.chars().count()).unwrap_or(0));
        let cells = (0..).zip(letters).map(|(at, one)| (one, mixed(color, WHITE, lit(at) / 10)));
        cells.collect()
      }
    }
  }
}

/// How white each letter of a word of `length` letters is, in thousandths, at `at` ms of the beat.
/// The front coasts from before the first letter to past the last, as the light of the tabs of
/// `OpenCode` v2 does, and each letter eases up as it nears and down as it falls behind.
fn light(at: u128, length: i64) -> impl Fn(i64) -> u16 {
  let thousandths = |part: u128, of: u128| i64::try_from(part * 1000 / of).unwrap_or(1000);
  let whole = match at {
    at if at < SWEEP => None,
    at if at < SWEEP + WHOLE => Some(1000),
    at if at < SWEEP + WHOLE + FADE => Some(1000 - smoother(thousandths(at - SWEEP - WHOLE, FADE))),
    _ => Some(0),
  };
  let front = -AHEAD * 1000 + coast(thousandths(at.min(SWEEP), SWEEP)) * (length + AHEAD);
  move |letter| {
    let near = front - letter * 1000;
    let share = whole.unwrap_or_else(|| match near {
      near if near < 0 => smoother(1000 + near / AHEAD),
      near => smoother(1000 - near / TAIL),
    });
    u16::try_from(share).unwrap_or(0)
  }
}

/// A move in thousandths that speeds up over its first fifth, holds, and slows over its last.
fn coast(at: i64) -> i64 {
  match at {
    at if at < 200 => at * at / 320,
    at if at > 800 => 1000 - (1000 - at) * (1000 - at) / 320,
    at => (at - 100) * 5 / 4,
  }
}

/// The smootherstep of a share in thousandths, clamped to 0 and 1000.
fn smoother(at: i64) -> i64 {
  let at = at.clamp(0, 1000);
  at * at * at * (6 * at * at - 15_000 * at + 10_000_000) / 1_000_000_000_000
}
