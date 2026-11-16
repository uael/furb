//! The input of a client: where it sends, and what it sends.
//!
//! The input sends to its target: the open thread, a new thread of a session, or a new session in
//! the project of the folder of the client. Each message waits in one queue, and only one request
//! of the queue is asked at a time, so each message goes after the one before it, and a message
//! that goes to the thread that the message before it made knows that thread when it goes. His
//! message shows in the thread at once, and a message that the daemon refuses comes back to the
//! input with the message after it.

use std::{mem, path::Path};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use furb_wire::renaming;
use furb_wire::{PICTURE, anchor::Anchor, protocol::BOUND};
use jiff::Timestamp;
use serde_json::Value;

use crate::{Client, Opened, Outgoing, Pending, Target, Thread};

/// How many emptied drafts the stash keeps, the oldest going first.
const STASH: usize = 50;

/// The start of a message that asks its question aside, in a fork of the chain of the open thread.
pub const ASIDE: &str = "/btw ";

/// The start of a message that names the session, as a word of his on its root: the rest of the
/// message is the name, and an empty rest takes the default name again.
pub const RENAME: &str = "/rename";

/// The start of a message that moves the chain of the open thread to a folder, as a word of his on
/// the thread: the rest of the message is the path.
pub const CD: &str = "/cd";

/// The sign of a long text that a paste folded into one mark, before its name.
pub const FOLDED: &str = "≡ ";

/// The most lines of a paste that the input shows whole: a longer one stands as its mark.
pub const FOLD: usize = 5;

/// The text of the input, whether it is a word of python that runs, each image that the operator
/// put in it, which a message takes when its text holds the mark of that image, each long text
/// that a paste folded, which a message says whole in the place of its mark, and the line of a
/// file that the next message is about.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Draft {
  pub text: String,
  pub python: bool,
  pub images: Vec<Pasted>,
  pub folds: Vec<Folded>,
  pub anchor: Option<Anchor>,
}

/// A long text that a paste folded: its name, `paste-<n>`, and the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folded {
  pub name: String,
  pub text: String,
}

impl Folded {
  /// The mark of the text in the input, which the window shows as one unit: its name and how many
  /// lines it holds.
  pub fn mark(&self) -> String {
    format!("{FOLDED}{} · {} lines", self.name, self.text.lines().count())
  }
}

/// An image that the operator put in the input: its name, `image-<n>.<extension>`, its bytes in
/// base64, and the text by which a message names it once the life of its message kept it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pasted {
  pub name: String,
  pub data: String,
  pub named: Option<String>,
}

impl Pasted {
  /// The mark of the image in the text of the input, which the window shows as one unit.
  pub fn mark(&self) -> String {
    format!("{PICTURE}{}", self.name)
  }
}

impl Outgoing {
  /// The text of a message as it goes: each mark of an image becomes the text by which its life
  /// names it, so that the model sees it.
  pub fn said(&self) -> String {
    let named = self.images.iter().filter_map(|one| Some((one.mark(), one.named.as_ref()?)));
    named.fold(self.text.clone(), |text, (mark, named)| text.replace(&mark, named))
  }
}

/// The name of the folder of a new session with no project until its first message names it,
/// which no slug takes, so no folder of the machine has it.
const UNNAMED: &str = "…";

/// The most words of the slug of a folder of a session with no project, and the most letters past
/// its first word.
const WORDS: usize = 3;
const SLUG: usize = 32;

impl Client {
  /// Where the input sends now: where the operator turned it, else the open thread, else a new
  /// session on the first machine, in the folder of the client.
  pub fn target(&self) -> Target {
    match (&self.aim, &self.thread) {
      (Some(target), _) => target.clone(),
      (None, Some(opened)) => Target::Thread(opened.thread.key.clone()),
      (None, None) => Target::New(0, self.folder.clone()),
    }
  }

  /// The input turned to a new thread of the session of the open thread, and back.
  pub fn turn(&mut self) {
    self.aim = match (&self.aim, &self.thread) {
      (None, Some(opened)) => {
        Some(Target::Session(opened.thread.key.0, opened.thread.key.1.clone()))
      }
      _ => None,
    };
  }

  /// The text of the input sent to its target: it waits in the queue, the client holds its life,
  /// and the thread that it goes to shows it at once. A message to no thread starts one, which
  /// opens, so the next message goes to it, and a new session opens on the actor that the operator
  /// chose for it. A word of python shows in no thread. A message from a moment of the past of the
  /// open thread goes as the word of a rewind, and one that starts with `/btw ` as the word of a
  /// question aside. While the input fills an input of a view, the text goes to that node alone. A
  /// machine that the client dials again, or a life that a fault ended, takes nothing, and the
  /// error says why.
  pub fn send(&mut self, text: String) -> Result<(), String> {
    let text = self.unfolded(text);
    if self.submit_fill(&text) {
      return Ok(());
    }
    let (at, life, thread, cwd) = match self.target() {
      Target::Thread(key) => (key.0, key.1.clone(), Some(key.2), self.cwd(key.0, &key.1)),
      Target::Session(at, life) => (at, life.clone(), None, self.cwd(at, &life)),
      Target::New(at, folder) => {
        let folder = self.slugged(at, folder, &text);
        (at, self.record(&folder), None, Some(folder))
      }
    };
    if let Some(away) = self.away(at) {
      return Err(away);
    }
    if self.only_reads(at).is_some() {
      return Err("Nothing went: this window only reads.".to_owned());
    }
    if let Some(why) = self.fault_of(at, &life) {
      return Err(format!("Nothing went: {why}"));
    }
    if let Some(word) = self.renamed(&text) {
      let (thread, text) = (None, word?);
      self.outgoing.push_back(Outgoing { at, life, thread, text, python: true, images: vec![] });
      return Ok(());
    }
    self.hold((at, life.clone()), cwd, self.moved);
    let chosen = self.chosen.take_if(|one| one.life.is_none() && one.at == at);
    if let (Some(chosen), Some(hold)) = (chosen, self.holds.get_mut(&(at, life.clone()))) {
      hold.actor = Some(chosen.actor);
    }
    let (text, python) = match self.worded(&text) {
      Some(Ok(word)) => (word, true),
      Some(Err(why)) => return Err(why),
      None => (text, self.draft.python),
    };
    // A message about a line of a file carries that line, as it stands now. On a thread, it is a
    // sub thread of its own, which opens once its life named it, so the thread does not show it.
    let anchor = self.draft.anchor.take().filter(|_| !python);
    let text = match &anchor {
      Some(anchor) => anchor.said(&text),
      None => text,
    };
    let images = mem::take(&mut self.draft.images);
    let images = images.into_iter().filter(|one| !python && text.contains(&one.mark())).collect();
    self.draft.folds.clear();
    let pending = Pending { act: None, text: text.clone() };
    match (&thread, python, self.thread.as_mut()) {
      (_, true, _) | (Some(_), false, None) => {}
      (None, false, _) => {
        let key = (at, life.clone(), String::new());
        let pending = vec![pending];
        let thread = Thread { key, above: Some(0), started: true, pending, ..Thread::default() };
        (self.selected, self.aim) = (Some(thread.key.clone()), None);
        self.thread = Some(Opened { thread, ..Opened::default() });
      }
      (Some(_), false, Some(_)) if anchor.is_some() => {}
      (Some(_), false, Some(open)) => open.thread.pending.push(pending),
    }
    self.outgoing.push_back(Outgoing { at, life, thread, text, python, images });
    self.walk = None;
    Ok(())
  }

  /// A text of the input with each mark of a long text that a paste folded in it as that text.
  pub fn unfolded(&self, text: String) -> String {
    let folds = self.draft.folds.iter();
    folds.fold(text, |text, one| text.replacen(&one.mark(), &one.text, 1))
  }

  /// A text that he pastes in the input: a long one folds into its mark, which this gives, and a
  /// short one stands as it is.
  pub fn fold(&mut self, text: &str) -> Option<String> {
    if text.lines().count() <= FOLD {
      return None;
    }
    let name = format!("paste-{}", self.draft.folds.len() + 1);
    let folded = Folded { name, text: text.to_owned() };
    let mark = folded.mark();
    self.draft.folds.push(folded);
    Some(mark)
  }

  /// The word that asks a message that starts with `/btw ` aside, as his word on the open thread:
  /// a fork of the chain of the thread, with all that it holds, answers the question, so the chain
  /// of the thread keeps nothing of it, and the answer stands under the line of the word. None for
  /// any other message, and while the input sends to no thread.
  fn aside(&self, text: &str) -> Option<Result<String, String>> {
    let question = text.strip_prefix(ASIDE)?.trim();
    self.thread.as_ref().filter(|one| self.aim.is_none() && !one.thread.key.2.is_empty())?;
    let question = Value::String(question.to_owned());
    Some(Ok(format!("prompt(str, {question}, on=chain('btw', source=__name__))")))
  }

  /// The word that names the session for a message that starts with `/rename`, which runs on the
  /// root, where the session folds its name from, or why it cannot: a session that no message
  /// started yet has no root to name.
  fn renamed(&self, text: &str) -> Option<Result<String, String>> {
    let rest = text.strip_prefix(RENAME).filter(|rest| rest.is_empty() || rest.starts_with(' '));
    let name = rest.filter(|_| !self.draft.python)?.trim();
    Some(if matches!(self.target(), Target::New(..)) {
      Err("A new session takes its name from its first message: send one first.".to_owned())
    } else {
      Ok(renaming(name))
    })
  }

  /// The word of his on the open thread that a message of a command says: an aside, a move to a
  /// folder, or a rewind, or why it cannot. A word of Python is no command.
  fn worded(&mut self, text: &str) -> Option<Result<String, String>> {
    if self.draft.python {
      return None;
    }
    self.aside(text).or_else(|| self.went(text)).or_else(|| self.rewound(text))
  }

  /// The word that moves the chain of the open thread to a folder for a message that starts with
  /// `/cd`, or why it cannot: it names no folder, or the session has no chain yet.
  fn went(&self, text: &str) -> Option<Result<String, String>> {
    let path = text.strip_prefix(CD).filter(|rest| rest.is_empty() || rest.starts_with(' '))?;
    Some(match path.trim() {
      "" => Err(format!("Name the folder: {CD} <path>.")),
      _ if matches!(self.target(), Target::New(..)) => {
        Err("A new session runs in the folder of the launcher: choose it there.".to_owned())
      }
      path => Ok(format!("cd({})", Value::String(path.to_owned()))),
    })
  }

  /// The input turned to name the session: `/rename ` and the name that the session has now, which
  /// he writes over.
  pub fn rename_draft(&mut self) {
    let key = self.thread.as_ref().map(|one| one.thread.key.clone());
    let name = key.and_then(|key| self.card(&key)?.session.clone()).unwrap_or_default();
    self.restore = Some(format!("{RENAME} {name}"));
  }

  /// The draft emptied into the stash, so no text that he wrote is lost: the last of the stash at
  /// most once, and the oldest goes past `STASH`.
  pub fn stash_draft(&mut self) {
    let text = mem::take(&mut self.draft.text);
    if !text.trim().is_empty() && self.stash.last() != Some(&text) {
      self.stash.push(text);
      let over = self.stash.len().saturating_sub(STASH);
      self.stash.drain(..over);
    }
  }

  /// The input put aside, or brought back, by one key: a text with nothing put aside goes into the
  /// stash, else the last text of the stash comes back and a text of the input takes its place, so
  /// the two trade places. Whether the input went aside, which the client then empties.
  pub fn toggle_stash(&mut self) -> bool {
    let aside = self.stash.is_empty() && !self.draft.text.trim().is_empty();
    if aside {
      self.stash_draft();
    } else {
      self.unstash();
    }
    aside
  }

  /// The last draft that he emptied taken back into the input, and a draft that the input holds
  /// then goes into the stash in its place.
  fn unstash(&mut self) {
    let Some(last) = self.stash.pop() else {
      self.notice = Some("Nothing is put aside.".to_owned());
      return;
    };
    self.stash_draft();
    self.restore = Some(last);
  }

  /// An image put in the input by its bytes and the suffix of the name of its type: the mark that
  /// the input takes at its caret, or why it does not go. A frame of the protocol holds the image.
  pub fn paste_image(&mut self, bytes: &[u8], suffix: &str) -> Result<String, String> {
    let data = STANDARD.encode(bytes);
    // The frame holds the data and the few names of the verb around it.
    if data.len() + 1024 > BOUND {
      return Err(format!("An image of at most {} MiB goes to a session.", (BOUND >> 20) * 3 / 4));
    }
    let name = format!("image-{}.{suffix}", self.draft.images.len() + 1);
    let pasted = Pasted { name, data, named: None };
    let mark = pasted.mark();
    self.draft.images.push(pasted);
    Ok(mark)
  }

  /// The text by which the life of the first message of the queue names the first of its images
  /// that it did not keep yet.
  pub fn imaged(&mut self, named: String) {
    let next = self.outgoing.front_mut().into_iter().flat_map(|one| one.images.iter_mut());
    if let Some(image) = next.into_iter().find(|one| one.named.is_none()) {
      image.named = Some(named);
    }
  }

  /// The input turned to a command that takes its text, as `/btw` or `/cd`: it starts with the
  /// command once, and keeps the text that it holds.
  pub fn drafted(&mut self, command: &str) {
    let rest = self.draft.text.trim_start().trim_start_matches(command).trim_start();
    self.restore = Some(format!("{command} {rest}"));
  }

  /// The folder that the life of a record opens in when no life holds the record: the folder that
  /// the client opened it in, else the folder of the client for a new record, else the project of
  /// its cards, else the folder whose sessions
  /// hold it, as for a session with no project.
  pub fn cwd(&self, at: usize, life: &str) -> Option<String> {
    let cards = self.machines.get(at).map(|told| &told.cards[..]).unwrap_or_default();
    let held = || checkout(life).map(|one| one.display().to_string());
    // A life that the client opened before its machine told its card runs where it was opened.
    let opened = || self.holds.get(&(at, life.to_owned())).and_then(|hold| hold.cwd.clone());
    match cards.iter().find(|card| card.life == life) {
      None => opened().or_else(|| Some(self.folder.clone())),
      Some(card) => card.project.clone().or_else(held),
    }
  }

  /// The record of a new session, in the sessions of the project of a folder, named by the second
  /// and the moment of the client, so no two clients share it, and past each record that the
  /// client knows, so no two sessions of one client share it when its clocks stand still.
  pub fn record(&self, folder: &str) -> String {
    let second = Timestamp::from_second(i64::try_from(self.second()).unwrap_or_default());
    let stamp = second.unwrap_or_default().strftime("%Y-%m-%dT%H-%M-%S");
    let sessions = Path::new(folder).join(".furb").join("sessions");
    let named = |n: u128| sessions.join(format!("{stamp}-{:04x}.jsonl", n % 0x1_0000));
    let known = |life: &str| {
      self.holds.keys().any(|held| held.1 == life)
        || self.machines.iter().any(|one| one.cards.iter().any(|card| card.life == life))
    };
    let mut n = self.moved.as_nanos();
    while known(&named(n).display().to_string()) {
      n += 1;
    }
    named(n).display().to_string()
  }

  /// Whether a folder of a machine is that of a session with no project: a folder in the folder of
  /// those sessions that the machine gave.
  pub fn alone(&self, at: usize, folder: &str) -> bool {
    let sessions = self.machines.get(at).and_then(|told| told.detached()).map(Path::new);
    sessions.is_some() && Path::new(folder).parent() == sessions
  }

  /// The project of a record: the folder that its life opens in, or none for a session with no
  /// project, as a card of its machine says it once the machine told one.
  pub fn project(&self, at: usize, life: &str) -> Option<String> {
    self.cwd(at, life).filter(|folder| !self.alone(at, folder))
  }

  /// The folder of a new session with no project on a machine, in the folder of those sessions
  /// that the machine gave, which its first message names.
  pub fn detached(&self, at: usize) -> Option<String> {
    let folder = self.machines.get(at)?.detached()?;
    Some(Path::new(folder).join(UNNAMED).display().to_string())
  }

  /// The folder of a new session named by its first message, when it is a session with no project:
  /// a short slug of the message, with a count after it when a session of the machine has it.
  fn slugged(&self, at: usize, folder: String, text: &str) -> String {
    let Some(told) = self.machines.get(at) else { return folder };
    let Some(root) = told.detached().map(Path::new) else { return folder };
    if Path::new(&folder) != root.join(UNNAMED) {
      return folder;
    }
    let words = text.split(|one: char| !one.is_alphanumeric()).filter(|one| !one.is_empty());
    let mut slug = String::new();
    for word in words.map(str::to_lowercase).take(WORDS) {
      if !slug.is_empty() && slug.len() + 1 + word.len() > SLUG {
        break;
      }
      slug = if slug.is_empty() { word } else { format!("{slug}-{word}") };
    }
    let slug = if slug.is_empty() { "session".to_owned() } else { slug };
    let lives =
      told.lives.iter().map(|one| &one.life).chain(told.cards.iter().map(|one| &one.life));
    let taken: Vec<&Path> = lives.filter_map(|one| checkout(one)).collect();
    let free = |name: &str| !taken.contains(&root.join(name).as_path());
    // One of as many names as the folders taken, and one more, is free.
    let name = (1..=taken.len() + 1)
      .map(|n| if n == 1 { slug.clone() } else { format!("{slug}-{n}") })
      .find(|one| free(one));
    root.join(name.unwrap_or(slug)).display().to_string()
  }

  /// The second of the clock of the machine of the client.
  pub fn second(&self) -> u64 {
    self.now / 1000
  }
}

/// The checkout whose `.furb/sessions` holds a record, from the one name of the record: with no
/// link and no short name in it, as the World writes its paths, where a project stands as the list
/// of the machine names it.
pub fn checkout(life: &str) -> Option<&Path> {
  Path::new(life).ancestors().nth(3)
}
