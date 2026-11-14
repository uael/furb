//! The tokens of the design of the window: each color, each loader, each role of a text and each
//! column that a screen starts at. A screen reads them here and writes no value of its own.

use std::time::Duration;

use opentui::{
  style::{Color, Style},
  theme::{Theme, ThemeMode},
};

/// The background when the terminal says none, or says a light one: the window has no light set
/// of colors yet, so it draws its own dark background there. It is the darkest grey of the scale
/// of OpenCode v2.
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
  /// The second line of a card, a thought, a hint, a placeholder, a mark of what is done.
  pub muted: Color,
  /// Work that runs, a link, the dot of what is not read.
  pub accent: Color,
  /// A thread that waits for the operator, the form of a question, a pause that furb made, an
  /// open past its bound.
  pub attention: Color,
  /// A failure, and the cross of a line that failed.
  pub danger: Color,
  /// The bar of a thread and the badge of a project: purple, blue and orange.
  pub hues: [Color; 3],
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
    let (purple, blue, orange) =
      (Color::hex(0xa3_83e2), Color::hex(0x76_99fe), Color::hex(0xec_b081));
    Self {
      base,
      panel,
      raised,
      lifted,
      border,
      text: Color::hex(0xe4_e4e4),
      muted: Color::hex(0x94_9494),
      accent: blue,
      attention: Color::hex(0xf5_a742),
      danger: Color::hex(0xef_6c6c),
      hues: [purple, blue, orange],
    }
  }

  /// What a text of a role looks like.
  pub fn style(&self, role: Role) -> Style {
    match role {
      Role::Title => Style::new().fg(self.text).bold(),
      Role::Prose => Style::new().fg(self.text),
      Role::Muted => Style::new().fg(self.muted),
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
fn mixed(from: Color, to: Color, share: u16) -> Color {
  let channel = |from: u8, to: u8| {
    let (from, to) = (u16::from(from), u16::from(to));
    let mixed = (from * (100 - share) + to * share) / 100;
    u8::try_from(mixed).unwrap_or(u8::MAX)
  };
  Color::rgb(channel(from.r, to.r), channel(from.g, to.g), channel(from.b, to.b))
}

/// The role of a text, which gives its look.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
  /// The first line of a head or of a card.
  Title,
  /// What a person reads: prose and code.
  Prose,
  /// What stands behind: a second line, a hint.
  Muted,
}

/// A mark that moves in one cell while something works.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Loader {
  /// A model that writes.
  Model,
  /// A command that runs.
  Command,
}

impl Loader {
  /// The frames of the mark, in their order.
  pub const fn frames(self) -> &'static [&'static str] {
    match self {
      Self::Model => &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
      Self::Command => &["◐", "◓", "◑", "◒"],
    }
  }

  /// The frame of the mark after a time of work. The marks of the window read one time, so two
  /// marks of one kind show the same frame.
  pub fn frame(self, after: Duration) -> &'static str {
    let frames = self.frames();
    let steps = after.as_millis() / self.step().as_millis();
    frames[usize::try_from(steps).unwrap_or_default() % frames.len()]
  }

  /// How long one frame stays.
  pub const fn step(self) -> Duration {
    match self {
      Self::Model => Duration::from_millis(80),
      Self::Command => Duration::from_millis(120),
    }
  }
}

#[cfg(test)]
#[path = "tokens.test.rs"]
mod test;
