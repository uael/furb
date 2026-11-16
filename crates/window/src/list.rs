//! The list of cards: its rows, and the runs of each row. It is a function of the model, and only
//! the rows that show are written.

use std::path::Path;

use furb::view::{Part, Standing};
use furb_host::protocol::Card;
use opentui::style::{Color, Span, Style};

use crate::{
  input::named,
  keys::Does,
  line::{Line, edged, run, spread},
  model::{Key, Link, Model},
  picture,
  tokens::{CALL, Loader, Role, WORKS, called, mixed, turn},
};

/// A row of the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Row {
  /// What the list shows, and how the machines stand.
  Scope,
  Blank,
  /// A text of the list with no card.
  Says(&'static str, Role),
  /// The key that does a thing, and what it does, as a hint.
  Key(Does, &'static str),
  Head(Part),
  /// The row between two cards: the card above it, and the card under it.
  Gap(Option<usize>, Option<usize>),
  /// A line of a card, by the place of the card in the settled list: its first line, or its second.
  Card(usize, bool),
  /// The one line of a resolved thread.
  Resolved(usize),
  /// A row of the part of the changes of the open thread, over the box of the context, by its place.
  Changes(usize),
  /// A row of the box of the context of the open thread, at the foot of the list, by its place.
  Context(usize),
}

impl Model {
  /// Each row of the list, from its top.
  pub(crate) fn rows(&self) -> Vec<Row> {
    let mut rows = vec![Row::Scope, Row::Blank];
    if self.client.settled.is_empty() && self.known() {
      rows.extend([Row::Says("No thread yet in this project.", Role::Prose), Row::Blank]);
      rows.push(Row::Key(Does::GoTo, "other projects and sessions"));
    }
    for part in [Part::NeedsYou, Part::Working, Part::Resting, Part::Resolved] {
      let settled = self.client.settled.iter().enumerate();
      let mut cards = settled.filter(|(_, one)| one.0 == part).map(|(at, _)| at).peekable();
      if cards.peek().is_none() {
        continue;
      }
      rows.push(Row::Head(part));
      // The open thread keeps its card in view while the resolved threads fold, so the list and
      // the feed always show the same thread, and its card stands whole, with its session, as the
      // card of each other part: a resolved thread takes one line only while he does not read it.
      if part == Part::Resolved {
        let open = |at: &usize| self.chosen(*at);
        let shown: Vec<usize> = cards.filter(|at| self.open || open(at)).collect();
        // A whole card brings the rows of its edges, as in each other part.
        rows.extend(shown.first().filter(|at| !open(at)).map(|_| Row::Blank));
        for at in shown {
          if open(&at) {
            rows.extend([Row::Gap(None, Some(at)), Row::Card(at, false), Row::Card(at, true)]);
            rows.push(Row::Gap(Some(at), None));
          } else {
            rows.push(Row::Resolved(at));
          }
        }
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
      Row::Key(does, says) => {
        let lit = |style: Style| if pointed == Some(row) { style.fg(t.hover) } else { style };
        let key = run(self.key(does), lit(t.style(Role::Prose)));
        vec![run("  ", Style::new()), key, run(format!("  {says}"), lit(t.style(Role::Muted)))]
      }
      Row::Head(part) => self.head(part, pointed == Some(row)),
      Row::Gap(above, under) => self.gap(above, under),
      Row::Card(card, second) => {
        let lines = self.card(card, self.ground(card, at(pointed)));
        let [first, then] = lines;
        if second { then } else { first }
      }
      Row::Resolved(card) => self.resolved(card, self.ground(card, at(pointed))),
      Row::Changes(at) => self.changes_line(at, pointed == Some(row)),
      Row::Context(at) => {
        self.context_line(at, (matches!(pointed, Some(Row::Context(_))), pointed == Some(row)))
      }
    }
  }

  fn chosen(&self, at: usize) -> bool {
    self.client.settled.get(at).is_some_and(|one| self.client.selected.as_ref() == Some(&one.1))
  }

  /// The hue of the thread of the card at a place of the list, or the accent when the list holds
  /// no card there.
  fn hue_at(&self, at: usize) -> Color {
    self.client.settled.get(at).map_or(self.tokens.accent, |one| self.hue(&one.1))
  }

  /// The ground of a card: the selected card is lifted, and the text of the card under the
  /// pointer goes bright, with no other change.
  fn ground(&self, at: usize, pointed: Option<usize>) -> Ground {
    let hue = || self.hue_at(at);
    let pulse = self.pulse(at).map(|share| mixed(self.tokens.panel, hue(), share / 5));
    let bg = self.chosen(at).then_some(self.tokens.lifted).or(pulse);
    Ground { bg, hover: (pointed == Some(at)).then_some(self.tokens.hover) }
  }

  /// The first row of the list: what it shows, which takes a click and lights under the pointer,
  /// and how the machines stand.
  fn scope(&self, pointed: bool) -> Line {
    let t = &self.tokens;
    let project =
      Path::new(&self.client.folder).file_name().map(|name| name.to_string_lossy().into_owned());
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
      (0, Some(0), _) => format!("{} Dialing", Loader::Model.frame(self.client.moved)),
      (0, Some(at), _) => {
        format!(
          "{} Dialing {}",
          Loader::Model.frame(self.client.moved),
          self.client.machines.get(at).map_or("", |one| one.name.as_str())
        )
      }
      (0, None, 1) => "This computer".to_owned(),
      (0, None, count) => format!("{count} machines"),
    };
    let style = if pointed { t.style(Role::Prose).fg(t.hover) } else { t.style(Role::Prose) };
    let left = vec![run(" ", Style::new()), run(format!("{scope} ▾"), style)];
    spread(
      left,
      vec![run(machines, t.style(Role::Muted))],
      usize::from(self.list_width()) - 2,
      Style::new(),
    )
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
    line.extend(spread(head, Vec::new(), usize::from(self.list_width()) - 2, on(Style::new())));
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
    let hue = self.hue_at(at);
    vec![
      run(bar, Style::new().fg(hue)),
      run(fill.repeat(self.card_width() + 2), ground),
      run(end, ground),
    ]
  }

  /// A line of a card between its two edges, with the bar in the hue of its thread.
  fn edged(&self, at: usize, ground: Option<Color>, line: Line) -> Line {
    let hue = self.hue_at(at);
    let bar = match self.pulse(at) {
      Some(share) if !self.chosen(at) => mixed(self.tokens.panel, hue, share),
      _ => hue,
    };
    edged(bar, ground, line)
  }

  /// How far the pulse of a card stands while it runs: a card that began to call its operator
  /// rises in the hue of its thread and settles back, unless he reads it.
  fn pulse(&self, at: usize) -> Option<u16> {
    let since = self.client.called.get(&self.client.settled.get(at)?.1)?;
    let after = self.client.moved.saturating_sub(*since);
    // A pulse at its first instant, as on a still clock, shows nothing.
    (after < CALL && !self.chosen(at)).then(|| called(after)).filter(|share| *share > 0)
  }

  /// The two lines of a card: its title and its state, then where it is and who works on it.
  fn card(&self, at: usize, ground: Ground) -> [Line; 2] {
    let t = &self.tokens;
    let Some((_, key, card)) = self.client.settled.get(at) else { return [Vec::new(), Vec::new()] };
    let Some(told) = self.client.machines.get(key.0) else { return [Vec::new(), Vec::new()] };
    let on = |style: Style| ground.on(style);
    // A session whose life runs has its mark and its title bright; one that rests in its record
    // has neither, so what runs reads at a glance down the list.
    let live = self.live(key, card);
    let (mark, style) =
      if live { ("● ", t.style(Role::Title)) } else { ("  ", t.style(Role::Prose)) };
    let marked = picture::marked(&card.thread.title, on(style), on(style.fg(t.accent)));
    let title = [vec![run(mark, on(Style::new().fg(t.live)))], marked].concat();
    let first = spread(title, self.state(key, card, ground), self.card_width(), on(Style::new()));

    let project = project(card);
    let machine = if key.0 > 0 { format!("@{}", told.name) } else { String::new() };
    let named =
      card.session.as_ref().filter(|name| **name != card.thread.title && !name.is_empty());
    let session = named.map(|name| format!(" · {name}")).unwrap_or_default();
    let changes = card.project.as_deref().and_then(|folder| self.client.changes(key.0, folder));
    let branch = changes.and_then(|one| one.branch.as_deref()).map(|one| format!(" · ⑂ {one}"));
    let place = format!("{project}{machine}{}{session}", branch.unwrap_or_default());
    let left = vec![
      run("  ", on(Style::new())),
      self.badge(&project),
      run(" ", on(Style::new())),
      run(place, on(t.style(Role::Muted))),
    ];
    let others = card.viewers.iter().filter(|login| **login != told.login);
    let initials: Vec<String> =
      others.map(|login| login.chars().take(2).collect::<String>().to_lowercase()).collect();
    let model = card.thread.model.as_deref().and_then(|model| model.rsplit(':').next());
    let agents = (card.thread.agents > 0).then(|| {
      format!(
        "{}{}",
        Loader::Agent.frames().first().copied().unwrap_or_default(),
        card.thread.agents
      )
    });
    let alone = self.unheld(key.0, card).then(|| "◌".to_owned());
    let files = changes.map(|one| one.files.len()).filter(|count| *count > 0);
    let marks = [files.map(|count| format!("±{count}")), model.map(str::to_owned), agents, alone];
    let marks = marks.into_iter().flatten().collect::<Vec<_>>().join(" ");
    let right =
      [initials.join(" "), marks].into_iter().filter(|one| !one.is_empty()).collect::<Vec<_>>();
    let right = vec![run(right.join("  "), on(t.style(Role::Muted)))];
    let second = spread(left, right, self.card_width(), on(Style::new()));
    [self.edged(at, ground.bg, first), self.edged(at, ground.bg, second)]
  }

  /// The one line of a resolved thread. A thread that no life of its machine opens says that it
  /// only reads, in place of its mark and its age, before the operator opens it.
  fn resolved(&self, at: usize, ground: Ground) -> Line {
    let t = &self.tokens;
    let Some((_, key, card)) = self.client.settled.get(at) else { return Vec::new() };
    let on = |style: Style| ground.on(style);
    let title = run(card.thread.title.clone(), on(t.style(Role::Muted)));
    let mark = if card.refused.is_some() { "  " } else { "✓ " };
    let runs = self.live(key, card);
    let look = if runs { Style::new().fg(t.live) } else { t.style(Role::Muted) };
    let left = vec![run(mark, on(look)), title];
    let age = match card.refused {
      Some(_) => "read only".to_owned(),
      None => self.age(card.since).unwrap_or_else(|| "now".to_owned()),
    };
    let line =
      spread(left, vec![run(age, on(t.style(Role::Muted)))], self.card_width(), on(Style::new()));
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
      let opens = format!("{} Opening{share}", Loader::Model.frame(self.client.moved));
      return vec![run(opens, colored(t.accent))];
    }
    // A life that a fault ended works no more, whatever the record left at work.
    if card.thread.works() && self.client.fault_of(key.0, &card.life).is_some() {
      return vec![run("Ended", muted)];
    }
    let (needs, age) = (self.needs(key, card), self.age(card.since));
    let since = || Some(age.clone().unwrap_or_else(|| "now".to_owned()));
    let working =
      |loader: Loader| (loader.frame(self.client.moved), WORKS.into(), Some(t.accent), age.clone());
    // The mark, the word, the color of both, and the time of the state.
    let standing = &card.thread.standing;
    let (mark, word, color, time) = match standing {
      _ if let Some(loader) = loader(standing) => working(loader),
      Standing::Asks { .. } if needs => ("◆", "Input".into(), Some(t.attention), None),
      Standing::Asks { of } => {
        ("", format!("Waits for {}", of.as_deref().unwrap_or_default()), None, None)
      }
      Standing::Failed => ("✗", "Failed".into(), Some(t.danger), None),
      Standing::Paused { .. } if needs => ("‖", "Paused".into(), Some(t.attention), None),
      Standing::Paused { .. } => ("‖", "Paused".into(), None, age.clone()),
      Standing::Stopped => ("", "Stopped".into(), None, age.clone()),
      // A new answer takes the small dot of an unread mail, apart from the dot of a live session.
      Standing::Resolved
        if self.client.machines.get(key.0).is_some_and(|one| card.thread.unread(&one.login)) =>
      {
        ("•", String::new(), Some(t.attention), since())
      }
      // A thread that works took its loader above.
      Standing::Thinks
      | Standing::Runs
      | Standing::Delegates
      | Standing::Rests
      | Standing::Resolved => ("", String::new(), None, since()),
    };
    if !confirmed {
      let heard = self.client.machines.get(key.0).map_or(0, |one| one.heard);
      let heard = format!("as of {}", self.hour(heard));
      return vec![run([word, heard].join(" ").trim_start().to_owned(), muted)];
    }
    let style = color.map_or(muted, colored);
    if word.is_empty() {
      let mark = (!mark.is_empty()).then(|| run(format!("{mark} "), style));
      return mark.into_iter().chain(time.map(|time| run(time, muted))).collect();
    }
    // The word of a thread at work takes the light that passes over `Working` in the row of hints,
    // so each place that says that a thread works moves in one way, on one beat.
    if loader(standing).is_some() {
      let lit = Loader::Light(WORKS).cells(self.client.moved, t.accent, t);
      let word = lit.into_iter().map(|(letter, color)| run(letter, colored(color)));
      let time = time.map(|time| run(format!(" {time}"), style));
      return [run(format!("{mark} "), style)].into_iter().chain(word).chain(time).collect();
    }
    let parts = [mark.to_owned(), word].into_iter().chain(time);
    vec![run(parts.filter(|one| !one.is_empty()).collect::<Vec<_>>().join(" "), style)]
  }
}

impl Model {
  /// The columns of a card between its two edges. The edges of a card stand in the columns where
  /// the head of the list starts and ends, so a card has one column of its ground on each side of
  /// its text.
  fn card_width(&self) -> usize {
    usize::from(self.list_width()) - 5
  }

  /// The badge of a project, as each row that stands in it draws it: its two letters, bold, or a
  /// muted slot of the same width for a session with no project.
  pub(crate) fn badge(&self, project: &str) -> Span<'static> {
    if project == NO_PROJECT {
      return run("··", self.tokens.style(Role::Muted).bg(self.tokens.base));
    }
    let projects = &self.tokens.projects;
    let Some(one) = turn(projects, hash(project)) else {
      return run("··", self.tokens.style(Role::Muted).bg(self.tokens.base));
    };
    run(badge(project), Style::new().fg(one.letters).bg(one.ground).bold())
  }

  /// Whether a life runs the session of a card: the life that the window opens, or the one that its
  /// machine holds as it told in this connection, since a card kept from before says no more.
  pub(crate) fn live(&self, key: &Key, card: &Card) -> bool {
    self.client.holds.contains_key(&(key.0, card.life.clone())) || card.held && self.confirmed(key)
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

/// What a session that stands in no project names as its project.
pub(crate) const NO_PROJECT: &str = "No project";

/// The name of the project of a card: the last part of its main checkout, or that it has none.
pub(crate) fn project(card: &Card) -> String {
  card.project.as_deref().map_or_else(|| NO_PROJECT.to_owned(), named)
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

/// The loader of a thread that works, by its work: a model, a command or agents.
pub(crate) fn loader(standing: &Standing) -> Option<Loader> {
  match standing {
    Standing::Thinks => Some(Loader::Model),
    Standing::Runs => Some(Loader::Command),
    Standing::Delegates => Some(Loader::Agent),
    _ => None,
  }
}
