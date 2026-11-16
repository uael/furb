//! The completion of the input at the word of its caret. A slash that starts a message lists the
//! commands of the client and the skills of the life of its target, and an at sign that starts a
//! word lists the files of the folder of that target, which the message then names: a file of
//! another folder too, when the word names that folder first, as `@../other/` or `@~/notes/`.
//! The list stands while he types the word and ranks its rows as each list of the client does. A
//! row that he takes writes its name in the place of the word with a space after it, so his next
//! word never joins it, and a command leaves the input as it was before its word, and runs.

use std::{
  collections::{HashMap, HashSet},
  ops::Range,
};

use furb_wire::{
  card::Skill,
  protocol::{Answer, Files, Verb},
};

use crate::{Asked, Client, Matched, Target, pathed, ranked};

/// What a word of the input completes: a slash that starts a message, or a mention of a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Completes {
  Slash,
  Mention,
  Emoji,
}

/// What a row of a completion takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
  Command,
  Skill,
  File,
  /// An emoji, by its short code, whose character the row says.
  Emoji,
}

/// A row that a completion offers: its name, what it is for, and what it takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
  pub name: String,
  pub says: String,
  pub kind: Kind,
}

/// The completion of the word of the caret: what it completes, the place of the first character of
/// the word among the characters of the draft, the word, the rows that it matches with their lit
/// letters, the best first, the place of the chosen one among them, whether he hid it until the
/// word changes, and the list that the client asked for it last.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
  pub completes: Completes,
  pub at: usize,
  pub word: String,
  pub found: Vec<Matched>,
  pub chosen: usize,
  pub hidden: bool,
  asked: Option<String>,
}

/// A row taken: the text of the input after it, the place of the caret in it, and the command
/// that it runs, by its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completed {
  pub text: String,
  pub caret: usize,
  pub command: Option<String>,
}

/// Where a mention looks: the folder whose files it names, the part of its word that names that
/// folder, and the part that the files match.
struct Looked {
  folder: String,
  named: String,
  query: String,
}

impl Client {
  /// The completion of the word at the caret, a place among the characters of the draft, after a
  /// change of the text: none for a word of python or in the form of a question. The same word
  /// keeps its completion and whether he hid it; another word chooses its first row.
  pub fn complete_at(&mut self, caret: usize) {
    let chars: Vec<char> = self.draft.text.chars().collect();
    let caret = caret.min(chars.len());
    let start =
      chars.iter().take(caret).rposition(|one| one.is_whitespace()).map_or(0, |at| at + 1);
    let end = chars
      .iter()
      .skip(caret)
      .position(|one| one.is_whitespace())
      .map_or(chars.len(), |at| caret + at);
    let word: String = chars.iter().take(end).skip(start).collect();
    let form = self.thread.as_ref().is_some_and(|opened| opened.form.is_some());
    let completes = match word.chars().next() {
      _ if self.draft.python || form => None,
      Some('/') if start == 0 => Some(Completes::Slash),
      Some('@') => Some(Completes::Mention),
      Some(':') => Some(Completes::Emoji),
      _ => None,
    };
    let Some(completes) = completes else {
      self.completion = None;
      return;
    };
    let same = self.completion.take().filter(|one| one.at == start && one.completes == completes);
    self.completion = Some(match same {
      Some(one) if one.word == word => one,
      Some(one) => Completion { word, chosen: 0, ..one },
      None => {
        let found = Vec::new();
        Completion { completes, at: start, word, found, chosen: 0, hidden: false, asked: None }
      }
    });
    self.refound();
  }

  /// The rows that the completion shows, at most a count, from the first that keeps the chosen one
  /// in view: each with its place among the rows that the word matches, its offer and its lit
  /// letters. None while he hid it.
  pub fn completing(&self, count: usize) -> Vec<(usize, Offer, Vec<usize>)> {
    let Some(completion) = self.completion.as_ref().filter(|one| !one.hidden) else {
      return Vec::new();
    };
    let first = (completion.chosen + 1).saturating_sub(count);
    let shown = completion.found.iter().enumerate().skip(first).take(count);
    shown.filter_map(|(place, one)| Some((place, self.offer(one.at)?, one.lit.clone()))).collect()
  }

  /// The chosen row moved by a count of rows, within the rows that the word matches.
  pub fn choose(&mut self, by: isize) {
    if let Some(completion) = self.completion.as_mut() {
      let last = completion.found.len().saturating_sub(1);
      completion.chosen = completion.chosen.saturating_add_signed(by).min(last);
    }
  }

  /// The completion hidden until the word changes, as an escape hides it.
  pub fn hide(&mut self) {
    if let Some(completion) = self.completion.as_mut() {
      completion.hidden = true;
    }
  }

  /// The row at a place taken, else the chosen one: its name in the place of the word, a file
  /// with the folder that the word named, a name with a blank in quotes, and a space after it; a
  /// command takes its word out of the input. None while no row shows.
  pub fn take(&mut self, place: Option<usize>) -> Option<Completed> {
    let completion = self.completion.as_ref().filter(|one| !one.hidden)?;
    let place = place.unwrap_or(completion.chosen);
    let at = completion.found.get(place)?.at;
    let offer = self.offer(at)?;
    let chars: Vec<char> = self.draft.text.chars().collect();
    let start = completion.at;
    let end = (start + completion.word.chars().count()).min(chars.len());
    let (named, command) = match offer.kind {
      Kind::Command => (String::new(), Some(offer.name)),
      Kind::Skill => (format!("/{} ", offer.name), None),
      Kind::File => {
        let path = format!("{}{}", self.looked(&completion.word).named, offer.name);
        (format!("{} ", Self::mention(&path)), None)
      }
      Kind::Emoji => (format!("{} ", offer.says), None),
    };
    let before: String = chars.iter().take(start).collect();
    let after: String = chars.iter().skip(end).collect();
    // A command leaves no blank where its word stood.
    let after = if command.is_some() { after.trim_start().to_owned() } else { after };
    let caret = start + named.chars().count();
    let text = format!("{before}{named}{after}");
    self.draft.text.clone_from(&text);
    self.completion = None;
    Some(Completed { text, caret, command })
  }

  /// The files of the folder of the target and the skills of its life, which the client did not
  /// ask yet: a new session has no life to ask its skills of.
  pub(crate) fn lists_due(&self) -> Vec<(usize, Verb, Asked)> {
    let at = self.target().machine();
    let Some(told) = self.machines.get(at) else { return Vec::new() };
    let asking = |asked: &Asked| self.asked.values().any(|one| one == asked);
    let mut due = Vec::new();
    let folder = self.looked("").folder;
    let files = Asked::Files(folder.clone());
    if !told.files.contains_key(&folder) && !asking(&files) {
      due.push((at, Verb::Files { cwd: folder }, files));
    }
    let (key, life) = self.lived();
    let skills = Asked::Skills(key.clone());
    let lives = !matches!(self.target(), Target::New(..));
    if lives && !told.skills.contains_key(&key) && !asking(&skills) {
      due.push((at, Verb::Skills { life }, skills));
    }
    due
  }

  /// The files of the folder of the target and the skills of its life that the client holds.
  pub fn files_and_skills(&self) -> (&[String], &[Skill]) {
    let told = self.machines.get(self.target().machine());
    let files = told.and_then(|one| one.files.get(&self.looked("").folder));
    let skills = told.and_then(|one| one.skills.get(&self.lived().0));
    (files.map_or(&[][..], |one| &one.paths), skills.map_or(&[][..], Vec::as_slice))
  }

  /// The draft with a text added at its end, after a blank, as a row of a list adds a mention of a
  /// file or a skill.
  pub fn added(&mut self, text: &str) {
    let draft = &self.draft.text;
    let blank = if draft.is_empty() || draft.ends_with(' ') { "" } else { " " };
    let text = format!("{draft}{blank}{text} ");
    self.draft.text.clone_from(&text);
    self.restore = Some(text);
  }

  /// The mention of a file as the input writes it: in quotes when its path holds a blank.
  pub fn mention(path: &str) -> String {
    if path.contains(char::is_whitespace) { format!("@\"{path}\"") } else { format!("@{path}") }
  }

  /// The list that the completion needs and the client did not ask for it yet, with its machine:
  /// the skills of the life of the target, or the files of the folder that a mention names. The
  /// completion asks it once, so a list that changed comes with the next completion.
  pub fn completion_due(&mut self) -> Option<(usize, Verb, Asked)> {
    let completion = self.completion.as_ref().filter(|one| !one.hidden)?;
    let at = self.target().machine();
    let (key, verb, asked) = match completion.completes {
      Completes::Slash => {
        let (key, life) = self.lived();
        (key.clone(), Verb::Skills { life }, Asked::Skills(key))
      }
      Completes::Mention => {
        let folder = self.looked(&completion.word).folder;
        (folder.clone(), Verb::Files { cwd: folder.clone() }, Asked::Files(folder))
      }
      // The emojis stand in the client: it asks no machine for them.
      Completes::Emoji => return None,
    };
    let completion = self.completion.as_mut()?;
    (completion.asked.as_ref() != Some(&key)).then(|| {
      completion.asked = Some(key);
      (at, verb, asked)
    })
  }

  /// The answer of a machine to the ask of a list: the client keeps it, and the word of the
  /// completion matches it again, or the changes of a folder. A refusal keeps nothing.
  pub fn listed_for(&mut self, at: usize, asked: Asked, answer: Answer) {
    let Some(told) = self.machines.get_mut(at) else { return };
    match (asked, answer) {
      (Asked::Skills(key), Answer::Skills(skills)) => drop(told.skills.insert(key, skills)),
      (Asked::Files(folder), Answer::Files(files)) => drop(told.files.insert(folder, files)),
      (asked @ Asked::Changes(..), answer) => return self.changed(at, asked, answer),
      _ => return,
    }
    self.refound();
  }

  /// The rows that the word matches again, from the lists that the client holds.
  fn refound(&mut self) {
    let Some(completion) = self.completion.as_ref() else { return };
    let found = match completion.completes {
      Completes::Slash => {
        // A row whose name matches comes before a row whose description alone matches.
        let offers = self.offers();
        let query = completion.word.get(1..).unwrap_or_default();
        let mut found = ranked(query, offers.iter().map(|one| one.name.as_str()));
        let says = ranked(query, offers.iter().map(|one| one.says.as_str()));
        let named = |one: &Matched| found.iter().any(|named| named.at == one.at);
        let says: Vec<Matched> = says
          .into_iter()
          .filter(|one| !named(one))
          .map(|one| Matched { lit: Vec::new(), ..one })
          .collect();
        found.extend(says);
        found
      }
      Completes::Mention => {
        let looked = self.looked(&completion.word);
        let paths = self.files(&looked.folder).map(|one| &one.paths[..]).unwrap_or_default();
        let named = self.named();
        let liked = |at: usize| {
          paths.get(at).and_then(|path| named.get(&format!("{}{path}", looked.named)).copied())
        };
        pathed(&looked.query, paths.iter().map(String::as_str), |at| liked(at).unwrap_or_default())
      }
      Completes::Emoji => {
        let query = completion.word.get(1..).unwrap_or_default();
        ranked(query, coded().map(|(code, _)| code))
      }
    };
    if let Some(completion) = self.completion.as_mut() {
      completion.chosen = completion.chosen.min(found.len().saturating_sub(1));
      completion.found = found;
    }
  }

  /// How often and how lately he named each path in what he sent where the input sends, as the
  /// pickers of files of other harnesses rank it: the count of the messages that named it, over one
  /// more than the count of the messages that he sent after the newest of them.
  fn named(&self) -> HashMap<String, f64> {
    let mut named: HashMap<String, (u32, u32)> = HashMap::new();
    for (age, sent) in (0..).zip(self.sent()) {
      let paths: HashSet<String> = mentions(&sent.text).into_iter().collect();
      for path in paths {
        named.entry(path).or_insert((0, age)).0 += 1;
      }
    }
    let share = |(count, age): (u32, u32)| f64::from(count) / (1.0 + f64::from(age));
    named.into_iter().map(|(path, seen)| (path, share(seen))).collect()
  }

  /// The offer at a place of the list that the completion matches.
  fn offer(&self, at: usize) -> Option<Offer> {
    let completion = self.completion.as_ref()?;
    match completion.completes {
      Completes::Slash => self.offers().into_iter().nth(at),
      Completes::Mention => {
        let folder = self.looked(&completion.word).folder;
        let path = self.files(&folder)?.paths.get(at)?;
        Some(Offer { name: path.clone(), says: String::new(), kind: Kind::File })
      }
      Completes::Emoji => {
        let (code, emoji) = coded().nth(at)?;
        Some(Offer { name: code.to_owned(), says: emoji.to_owned(), kind: Kind::Emoji })
      }
    }
  }

  /// What a slash offers, by name: the commands of the client, and each skill of the life of the
  /// target whose name no command takes.
  fn offers(&self) -> Vec<Offer> {
    let at = self.target().machine();
    let key = self.lived().0;
    let skills = self.machines.get(at).and_then(|told| told.skills.get(&key));
    let free = |one: &&Skill| !self.commands.iter().any(|command| command.name == one.name);
    let skill = |one: &Skill| Offer {
      name: one.name.clone(),
      says: one.description.clone(),
      kind: Kind::Skill,
    };
    let skills = skills.into_iter().flatten().filter(free).map(skill);
    let mut offers: Vec<Offer> = self.commands.iter().cloned().chain(skills).collect();
    offers.sort_by(|one, other| one.name.cmp(&other.name));
    offers
  }

  /// The key under which the client keeps the skills of the target, and the record that the
  /// machine reads them from: the life of a session, or the folder of a new one, whose new record
  /// gives the skills of the last session of its project.
  fn lived(&self) -> (String, String) {
    match self.target() {
      Target::Thread(key) => (key.1.clone(), key.1),
      Target::Session(_, life) => (life.clone(), life),
      Target::New(_, folder) => (folder.clone(), self.record(&folder)),
    }
  }

  /// The files that the machine of the target gave for a folder.
  fn files(&self, folder: &str) -> Option<&Files> {
    self.machines.get(self.target().machine())?.files.get(folder)
  }

  /// Where the word of a mention looks: in the folder of the target, or in the folder that its
  /// word names first, from the root, from the home or from a folder above.
  fn looked(&self, word: &str) -> Looked {
    let query = word.trim_start_matches('@').trim_start_matches('"');
    let at = self.target().machine();
    let here = match self.target() {
      Target::Thread(key) => self.cwd(at, &key.1),
      Target::Session(_, life) => self.cwd(at, &life),
      Target::New(_, folder) => Some(folder),
    };
    let here = here.unwrap_or_else(|| self.folder.clone());
    let named = query.rfind('/').and_then(|end| query.get(..=end)).unwrap_or_default();
    let away = named.starts_with('/') || named.starts_with('~') || named.starts_with("..");
    if !away {
      return Looked { folder: here, named: String::new(), query: query.to_owned() };
    }
    let folder = if named.starts_with("..") { format!("{here}/{named}") } else { named.to_owned() };
    let query = query.strip_prefix(named).unwrap_or(query).to_owned();
    Looked { folder, named: named.to_owned(), query }
  }
}

/// Each emoji by each of its short codes, as GitHub and Slack name them, in the order of Unicode.
fn coded() -> impl Iterator<Item = (&'static str, &'static str)> {
  emojis::iter().flat_map(|one| one.shortcodes().map(move |code| (code, one.as_str())))
}

/// The paths that a text mentions, as a mention writes them: `@path`, or `@"path"` for a path with
/// a blank. A mark that ends a sentence after a path is no part of it.
fn mentions(text: &str) -> Vec<String> {
  mentioned(text).into_iter().map(|(_, path)| path.to_owned()).collect()
}

/// Each mention of a text with the bytes that it takes, from its `@` to the end of its path, and
/// its path, as `mentions` reads them.
pub fn mentioned(text: &str) -> Vec<(Range<usize>, &str)> {
  let mut found = Vec::new();
  let mut from = 0;
  while let Some(at) = text.get(from..).and_then(|rest| rest.find('@')).map(|at| from + at) {
    let rest = text.get(at + 1..).unwrap_or_default();
    // A quoted path takes its two quotes, and one with no closing quote runs to the end.
    let (path, end) = match rest.strip_prefix('"').map(|quoted| quoted.split_once('"')) {
      Some(Some((path, _))) => (path, at + path.len() + 3),
      Some(None) => (rest.get(1..).unwrap_or_default(), text.len()),
      None => {
        let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        let path =
          rest.get(..end).unwrap_or(rest).trim_end_matches([',', '.', ';', ':', ')', '?', '!']);
        (path, at + 1 + path.len())
      }
    };
    if !path.is_empty() {
      found.push((at..end, path));
    }
    from = end.max(at + 1);
  }
  found
}
