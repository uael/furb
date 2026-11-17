//! The changes of the checkouts of git where the operator works: the branch and the changed files
//! of the project of each card and of the target of the input, which the client asks of the
//! machine of each and asks again once a life of that project told a change, or once the terminal
//! of the client took the focus again, since the operator may have changed files with no life; and
//! the review of the changes of one project, file by file, with the diff of the file that he reads.

use std::borrow::Cow;

use furb_wire::protocol::{Answer, Changed, Changes, Frame, Verb};

use crate::{Aim, Asked, Client, Key};

/// The changes of a folder as its machine last gave them, and whether a life of its project told a
/// change since, so the client asks them again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seen {
  pub changes: Changes,
  pub fresh: bool,
}

/// The review of the changes of the project of a folder of a machine: the file that he reads, by
/// its place among the changed files, and its diff once the machine gave it, with its path; each
/// thread about its lines whose conversation he folded or opened against how it stands, by its
/// prompt; the thread that he replies to under its lines; and the place that the review opens on,
/// a path and a line, until it reads that file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Review {
  pub at: usize,
  pub folder: String,
  pub chosen: usize,
  pub diff: Option<(String, String)>,
  pub toggled: Vec<String>,
  pub replying: Option<String>,
  pub wanted: Option<(String, u64)>,
}

impl Client {
  /// The changes of the folder of a machine, once the machine gave them.
  pub fn changes(&self, at: usize, folder: &str) -> Option<&Changes> {
    self.machines.get(at)?.changes.get(folder).map(|one| &one.changes)
  }

  /// The folder of the target of the input, whose branch the launcher names: the folder of the
  /// chain of the open thread, where its next word runs, else the project of the session of a
  /// thread, or the folder of a new session.
  pub fn target_folder(&self) -> Option<(usize, Cow<'_, str>)> {
    let at = self.target().machine();
    let opened = |key: &Key| self.thread.as_ref().filter(|one| one.thread.key == *key);
    let folder = match self.target() {
      Aim::Thread(key) => opened(key)
        .and_then(|one| one.thread.folder.as_deref())
        .map(Cow::Borrowed)
        .or_else(|| self.cwd(at, &key.1)),
      Aim::Session(_, life) => self.cwd(at, life),
      Aim::New(_, folder) => Some(Cow::Borrowed(folder)),
    };
    folder.map(|folder| (at, folder))
  }

  /// The asks of changes that are due, each with its machine: the folder of the target of the
  /// input and of each card whose changes the client does not hold or that a life changed, and the
  /// diff of the file that the review reads. A folder that the client asked waits for its answer.
  pub(crate) fn changes_due(&self) -> Vec<(usize, Verb, Asked)> {
    let asking = |at: usize, folder: &str| {
      self
        .asked
        .iter()
        .any(|(key, one)| key.0 == at && matches!(one, Asked::Changes(asked, _) if asked == folder))
    };
    let mut folders: Vec<(usize, Cow<'_, str>)> = self.target_folder().into_iter().collect();
    for (at, told) in self.machines.iter().enumerate() {
      let projects = told.cards.iter().filter_map(|card| card.project.as_deref());
      folders.extend(projects.map(|folder| (at, Cow::Borrowed(folder))));
    }
    folders.sort();
    folders.dedup();
    let stale = |(at, folder): &(usize, Cow<'_, str>)| {
      let seen = self.machines.get(*at).and_then(|told| told.changes.get(&**folder));
      seen.is_none_or(|one| !one.fresh) && !asking(*at, folder)
    };
    let ask = |(at, folder): (usize, Cow<'_, str>)| {
      let verb = Verb::Changes { cwd: folder.to_string(), path: None };
      (at, verb, Asked::Changes(folder.into_owned(), None))
    };
    let mut due: Vec<_> = folders.into_iter().filter(stale).map(ask).collect();
    due.extend(self.review_due());
    due
  }

  /// The ask of the diff of the file that the review reads, while the client holds none of it.
  fn review_due(&self) -> Option<(usize, Verb, Asked)> {
    let review = self.review.as_ref()?;
    let path = &self.reviewed()?.path;
    let shown = review.diff.as_ref().is_some_and(|one| one.0 == *path);
    let asked = self.asked.values().any(|one| {
      matches!(one, Asked::Changes(folder, Some(asked)) if *folder == review.folder && asked == path)
    });
    (!shown && !asked).then(|| {
      let verb = Verb::Changes { cwd: review.folder.clone(), path: Some(path.clone()) };
      (review.at, verb, Asked::Changes(review.folder.clone(), Some(path.clone())))
    })
  }

  /// The answer of a machine to an ask of changes: the client holds them as fresh, and the review
  /// takes the diff of the file that it reads.
  pub(crate) fn changed(&mut self, at: usize, asked: Asked, answer: Answer) {
    let (Asked::Changes(folder, path), Answer::Changes(mut changes)) = (asked, answer) else {
      return;
    };
    let diff = changes.diff.take();
    if let (Some(path), Some(diff), Some(review)) = (path, diff, self.review.as_mut())
      && review.at == at
      && review.folder == folder
    {
      review.diff = Some((path, diff));
    }
    if let Some(told) = self.machines.get_mut(at) {
      told.changes.insert(folder, Seen { changes, fresh: true });
    }
    self.chose_wanted();
  }

  /// The changes of the project of each card of a life that a frame changed, which the client asks
  /// again.
  pub(crate) fn staled(&mut self, at: usize, frame: &Frame) {
    let Frame::Cards { changed, .. } = frame else { return };
    let Some(told) = self.machines.get_mut(at) else { return };
    for folder in changed.iter().filter_map(|card| card.project.as_ref()) {
      if let Some(seen) = told.changes.get_mut(folder) {
        seen.fresh = false;
      }
    }
  }

  /// The terminal of the client took the focus again: the operator may have changed files with no
  /// life, as in an editor or with git, so the client asks the changes of each folder again.
  pub fn focused(&mut self) {
    self.stale(|_| true);
  }

  /// The changes of each folder that matches asked again. An answer on its way read the folder
  /// before what made it stale, so the client forgets its ask and takes no such answer.
  fn stale(&mut self, matches: impl Fn(&str) -> bool) {
    self.asked.retain(|_, one| !matches!(one, Asked::Changes(folder, None) if matches(folder)));
    for (folder, seen) in self.machines.iter_mut().flat_map(|told| told.changes.iter_mut()) {
      if matches(folder) {
        seen.fresh = false;
      }
    }
  }

  /// The review of the changes of the target of the input opened, from its first file, and asked
  /// again; or closed when it stands.
  pub fn review(&mut self) {
    if self.review.take().is_some() {
      return;
    }
    let Some((at, folder)) = self.target_folder().map(|(at, one)| (at, one.into_owned())) else {
      return;
    };
    self.stale(|one| one == folder);
    self.review = Some(Review { at, folder, ..Review::default() });
  }

  /// The review opened on the file of a place and on its line, as a click on the head of a thread
  /// about lines opens it: the file once the changes of its project came.
  pub fn review_at(&mut self, path: &str, line: u64) {
    if self.review.is_none() {
      self.review();
    }
    if let Some(review) = self.review.as_mut() {
      review.wanted = Some((path.to_owned(), line));
    }
    self.chose_wanted();
  }

  /// The file that the review was opened on chosen, once the changes of its project hold it.
  fn chose_wanted(&mut self) {
    let Some(review) = self.review.as_ref() else { return };
    let Some((path, _)) = review.wanted.as_ref() else { return };
    let files = self.changes(review.at, &review.folder).map(|one| &one.files[..]);
    let Some(at) = files.and_then(|files| files.iter().position(|one| one.path == *path)) else {
      return;
    };
    if let Some(review) = self.review.as_mut() {
      review.chosen = at;
    }
  }

  /// The conversation of a thread about lines folded when it showed, or shown when it was folded.
  pub fn toggle_talk(&mut self, prompt: &str) {
    let Some(review) = self.review.as_mut() else { return };
    match review.toggled.iter().position(|one| one == prompt) {
      Some(at) => drop(review.toggled.remove(at)),
      None => review.toggled.push(prompt.to_owned()),
    }
  }

  /// The file that the review reads, among the changed files of its project.
  pub fn reviewed(&self) -> Option<&Changed> {
    let review = self.review.as_ref()?;
    self.changes(review.at, &review.folder)?.files.get(review.chosen)
  }

  /// The file that the review reads moved by a count of files.
  pub fn review_by(&mut self, by: isize) {
    let count = self
      .review
      .as_ref()
      .and_then(|one| self.changes(one.at, &one.folder))
      .map_or(0, |one| one.files.len());
    if let Some(review) = self.review.as_mut() {
      review.chosen = review.chosen.saturating_add_signed(by).min(count.saturating_sub(1));
    }
  }
}
