//! The sessions that run, as the list part shows them: a part of Go to, the choices for one
//! session, and the choices that a leave asks while a session that the window holds lives. Each
//! choice is a row, so each works with the keys of a list and with a click.

use furb_client::{Control, Running, Target};

use crate::{
  input::named,
  list::loader,
  model::{Acts, Model},
  pick::{Choice, Entry, Pick, Picker},
  tokens::WORKS,
};

impl Model {
  /// The part of Go to that names each session that runs, the ones that work first, with what
  /// it does and who holds it.
  pub(crate) fn running_places(&self) -> Vec<Entry> {
    let entry = |one: Running| Entry {
      part: "Running".into(),
      project: one.project.as_deref().map(named),
      right: self.runs(&one),
      left: one.name,
      choice: Choice::Running(one.at, one.life),
      ..Entry::default()
    };
    self.client.running().into_iter().map(entry).collect()
  }

  /// What a session that runs does and who holds it, as `working · here`, `asks you · here` or
  /// `idle · in the background`.
  fn runs(&self, one: &Running) -> String {
    let state = match (one.works(), one.live.is_empty()) {
      (true, _) => "working",
      (false, false) => "asks you",
      (false, true) => "idle",
    };
    let mut holders: Vec<String> = Vec::new();
    if one.here {
      holders.push("here".to_owned());
    }
    let login = self.client.machines.get(one.at).map(|told| told.login.as_str());
    holders.extend(one.viewers.iter().filter(|who| Some(who.as_str()) != login).cloned());
    if one.background {
      holders.push("in the background".to_owned());
    }
    [state.to_owned()].into_iter().chain(holders).collect::<Vec<_>>().join(" · ")
  }

  /// The choices for the session that the list part acts on: open it, keep it running in the
  /// background while no pin keeps it, pause it, or kill it.
  pub(crate) fn session_choices(&self) -> Vec<Entry> {
    let Some(Acts::On(at, life)) = &self.acts else { return Vec::new() };
    let one = self.client.running().into_iter().find(|one| one.at == *at && one.life == *life);
    let row = |left: &str, choice| Entry { left: left.to_owned(), choice, ..Entry::default() };
    let mut all = vec![row("Open it", Choice::Session(Target::Session(*at, life.clone())))];
    if one.as_ref().is_some_and(|one| !one.background) {
      all.push(row("Keep it running in the background", Choice::Control(Control::Background)));
    }
    if one.is_some() {
      all.push(row("Pause it: its work goes on at its next open", Choice::Control(Control::Pause)));
      all.push(row("Kill it: its work stops", Choice::Control(Control::Kill)));
    }
    all
  }

  /// The choices of a leave while sessions that the window holds live: each thread that lives
  /// in them, with whether it works or asks him, then keep them running in the background, pause
  /// them, kill them, or stay. The words take the number of the threads.
  pub(crate) fn leave_choices(&self) -> Vec<Entry> {
    let mut all: Vec<Entry> = Vec::new();
    for one in self.client.leaving() {
      let project = one.project.as_deref().map(named);
      all.extend(one.live.into_iter().map(|thread| Entry {
        part: "Still live".into(),
        project: project.clone(),
        // A thread says its state as its card does: its mark, its word and its color.
        right: loader(&thread.standing).map_or_else(
          || "◆ Input".to_owned(),
          |loader| format!("{} {WORKS}", loader.frame(self.client.moved)),
        ),
        tone: Some(if thread.standing.works() {
          self.tokens.accent
        } else {
          self.tokens.attention
        }),
        left: thread.title,
        ..Entry::default()
      }));
    }
    let (them, their) = if all.len() == 1 { ("it", "its") } else { ("them", "their") };
    let row = |left: String, choice| Entry {
      part: "Leave and".into(),
      left,
      choice: Choice::Leave(choice),
      ..Entry::default()
    };
    all.push(row(format!("keep {them} running in the background"), Some(Control::Background)));
    let pause = format!("pause {them}: {their} work goes on at {their} next open");
    all.push(row(pause, Some(Control::Pause)));
    all.push(row(format!("kill {them}: {their} work stops"), Some(Control::Kill)));
    all.push(Entry { part: "Or".into(), ..row("Stay in the window".to_owned(), None) });
    all
  }

  /// The choices for the session of the open thread, or for a session of Go to.
  pub(crate) fn act_on(&mut self, at: usize, life: String) {
    self.acts = Some(Acts::On(at, life));
    self.picker = Some(Picker { of: Pick::Session, filter: String::new(), at: 0, effort: None });
  }

  /// The choices for the session of the open thread, while one is open.
  pub(crate) fn act_on_open(&mut self) {
    if let Some(key) = self.client.thread.as_ref().map(|one| one.thread.key.clone()) {
      self.act_on(key.0, key.1);
    }
  }

  /// A choice for the session that the list part acts on, which goes to its machine.
  pub(crate) fn controlled(&mut self, control: Control) {
    if let Some(Acts::On(at, life)) = self.acts.take() {
      self.client.control(at, &life, control);
    }
  }

  /// A choice of a leave: each session that works takes it and the window leaves once each went,
  /// or he stays.
  pub(crate) fn left_with(&mut self, control: Option<Control>) {
    let Some(control) = control else { return };
    for one in self.client.leaving() {
      self.client.control(one.at, &one.life, control);
    }
    self.acts = Some(Acts::Leaving);
  }

  /// A second leave: the window leaves at once while no session that it holds lives, else the
  /// list part asks what each of them does, on its first choice under the threads, and the leave
  /// is armed no more. Whether it leaves now.
  pub(crate) fn leaves_now(&mut self) -> bool {
    if self.client.leaving().is_empty() {
      return true;
    }
    let first = self.leave_choices().iter().position(|one| matches!(one.choice, Choice::Leave(_)));
    self.armed = None;
    self.picker =
      Some(Picker { of: Pick::Leave, filter: String::new(), at: first.unwrap_or(0), effort: None });
    false
  }
}
