//! The sign-in of Codex, which the operator asks from the models of a session: the account of
//! his `ChatGPT` plan serves the models of `OpenAI`, with no key. The machine of the session runs
//! it and tells each change of its state, and turns Codex on in the project of the session once it
//! ends.

use furb_wire::protocol::{Config, Frame, Verb};
use serde_json::Value;

use crate::{Asked, Client, Target};

/// A sign-in of Codex while it runs: the machine and the record of the life whose project takes
/// Codex, whether the client asked it, whether the operator canceled it and the client asked the
/// cancel, the state that the machine told last, and the address of its page in the browser while
/// it waits for him.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SignIn {
  pub life: (usize, String),
  pub asked: bool,
  pub canceled: Option<bool>,
  pub state: Option<String>,
  pub url: Option<String>,
}

impl Client {
  /// A sign-in of Codex for the session that the input sends to, unless one runs, or why none
  /// starts: a new session has no project that takes Codex yet.
  pub fn sign_in(&mut self) -> Result<(), String> {
    let life = match self.target() {
      Target::Thread(key) => (key.0, key.1),
      Target::Session(at, life) => (at, life),
      Target::New(..) => {
        return Err("Send a first message: Codex connects in the project of a session.".to_owned());
      }
    };
    self.signing.get_or_insert(SignIn { life, ..SignIn::default() });
    Ok(())
  }

  /// The sign-in that runs canceled: the machine refuses the change that asked it, and turns
  /// nothing on.
  pub fn cancel_sign_in(&mut self) {
    if let Some(signing) = self.signing.as_mut().filter(|one| one.asked) {
      signing.canceled.get_or_insert(false);
    }
  }

  /// The change that turns Codex on, which the machine answers once the sign-in ends, or the
  /// cancel of the sign-in.
  pub(crate) fn sign_in_due(&mut self) -> Option<(usize, Verb, Asked)> {
    let signing = self.signing.as_mut()?;
    let (at, life) = (signing.life.0, signing.life.1.clone());
    if !signing.asked {
      signing.asked = true;
      let key = vec!["providers".to_owned(), "codex".to_owned()];
      let config = Config::Set { key, value: Some(Value::Bool(true)), layer: None };
      return Some((at, Verb::Config { life, config }, Asked::SignIn));
    }
    if signing.canceled == Some(false) {
      signing.canceled = Some(true);
      return Some((at, Verb::CancelSignIn { life }, Asked::Control));
    }
    None
  }

  /// A state that a machine told of the sign-in that runs there, with the address of its page
  /// while it waits: the address when it is new, which the client opens once.
  pub fn told_sign_in(&mut self, at: usize, state: String, url: Option<String>) -> Option<String> {
    let signing = self.signing.as_mut().filter(|one| one.life.0 == at)?;
    let new = url.clone().filter(|one| signing.url.as_ref() != Some(one));
    (signing.state, signing.url) = (Some(state), url);
    new
  }

  /// What the machine answered to the change that turned Codex on: the sign-in ended, and the
  /// models of the machine are asked again, those of Codex among them, or the client says why
  /// Codex did not connect.
  pub fn signed_in(&mut self, heard: Option<Frame>) {
    let Some(signing) = self.signing.take() else { return };
    self.notice = Some(match heard {
      Some(Frame::Reply { .. }) => {
        if let Some(told) = self.machines.get_mut(signing.life.0) {
          told.models = None;
        }
        "Codex is connected: the models of your ChatGPT plan are in the list.".to_owned()
      }
      // The machine pushes the state of the sign-in apart from its answer to the change, so the
      // refusal can come before the state that says the cancel.
      Some(Frame::Refused { .. }) if signing.canceled.is_some() => {
        "The sign-in of Codex stopped. Nothing changed.".to_owned()
      }
      Some(Frame::Refused { why, .. }) => format!("Codex did not connect: {why}"),
      _ => "Codex did not connect: its machine went away.".to_owned(),
    });
  }
}
