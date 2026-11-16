//! The follow-ups that the operator queues on a thread while its chain works: the life sends each
//! as a message of the thread once the chain rests, and each window of the life shows it until it
//! goes.

use furb_wire::{
  Event, FollowUp,
  protocol::{Answer, Frame, Verb},
};

use crate::{Asked, Client, Target};

impl Client {
  /// The text of the input queued on the open thread, which its life sends once the chain of the
  /// thread rests, or why it does not go: a follow-up is a message of a thread, with no image.
  pub fn queue(&mut self, text: &str) -> Result<(), String> {
    let Target::Thread(key) = self.target() else {
      return Err("A follow-up waits on a thread: open one first.".to_owned());
    };
    if self.draft.python {
      return Err("A word of Python runs now: send it with Enter.".to_owned());
    }
    if !self.draft.images.is_empty() {
      return Err("A message with an image goes now: send it with Enter.".to_owned());
    }
    let (at, life, thread) = key;
    let text = self.unfolded(text.to_owned());
    self.draft.folds.clear();
    // The client holds the life, so its machine pushes each change of the queue.
    self.hold((at, life.clone()), self.cwd(at, &life), self.moved);
    let entry = format!("{}-{}", self.machines[at].login, self.now);
    let message = text.trim().to_owned();
    let on = Some(thread.clone());
    let verb = Verb::Queue { life: life.clone(), entry, shape: "None".to_owned(), message, on };
    self.controls.push((at, verb));
    self.following = Some((at, life, thread));
    Ok(())
  }

  /// The follow-ups that wait on the open thread, in the order that the operator queued them.
  pub fn follow_ups(&self) -> Vec<&FollowUp> {
    let Some(opened) = self.thread.as_ref() else { return Vec::new() };
    let (at, life, thread) = &opened.thread.key;
    let queued = self.machines.get(*at).and_then(|told| told.queued.get(life));
    let waiting = queued.into_iter().flat_map(|chains| chains.values().flatten());
    waiting.filter(|one| one.thread.as_ref() == Some(thread)).collect()
  }

  /// A follow-up taken out of its queue, by its entry: its text comes back to the input.
  pub fn unqueue(&mut self, entry: &str) -> Option<String> {
    let opened = self.thread.as_ref()?;
    let (at, life) = (opened.thread.key.0, opened.thread.key.1.clone());
    let text = self.follow_ups().into_iter().find(|one| one.entry == entry)?.message.clone();
    self.controls.push((at, Verb::Unqueue { life: life.clone(), entry: entry.to_owned() }));
    self.following = Some((at, life, opened.thread.key.2.clone()));
    Some(text)
  }

  /// The follow-ups of the open thread asked again while some wait, as a change of its page may
  /// be the send of one: a window that holds no life of the thread hears no change of its queue.
  pub fn refollow(&mut self) {
    if self.follow_ups().is_empty() {
      return;
    }
    let Some(opened) = self.thread.as_ref() else { return };
    let (at, life, thread) = opened.thread.key.clone();
    self.following = Some((at, life, thread));
  }

  /// The ask of the follow-ups of a thread after a change of its queue that the client asked,
  /// which goes after that change.
  pub(crate) fn follows_due(&mut self) -> Option<(usize, Verb, Asked)> {
    let (at, life, thread) = self.following.take()?;
    let verb = Verb::Queued { life: life.clone(), on: Some(thread.clone()) };
    Some((at, verb, Asked::Queued(life, thread)))
  }

  /// What a machine said to the ask of the follow-ups of a thread: they take the place of those
  /// that the client held for that thread. A refusal changes nothing.
  pub fn followed(&mut self, at: usize, (life, thread): (String, String), heard: Option<Frame>) {
    let Some(Frame::Reply { answer: Answer::Values(values), .. }) = heard else { return };
    let got = values.into_iter().filter_map(|one| serde_json::from_value::<FollowUp>(one).ok());
    let Some(told) = self.machines.get_mut(at) else { return };
    let chains = told.queued.entry(life).or_default();
    for waiting in chains.values_mut() {
      waiting.retain(|one| one.thread.as_ref() != Some(&thread));
    }
    for one in got {
      chains.entry(one.chain.clone()).or_default().push(one);
    }
  }

  /// The follow-ups that wait on a chain of a life, as its machine pushed them.
  pub(crate) fn queued(&mut self, at: usize, life: String, event: &Event) {
    let (Event::Queued { chain, waiting }, Some(told)) = (event, self.machines.get_mut(at)) else {
      return;
    };
    told.queued.entry(life).or_default().insert(chain.clone(), waiting.clone());
  }
}
