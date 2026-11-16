//! A name of the code of the open thread that the operator points at, as a debugger shows it: the
//! machine reads it in the module of the chain of the thread, where the code bound it.

use furb_wire::protocol::{Answer, Frame, Verb};

use crate::{Asked, Client, Key};

/// A name that the operator points at in the code of a thread, whether the client asked it of the
/// machine, and what the machine told once it answered: the name of the type of its value and its
/// representation, or why it read nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Inspection {
  pub key: Key,
  pub name: String,
  pub asked: bool,
  pub told: Option<Result<(String, String), String>>,
}

impl Client {
  /// The name that the pointer is on in the code of a thread, which the machine reads once, or no
  /// name. A name that the client holds already keeps what the machine told.
  pub fn inspect(&mut self, pointed: Option<(&Key, &str)>) {
    let held =
      |one: &Inspection| pointed.is_some_and(|(key, name)| one.key == *key && one.name == name);
    if self.inspected.as_ref().is_some_and(held) {
      return;
    }
    self.inspected = pointed.map(|(key, name)| Inspection {
      key: key.clone(),
      name: name.to_owned(),
      ..Inspection::default()
    });
  }

  /// The read of the name that the pointer is on, on the chain of its thread, which the prompt of
  /// the thread names.
  pub(crate) fn inspect_due(&mut self) -> Option<(usize, Verb, Asked)> {
    let one = self.inspected.as_mut().filter(|one| !one.asked)?;
    one.asked = true;
    let (at, life, on) = one.key.clone();
    let verb = Verb::Inspect { life, name: one.name.clone(), on: Some(on) };
    Some((at, verb, Asked::Inspect(one.key.clone(), one.name.clone())))
  }

  /// What the machine told of a name, while the pointer is still on it.
  pub fn told_inspected(&mut self, (key, name): (&Key, &str), heard: Option<Frame>) {
    let Some(one) = self.inspected.as_mut().filter(|one| one.key == *key && one.name == name)
    else {
      return;
    };
    one.told = Some(match heard {
      Some(Frame::Reply { answer: Answer::Inspected { kind, repr, .. }, .. }) => Ok((kind, repr)),
      Some(Frame::Refused { why, .. }) => Err(why),
      _ => Err("its machine went away".to_owned()),
    });
  }
}
