//! The list part: one list over the dimmed window, with a chosen row, which Go to, each key and
//! the sessions that the input sends to draw alike. Letters filter it, as the pickers
//! of a terminal match them, the up and down arrows move the chosen row, enter takes it, and escape
//! closes it and gives the focus back to the input. The pointer does the same: a click takes a row,
//! and a click beside the part closes it.

use std::{borrow::Cow, cmp::Reverse};

use furb::view::{Part, Sent, Skill};
use furb_host::protocol::Card;
use opentui::style::Style;
use unicode_width::UnicodeWidthStr;

use furb_client::{Client, Control, Link, Matched, Target, Told, ranked};
use furb_host::protocol::Model as Offered;

use crate::{
  input::named,
  keys::{self, Bound, Does},
  line::{Line, capital, run, spread, wrapped},
  list::project,
  model::{Key, Model},
  thread::{Click, Row, hued},
  tokens::{BESIDE, OVER, PART, PARTS, Role, SIDE},
};

/// What a list part lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pick {
  /// Everything that the operator goes to or runs: what needs him, the sessions that work, the
  /// sessions that he used last, the projects and each command, and once he searches, each thread,
  /// each session of each machine, and the files and the skills of the target of the input.
  GoTo,
  /// Each key of the table of the keys.
  Keys,
  /// The sessions that the input can send to.
  Sessions,
  /// The machines that a new session runs on.
  Machines,
  /// The projects of the machine of the input, where a new session runs.
  Projects,
  /// The models of the machine of the input, each provider once.
  Models,
  /// What he sent where the input sends, the newest first.
  History,
  /// What the operator does to one session that runs.
  Session,
  /// What a leave does to the sessions that the window holds and that work.
  Leave,
  /// The settings of the session that the input sends to.
  Settings,
  /// The value of a setting, by its place among the settings, which he writes in the filter.
  Value(usize),
  /// The skills of the session that the input sends to, with how each plays.
  Skills,
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
  /// A session that the input sends to a new thread of.
  Session(Target),
  /// A session that the operator returns to, by its machine and its record: its last thread
  /// opens, else the input sends to a new thread of it.
  Visit(usize, String),
  /// A text that the draft adds at its end: the mention of a file, or a skill.
  Added(String),
  /// A project, whose sessions the list then shows.
  Project(String),
  /// An actor of a model, which the target of the input takes.
  Actor(String),
  /// A text that he sent, which the input takes.
  Sent(Sent),
  /// What a key does.
  Key(Does),
  /// A session that runs, by its machine and its record, whose choices the list then shows.
  Running(usize, String),
  /// What the operator does to the session that the list part acts on.
  Control(Control),
  /// What a leave does to each session that works, or none to stay.
  Leave(Option<Control>),
  /// A setting, by its place among the settings, which takes one step, or the value that he writes.
  Setting(usize),
  /// The value that he wrote for a setting, by its place among the settings.
  Value(usize),
  /// A skill, by its place among the skills of the session: one with code turns on or off, and the
  /// draft adds the mention of one with none.
  Skill(usize),
  /// Nothing: a key that only a form or a list hears.
  #[default]
  None,
}

/// A row of a list part: the part of the list that it stands in, the project that it stands in,
/// whose badge leads it as it leads a card, its text, the places of the letters of its text that
/// the filter matched, what follows its text in the column after it, what its right says, the hue
/// of its bar while it is chosen, and what it takes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Entry {
  pub(crate) part: Cow<'static, str>,
  pub(crate) project: Option<String>,
  pub(crate) left: String,
  pub(crate) lit: Vec<usize>,
  pub(crate) says: String,
  pub(crate) right: String,
  pub(crate) hue: Option<usize>,
  pub(crate) choice: Choice,
}

impl Entry {
  /// The columns that the entry takes on one row: its bar, the slot of a badge when the list has
  /// badges, its text, what it says, and its right after a gap, with the column after it.
  fn width(&self, badged: bool) -> usize {
    let badge = if badged { 3 } else { 0 };
    1 + badge + self.left.width() + self.says.width() + 2 + self.right.width() + 1
  }
}

/// Whether a row of the entries stands in a project, so each row keeps the slot of a badge.
fn badged(entries: &[Entry]) -> bool {
  entries.iter().any(|one| one.project.is_some())
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

/// How many sessions that he used last Go to shows before he searches.
const RECENT: usize = 5;

/// The rows of a list part that are no entry: the blank row above its title, its title, its filter,
/// the blank row under it, the blank row above its foot, its foot and the blank row under it.
const FRAME: usize = 7;

impl Pick {
  /// Whether each entry of the list is a thing that says what it does, as a setting or an
  /// extension, so what it says stands whole under its name, as a page of settings shows it.
  fn describes(self) -> bool {
    matches!(self, Self::Settings | Self::Value(_) | Self::Skills)
  }

  fn title(self) -> &'static str {
    match self {
      Self::GoTo => "Go to",
      Self::Keys => "Each key",
      Self::Sessions => "Send to",
      Self::Machines => "Machines",
      Self::Projects => "Projects",
      Self::Models => "Models",
      Self::History => "History",
      Self::Session => "The session",
      Self::Leave => "Sessions still work",
      Self::Settings | Self::Value(_) => "Settings",
      Self::Skills => "Extensions",
    }
  }

  fn placeholder(self) -> &'static str {
    match self {
      Self::GoTo => "Search a thread, a session, a project, a file, a skill, a command",
      Self::Keys => "Search a key",
      Self::Sessions => "Search a session",
      Self::Machines => "Search a machine",
      Self::Projects => "Search a project",
      Self::Models => "Search a model",
      Self::History => "Search what you sent here",
      Self::Session | Self::Leave => "Search a choice",
      Self::Settings => "Search a setting",
      Self::Value(_) => "Write the value",
      Self::Skills => "Search a skill or an extension",
    }
  }

  /// What enter does on a row, as the foot of the part says it.
  fn takes(self) -> &'static str {
    match self {
      Self::GoTo => "open",
      Self::Keys => "run",
      Self::Sessions | Self::Machines | Self::Projects => "send there",
      Self::Models => "take it",
      Self::History => "write it",
      Self::Session | Self::Leave => "do it",
      Self::Settings => "change it",
      Self::Value(_) => "set it",
      Self::Skills => "turn it on or off",
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
    // The history opens on what he wrote, as the search of a shell does, and asks what he sent
    // again.
    let filter = if of == Pick::History { self.client.draft.text.clone() } else { String::new() };
    if of == Pick::History {
      self.client.ask_history();
    }
    if matches!(of, Pick::Settings | Pick::Skills)
      && let Err(why) = self.client.read_settings(of == Pick::Skills)
    {
      self.client.notice = Some(why);
      return;
    }
    self.picker = Some(Picker { of, filter, at: 0 });
    if of == Pick::Sessions {
      let now = match self.client.target() {
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
      Pick::Keys => self.each_key(),
      Pick::Sessions => self.sessions(),
      Pick::Machines => self.machines(),
      Pick::Projects => self.projects(),
      Pick::Models => self.models(),
      Pick::History => self.sent(),
      Pick::Session => self.session_choices(),
      Pick::Leave => self.leave_choices(),
      Pick::Settings => self.settings(),
      Pick::Skills => self.played(),
      // The filter is the value that he writes, so it hides no row.
      Pick::Value(place) => return self.value(place),
    };
    // The filter matches the text of a row and its right, and lights the letters of its text.
    let texts: Vec<String> = all.iter().map(|one| format!("{} {}", one.left, one.right)).collect();
    let mut all: Vec<Option<Entry>> = all.into_iter().map(Some).collect();
    let taken = |one: Matched| {
      let mut entry = all[one.at].take()?;
      let letters = entry.left.chars().count();
      entry.lit = one.lit.into_iter().filter(|at| *at < letters).collect();
      Some(entry)
    };
    ranked(&picker.filter, texts.iter().map(String::as_str)).into_iter().filter_map(taken).collect()
  }

  /// The places of Go to: each thread that needs the operator, each thread that works, each
  /// session that runs, the sessions that he used last, each project, then each command; and while
  /// he searches, each other thread, each session of each machine, a session that holds no thread
  /// of his too, and the files and the skills of the target of the input. A thread row opens its
  /// thread, a session row its last thread, a file or a skill row adds it to the draft, and a
  /// command row runs it.
  fn places(&self, searches: bool) -> Vec<Entry> {
    let thread = |part: &'static str, (_, key, card): &(Part, Key, Card)| Entry {
      part: part.into(),
      project: Some(project(card)),
      left: card.thread.title.clone(),
      right: place(card),
      hue: Some(hued(&key.1, &card.thread.title)),
      choice: Choice::Thread(key.clone()),
      ..Entry::default()
    };
    let needs = |one: &&(Part, Key, Card)| self.needs(&one.1, &one.2);
    let works =
      |one: &&(Part, Key, Card)| !needs(one) && one.2.thread.works() && self.confirmed(&one.1);
    let mut all: Vec<Entry> =
      self.client.settled.iter().filter(needs).map(|one| thread("Needs you", one)).collect();
    all.extend(self.client.settled.iter().filter(works).map(|one| thread("Working", one)));
    all.extend(self.running_places());
    let sessions = self.known_sessions();
    let session = |one: &Session| Entry {
      part: "Sessions".into(),
      project: Some(one.project.clone()),
      left: one.name.clone(),
      right: one.project.clone(),
      choice: Choice::Visit(one.at, one.life.clone()),
      ..Entry::default()
    };
    if searches {
      let rest = |one: &&(Part, Key, Card)| !needs(one) && !works(one);
      all.extend(self.client.settled.iter().filter(rest).map(|one| thread("Threads", one)));
      all.extend(sessions.iter().map(session));
      all.extend(self.lists());
    } else {
      all.extend(sessions.iter().take(RECENT).map(session));
    }
    let mut projects: Vec<(&str, usize)> = Vec::new();
    for one in sessions.iter().filter(|one| !one.project.is_empty()) {
      if let Some(project) = projects.iter_mut().find(|project| project.0 == one.project) {
        project.1 += 1;
      } else {
        projects.push((&one.project, 1));
      }
    }
    all.extend(projects.into_iter().map(|(name, count)| Entry {
      part: "Projects".into(),
      project: Some(name.to_owned()),
      left: name.to_owned(),
      right: format!("{count} session{}", if count == 1 { "" } else { "s" }),
      choice: Choice::Project(name.to_owned()),
      ..Entry::default()
    }));
    all.extend(self.commands());
    all
  }

  /// The files and the skills of the target of the input that the client holds: a row adds the
  /// mention of a file, or a skill, to the draft.
  fn lists(&self) -> Vec<Entry> {
    let (files, skills) = self.client.files_and_skills();
    let file = |path: &String| Entry {
      part: "Files".into(),
      left: path.clone(),
      choice: Choice::Added(Client::mention(path)),
      ..Entry::default()
    };
    let skill = |one: &Skill| Entry {
      part: "Skills".into(),
      left: one.name.clone(),
      says: one.description.clone(),
      choice: Choice::Added(format!("/{}", one.name)),
      ..Entry::default()
    };
    files.iter().map(file).chain(skills.iter().map(skill)).collect()
  }

  /// Each session of each machine once: those of its cards, then each record that it listed with
  /// no card, by its name, else its file.
  fn known_sessions(&self) -> Vec<Session> {
    let mut sessions: Vec<Session> = Vec::new();
    for (at, told) in self.client.machines.iter().enumerate() {
      let used = |life: &str| told.lives.iter().find(|one| one.life == life)?.used;
      for card in &told.cards {
        if !sessions.iter().any(|one| one.at == at && one.life == card.life) {
          let named = card.session.clone().filter(|name| !name.is_empty());
          let name = named.unwrap_or_else(|| card.thread.title.clone());
          let life = card.life.clone();
          let used = used(&life);
          sessions.push(Session { at, life, name, project: project(card), used });
        }
      }
      for listed in &told.lives {
        if !sessions.iter().any(|one| one.at == at && one.life == listed.life) {
          let name = listed.name.clone().filter(|name| !name.is_empty());
          let name = name.unwrap_or_else(|| stem(&listed.life));
          let project = listed.project.as_deref().map_or("", |one| tail(one)).to_owned();
          let life = listed.life.clone();
          sessions.push(Session { at, life, name, project, used: listed.used });
        }
      }
    }
    // A session that lasts is a place that he comes back to: the one that he used last first.
    sessions.sort_by_key(|one| Reverse(one.used));
    sessions
  }

  /// Each command of the table of the keys, with its key, by the group of its key, but Go to,
  /// which lists them.
  fn commands(&self) -> Vec<Entry> {
    let commands =
      keys::each(self.system).filter(|one| one.1.command.is_some() && one.1.does != Does::GoTo);
    let entry = |(group, key, forms): (&'static str, &Bound, Vec<String>)| Entry {
      part: group.into(),
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
    // The column of the keys is as wide as the widest of them on this system, where a key in words
    // takes far more columns than its signs, and two columns part it from what the key does.
    let joined = |forms: Vec<String>| forms.join(" or ");
    let column = keys::each(self.system).map(|(_, _, forms)| joined(forms).width()).max();
    let column = column.unwrap_or_default() + 2;
    let entry = |(group, key, forms): (&'static str, &Bound, Vec<String>)| {
      let forms = joined(forms);
      Entry {
        part: group.into(),
        left: format!("{forms}{}", " ".repeat(column.saturating_sub(forms.width()))),
        says: key.says.to_owned(),
        choice: if key.command.is_some() { Choice::Key(key.does) } else { Choice::None },
        ..Entry::default()
      }
    };
    keys::each(self.system).map(entry).collect()
  }

  /// The sessions that the input can send to: a new session on this machine, then each session
  /// of the cards once, by its machine and its record, with its name and its project.
  fn sessions(&self) -> Vec<Entry> {
    let new = Choice::Session(Target::New(0, self.client.folder.clone()));
    let left = "+ new session".to_owned();
    let here = self.project();
    let mut all = vec![Entry {
      project: Some(here.clone()),
      left,
      right: here,
      choice: new,
      ..Entry::default()
    }];
    for (_, key, card) in &self.client.settled {
      let choice = Choice::Session(Target::Session(key.0, key.1.clone()));
      if all.iter().any(|one| one.choice == choice) {
        continue;
      }
      let named = card.session.clone().filter(|name| !name.is_empty());
      let left = named.unwrap_or_else(|| card.thread.title.clone());
      all.push(Entry {
        part: "".into(),
        project: Some(project(card)),
        left,
        right: project(card),
        hue: None,
        choice,
        ..Entry::default()
      });
    }
    all
  }

  /// The machines that a new session runs on: this computer first, each with its state, and a new
  /// session on each runs in the project that it used last, or in the folder of the window here.
  fn machines(&self) -> Vec<Entry> {
    let here = self.client.folder.clone();
    let entry = |(at, told): (usize, &Told)| {
      let name = if at == 0 { "This computer".to_owned() } else { told.name.clone() };
      let folder = told.projects.first().map(|one| one.folder.clone());
      let folder = if at == 0 { Some(here.clone()) } else { folder };
      let state = match (&told.link, &folder) {
        (Link::Lost(_), _) => "offline".to_owned(),
        (Link::Dialing, _) => "dialing".to_owned(),
        (Link::Listed, Some(folder)) => named(folder),
        (Link::Listed, None) => "no project yet".to_owned(),
      };
      let choice = folder.map_or(Choice::None, |folder| Choice::Session(Target::New(at, folder)));
      Entry { left: name, right: state, choice, ..Entry::default() }
    };
    self.client.machines.iter().enumerate().map(entry).collect()
  }

  /// The projects of the machine of the input, where a new session runs: the project of the
  /// folder of the window first on this computer, then each that the machine gave, the one used
  /// last first, with its count of sessions.
  fn projects(&self) -> Vec<Entry> {
    let at = self.client.target().machine();
    let Some(told) = self.client.machines.get(at) else { return Vec::new() };
    let here = self.client.folder.clone();
    let first = (at == 0 && !told.projects.iter().any(|one| one.folder == here)).then_some(&here);
    let counted = told.projects.iter().map(|one| (&one.folder, Some(one.sessions)));
    let all = first.map(|one| (one, None)).into_iter().chain(counted);
    let entry = |(folder, sessions): (&String, Option<usize>)| {
      let count = match sessions {
        None | Some(0) => "no session yet".to_owned(),
        Some(1) => "1 session".to_owned(),
        Some(many) => format!("{many} sessions"),
      };
      Entry {
        project: Some(named(folder)),
        left: named(folder),
        says: format!("  {}", self.homed(folder)),
        right: count,
        choice: Choice::Session(Target::New(at, folder.clone())),
        ..Entry::default()
      }
    };
    all.map(entry).collect()
  }

  /// What he sent where the input sends, the newest first: a text of many lines on one row, and a
  /// word of python marked so.
  fn sent(&self) -> Vec<Entry> {
    let entry = |sent: &Sent| Entry {
      left: sent.text.replace('\n', "↵"),
      right: if sent.python { "python".to_owned() } else { String::new() },
      choice: Choice::Sent(sent.clone()),
      ..Entry::default()
    };
    self.client.sent().iter().map(entry).collect()
  }

  /// The models of the machine of the input, under each provider once, each with its effort, the
  /// one that it takes when the operator names none. A model takes the effort that the input
  /// speaks at when it offers it.
  fn models(&self) -> Vec<Entry> {
    let at = self.client.target().machine();
    let offered = self.client.machines.get(at).and_then(|told| told.models.as_ref());
    let effort = self.actor().and_then(|one| one.1);
    let entry = |model: &Offered| {
      let (provider, name) = model.name.split_once(':').unwrap_or(("", &model.name));
      let taken =
        effort.as_ref().filter(|one| model.efforts.contains(one)).or(model.effort.as_ref());
      let actor = taken.map_or_else(|| model.name.clone(), |one| format!("{}/{one}", model.name));
      Entry {
        part: provider_named(provider).into(),
        left: name.to_owned(),
        right: taken.cloned().unwrap_or_default(),
        choice: Choice::Actor(actor),
        ..Entry::default()
      }
    };
    offered.into_iter().flatten().map(entry).collect()
  }

  /// The chosen row moved by a count of rows on the screen, within the rows that pass the filter.
  /// The history rises from the input, so a row up there is an older one.
  pub(crate) fn chose(&mut self, by: isize) {
    let count = self.entries().len();
    if let Some(picker) = self.picker.as_mut() {
      let by = if picker.of == Pick::History { -by } else { by };
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
    let (of, filter) = self.picker.take().map(|one| (one.of, one.filter))?;
    match entry.choice {
      Choice::Thread(key) => self.went(key),
      Choice::Session(target) => self.aimed(target),
      Choice::Visit(at, life) => self.visited(at, life),
      Choice::Added(text) => self.client.added(&text),
      Choice::Project(name) => {
        self.picker = Some(Picker { of, filter: name, at: 0 });
      }
      Choice::Actor(actor) => self.took_actor(actor),
      Choice::Sent(sent) => self.client.recall(&sent),
      Choice::Key(does) => return Some(does),
      Choice::Running(at, life) => self.act_on(at, life),
      Choice::Control(control) => self.controlled(control),
      Choice::Leave(control) => self.left_with(control),
      Choice::Setting(place) => {
        self.picker = Some(if self.client.step_setting(place) {
          Picker { of, filter, at }
        } else {
          Picker { of: Pick::Value(place), filter: self.client.setting_text(place), at: 0 }
        });
      }
      Choice::Value(place) => {
        self.client.write_setting(place, &filter);
        self.picker = Some(Picker { of: Pick::Settings, filter: String::new(), at: place });
      }
      Choice::Skill(place) => {
        if self.client.turn_skill(place) {
          self.picker = Some(Picker { of, filter, at });
        } else if let Some(one) = self.client.skill_at(place) {
          let mention = format!("/{}", one.name);
          self.client.added(&mention);
        }
      }
      Choice::None => self.picker = Some(Picker { of, filter: String::new(), at }),
    }
    None
  }

  /// The thread of a card opened in the window.
  fn went(&mut self, key: Key) {
    self.client.selected = Some(key);
    self.open_thread();
  }

  /// A session returned to: its last thread opens, else the input turns to it.
  fn visited(&mut self, at: usize, life: String) {
    let last = self.client.settled.iter().find(|one| one.1.0 == at && one.1.1 == life);
    match last.map(|one| one.1.clone()) {
      Some(key) => self.went(key),
      None => self.aimed(Target::Session(at, life)),
    }
  }

  /// The input turned to a target: none for the thread that is open, or for a new session when no
  /// thread is.
  fn aimed(&mut self, target: Target) {
    let open = self.client.thread.as_ref().map(|one| (one.thread.key.0, one.thread.key.1.clone()));
    self.client.aim = match target {
      Target::Session(at, life) if open == Some((at, life.clone())) => None,
      Target::New(0, ref folder)
        if self.client.thread.is_none() && *folder == self.client.folder =>
      {
        None
      }
      other => Some(other),
    };
  }

  /// The columns where a list part stands at its middle: those of the home while the list stands
  /// beside it, so the part covers no card, else the whole window. The columns of the part, which
  /// leave two on each side at least, or those of the box of the input for the history, which
  /// grows from it. A part grows past its least width as far as its widest entry asks, up to its
  /// most, so each entry stands whole on one row while the window has the room.
  fn spans(&self, entries: &[Entry]) -> (u16, u16) {
    let beside = self.listed() && self.size.0 >= BESIDE;
    let columns = if beside { self.size.0.saturating_sub(SIDE) } else { self.size.0 };
    let rises = self.picker.as_ref().is_some_and(|one| one.of == Pick::History);
    let input = u16::try_from(self.wide().saturating_sub(2)).unwrap_or(PART);
    let asked = entries.iter().map(|one| one.width(badged(entries)) + 4).max().unwrap_or_default();
    let asked = u16::try_from(asked).unwrap_or(PARTS).clamp(PART, PARTS);
    (columns, if rises { input } else { asked.min(columns.saturating_sub(4)) })
  }

  /// The open list part as the window draws it, at the middle of the columns of the home and under
  /// the rows that stay above it, with as many of its rows as the window holds, the chosen one
  /// among them. The history of the input stands over the input instead, where he typed: its text
  /// at the foot, as the input, and the newest of what he sent just above it.
  pub(crate) fn over(&self) -> Option<Over> {
    let picker = self.picker.as_ref()?;
    let t = &self.tokens;
    let (muted, prose) = (t.style(Role::Muted), t.style(Role::Prose));
    let entries = self.entries();
    let (columns, width) = self.spans(&entries);
    let room = usize::from(width).saturating_sub(4);
    let padded = |line: Line| {
      let mut row = vec![run("  ", Style::new())];
      row.extend(line);
      row.push(run("  ", Style::new()));
      row
    };
    let rises = picker.of == Pick::History;
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
    let filter = Row::of(padded(spread(filter, vec![run(count, muted)], room, Style::new())));
    if !rises {
      rows.extend([filter.clone(), Row::default()]);
    }
    let fits = usize::from(self.size.1).saturating_sub(2 * usize::from(OVER) + FRAME).max(1);
    let mut shown = self.shown_entries(&entries, picker, (room, fits));
    if rises {
      rows.push(Row::default());
      shown.reverse();
    }
    for (at, line) in shown {
      let Some(at) = at else {
        rows.push(Row::of(padded(line)));
        continue;
      };
      let clicked = (2..2 + room, Some(Click::Pick(at)));
      rows.push(Row { hot: vec![clicked], ..Row::of(padded(line)) });
    }
    rows.push(Row::default());
    if rises {
      rows.extend([filter, Row::default()]);
    }
    rows.push(Row::of(padded(vec![run(self.foot(picker), muted)])));
    rows.push(Row::default());
    if rises {
      // Its foot stands on the last row of the input, over the row of hints.
      let y = self.size.1.saturating_sub(1).saturating_sub(u16::try_from(rows.len()).ok()?);
      return Some(Over { x: self.margin(), y, width, rows });
    }
    let x = columns.saturating_sub(width) / 2;
    Some(Over { x, y: OVER, width, rows })
  }

  /// The keys that the foot of a list part names: to move, to take the chosen row, to close.
  fn foot(&self, picker: &Picker) -> String {
    format!(
      "{} move · {} {} · {} close",
      [Does::Up, Does::Down].map(|one| self.key(one)).join(""),
      self.key(Does::Send),
      picker.of.takes(),
      self.key(Does::Close),
    )
  }

  /// The rows of the entries that the part shows: a head before the first entry of each part of the
  /// list while no text filters it, and each entry, the chosen one on the lifted ground with a bar
  /// in the hue of its thread, as the selected card of the list. In a list of things that say what
  /// they do, what each says stands whole under its name. A row of an entry carries its place. The rows start at the one that keeps the chosen entry in
  /// view.
  fn shown_entries(
    &self,
    entries: &[Entry],
    picker: &Picker,
    (room, fits): (usize, usize),
  ) -> Vec<(Option<usize>, Line)> {
    let t = &self.tokens;
    let (muted, prose) = (t.style(Role::Muted), t.style(Role::Prose));
    let mut all: Vec<(Option<usize>, Line)> = Vec::new();
    let mut part: &str = "";
    // A part whose rows stand in projects keeps the slot of the badge in each row, so each text
    // starts in one column.
    let badged = badged(entries);
    let under = picker.of.describes();
    for (at, entry) in entries.iter().enumerate() {
      if picker.filter.is_empty() && entry.part != part && !entry.part.is_empty() {
        if !all.is_empty() {
          all.push((None, Vec::new()));
        }
        all.push((None, vec![run(entry.part.clone(), muted)]));
        part = &entry.part;
      }
      let chosen = at == picker.at;
      let ground = |style: Style| if chosen { style.bg(t.lifted) } else { style };
      let hue = entry.hue.map_or(t.accent, |hue| t.hues[hue % t.hues.len()]);
      let bar =
        if chosen { run("▌", ground(Style::new().fg(hue))) } else { run(" ", Style::new()) };
      let badge = match &entry.project {
        Some(project) => {
          let badge = self.badge(project);
          vec![run(badge.content, ground(badge.style)), run(" ", ground(Style::new()))]
        }
        None if badged => vec![run("   ", ground(Style::new()))],
        None => Vec::new(),
      };
      let indent = 1 + badge.iter().map(|one| one.content.width()).sum::<usize>();
      let mut left = vec![bar.clone()];
      left.extend(badge);
      left.extend(lit(&entry.left, &entry.lit, ground(prose), ground(prose.fg(t.accent))));
      let says = entry.says.trim();
      if !under {
        left.push(run(entry.says.clone(), ground(muted)));
      }
      let right = vec![run(entry.right.clone(), ground(muted)), run(" ", ground(Style::new()))];
      all.push((Some(at), spread(left, right, room, ground(Style::new()))));
      let lines = if under && !says.is_empty() { wrapped(says, room - indent - 1) } else { vec![] };
      for line in lines {
        let line = vec![
          bar.clone(),
          run(" ".repeat(indent - 1), ground(Style::new())),
          run(line, ground(muted)),
        ];
        all.push((Some(at), spread(line, Vec::new(), room, ground(Style::new()))));
      }
    }
    let chosen = all.iter().rposition(|one| one.0 == Some(picker.at)).unwrap_or_default();
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

/// A session of a machine: the machine, the record, its name and its project.
struct Session {
  at: usize,
  life: String,
  name: String,
  project: String,
  used: Option<u64>,
}

/// A text with the letters at some places in the bright style, and the rest in the plain one.
pub(crate) fn lit(text: &str, places: &[usize], plain: Style, bright: Style) -> Line {
  let mut line: Line = Vec::new();
  let mut chars = text.chars().enumerate().peekable();
  while let Some((at, first)) = chars.next() {
    let on = places.contains(&at);
    let mut content = first.to_string();
    while let Some((_, next)) = chars.next_if(|(at, _)| places.contains(at) == on) {
      content.push(next);
    }
    line.push(run(content, if on { bright } else { plain }));
  }
  line
}

/// Where a card stands: its project, and its session when it has a name.
fn place(card: &Card) -> String {
  let named = card.session.as_ref().filter(|name| !name.is_empty() && **name != card.thread.title);
  named.map_or_else(|| project(card), |name| format!("{} · {name}", project(card)))
}

/// The name of a provider as a head of the models says it.
fn provider_named(provider: &str) -> &'static str {
  match provider {
    "claude-cli" => "Claude",
    "codex" => "Codex",
    "anthropic" => "Anthropic",
    "openai" => "OpenAI",
    "local" => "Local",
    _ => "Other",
  }
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
