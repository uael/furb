//! The list part: one list over the dimmed window, with a chosen row, which Go to, the commands,
//! each key and the sessions that the input sends to draw alike. Letters filter it, as the pickers
//! of a terminal match them, the up and down arrows move the chosen row, enter takes it, and escape
//! closes it and gives the focus back to the input. The pointer does the same: a click takes a row,
//! and a click beside the part closes it.

use furb_host::protocol::Card;
use nucleo_matcher::{
  Config, Matcher,
  pattern::{CaseMatching, Normalization, Pattern},
};
use opentui::style::Style;

use crate::{
  input::Target,
  keys::{self, Bound, Does},
  line::{Line, run, spread},
  list::{hash, project},
  model::{Key, Link, Model},
  thread::{Click, Row},
  tokens::{KEYS, OVER, PART, Role},
};

/// What a list part lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pick {
  /// What needs the operator, the sessions that work and the projects, and each session of each
  /// machine once he searches.
  GoTo,
  /// Each command of the table of the keys.
  Commands,
  /// Each key of the table of the keys.
  Keys,
  /// The sessions that the input can send to.
  Sessions,
}

/// The list part, while it is open: what it lists, the text that filters it, and the place of the
/// chosen row among those that pass the filter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Picker {
  pub(crate) of: Pick,
  pub(crate) filter: String,
  pub(crate) at: usize,
}

/// What a row of a list part takes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum Choice {
  /// The thread of a card, which opens.
  Thread(Key),
  /// A session: its last thread opens, or the input sends to a new thread of it.
  Session(Target),
  /// A project, whose sessions the list then shows.
  Project(String),
  /// What a key does.
  Key(Does),
  /// Nothing: a key that only a form or a list hears.
  #[default]
  None,
}

/// A row of a list part: the part of the list that it stands in, its text, what follows its text
/// in the column after it, what its right says, the hue of its bar while it is chosen, and what it
/// takes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Entry {
  pub(crate) part: &'static str,
  pub(crate) left: String,
  pub(crate) says: String,
  pub(crate) right: String,
  pub(crate) hue: Option<usize>,
  pub(crate) choice: Choice,
}

/// The list part as the window draws it over the dimmed window: its column, its row, its columns,
/// and its rows, each row of an entry with the click that takes it.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Over {
  pub(crate) x: u16,
  pub(crate) y: u16,
  pub(crate) width: u16,
  pub(crate) rows: Vec<Row>,
}

/// The rows of a list part that are no entry: the blank row above its title, its title, its filter,
/// the blank row under it, the blank row above its foot, its foot and the blank row under it.
const FRAME: usize = 7;

impl Pick {
  fn title(self) -> &'static str {
    match self {
      Self::GoTo => "Go to",
      Self::Commands => "Commands",
      Self::Keys => "Each key",
      Self::Sessions => "Send to",
    }
  }

  fn placeholder(self) -> &'static str {
    match self {
      Self::GoTo => "Search a thread, a session, a project",
      Self::Commands => "Search a command",
      Self::Keys => "Search a key",
      Self::Sessions => "Search a session",
    }
  }

  /// What enter does on a row, as the foot of the part says it.
  fn takes(self) -> &'static str {
    match self {
      Self::GoTo => "open",
      Self::Commands | Self::Keys => "run",
      Self::Sessions => "send there",
    }
  }
}

impl Model {
  /// A list part opened, or closed when the part that is open lists the same. The sessions open on
  /// the one that the input sends to.
  pub(crate) fn pick(&mut self, of: Pick) {
    if self.picker.as_ref().is_some_and(|one| one.of == of) {
      self.picker = None;
      return;
    }
    self.picker = Some(Picker { of, filter: String::new(), at: 0 });
    if of == Pick::Sessions {
      let now = match self.target() {
        Target::Thread(key) => Target::Session(key.0, key.1),
        other => other,
      };
      let at = self.entries().iter().position(|one| one.choice == Choice::Session(now.clone()));
      if let Some(picker) = self.picker.as_mut() {
        picker.at = at.unwrap_or_default();
      }
    }
  }

  /// The rows of the open list part that pass its filter: all of them in their order while no text
  /// filters it, else those that match the text, the best match first.
  pub(crate) fn entries(&self) -> Vec<Entry> {
    let Some(picker) = &self.picker else { return Vec::new() };
    let all = match picker.of {
      Pick::GoTo => self.places(!picker.filter.is_empty()),
      Pick::Commands => self.commands(),
      Pick::Keys => self.each_key(),
      Pick::Sessions => self.sessions(),
    };
    if picker.filter.is_empty() {
      return all;
    }
    let pattern = Pattern::parse(&picker.filter, CaseMatching::Ignore, Normalization::Smart);
    let mut matcher = Matcher::new(Config::DEFAULT);
    let texts = all.iter().enumerate().map(|(at, one)| (at, format!("{} {}", one.left, one.right)));
    let texts: Vec<(usize, String)> = texts.collect();
    let haystacks = texts.iter().map(|(at, text)| Haystack(*at, text));
    let ranked = pattern.match_list(haystacks, &mut matcher);
    let mut all: Vec<Option<Entry>> = all.into_iter().map(Some).collect();
    ranked.into_iter().filter_map(|(one, _)| all[one.0].take()).collect()
  }

  /// The places of Go to: each thread that needs the operator, each session that works, then each
  /// project; and while he searches, each session of each machine, a session that holds no thread
  /// of his too.
  fn places(&self, searches: bool) -> Vec<Entry> {
    let needs = self.settled.iter().filter(|one| self.needs(&one.1, &one.2));
    let mut all: Vec<Entry> = needs
      .map(|(_, key, card)| Entry {
        part: "Needs you",
        left: card.thread.title.clone(),
        right: place(card),
        hue: Some(hash(&key.2)),
        choice: Choice::Thread(key.clone()),
        ..Entry::default()
      })
      .collect();
    let sessions = self.known_sessions();
    let part = if searches { "Sessions" } else { "Sessions that work" };
    all.extend(sessions.iter().filter(|one| one.works || searches).map(|one| Entry {
      part,
      left: one.name.clone(),
      right: one.project.clone(),
      choice: Choice::Session(Target::Session(one.at, one.life.clone())),
      ..Entry::default()
    }));
    let mut projects: Vec<(&str, usize)> = Vec::new();
    for one in sessions.iter().filter(|one| !one.project.is_empty()) {
      if let Some(project) = projects.iter_mut().find(|project| project.0 == one.project) {
        project.1 += 1;
      } else {
        projects.push((&one.project, 1));
      }
    }
    all.extend(projects.into_iter().map(|(name, count)| Entry {
      part: "Projects",
      left: name.to_owned(),
      right: format!("{count} session{}", if count == 1 { "" } else { "s" }),
      choice: Choice::Project(name.to_owned()),
      ..Entry::default()
    }));
    all
  }

  /// Each session of each machine once: those of its cards, then each record that it listed with
  /// no card, by its name, else its file.
  fn known_sessions(&self) -> Vec<Session> {
    let mut sessions: Vec<Session> = Vec::new();
    for (at, told) in self.machines.iter().enumerate() {
      for card in &told.cards {
        let works = card.thread.works() && told.link == Link::Listed;
        if let Some(one) = sessions.iter_mut().find(|one| one.at == at && one.life == card.life) {
          one.works |= works;
        } else {
          let named = card.session.clone().filter(|name| !name.is_empty());
          let name = named.unwrap_or_else(|| card.thread.title.clone());
          let life = card.life.clone();
          sessions.push(Session { at, life, name, project: project(card), works });
        }
      }
      for listed in &told.lives {
        if !sessions.iter().any(|one| one.at == at && one.life == listed.life) {
          let name = listed.name.clone().filter(|name| !name.is_empty());
          let name = name.unwrap_or_else(|| stem(&listed.life));
          let project = listed.workspace.as_deref().map_or("", |one| tail(one)).to_owned();
          let life = listed.life.clone();
          sessions.push(Session { at, life, name, project, works: false });
        }
      }
    }
    sessions
  }

  /// Each command of the table of the keys, with its key, by the group of its key.
  fn commands(&self) -> Vec<Entry> {
    let commands = keys::each(self.system).filter(|one| one.1.command);
    let entry = |(group, key, forms): (&'static str, &Bound, Vec<String>)| Entry {
      part: group,
      left: capital(key.says),
      right: forms.first().cloned().unwrap_or_default(),
      hue: None,
      choice: Choice::Key(key.does),
      ..Entry::default()
    };
    commands.map(entry).collect()
  }

  /// Each key of the table of the keys, with each of its forms, by its group: the forms in the
  /// column of the keys, then what it does, as the home names a key. A command runs from it.
  fn each_key(&self) -> Vec<Entry> {
    let entry = |(group, key, forms): (&'static str, &Bound, Vec<String>)| {
      let forms = forms.join(" or ");
      Entry {
        part: group,
        left: format!("{forms:<KEYS$}"),
        says: key.says.to_owned(),
        choice: if key.command { Choice::Key(key.does) } else { Choice::None },
        ..Entry::default()
      }
    };
    keys::each(self.system).map(entry).collect()
  }

  /// The sessions that the input can send to: a new session on this machine, then each session
  /// of the cards once, by its machine and its record, with its name and its project.
  fn sessions(&self) -> Vec<Entry> {
    let new = Choice::Session(Target::New(0));
    let left = "+ new session".to_owned();
    let mut all = vec![Entry { left, right: self.project(), choice: new, ..Entry::default() }];
    for (_, key, card) in &self.settled {
      let choice = Choice::Session(Target::Session(key.0, key.1.clone()));
      if all.iter().any(|one| one.choice == choice) {
        continue;
      }
      let named = card.session.clone().filter(|name| !name.is_empty());
      let left = named.unwrap_or_else(|| card.thread.title.clone());
      all.push(Entry {
        part: "",
        left,
        right: project(card),
        hue: None,
        choice,
        ..Entry::default()
      });
    }
    all
  }

  /// The chosen row moved by a count of rows, within the rows that pass the filter.
  pub(crate) fn chose(&mut self, by: isize) {
    let count = self.entries().len();
    if let Some(picker) = self.picker.as_mut() {
      picker.at = picker.at.saturating_add_signed(by).min(count.saturating_sub(1));
    }
  }

  /// The text that filters the open list part changed: the first row that passes it is chosen.
  pub(crate) fn filtered(&mut self, change: impl FnOnce(&mut String)) {
    if let Some(picker) = self.picker.as_mut() {
      change(&mut picker.filter);
      picker.at = 0;
    }
  }

  /// The row at a place taken, and the part closed, or kept with a filter for a project. A key
  /// that a row names is done by the hand, which this gives.
  pub(crate) fn took(&mut self, at: usize) -> Option<Does> {
    let entry = self.entries().into_iter().nth(at)?;
    let of = self.picker.as_ref()?.of;
    self.picker = None;
    match entry.choice {
      Choice::Thread(key) => self.went(key),
      Choice::Session(target) if of == Pick::Sessions => self.aimed(target),
      Choice::Session(Target::Session(at, life)) => {
        let thread =
          |one: &&(_, Key, Card)| one.1.0 == at && one.1.1 == life && !one.1.2.is_empty();
        let last = self.settled.iter().rev().find(thread);
        match last.map(|one| one.1.clone()) {
          Some(key) => self.went(key),
          None => self.aimed(Target::Session(at, life)),
        }
      }
      Choice::Session(target) => self.aimed(target),
      Choice::Project(name) => {
        self.picker = Some(Picker { of, filter: name, at: 0 });
      }
      Choice::Key(does) => return Some(does),
      Choice::None => self.picker = Some(Picker { of, filter: String::new(), at }),
    }
    None
  }

  /// The thread of a card opened in the window.
  fn went(&mut self, key: Key) {
    self.selected = Some(key);
    self.open_thread();
  }

  /// The input turned to a target: none for the thread that is open, or for a new session when no
  /// thread is.
  fn aimed(&mut self, target: Target) {
    let open = self.thread.as_ref().map(|one| (one.key.0, one.key.1.clone()));
    self.aim = match target {
      Target::Session(at, life) if open == Some((at, life.clone())) => None,
      Target::New(_) if self.thread.is_none() => None,
      other => Some(other),
    };
  }

  /// The open list part as the window draws it, at the middle of the window and under the rows
  /// that stay above it, with as many of its rows as the window holds, the chosen one among them.
  pub(crate) fn over(&self) -> Option<Over> {
    let picker = self.picker.as_ref()?;
    let t = &self.tokens;
    let (muted, prose) = (t.style(Role::Muted), t.style(Role::Prose));
    let width = PART.min(self.size.0.saturating_sub(4));
    let room = usize::from(width).saturating_sub(4);
    let padded = |line: Line| {
      let mut row = vec![run("  ", Style::new())];
      row.extend(line);
      row.push(run("  ", Style::new()));
      row
    };
    let entries = self.entries();
    let mut rows = vec![Row::default()];
    let close = vec![run(self.key(Does::Close), muted)];
    let title = vec![run(picker.of.title(), t.style(Role::Title))];
    rows.push(Row::of(padded(spread(title, close, room, Style::new()))));
    let filter = if picker.filter.is_empty() {
      run(picker.of.placeholder(), muted)
    } else {
      run(picker.filter.clone(), prose)
    };
    let count = match entries.len() {
      0 => "nothing".to_owned(),
      many => format!("{} / {many}", picker.at + 1),
    };
    let filter = vec![run("▌", Style::new().fg(t.accent)), filter];
    rows.push(Row::of(padded(spread(filter, vec![run(count, muted)], room, Style::new()))));
    rows.push(Row::default());
    let fits = usize::from(self.size.1).saturating_sub(2 * usize::from(OVER) + FRAME).max(1);
    let shown = self.shown_entries(&entries, picker, fits);
    for (at, line) in shown {
      let Some(at) = at else {
        rows.push(Row::of(padded(line)));
        continue;
      };
      let clicked = (2..2 + room, Some(Click::Pick(at)));
      rows.push(Row { hot: vec![clicked], ..Row::of(padded(line)) });
    }
    rows.push(Row::default());
    let foot = format!(
      "{} move · {} {} · {} close",
      [Does::Up, Does::Down].map(|one| self.key(one)).join(""),
      self.key(Does::Send),
      picker.of.takes(),
      self.key(Does::Close),
    );
    rows.push(Row::of(padded(vec![run(foot, muted)])));
    rows.push(Row::default());
    let x = self.size.0.saturating_sub(width) / 2;
    Some(Over { x, y: OVER, width, rows })
  }

  /// The rows of the entries that the part shows: a head before the first entry of each part of the
  /// list while no text filters it, and each entry, the chosen one on the lifted ground with a bar
  /// in the hue of its thread, as the selected card of the list. A row of an entry carries its
  /// place. The rows start at the one that keeps the chosen entry in view.
  fn shown_entries(
    &self,
    entries: &[Entry],
    picker: &Picker,
    fits: usize,
  ) -> Vec<(Option<usize>, Line)> {
    let t = &self.tokens;
    let (muted, prose) = (t.style(Role::Muted), t.style(Role::Prose));
    let room = usize::from(PART.min(self.size.0.saturating_sub(4))).saturating_sub(4);
    let mut all: Vec<(Option<usize>, Line)> = Vec::new();
    let mut part = "";
    for (at, entry) in entries.iter().enumerate() {
      if picker.filter.is_empty() && entry.part != part && !entry.part.is_empty() {
        if !all.is_empty() {
          all.push((None, Vec::new()));
        }
        all.push((None, vec![run(entry.part, muted)]));
        part = entry.part;
      }
      let chosen = at == picker.at;
      let ground = |style: Style| if chosen { style.bg(t.lifted) } else { style };
      let hue = entry.hue.map_or(t.accent, |hue| t.hues[hue % t.hues.len()]);
      let bar =
        if chosen { run("▌", ground(Style::new().fg(hue))) } else { run(" ", Style::new()) };
      let left =
        vec![bar, run(entry.left.clone(), ground(prose)), run(entry.says.clone(), ground(muted))];
      let right = vec![run(entry.right.clone(), ground(muted)), run(" ", ground(Style::new()))];
      all.push((Some(at), spread(left, right, room, ground(Style::new()))));
    }
    let chosen = all.iter().position(|one| one.0 == Some(picker.at)).unwrap_or_default();
    let top = (chosen + 1).saturating_sub(fits);
    let mut shown: Vec<(Option<usize>, Line)> = all.into_iter().skip(top).take(fits).collect();
    // A head or a blank row that the room cuts from the entries under it does not show.
    while shown.last().is_some_and(|one| one.0.is_none()) {
      shown.pop();
    }
    shown
  }

  /// The open list part as the window draws it: its column, its row and its columns, and its
  /// lines, the text of the row under the pointer bright.
  pub(crate) fn drawn_over(&self) -> Option<(u16, u16, u16, Vec<Line>)> {
    let over = self.over()?;
    let lit = |(y, row): (usize, Row)| match self.pointer {
      Some((x, at)) if usize::from(at) == usize::from(over.y) + y && x >= over.x => {
        row.lit(usize::from(x - over.x), self.tokens.hover).line
      }
      _ => row.line,
    };
    let lines = over.rows.into_iter().enumerate().map(lit).collect();
    Some((over.x, over.y, over.width, lines))
  }

  /// A click on a cell of the window while a list part is open: a row of the part takes its entry,
  /// and a cell beside the part closes it. The key that the taken row names.
  pub(crate) fn clicked_over(&mut self, x: u16, y: u16) -> Option<Does> {
    let over = self.over()?;
    let inside = (over.x..over.x + over.width).contains(&x) && y >= over.y;
    let row = inside.then(|| over.rows.get(usize::from(y - over.y))).flatten();
    if let Some(row) = row {
      return match row.click(usize::from(x - over.x)) {
        Some(Click::Pick(at)) => self.took(at),
        _ => None,
      };
    }
    self.picker = None;
    None
  }
}

/// A session of a machine: the machine, the record, its name, its project, and whether a thread of
/// it works.
struct Session {
  at: usize,
  life: String,
  name: String,
  project: String,
  works: bool,
}

/// The text that a matcher reads of an entry, with its place.
struct Haystack<'a>(usize, &'a String);

impl AsRef<str> for Haystack<'_> {
  fn as_ref(&self) -> &str {
    self.1
  }
}

/// Where a card stands: its project, and its session when it has a name.
fn place(card: &Card) -> String {
  let named = card.session.as_ref().filter(|name| !name.is_empty() && **name != card.thread.title);
  named.map_or_else(|| project(card), |name| format!("{} · {name}", project(card)))
}

/// The name of a record with no name: its file, with no extension.
fn stem(life: &str) -> String {
  let name = life.rsplit(['/', '\\']).next().unwrap_or(life);
  name.strip_suffix(".jsonl").unwrap_or(name).to_owned()
}

/// The last part of a path of a folder.
fn tail(path: &str) -> &str {
  path.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).next().unwrap_or(path)
}

/// A text with its first letter in upper case.
fn capital(text: &str) -> String {
  let mut chars = text.chars();
  chars.next().map(|first| first.to_uppercase().chain(chars).collect()).unwrap_or_default()
}
