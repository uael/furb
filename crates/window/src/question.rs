//! The form of a question: a model of the open thread asks the operator, and the form takes the
//! place of the input in that thread alone. Its head names who asks and how many questions of the
//! thread wait, the whole question follows, then what its shape offers: each choice, with a last
//! row for his own words, each value to tick, or a line to write.
//!
//! The form shows the one question that it showed first until that question is closed, so it
//! never changes under the hands of the operator, and it takes no key for [`HOLD`] after it shows.
//! A question that comes while he writes a message takes nothing: his text stays, and one row says
//! that the question waits. Which question is open, and what its shape offers and takes, comes
//! from the crate: the form only draws it and says his answer.

use std::collections::BTreeSet;

use furb::view::{
  Answered, Maker, Part, REFUSED, Reply, Saying, Standing, Tick, choices, declined, hint, ticks,
};
use furb_host::protocol::{Card, Verb};
use opentui::style::Style;
use serde_json::Value;

use furb_client::{Asked, Form, HOLD};

use crate::{
  keys::Does,
  line::{BARRED, Line, barred, run, spread, wrapped},
  model::{Key, Model},
  thread::{Click, Row, width},
  tokens::Role,
};

/// A question of the open thread that waits for the operator: its act, who asks, its text and the
/// shape of its answer.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Question<'a> {
  pub(crate) act: &'a str,
  pub(crate) by: Option<&'a Maker>,
  pub(crate) text: &'a str,
  pub(crate) shape: &'a str,
}

impl Question<'_> {
  /// Whether the question takes a line that the operator writes, and offers no value.
  pub(crate) fn offers_line(&self) -> bool {
    matches!(Offers::of(self.shape), Offers::Line)
  }

  /// What the line of the operator is, in words, when it is more than a text.
  pub(crate) fn hint(&self) -> String {
    hint(self.shape)
  }
}

/// What the shape of a question offers: values to choose one of, values to tick, or a line.
enum Offers {
  Choices(Vec<String>),
  Ticks(Vec<Tick>),
  Line,
}

impl Offers {
  fn of(shape: &str) -> Self {
    let (choices, ticks) = (choices(shape), ticks(shape));
    match () {
      () if !ticks.is_empty() => Self::Ticks(ticks),
      () if !choices.is_empty() => Self::Choices(choices),
      () => Self::Line,
    }
  }

  /// How many values a row shows, before the row of his own words: none for a line.
  fn values(&self) -> usize {
    match self {
      Self::Choices(all) => all.len(),
      Self::Ticks(all) => all.len(),
      Self::Line => 0,
    }
  }
}

/// What a key does to the form: nothing, as within [`HOLD`], it goes to the editor of the input,
/// it says an answer to a machine, or it empties the input.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Keyed {
  Taken,
  Passed,
  Says(usize, Verb),
  Empties,
}

impl Model {
  /// Each question of the open thread that waits for the operator, in the order of the thread.
  pub(crate) fn questions(&self) -> Vec<Question<'_>> {
    let Some(opened) = &self.client.thread else { return Vec::new() };
    let open = opened.thread.exchanges.values().flat_map(|exchange| {
      let by = exchange.block.as_ref().map(|block| &block.maker);
      exchange.said.iter().filter_map(move |said| match &said.kind {
        Saying::Question { shape, answered: None } => {
          Some(Question { act: &said.act, by, text: &said.text, shape })
        }
        _ => None,
      })
    });
    open.collect()
  }

  /// The form of the open thread made, kept or let go after a change: it keeps its question while
  /// that question waits, and a new form shows the first question that waits once the input of
  /// the thread is empty. The text of a form that goes leaves the input.
  pub(crate) fn questioned(&mut self) {
    let questions: Vec<String> = self.questions().iter().map(|one| one.act.to_owned()).collect();
    let free =
      self.client.aim.is_none() && self.picker.is_none() && self.client.draft.text.is_empty();
    let moved = self.client.moved;
    let Some(opened) = self.client.thread.as_mut() else { return };
    let waits = |form: &Form| questions.contains(&form.act);
    if opened.form.as_ref().is_some_and(|form| !waits(form)) {
      opened.form = None;
      if !self.client.draft.text.is_empty() {
        self.client.restore = Some(String::new());
      }
    }
    if opened.form.is_none()
      && free
      && let Some(act) = questions.first()
    {
      let form = Form { act: act.clone(), at: 0, ticked: BTreeSet::new(), since: moved, why: None };
      opened.form = Some(form);
    }
  }

  /// The form that shows, with its question.
  pub(crate) fn form(&self) -> Option<(&Form, Question<'_>)> {
    let opened = self.client.thread.as_ref()?;
    // A window that only reads answers nothing.
    let form =
      opened.form.as_ref().filter(|_| self.client.only_reads(opened.thread.key.0).is_none())?;
    let question = self.questions().into_iter().find(|one| one.act == form.act)?;
    Some((form, question))
  }

  /// What a key does while a form shows: within [`HOLD`] it does nothing. The arrows choose a row,
  /// the space ticks a value, enter answers, tab goes to the next question of the thread, and the
  /// key that empties the input empties it, then refuses the question. A letter writes his own
  /// words on their row, or a line, and does nothing on a row of a value. Each other key of the
  /// window does its thing.
  pub(crate) fn keyed(&mut self, does: Option<Does>, typed: bool, digit: Option<usize>) -> Keyed {
    let Some((form, question)) = self.form() else { return Keyed::Passed };
    let offers = Offers::of(question.shape);
    let (values, at) = (offers.values(), form.at);
    let (own, ticks) = (at == values, matches!(offers, Offers::Ticks(_)));
    let writes = own || values == 0;
    let mine = typed
      || matches!(
        does,
        Some(Does::Send | Does::Empty | Does::NextQuestion | Does::PreviousQuestion | Does::Tick)
      )
      || (values > 0 && matches!(does, Some(Does::Up | Does::Down)));
    if mine && self.client.moved.saturating_sub(form.since) < self.hold {
      return Keyed::Taken;
    }
    let says =
      |asked: Option<(usize, Verb)>| asked.map_or(Keyed::Taken, |(at, verb)| Keyed::Says(at, verb));
    match does {
      Some(Does::Send) => says(self.answer()),
      Some(Does::Empty) if !self.client.draft.text.is_empty() => Keyed::Empties,
      Some(Does::Empty) => says(self.refuse()),
      // Escape refuses the question as the key that empties does, and only on an empty form.
      Some(Does::Close) if !self.client.draft.text.is_empty() => Keyed::Empties,
      Some(Does::Close) => says(self.refuse()),
      // A digit takes its row of a choice, or ticks its row of values, as he reads its number.
      _ if let Some(row) = digit.filter(|one| !own && (1..=values.min(9)).contains(one)) => {
        if ticks {
          self.formed(|form| {
            form.at = row - 1;
            if !form.ticked.remove(&(row - 1)) {
              form.ticked.insert(row - 1);
            }
          })
        } else {
          self.formed(|form| form.at = row - 1);
          says(self.answer())
        }
      }
      Some(does @ (Does::NextQuestion | Does::PreviousQuestion)) => {
        match self.other_question(does == Does::PreviousQuestion) {
          Some(act) => {
            self.formed(|form| {
              *form = Form { act, at: 0, ticked: BTreeSet::new(), why: None, ..*form }
            });
            Keyed::Empties
          }
          None => Keyed::Taken,
        }
      }
      Some(Does::Up | Does::Down) if values > 0 => {
        let at = if does == Some(Does::Up) { at.saturating_sub(1) } else { (at + 1).min(values) };
        self.formed(|form| form.at = at)
      }
      Some(Does::Tick) if ticks && !own => self.formed(|form| {
        if !form.ticked.remove(&at) {
          form.ticked.insert(at);
        }
      }),
      _ if typed && !writes => Keyed::Taken,
      _ => Keyed::Passed,
    }
  }

  /// A change of the form, which takes the key.
  fn formed(&mut self, change: impl FnOnce(&mut Form)) -> Keyed {
    if let Some(form) = self.client.thread.as_mut().and_then(|one| one.form.as_mut()) {
      change(form);
    }
    Keyed::Taken
  }

  /// The question after the one of the form, from the first again after the last, or the one
  /// before it, from the last again before the first.
  fn other_question(&self, back: bool) -> Option<String> {
    let (form, _) = self.form()?;
    let questions = self.questions();
    let at = questions.iter().position(|one| one.act == form.act)?;
    let step = if back { questions.len() - 1 } else { 1 };
    let next = questions.get((at + step) % questions.len())?;
    (next.act != form.act).then(|| next.act.to_owned())
  }

  /// A click on a row of a value of the form: the row is chosen, and the key that answers answers
  /// there. Within [`HOLD`] the click does nothing.
  pub(crate) fn choose(&mut self, at: usize) -> Option<Does> {
    let (form, _) = self.form()?;
    if self.client.moved.saturating_sub(form.since) < HOLD {
      return None;
    }
    self.client.thread.as_mut()?.form.as_mut()?.at = at;
    Some(Does::Send)
  }

  /// The answer of the form, which waits for no other: the value of the row chosen, each value
  /// ticked as a list, his own words on the last row, which answer a Literal as its last entry and
  /// join the values that he ticked, and close a yes or no as a refusal that its model reads, or
  /// the line that he wrote.
  fn answer(&self) -> Option<(usize, Verb)> {
    let (form, question) = self.form().filter(|_| !self.answering())?;
    let opened = self.client.thread.as_ref()?;
    let (at, life, act) = (opened.thread.key.0, opened.thread.key.1.clone(), form.act.clone());
    let answer = |line: String| Verb::Answer { life: life.clone(), act: act.clone(), line };
    let words = self.client.draft.text.clone();
    let verb = match Offers::of(question.shape) {
      Offers::Choices(all) if form.at < all.len() => answer(all[form.at].clone()),
      Offers::Ticks(all) if form.at < all.len() => {
        let ticked: Vec<&str> = form.ticked.iter().map(|at| all[*at].json.as_str()).collect();
        answer(format!("[{}]", ticked.join(", ")))
      }
      Offers::Line => answer(words),
      _ if words.trim().is_empty() => return None,
      Offers::Choices(_) if question.shape.trim() == "bool" => {
        Verb::Close { life, act, value: declined(&words) }
      }
      Offers::Choices(_) => answer(words.trim().to_owned()),
      Offers::Ticks(all) => {
        let ticked = form.ticked.iter().map(|at| all[*at].json.clone());
        let own = Value::String(words.trim().to_owned()).to_string();
        answer(format!("[{}]", ticked.chain([own]).collect::<Vec<_>>().join(", ")))
      }
    };
    Some((at, verb))
  }

  /// The refusal of the question of the form, with no words.
  fn refuse(&self) -> Option<(usize, Verb)> {
    let (form, _) = self.form().filter(|_| !self.answering())?;
    let opened = self.client.thread.as_ref()?;
    let (at, life, act) = (opened.thread.key.0, opened.thread.key.1.clone(), form.act.clone());
    Some((at, Verb::Close { life, act, value: declined(REFUSED) }))
  }

  /// Whether an answer of a form waits for its machine, so no second one goes.
  fn answering(&self) -> bool {
    self.client.asked.values().any(|one| *one == Asked::Answer)
  }

  /// The refusal of the daemon of the last answer of the form, and why: the form stays as it was,
  /// with his words, and says why.
  pub(crate) fn unanswered(&mut self, why: String) {
    if let Some(form) = self.client.thread.as_mut().and_then(|one| one.form.as_mut()) {
      form.why = Some(why);
    }
  }

  /// The rows of the form above the launcher, in the box of the input with its padding: who asks
  /// and how many questions wait, the whole question, then each value and the row of his own
  /// words, or the rows of the line, and why the daemon refused his last answer. The row where the
  /// editor of the input shows, when it shows.
  pub(crate) fn form_rows(&self, grown: usize) -> (Vec<Row>, Option<usize>) {
    let Some((form, question)) = self.form() else { return (Vec::new(), None) };
    let t = &self.tokens;
    let (prose, muted) = (t.style(Role::Prose), t.style(Role::Muted));
    let room = self.wide().saturating_sub(BARRED + 4);
    let questions = self.questions();
    let at = questions.iter().position(|one| one.act == form.act).unwrap_or_default();
    let asks = question.by.map_or_else(|| "furb".to_owned(), Maker::shown);
    let head = vec![run("◆ ", Style::new().fg(t.attention)), run(format!("{asks} asks"), muted)];
    let of = match questions.len() {
      1 => Vec::new(),
      count => vec![run(format!("{} of {count}", at + 1), muted)],
    };
    let mut lines: Vec<(Line, Option<Click>)> =
      vec![(Vec::new(), None), (spread(head, of, room, Style::new()), None)];
    let text = self.homed(question.text);
    let words = text.lines().flat_map(|line| wrapped(line, room));
    lines.extend(words.map(|line| (vec![run(line, prose)], None)));
    lines.push((Vec::new(), None));
    let editor = match Offers::of(question.shape) {
      Offers::Line => {
        let at = lines.len();
        lines.extend((0..grown).map(|_| (Vec::new(), None)));
        Some(at)
      }
      offers => {
        lines.extend(self.values(form, &offers));
        let at = lines.len() - 1;
        (form.at == offers.values()).then_some(at)
      }
    };
    if let Some(why) = &form.why {
      let danger = Style::new().fg(t.danger);
      lines.extend(
        wrapped(&self.homed(why), room).into_iter().map(|one| (vec![run(one, danger)], None)),
      );
    }
    let band = Style::new().bg(t.raised);
    let banded = |(line, click): (Line, Option<Click>)| {
      let wide = width(&line);
      let line = line.into_iter().map(|one| run(one.content, one.style.bg(t.raised))).collect();
      let mut row = barred(t.attention, t.raised);
      row.extend(spread(line, Vec::new(), self.wide().saturating_sub(BARRED + 4), band));
      row.push(run("  ", band));
      let hot = click.map(|click| (BARRED..BARRED + wide, Some(click))).into_iter().collect();
      Row { hot, ..Row::of(row) }
    };
    (lines.into_iter().map(banded).collect(), editor)
  }

  /// The rows of what a form offers: each value, the chosen one marked, each that takes a click,
  /// then the row of the words of the operator.
  fn values(&self, form: &Form, offers: &Offers) -> Vec<(Line, Option<Click>)> {
    let t = &self.tokens;
    let (prose, muted) = (t.style(Role::Prose), t.style(Role::Muted));
    let values: Vec<String> = match offers {
      Offers::Choices(all) => all.clone(),
      Offers::Ticks(all) => {
        let tick = |(at, one): (usize, &Tick)| {
          let mark = if form.ticked.contains(&at) { "[x]" } else { "[ ]" };
          format!("{mark} {}", one.shown)
        };
        all.iter().enumerate().map(tick).collect()
      }
      Offers::Line => Vec::new(),
    };
    let mark =
      |at: usize| run(if at == form.at { "› " } else { "  " }, Style::new().fg(t.attention));
    let mut lines: Vec<(Line, Option<Click>)> = values
      .into_iter()
      .enumerate()
      .map(|(at, value)| {
        let style = if at == form.at { prose } else { muted };
        (vec![mark(at), run(value, style)], Some(Click::Answer(at)))
      })
      .collect();
    let own = offers.values();
    let words = if form.at == own { String::new() } else { "type your own answer".to_owned() };
    lines.push((vec![mark(own), run(words, muted)], None));
    lines
  }

  /// The row that says that a question of the open thread waits while the operator writes.
  pub(crate) fn waiting_row(&self) -> Option<Row> {
    let shows = self.form().is_some() || self.client.aim.is_some() || self.questions().is_empty();
    if shows {
      return None;
    }
    let t = &self.tokens;
    let says = "A question waits. It shows when the input is empty.";
    let line = vec![run("  ◆ ", Style::new().fg(t.attention)), run(says, t.style(Role::Muted))];
    Some(Row::of(line))
  }

  /// The left of the row of hints while a form shows: the keys that answer it.
  pub(crate) fn form_hints(&self) -> Option<Line> {
    let (form, question) = self.form()?;
    let muted = self.tokens.style(Role::Muted);
    let key = |does| self.key(does);
    let offers = Offers::of(question.shape);
    let mut said = Vec::new();
    if offers.values() > 0 {
      said.push(format!("{}{} choose", key(Does::Up), key(Does::Down)));
    }
    if matches!(offers, Offers::Ticks(_)) && form.at < offers.values() {
      said.push(format!("{} tick", key(Does::Tick)));
    }
    said.push(format!("{} answer · {} refuse", key(Does::Send), key(Does::Close)));
    if self.questions().len() > 1 {
      said.push(format!(
        "{} next · {} before",
        key(Does::NextQuestion),
        key(Does::PreviousQuestion)
      ));
    }
    Some(vec![run(format!("  {}", said.join(" · ")), muted)])
  }

  /// How many threads but the open one ask the operator, as the row of hints of a thread says it
  /// in a count of columns: whole, then shorter. The home says what needs him.
  pub(crate) fn others_ask(&self, room: usize) -> Line {
    let Some(open) = self.client.thread.as_ref().map(|one| &one.thread.key) else {
      return Vec::new();
    };
    let asks = |one: &&(Part, Key, Card)| {
      one.1 != *open
        && matches!(one.2.thread.standing, Standing::Asks { .. })
        && self.needs(&one.1, &one.2)
    };
    let says = match self.client.settled.iter().filter(asks).count() {
      0 => return Vec::new(),
      1 if room >= 24 => " 1 other thread asks   ".to_owned(),
      count if room >= 26 => format!(" {count} other threads ask   "),
      count if room >= 12 => format!(" {count} other   "),
      count => format!(" {count}   "),
    };
    let t = &self.tokens;
    vec![run("◆", Style::new().fg(t.attention)), run(says, t.style(Role::Muted))]
  }
}

/// Who a maker is to the operator of a login: `you` for himself, else his name.
pub(crate) fn person(by: &Maker, login: &str) -> String {
  match by {
    Maker::Member { operator: member } if member != login => member.clone(),
    Maker::Operator | Maker::Member { .. } => "you".to_owned(),
    other => other.shown(),
  }
}

/// What closed a question, as one line of its thread shows it: the reply, then who closed it, which
/// is the operator himself when it is the login that he has on the machine of the thread.
pub(crate) fn closed(answered: &Answered, login: &str) -> String {
  let who = person(&answered.by, login);
  let reply = match &answered.reply {
    Reply::Value(text) => text.clone(),
    Reply::Refused(why) if why == REFUSED => "refused".to_owned(),
    Reply::Refused(why) => format!("refused: {why}"),
    Reply::Withdrawn(None) => "withdrawn".to_owned(),
    Reply::Withdrawn(Some(why)) => format!("withdrawn: {why}"),
  };
  format!("{reply} · {who}")
}
