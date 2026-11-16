//! The home, at the left of the list while no thread is open: what waits for the operator, and
//! what worked while no window was open.

use std::{iter, path::Path};

use crate::cells::Cells;
use furb::view::Standing;
use furb_client::Target;
use opentui::style::Style;

use crate::{
  keys::{self, Does},
  line::{self, Line, run, spread},
  list::{NO_PROJECT, project},
  model::{Link, Model},
  thread::{Click, Row, width},
  tokens::{KEYS, Role},
};

impl Model {
  /// The rows of the home, above the input.
  pub(crate) fn home(&self) -> Vec<Row> {
    let mut rows: Vec<Row> = self.says().into_iter().map(Row::of).collect();
    // A key of the home does on a click what it does on its key.
    if rows.len() > 2 {
      rows.extend([Does::GoTo, Does::Keys].map(|does| self.key_row(does)));
    }
    // What the home says stands just above the input, where the eye goes next.
    let room = usize::from(self.size.1).saturating_sub(self.input_rows() + 1);
    let pad = room.saturating_sub(rows.len());
    rows.splice(2..2, iter::repeat_n(Row::default(), pad));
    rows
  }

  /// A row of the home that names a key and what it does, which a click does too.
  fn key_row(&self, does: Does) -> Row {
    let (prose, muted) = (self.tokens.style(Role::Prose), self.tokens.style(Role::Muted));
    let key = run(format!("{:<KEYS$}", self.key(does)), prose);
    let line = vec![run("  ", Style::new()), key, run(keys::says(does), muted)];
    let hot = vec![(2..width(&line), Some(Click::Key(does)))];
    Row { hot, ..Row::of(line) }
  }

  /// The name of the project of the folder of the window.
  pub(crate) fn project(&self) -> String {
    Path::new(&self.client.folder)
      .file_name()
      .map(|name| name.to_string_lossy().into_owned())
      .unwrap_or_default()
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
  /// is lost with its cause, and the keys that lead out. It says nothing before it knows what
  /// stands, or before the patience of a dial passed, so no row moves when the machine answers.
  fn says(&self) -> Vec<Line> {
    let t = &self.tokens;
    let (prose, muted) = (t.style(Role::Prose), t.style(Role::Muted));
    let room = self.wide().saturating_sub(4);
    let wrapped = |text: &str, style: Style| {
      let lines = line::wrapped(text, room);
      lines.iter().map(|line| self.said(line, style, "")).chain([Vec::new()]).collect::<Vec<_>>()
    };
    let mut rows = vec![vec![run("  furb", t.style(Role::Title))], Vec::new()];
    if self.client.settled.is_empty() && !self.known() && !self.slow {
      return rows;
    }
    if !self.client.settled.is_empty() {
      rows.extend(self.waiting(room));
      rows.extend(self.running());
      rows.push(Vec::new());
      let alone = self.alone();
      if !alone.is_empty() {
        rows.push(self.said("While no window was open", prose, ""));
        rows.extend(alone.iter().map(|(left, right)| self.said(left, muted, right)));
        rows.push(Vec::new());
      }
    } else if let Some(reads) = self.client.only_reads(0) {
      rows.extend(wrapped(&format!("{reads}."), prose));
      rows.pop();
      rows.extend(wrapped("It shows each session live once another window starts one.", muted));
    } else if self.known() {
      rows.extend(wrapped("Write below to start a thread.", prose));
      rows.pop();
      let (place, actor) = self.starts();
      // A sentence that takes two rows breaks at its comma, so the model and its effort read as one
      // part and no word of it stands alone on a row.
      let said = match actor {
        Some(actor) if place.width() + actor.width() + 3 > room => {
          let mut rows = wrapped(&format!("{place},"), muted);
          rows.pop();
          rows.extend(wrapped(&format!("{actor}."), muted));
          rows
        }
        Some(actor) => wrapped(&format!("{place}, {actor}."), muted),
        None => wrapped(&format!("{place}."), muted),
      };
      rows.extend(said);
    }
    for told in &self.client.machines {
      if let Link::Lost(why) = &told.link {
        let says = format!("{} is offline: {why}. The window dials it again.", told.name);
        rows.extend(wrapped(&says, muted));
      }
    }
    rows
  }

  /// What waits for the operator: how many threads need him, with the key that goes to one, and
  /// how many answers he did not read, whose cards bear the mark of a new answer. Such an answer
  /// asks for his reading and for no act, so the home names it apart from what needs him.
  fn waiting(&self, room: usize) -> Vec<Line> {
    let t = &self.tokens;
    let needs = self.client.settled.iter().filter(|one| self.needs(&one.1, &one.2)).count();
    let needed = self.key(Does::Needed);
    let (says, key) = match needs {
      0 => ("Nothing needs you.".to_owned(), String::new()),
      1 => ("1 thread needs you.".to_owned(), format!("{needed} goes to it")),
      count => (format!("{count} threads need you."), format!("{needed} goes to one")),
    };
    // The key shows where the row holds the two whole.
    let key = if says.width() + 2 + key.width() <= room { key } else { String::new() };
    let mut rows = vec![self.said(&says, t.style(Role::Prose), &key)];
    let unread = self.client.settled.iter().filter(|(_, key, card)| {
      !self.needs(key, card) && card.thread.unread(&self.client.machines[key.0].login)
    });
    let new = match unread.count() {
      0 => None,
      1 => Some("1 answer is new.".to_owned()),
      count => Some(format!("{count} answers are new.")),
    };
    rows.extend(new.map(|new| self.said(&new, t.style(Role::Muted), "")));
    rows
  }

  /// Where a thread that the operator starts here runs, and the model that answers it with its
  /// effort, none before the window knows it.
  fn starts(&self) -> (String, Option<String>) {
    let actor = self.maker().map(|(model, effort)| match effort {
      Some(effort) => format!("with {model} at {effort} effort"),
      None => format!("with {model}"),
    });
    let (project, at) = match self.client.target() {
      Target::New(at, folder) => (self.folder_named(at, &folder), at),
      _ => (self.project(), 0),
    };
    let machine = if at == 0 { "this computer" } else { &self.client.machines[at].name };
    let session = if project == NO_PROJECT {
      "a new session with no project".to_owned()
    } else {
      format!("a new session of {project}")
    };
    let place = format!("It runs in {session} on {machine}");
    (place, actor)
  }

  /// A folder of a machine with the home folder of its user as `~`, and `/` between its parts on
  /// each system, so a folder of Windows shows as `~/furb` too: the home of the window for this
  /// computer, and the home that its daemon told for another. A folder outside the home shows as it
  /// is.
  pub(crate) fn folder_shown(&self, at: usize, folder: &str) -> String {
    let told = self.client.machines.get(at).and_then(|one| one.home.as_deref());
    let here = self.home.as_ref().map(|one| one.display().to_string());
    let home = if at == 0 { here.as_deref() } else { told };
    shown_folder(folder, told, home)
  }

  /// The row that counts the sessions whose life runs, behind the mark of their cards, and those of
  /// them that no window views; none while no life runs.
  fn running(&self) -> Option<Line> {
    let mut lives: Vec<(usize, &str, bool)> = self
      .client
      .settled
      .iter()
      .filter(|(_, key, card)| self.live(key, card))
      .map(|(_, key, card)| (key.0, card.life.as_str(), self.unheld(key.0, card)))
      .collect();
    lives.sort_unstable();
    lives.dedup();
    let alone = lives.iter().filter(|one| one.2).count();
    let says = match (lives.len(), alone) {
      (0, _) => return None,
      (1, 0) => "1 session runs.".to_owned(),
      (1, _) => "1 session runs, with no window.".to_owned(),
      (count, 0) => format!("{count} sessions run."),
      (count, alone) => format!("{count} sessions run, {alone} with no window."),
    };
    let muted = self.tokens.style(Role::Muted);
    Some(vec![
      run("  ", Style::new()),
      run("● ", Style::new().fg(self.tokens.live)),
      run(says, muted),
    ])
  }

  /// Each session that works with no view, since when, and each thread that the operator paused,
  /// as the left and the right of a row of the home. A machine that did not answer says neither:
  /// the window does not know what works there now.
  fn alone(&self) -> Vec<(String, String)> {
    let mut rows: Vec<(String, String)> = Vec::new();
    let mut lives: Vec<&str> = Vec::new();
    for (_, key, card) in &self.client.settled {
      let named = card.session.clone().filter(|name| !name.is_empty());
      let name = named.unwrap_or_else(|| card.thread.title.clone());
      let works = self.unheld(key.0, card) && card.thread.works() && self.confirmed(key);
      if works && !lives.contains(&card.life.as_str()) {
        lives.push(&card.life);
        let machine =
          if key.0 > 0 { format!("@{}", self.client.machines[key.0].name) } else { String::new() };
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

/// A folder as a machine told it, shown from a home: a folder that its machine told from its own
/// home, as the files of the World tell each path, as `~\furb` on Windows, stands from the home
/// that the machine told first, so it shows from the home of the window as any other folder.
fn shown_folder(folder: &str, told: Option<&str>, home: Option<&str>) -> String {
  let tilded =
    folder.strip_prefix('~').filter(|rest| rest.is_empty() || rest.starts_with(['/', '\\']));
  let whole = match (tilded, told) {
    (Some(rest), Some(told)) => format!("{told}{rest}"),
    _ => folder.to_owned(),
  };
  let from = |home: &str| {
    let rest = whole.strip_prefix(home)?;
    (rest.is_empty() || rest.starts_with(['/', '\\']))
      .then(|| format!("~{}", rest.replace('\\', "/")))
  };
  match home.and_then(from) {
    Some(shown) => shown,
    None if whole.starts_with('~') => whole.replace('\\', "/"),
    None => whole,
  }
}

#[cfg(test)]
#[path = "home.test.rs"]
mod tests;
