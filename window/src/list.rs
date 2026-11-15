//! The list of cards: its rows, and the runs of each row. It is a function of the model, and only
//! the rows that show are written.

use std::path::PathBuf;

use furb::view::{Part, Standing};
use furb_host::protocol::Card;
use opentui::style::{Color, Span, Style};

use crate::{
  keys::Does,
  line::{Line, run, spread},
  model::{Key, Link, Model},
  tokens::{LIST, Loader, Role},
};

/// The columns of a card between its two edges. The edges of a card stand in the columns where the
/// head of the list starts and ends, so a card has one column of its ground on each side of its
/// text.
const CARD: usize = LIST as usize - 5;

/// A row of the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Row {
  /// What the list shows, and how the machines stand.
  Scope,
  Blank,
  /// A text of the list with no card.
  Says(&'static str, Role),
  /// The key that does a thing, as a hint.
  Key(Does),
  Head(Part),
  /// The row between two cards: the card above it, and the card under it.
  Gap(Option<usize>, Option<usize>),
  /// A line of a card, by the place of the card in the settled list: its first line, or its second.
  Card(usize, bool),
  /// The one line of a resolved thread.
  Resolved(usize),
  /// A row of the box of the context of the open thread, at the foot of the list, by its place.
  Context(usize),
}

impl Model {
  /// Each row of the list, from its top.
  pub(crate) fn rows(&self) -> Vec<Row> {
    let mut rows = vec![Row::Scope, Row::Blank];
    if self.client.settled.is_empty() && self.known() {
      let (first, more) = ("No thread yet in this project.", "Other projects and sessions:");
      rows.extend([Row::Says(first, Role::Prose), Row::Blank, Row::Says(more, Role::Muted)]);
      rows.push(Row::Key(Does::GoTo));
    }
    for part in [Part::NeedsYou, Part::Working, Part::Resting, Part::Resolved] {
      let mut cards =
        (0..self.client.settled.len()).filter(|at| self.client.settled[*at].0 == part).peekable();
      if cards.peek().is_none() {
        continue;
      }
      rows.push(Row::Head(part));
      if part == Part::Resolved {
        rows.extend(self.open.then_some(Row::Blank));
        rows.extend(cards.filter(|_| self.open).map(Row::Resolved));
        continue;
      }
      let mut above = None;
      for at in cards {
        rows.extend([Row::Gap(above, Some(at)), Row::Card(at, false), Row::Card(at, true)]);
        above = Some(at);
      }
      rows.push(Row::Gap(above, None));
    }
    rows
  }

  /// The runs of each row of the list that shows.
  pub(crate) fn list(&self) -> Vec<Line> {
    let on = self.pointer.filter(|(x, _)| self.on_list(*x));
    let pointed = on.and_then(|(_, y)| self.row(usize::from(y)));
    self.shown().map(|row| self.line(row, pointed)).collect()
  }

  fn line(&self, row: Row, pointed: Option<Row>) -> Line {
    let t = &self.tokens;
    let at = |row: Option<Row>| match row {
      Some(Row::Card(at, _) | Row::Resolved(at)) => Some(at),
      _ => None,
    };
    match row {
      Row::Scope => self.scope(pointed == Some(row)),
      Row::Blank => Vec::new(),
      Row::Says(text, role) => vec![run(format!("  {text}"), t.style(role))],
      Row::Key(does) => {
        let style = t.style(Role::Muted);
        let style = if pointed == Some(row) { style.fg(t.hover) } else { style };
        vec![run("  ", Style::new()), run(self.key(does), style)]
      }
      Row::Head(part) => self.head(part, pointed == Some(row)),
      Row::Gap(above, under) => self.gap(above, under),
      Row::Card(card, second) => {
        let lines = self.card(card, self.ground(card, at(pointed)));
        let [first, then] = lines;
        if second { then } else { first }
      }
      Row::Resolved(card) => self.resolved(card, self.ground(card, at(pointed))),
      Row::Context(at) => {
        let line = self.context_rows().into_iter().nth(at).unwrap_or_default();
        // The text of the box goes bright under the pointer, and its colored cells keep their
        // colors.
        let lit = |one: Span<'static>| {
          let cells = one.content.starts_with(['━', '■']);
          let style = if cells { one.style } else { one.style.fg(t.hover) };
          run(one.content, style)
        };
        // The box takes a click once an answer told the context, which its parts then show.
        let told = self.thread.as_ref().is_some_and(|one| one.thread.context.is_some());
        if told && matches!(pointed, Some(Row::Context(_))) {
          line.into_iter().map(lit).collect()
        } else {
          line
        }
      }
    }
  }

  fn chosen(&self, at: usize) -> bool {
    self.selected.as_ref() == Some(&self.client.settled[at].1)
  }

  /// The ground of a card: the selected card is lifted, and the text of the card under the
  /// pointer goes bright, with no other change.
  fn ground(&self, at: usize, pointed: Option<usize>) -> Ground {
    let bg = self.chosen(at).then_some(self.tokens.lifted);
    Ground { bg, hover: (pointed == Some(at)).then_some(self.tokens.hover) }
  }

  /// The first row of the list: what it shows, which takes a click and lights under the pointer,
  /// and how the machines stand.
  fn scope(&self, pointed: bool) -> Line {
    let t = &self.tokens;
    let project = self.folder.file_name().map(|name| name.to_string_lossy().into_owned());
    let scope = if self.client.settled.is_empty() {
      project.unwrap_or_default()
    } else {
      "All projects".to_owned()
    };
    let lost = || self.client.machines.iter().filter(|one| matches!(one.link, Link::Lost(_)));
    let dials =
      self.client.machines.iter().position(|one| one.link == Link::Dialing).filter(|_| self.slow);
    let machines = match (lost().count(), dials, self.client.machines.len()) {
      (1, ..) => format!("{} offline", lost().next().map_or("", |one| &one.name)),
      (2.., ..) => format!("{} machines offline", lost().count()),
      (0, Some(0), _) => format!("{} Dialing", Loader::Model.frame(self.moved)),
      (0, Some(at), _) => {
        format!("{} Dialing {}", Loader::Model.frame(self.moved), self.client.machines[at].name)
      }
      (0, None, 1) => "this machine".to_owned(),
      (0, None, count) => format!("{count} machines"),
    };
    let style = if pointed { t.style(Role::Prose).fg(t.hover) } else { t.style(Role::Prose) };
    let left = vec![run(" ", Style::new()), run(format!("{scope} ▾"), style)];
    spread(left, vec![run(machines, t.style(Role::Muted))], usize::from(LIST) - 2, Style::new())
  }

  /// The head of a part: its name and its count. The head of the resolved threads takes a click,
  /// so it rises under the pointer.
  fn head(&self, part: Part, pointed: bool) -> Line {
    let t = &self.tokens;
    let (name, fold) = match part {
      Part::NeedsYou => ("Needs you", ""),
      Part::Working => ("Working", ""),
      Part::Resting => ("Resting", ""),
      Part::Resolved if self.open => ("Resolved", " ▾"),
      Part::Resolved => ("Resolved", " ▸"),
    };
    // Only the head of the resolved threads takes a click, which folds them.
    let lit = pointed && part == Part::Resolved;
    let on = |style: Style| if lit { style.fg(t.hover) } else { style };
    let count = self.client.settled.iter().filter(|one| one.0 == part).count();
    let head = vec![
      run(name, on(t.style(Role::Prose))),
      run(format!(" {count}{fold}"), on(t.style(Role::Muted))),
    ];
    let mut line = vec![run(" ", Style::new())];
    line.extend(spread(head, Vec::new(), usize::from(LIST) - 2, on(Style::new())));
    line
  }

  /// The row between two cards. The selected card takes half of it, above and under, so no card
  /// changes height: its ground and its bar in the hue of its thread stand as tall as each other.
  fn gap(&self, above: Option<usize>, under: Option<usize>) -> Line {
    let chosen = |at: Option<usize>| at.filter(|at| self.chosen(*at));
    let (at, [bar, fill, end]) = match (chosen(above), chosen(under)) {
      (Some(at), _) => (at, ["▝", "▀", "▘"]),
      (_, Some(at)) => (at, ["▗", "▄", "▖"]),
      _ => return Vec::new(),
    };
    let ground = Style::new().fg(self.tokens.lifted);
    let hue = self.hue(&self.client.settled[at].1);
    vec![run(bar, Style::new().fg(hue)), run(fill.repeat(CARD + 2), ground), run(end, ground)]
  }

  /// A line of a card between its two edges. The selected card stands on a ground with its bar at
  /// its left, in the right half of a column: the ground takes one column on each side of its text
  /// and the left half of the column after it, so the card stands as far from each side of the
  /// list, and its text as far from each side of the card.
  fn edged(&self, at: usize, ground: Option<Color>, line: Line) -> Line {
    let Some(ground) = ground else {
      let mut row = vec![run("  ", Style::new())];
      row.extend(line);
      return row;
    };
    let edge = || run(" ", Style::new().bg(ground));
    let mut row = vec![run("▐", Style::new().fg(self.hue(&self.client.settled[at].1))), edge()];
    row.extend(line);
    row.extend([edge(), run("▌", Style::new().fg(ground))]);
    row
  }

  /// The two lines of a card: its title and its state, then where it is and who works on it.
  fn card(&self, at: usize, ground: Ground) -> [Line; 2] {
    let t = &self.tokens;
    let (_, key, card) = &self.client.settled[at];
    let told = &self.client.machines[key.0];
    let on = |style: Style| ground.on(style);
    let title = vec![run(card.thread.title.clone(), on(t.style(Role::Title)))];
    let first = spread(title, self.state(key, card, ground), CARD, on(Style::new()));

    let project = project(card);
    let machine = if key.0 > 0 { format!("@{}", told.name) } else { String::new() };
    let named =
      card.session.as_ref().filter(|name| **name != card.thread.title && !name.is_empty());
    let session = named.map(|name| format!(" · {name}")).unwrap_or_default();
    let place = format!("{project}{machine}{session}");
    let badge = self.badge(&project);
    let badge = run(badge.content, on(badge.style));
    let left = vec![badge, run(" ", on(Style::new())), run(place, on(t.style(Role::Muted)))];
    let others = card.viewers.iter().filter(|login| **login != told.login);
    let initials: Vec<String> =
      others.map(|login| login.chars().take(2).collect::<String>().to_lowercase()).collect();
    let model = card.thread.model.as_deref().and_then(|model| model.rsplit(':').next());
    let agents = (card.thread.agents > 0).then(|| format!("└{}", card.thread.agents));
    let alone = self.unheld(key.0, card).then(|| "◌".to_owned());
    let marks = [model.map(str::to_owned), agents, alone];
    let marks = marks.into_iter().flatten().collect::<Vec<_>>().join(" ");
    let right =
      [initials.join(" "), marks].into_iter().filter(|one| !one.is_empty()).collect::<Vec<_>>();
    let right = vec![run(right.join("  "), on(t.style(Role::Muted)))];
    let second = spread(left, right, CARD, on(Style::new()));
    [self.edged(at, ground.bg, first), self.edged(at, ground.bg, second)]
  }

  /// The one line of a resolved thread. A thread that no life of its machine opens says that it
  /// only reads, in place of its mark and its age, before the operator opens it.
  fn resolved(&self, at: usize, ground: Ground) -> Line {
    let t = &self.tokens;
    let card = &self.client.settled[at].2;
    let on = |style: Style| ground.on(style);
    let title = run(card.thread.title.clone(), on(t.style(Role::Muted)));
    let mark = if card.refused.is_some() { "  " } else { "✓ " };
    let left = vec![run(mark, on(t.style(Role::Muted))), title];
    let age = match card.refused {
      Some(_) => "read only".to_owned(),
      None => self.age(card.since).unwrap_or_else(|| "now".to_owned()),
    };
    let line = spread(left, vec![run(age, on(t.style(Role::Muted)))], CARD, on(Style::new()));
    self.edged(at, ground.bg, line)
  }

  /// The state of a card: its mark and its word in the color of the state, then its time. A card
  /// that its machine did not confirm keeps its word, muted and with no mark, since the mark says
  /// how it stands now, and says the hour at which the window last heard of it.
  fn state(&self, key: &Key, card: &Card, ground: Ground) -> Line {
    let t = &self.tokens;
    let on = |style: Style| ground.on(style);
    let (muted, colored) = (on(t.style(Role::Muted)), |color| on(Style::new().fg(color)));
    let confirmed = self.confirmed(key);
    if let Some(hold) = self.shown_open(key.0, &card.life).filter(|_| confirmed) {
      let share = hold.share().map(|share| format!(" {share}%")).unwrap_or_default();
      let opens = format!("{} Opening{share}", Loader::Model.frame(self.moved));
      return vec![run(opens, colored(t.accent))];
    }
    // A life that a fault ended works no more, whatever the record left at work.
    if card.thread.works() && self.client.fault_of(key.0, &card.life).is_some() {
      return vec![run("Ended", muted)];
    }
    let (needs, age) = (self.needs(key, card), self.age(card.since));
    let since = || Some(age.clone().unwrap_or_else(|| "now".to_owned()));
    let working =
      |loader: Loader| (loader.frame(self.moved), "Working".into(), Some(t.accent), age.clone());
    // The mark, the word, the color of both, and the time of the state.
    let (mark, word, color, time) = match &card.thread.standing {
      Standing::Asks { .. } if needs => ("◆", "Input".into(), Some(t.attention), None),
      Standing::Asks { of } => {
        ("", format!("Waits for {}", of.as_deref().unwrap_or_default()), None, None)
      }
      Standing::Failed => ("✗", "Failed".into(), Some(t.danger), None),
      Standing::Paused { .. } if needs => ("‖", "Paused".into(), Some(t.attention), None),
      Standing::Paused { .. } => ("‖", "Paused".into(), None, age.clone()),
      Standing::Thinks => working(Loader::Model),
      Standing::Runs => working(Loader::Command),
      Standing::Delegates => working(Loader::Agent),
      Standing::Stopped => ("", "Stopped".into(), None, age.clone()),
      Standing::Resolved if card.thread.unread(&self.client.machines[key.0].login) => {
        ("●", String::new(), Some(t.accent), since())
      }
      Standing::Rests | Standing::Resolved => ("", String::new(), None, since()),
    };
    if !confirmed {
      let heard = format!("as of {}", self.hour(self.client.machines[key.0].heard));
      return vec![run([word, heard].join(" ").trim_start().to_owned(), muted)];
    }
    let style = color.map_or(muted, colored);
    if word.is_empty() {
      let mark = (!mark.is_empty()).then(|| run(format!("{mark} "), style));
      return mark.into_iter().chain(time.map(|time| run(time, muted))).collect();
    }
    let parts = [mark.to_owned(), word].into_iter().chain(time);
    vec![run(parts.filter(|one| !one.is_empty()).collect::<Vec<_>>().join(" "), style)]
  }
}

impl Model {
  /// The badge of a project, as each row that stands in it draws it: its two letters, bold in the
  /// hue of the project.
  pub(crate) fn badge(&self, project: &str) -> Span<'static> {
    let hue = self.tokens.hues[hash(project) % self.tokens.hues.len()];
    run(badge(project), Style::new().fg(hue).bold())
  }

  /// Whether a life holds the record of a card and no view holds that life. The window is a view of
  /// each life that it holds, before its machine says so.
  pub(crate) fn unheld(&self, at: usize, card: &Card) -> bool {
    let held = self.client.holds.contains_key(&(at, card.life.clone()));
    card.held && card.viewers.is_empty() && !held
  }
}

/// The look of a card: its ground, and the color of its text under the pointer.
#[derive(Clone, Copy)]
struct Ground {
  bg: Option<Color>,
  hover: Option<Color>,
}

impl Ground {
  fn on(self, style: Style) -> Style {
    let style = self.bg.map_or(style, |bg| style.bg(bg));
    self.hover.map_or(style, |hover| style.fg(hover))
  }
}

/// The name of the project of a card: the last part of its main checkout, or of the folder of its
/// record when its machine names no project for it.
pub(crate) fn project(card: &Card) -> String {
  let record = PathBuf::from(&card.life);
  let folder =
    card.project.as_deref().map(PathBuf::from).or_else(|| record.parent().map(PathBuf::from));
  folder
    .and_then(|one| one.file_name().map(|name| name.to_string_lossy().into_owned()))
    .unwrap_or_default()
}

/// The two letters of a project: the first letter of its two first words, or its two first
/// letters.
fn badge(project: &str) -> String {
  let mut words = project.split(|one: char| !one.is_alphanumeric()).filter(|word| !word.is_empty());
  let letters: String = match (words.next(), words.next()) {
    (Some(one), Some(other)) => one.chars().take(1).chain(other.chars().take(1)).collect(),
    (Some(one), None) => one.chars().take(2).collect(),
    _ => String::new(),
  };
  format!("{:<2}", letters.to_uppercase())
}

/// A number from a text, the same in each build, which picks a hue.
pub(crate) fn hash(text: &str) -> usize {
  text
    .bytes()
    .fold(0x811c_9dc5_u32, |hash, byte| (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193))
    as usize
}
