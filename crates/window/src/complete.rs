//! The completion of the input as the window draws it: a panel on the top of the box of the input,
//! over the feed, so the text that he types stays where it is. It continues the bar of the box,
//! and lists at most eight rows under a head that says what it lists and where the chosen row
//! stands among the rows that match, the chosen one on the lifted ground. The completion itself
//! stands in the client.

use crate::cells::Cells;
use opentui::style::Style;

use furb_client::{Asked, Completes, Kind, Offer, Target};

use crate::{
  input::named,
  keys::{self, Does},
  line::{BARRED, Line, capital, run, spread},
  model::Model,
  pick::lit,
  tokens::Role,
};

/// The rows of the panel of the completion above its first offer: a blank row and its head.
const HEAD: usize = 2;

/// The most rows of a completion that the panel shows at once.
const ROWS: usize = 8;

impl Model {
  /// The commands of the table of the keys, which a slash at the start of a message offers.
  pub(crate) fn offered_commands() -> Vec<Offer> {
    let offer = |one: &keys::Bound| Offer {
      name: one.command.unwrap_or_default().to_owned(),
      says: one.says.to_owned(),
      kind: Kind::Command,
    };
    keys::commands().map(offer).collect()
  }

  /// What the command of a name does.
  pub(crate) fn command(name: &str) -> Option<Does> {
    keys::commands().find(|one| one.command == Some(name)).map(|one| one.does)
  }

  /// The rows of the panel of the completion: a blank row and its head, then each row that it
  /// shows, or one row that says why none shows. None while no completion shows.
  pub(crate) fn completion_rows(&self) -> Vec<Line> {
    let Some(completion) = self.client.completion.as_ref().filter(|one| !one.hidden) else {
      return Vec::new();
    };
    let t = &self.tokens;
    let muted = t.style(Role::Muted).bg(t.raised);
    let count = match completion.found.len() {
      0 => String::new(),
      many => format!("{} / {many}", completion.chosen + 1),
    };
    let head = match completion.completes {
      Completes::Slash => "Commands and skills".to_owned(),
      Completes::Mention => format!("Files of {}", self.mentioned_in()),
      Completes::Emoji => "Emoji".to_owned(),
    };
    // A blank row stands above the head, as the box of the input keeps one above its text.
    let blank = self.panel_row(Vec::new(), Vec::new(), false);
    let mut rows =
      vec![blank, self.panel_row(vec![run(head, muted)], vec![run(count, muted)], false)];
    let shown = self.client.completing(ROWS);
    // A mention may match thousands of files, so only a slash reads every row for its column.
    let commands = match completion.completes {
      Completes::Slash => {
        self.client.completing(usize::MAX).iter().map(|one| one.1.name.width()).max()
      }
      Completes::Mention | Completes::Emoji => None,
    };
    let commands = commands.unwrap_or_default();
    // The folders of the files that show stand in one column, past the longest name of a file.
    let file = |one: &(usize, Offer, Vec<usize>)| one.1.name.rsplit('/').next().map(Cells::width);
    let files = shown.iter().filter_map(file).max().unwrap_or_default();
    if shown.is_empty() {
      let says = self.unmatched();
      rows.push(self.panel_row(vec![run(says, muted)], Vec::new(), false));
    }
    for (place, offer, letters) in shown {
      let chosen = place == completion.chosen;
      let (left, right) = self.offer_row((&offer, &letters), chosen, (commands, files));
      rows.push(self.panel_row(left, right, chosen));
    }
    rows
  }

  /// The two sides of the row of an offer, on the ground of the row that he chose or not, with the
  /// widths of the longest command and of the longest name of a file that the list shows.
  fn offer_row(
    &self,
    (offer, letters): (&Offer, &[usize]),
    chosen: bool,
    (commands, files): (usize, usize),
  ) -> (Line, Line) {
    let t = &self.tokens;
    let ground = |style: Style| if chosen { style.bg(t.lifted) } else { style.bg(t.raised) };
    let (prose, dim) = (ground(t.style(Role::Prose)), ground(t.style(Role::Muted)));
    let bright = ground(t.style(Role::Prose).fg(t.accent));
    match offer.kind {
      Kind::File => {
        // The name of a file leads, so a file deep in folders reads by its name.
        let split = offer
          .name
          .rfind('/')
          .and_then(|at| offer.name.get(..=at))
          .map_or(0, |head| head.chars().count());
        let name: String = offer.name.chars().skip(split).collect();
        let folder: String = offer.name.chars().take(split).collect();
        let named: Vec<usize> = letters.iter().filter_map(|at| at.checked_sub(split)).collect();
        let mut left = lit(&name, &named, prose, bright);
        left.push(run(" ".repeat(files.saturating_sub(name.width()) + 2), dim));
        left.extend(lit(&folder, letters, dim, ground(t.style(Role::Muted).fg(t.accent))));
        (left, Vec::new())
      }
      Kind::Emoji => {
        // A symbol of one column takes a slot of two, so each name starts on one column.
        let glyph = &offer.says;
        let pad = " ".repeat(2usize.saturating_sub(glyph.width()) + 2);
        let mut left = vec![run(format!("{glyph}{pad}:"), prose)];
        left.extend(lit(&offer.name, letters, prose, bright));
        (left, Vec::new())
      }
      Kind::Command | Kind::Skill => {
        let mut left = vec![run("/", prose)];
        left.extend(lit(&offer.name, letters, prose, bright));
        // The descriptions stand in one column, past the longest name of the list.
        let pad = commands.saturating_sub(offer.name.width()) + 2;
        left.push(run(format!("{}{}", " ".repeat(pad), capital(&offer.says)), dim));
        let right = match offer.kind {
          Kind::Skill => "skill".to_owned(),
          _ => self.command_key(&offer.name),
        };
        (left, vec![run(right, dim)])
      }
    }
  }

  /// The panel of the completion as the window draws it: its column, its row, its columns and its
  /// lines, on the rows of the feed right above the box of the input.
  pub(crate) fn drawn_completion(&self) -> Option<(u16, u16, u16, Vec<Line>)> {
    let rows = self.completion_rows();
    if rows.is_empty() {
      return None;
    }
    let top = usize::from(self.size.1).saturating_sub(self.input_rows());
    let y = u16::try_from(top.saturating_sub(rows.len())).ok()?;
    // The panel is as wide as the rows of the box of the input, which stop two columns short.
    let width = u16::try_from(self.wide().saturating_sub(2)).ok()?;
    let pointed = self.pointed().and_then(|(x, at)| at.checked_sub(y).map(|row| (x, row)));
    let lines = rows.into_iter().enumerate().map(|(row, line)| match pointed {
      Some((x, at)) if usize::from(at) == row && row >= HEAD && usize::from(x) < self.wide() => {
        line.into_iter().map(|one| run(one.content, one.style.fg(self.tokens.hover))).collect()
      }
      _ => line,
    });
    Some((0, y, width, lines.collect()))
  }

  /// The place among the rows that match of the row of the panel at a cell, which a click takes.
  pub(crate) fn completion_at(&self, x: u16, y: u16) -> Option<usize> {
    let (_, top, width, lines) = self.drawn_completion()?;
    let row = usize::from(y.checked_sub(top)?);
    if x >= width || row < HEAD || row >= lines.len() {
      return None;
    }
    let completion = self.client.completion.as_ref()?;
    let first = (completion.chosen + 1).saturating_sub(ROWS);
    let place = first + row - HEAD;
    (place < completion.found.len()).then_some(place)
  }

  /// The hint of the keys of the completion, while one shows.
  pub(crate) fn completion_hints(&self) -> Option<Line> {
    self.client.completion.as_ref().filter(|one| !one.hidden)?;
    let keys = format!(
      "  {}{} choose · {} or {} take · {} hide",
      self.key(Does::Up),
      self.key(Does::Down),
      self.key(Does::Send),
      self.key(Does::NextQuestion),
      self.key(Does::Close),
    );
    Some(vec![run(keys, self.tokens.style(Role::Muted))])
  }

  /// A row of the panel: the bar of the box of the input, then a line of its columns with one blank
  /// column on each side, on the lifted ground while it is the chosen row.
  fn panel_row(&self, line: Line, right: Line, chosen: bool) -> Line {
    let t = &self.tokens;
    let raised = Style::new().bg(t.raised);
    let ground = if chosen { Style::new().bg(t.lifted) } else { raised };
    let room = self.wide().saturating_sub(BARRED + 4);
    let bar = if chosen { "▌" } else { "┃" };
    let mut row = vec![run("  ", Style::new()), run(bar, raised.fg(self.bar())), run(" ", ground)];
    row.extend(spread(line, right, room, ground));
    row.push(run(" ", ground));
    row.push(run(" ", raised));
    row
  }

  /// The key of a command, by its name.
  fn command_key(&self, name: &str) -> String {
    Self::command(name).map(|one| self.key(one)).unwrap_or_default()
  }

  /// The name of the folder whose files a mention lists.
  fn mentioned_in(&self) -> String {
    let at = self.client.target().machine();
    let folder = match self.client.target() {
      Target::Thread(key) => self.client.cwd(at, &key.1),
      Target::Session(_, life) => self.client.cwd(at, &life),
      Target::New(_, folder) => Some(folder),
    };
    let word = self.client.completion.as_ref().map(|one| one.word.clone()).unwrap_or_default();
    let query = word.trim_start_matches(['@', '"']);
    match query.rfind('/') {
      Some(end) if query.starts_with(['/', '~', '.']) => {
        query.get(..=end).unwrap_or(query).to_owned()
      }
      _ => folder.map_or_else(|| "this project".to_owned(), |one| named_or(&one)),
    }
  }

  /// Why the completion shows no row: its list did not come yet, or nothing matches its word.
  fn unmatched(&self) -> String {
    let Some(completion) = self.client.completion.as_ref() else { return String::new() };
    match completion.completes {
      Completes::Slash => format!("No command or skill matches {}", completion.word),
      Completes::Mention
        if self.client.asked.values().any(|one| matches!(one, Asked::Files(_))) =>
      {
        "Reading the files…".to_owned()
      }
      Completes::Mention => format!("No file matches {}", completion.word),
      Completes::Emoji => format!("No emoji matches {}", completion.word),
    }
  }
}

/// The name of a project from its folder, or the folder when it has none.
fn named_or(folder: &str) -> String {
  let name = named(folder);
  if name.is_empty() { folder.to_owned() } else { name }
}
