//! The whole of what a command wrote, which the window opens under its line once the operator asks
//! for it: the machine gives the exit of the act, and the client keeps the text of its streams.

use furb_wire::protocol::{Answer, Frame, Verb};
use serde_json::Value;

use crate::{Asked, Client};

/// A command of a life of a machine, by its machine, its life and the name of its act.
pub type Command = (usize, String, String);

/// What the client holds of the whole of what a command wrote: asked of its machine, then its text,
/// or why the machine gave none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Whole {
  Asked,
  Told(Result<String, String>),
}

impl Client {
  /// The whole of what a command wrote, once its machine told it.
  pub fn whole(&self, command: &Command) -> Option<&Result<String, String>> {
    match self.wholes.get(command)? {
      Whole::Told(told) => Some(told),
      Whole::Asked => None,
    }
  }

  /// The commands whose whole the window shows: each that the client holds nothing of is asked
  /// once, and the client forgets each that the window shows no more.
  pub fn show_wholes(&mut self, shown: &[Command]) {
    self.wholes.retain(|command, _| shown.contains(command));
    self.wanted = shown.iter().filter(|one| !self.wholes.contains_key(*one)).cloned().collect();
  }

  /// The read of each command whose whole the window shows and the client holds nothing of.
  pub(crate) fn wholes_due(&mut self) -> Vec<(usize, Verb, Asked)> {
    let wanted = std::mem::take(&mut self.wanted);
    let mut due = Vec::new();
    for (at, life, act) in wanted {
      self.wholes.insert((at, life.clone(), act.clone()), Whole::Asked);
      let verb = Verb::Peek { life: life.clone(), act: act.clone() };
      due.push((at, verb, Asked::Whole((at, life, act))));
    }
    due
  }

  /// What a machine told of the whole of a command: the text of its stdout, then of its stderr.
  pub(crate) fn told_whole(&mut self, command: Command, heard: Option<Frame>) {
    let told = match heard {
      Some(Frame::Reply { answer: Answer::Value(exit), .. }) => streams(&exit)
        .ok_or_else(|| "The command gave no streams, as a command that a later life ends.".into()),
      Some(Frame::Refused { why, .. }) => Err(why),
      _ => Err("Its machine went away.".to_owned()),
    };
    self.wholes.insert(command, Whole::Told(told));
  }
}

/// The text of the two streams of an exit, the stdout first.
fn streams(exit: &Value) -> Option<String> {
  let stream = |name: &str| exit.get(name)?.get("content")?.as_str();
  let (stdout, stderr) = (stream("stdout")?, stream("stderr").unwrap_or_default());
  Some(if stderr.is_empty() { stdout.to_owned() } else { format!("{stdout}\n{stderr}") })
}
