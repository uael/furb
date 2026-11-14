//! The home, at the left of the list: what waits for the operator, what worked while no window was
//! open, and the input of a new session, which sends nothing yet.

use std::env;

use furb::view::Standing;
use opentui::style::{Color, Style};
use unicode_width::UnicodeWidthStr;

use crate::{
  line::{self, Line, run, spread},
  list::{project, unheld},
  model::{Link, Model},
  tokens::Role,
};

impl Model {
  /// The rows of the home. The input and the row of hints stand on the last rows of the window.
  pub(crate) fn home(&self) -> Vec<Line> {
    let mut rows = self.says();
    rows.resize(usize::from(self.size.1).saturating_sub(3), Vec::new());
    rows.extend(self.hinted());
    rows
  }

  /// The name of the project of the folder of the window.
  fn project(&self) -> String {
    self.folder.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default()
  }

  /// A row of the home under its head: a text, and a hint at its end.
  fn said(&self, left: &str, style: Style, right: &str) -> Line {
    let muted = self.tokens.style(Role::Muted);
    let mut line = vec![run("  ", Style::new())];
    let room = self.wide().saturating_sub(4);
    line.extend(spread(vec![run(left, style)], vec![run(right, muted)], room, Style::new()));
    line
  }

  /// What the home says: what waits for the operator, what worked with no view, each machine that
  /// is lost with its cause, and the keys that lead out.
  fn says(&self) -> Vec<Line> {
    let t = &self.tokens;
    let (prose, muted) = (t.style(Role::Prose), t.style(Role::Muted));
    let room = self.wide().saturating_sub(4);
    let wrapped = |text: &str, style: Style| {
      let lines = line::wrapped(text, room);
      lines.iter().map(|line| self.said(line, style, "")).chain([Vec::new()]).collect::<Vec<_>>()
    };
    let mut rows = vec![vec![run(" furb", t.style(Role::Title))], Vec::new()];
    if !self.settled.is_empty() {
      let needs = self.settled.iter().filter(|one| self.needs(&one.1, &one.2)).count();
      let (says, key) = match needs {
        0 => ("Nothing needs you.".to_owned(), ""),
        1 => ("1 thread needs you.".to_owned(), "alt+a goes to it"),
        count => (format!("{count} threads need you."), "alt+a goes to one"),
      };
      // The key shows where the row holds the two whole.
      let key = if says.width() + 2 + key.width() <= room { key } else { "" };
      rows.push(self.said(&says, prose, key));
      rows.push(Vec::new());
      let alone = self.alone();
      if !alone.is_empty() {
        rows.push(self.said("While no window was open", prose, ""));
        rows.extend(alone.iter().map(|(left, right)| self.said(left, muted, right)));
        rows.push(Vec::new());
      }
    } else if self.known() {
      rows.extend(wrapped("No thread yet.", prose));
      let project = self.project();
      let how = "Write below to start one. It runs in a new session of the project";
      rows.extend(wrapped(&format!("{how} {project}, on this machine."), prose));
    }
    for told in &self.machines {
      if let Link::Lost(why) = &told.link {
        rows.extend(wrapped(&format!("{} is offline: {why}", told.name), muted));
      }
    }
    let keys = [
      ("ctrl+o", "go to a project, a session or a machine"),
      ("ctrl+p", "commands"),
      ("ctrl+x ?", "each key"),
    ];
    let key = |(key, does): (&str, &str)| {
      let line = vec![run(format!("  {key:<10}"), prose), run(does, muted)];
      spread(line, Vec::new(), room + 2, Style::new())
    };
    rows.extend(keys.map(key));
    rows
  }

  /// The input of a new session, which sends nothing yet, the row under it, and the row of hints.
  fn hinted(&self) -> [Line; 3] {
    let t = &self.tokens;
    let target = format!("+ new session in {}", self.project());
    let [ask, under] = self.input(
      t.accent,
      "Ask for a change, a review or an answer",
      (vec![run(target, t.style(Role::Muted))], Vec::new()),
    );
    let hint = if self.listed() { "ctrl+p commands" } else { "ctrl+x l the list" };
    let hints = match &self.fault {
      Some(fault) => self.said(fault, Style::new().fg(t.danger), hint),
      None => self.said(&self.folder_shown(), t.style(Role::Muted), hint),
    };
    [ask, under, hints]
  }

  /// The input, which sends nothing yet: a row with a bar in a color and what it asks, then a row
  /// that says where it goes, at its left and at its right.
  pub(crate) fn input(&self, bar: Color, ask: &str, (left, right): (Line, Line)) -> [Line; 2] {
    let t = &self.tokens;
    let raised = Style::new().bg(t.raised);
    let on = |line: Line| line.into_iter().map(|one| run(one.content, one.style.bg(t.raised)));
    let input = |text: Line, right: Line| {
      let mut line = vec![run("┃", Style::new().fg(bar).bg(t.raised)), run(" ", raised)];
      line.extend(spread(
        on(text).collect(),
        on(right).collect(),
        self.wide().saturating_sub(4),
        raised,
      ));
      line.push(run("  ", raised));
      line
    };
    let cursor = run("▌", t.style(Role::Prose));
    [input(vec![cursor, run(ask, t.style(Role::Muted))], Vec::new()), input(left, right)]
  }

  /// The folder of the window, with the home folder of the user as `~`.
  pub(crate) fn folder_shown(&self) -> String {
    let under =
      |home| self.folder.strip_prefix(home).ok().map(|rest| format!("~/{}", rest.display()));
    env::home_dir().and_then(under).unwrap_or_else(|| self.folder.display().to_string())
  }

  /// Each session that works with no view, since when, and each thread that the operator paused,
  /// as the left and the right of a row of the home. A machine that did not answer says neither:
  /// the window does not know what works there now.
  fn alone(&self) -> Vec<(String, String)> {
    let mut rows: Vec<(String, String)> = Vec::new();
    let mut lives: Vec<&str> = Vec::new();
    for (_, key, card) in &self.settled {
      let named = card.session.clone().filter(|name| !name.is_empty());
      let name = named.unwrap_or_else(|| card.thread.title.clone());
      let works = unheld(card) && card.thread.works() && self.confirmed(key);
      if works && !lives.contains(&card.life.as_str()) {
        lives.push(&card.life);
        let machine =
          if key.0 > 0 { format!("@{}", self.machines[key.0].name) } else { String::new() };
        let left = format!("◌ {name} · {}{machine}", project(card));
        rows.push((left, format!("works since {}", self.hour(card.since))));
      }
      if matches!(&card.thread.standing, Standing::Paused { .. }) && !self.needs(key, card) {
        rows.push((format!("‖ {name}"), format!("paused at {}", self.hour(card.since))));
      }
    }
    rows
  }
}
