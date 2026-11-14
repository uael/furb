//! The home, at the left of the list while no thread is open: what waits for the operator, and
//! what worked while no window was open.

use furb::view::Standing;
use opentui::style::Style;
use unicode_width::UnicodeWidthStr;

use crate::{
  keys::Does,
  line::{self, Line, run, spread},
  list::{project, unheld},
  model::{Link, Model},
  thread::Row,
  tokens::Role,
};

impl Model {
  /// The rows of the home, above the input.
  pub(crate) fn home(&self) -> Vec<Row> {
    self.says().into_iter().map(Row::of).collect()
  }

  /// The name of the project of the folder of the window.
  pub(crate) fn project(&self) -> String {
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
      let needed = self.key(Does::Needed);
      let (says, key) = match needs {
        0 => ("Nothing needs you.".to_owned(), String::new()),
        1 => ("1 thread needs you.".to_owned(), format!("{needed} goes to it")),
        count => (format!("{count} threads need you."), format!("{needed} goes to one")),
      };
      // The key shows where the row holds the two whole.
      let key = if says.width() + 2 + key.width() <= room { key } else { String::new() };
      rows.push(self.said(&says, prose, &key));
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
      (Does::GoTo, "go to a project, a session or a machine"),
      (Does::Commands, "commands"),
      (Does::Keys, "each key"),
    ];
    let key = |(key, does): (Does, &str)| {
      let line = vec![run(format!("  {:<10}", self.key(key)), prose), run(does, muted)];
      spread(line, Vec::new(), room + 2, Style::new())
    };
    rows.extend(keys.map(key));
    rows
  }

  /// The folder of the window, with the home folder of the user as `~`, and `/` between its parts
  /// on each system, so a folder of Windows shows as `~/furb` too. A folder outside the home shows
  /// as it is.
  pub(crate) fn folder_shown(&self) -> String {
    let shown = self.folder.display().to_string();
    let home = self.home.as_ref().map(|one| one.display().to_string());
    match home.as_deref().and_then(|home| shown.strip_prefix(home)) {
      Some("") => "~".to_owned(),
      Some(rest) if rest.starts_with('/') => format!("~{rest}"),
      Some(rest) if rest.starts_with('\\') => format!("~{}", rest.replace('\\', "/")),
      _ => shown,
    }
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
